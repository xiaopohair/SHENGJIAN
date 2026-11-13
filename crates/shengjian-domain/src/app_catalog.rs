//! Bundled, editable application preset catalog.
//!
//! The authoritative data lives in `config/app-presets.json` at the repository root
//! (copied into the installer resources by `scripts/prepare-config.ps1`). The same
//! file is embedded here as a fallback so tests and `tauri dev` keep working when
//! the packaged copy is missing.
//!
//! Consumers: `shengjian-windows` (launch/probe/focus/readiness/AUMID) and
//! `portable_macros` (macOS bundle id for portable export/import).

use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};

/// Embedded fallback copy of `config/app-presets.json`.
pub const DEFAULT_APP_PRESETS_JSON: &str = include_str!("../../../config/app-presets.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPresetCatalog {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub apps: Vec<AppPreset>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub exe_names: Vec<String>,
    #[serde(default)]
    pub install_paths: Vec<String>,
    #[serde(default)]
    pub shortcut_names: Vec<String>,
    #[serde(default)]
    pub relaunch_when_hidden: bool,
    /// Legacy entries stay resolvable for existing mappings but are hidden from lists.
    #[serde(default)]
    pub hidden: bool,
    /// 遥控器自身的宿主进程（激活失败时用自身 exe 重新拉起）。
    #[serde(default)]
    pub self_app: bool,
    /// 恒视为已安装（遥控器自身；映射运行时它必然在运行）。
    #[serde(default)]
    pub always_installed: bool,
    /// Packaged (UWP/MSIX) app id for `shell:AppsFolder\<aumid>` activation.
    #[serde(default)]
    pub aumid: Option<String>,
    #[serde(default)]
    pub focus: Option<AppFocusConfig>,
    /// macOS bundle id used by the portable macro format.
    #[serde(default)]
    pub portable_bundle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppFocusConfig {
    /// `appShortcut` | `autoComposer` | `openOnly`.
    pub strategy: String,
    /// `KeyCode` serde names, e.g. `left_control`, `l`.
    #[serde(default)]
    pub shortcut: Vec<String>,
    #[serde(default)]
    pub click: Option<FocusClickPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusClickPoint {
    pub x: f64,
    pub y: f64,
}

static CATALOG: OnceLock<RwLock<Vec<AppPreset>>> = OnceLock::new();

const FOCUS_STRATEGIES: [&str; 3] = ["appShortcut", "autoComposer", "openOnly"];

fn catalog_slot() -> &'static RwLock<Vec<AppPreset>> {
    CATALOG.get_or_init(|| {
        let parsed = serde_json::from_str::<AppPresetCatalog>(DEFAULT_APP_PRESETS_JSON)
            .map(|catalog| catalog.apps)
            .unwrap_or_default();
        RwLock::new(parsed)
    })
}

fn validate(apps: &[AppPreset]) -> Result<(), String> {
    if apps.is_empty() {
        return Err("app preset catalog is empty".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    for app in apps {
        if app.id.trim().is_empty() || app.name.trim().is_empty() {
            return Err("app preset entries need a non-empty id and name".to_owned());
        }
        if !seen.insert(app.id.to_ascii_lowercase()) {
            return Err(format!("duplicate app preset id: {}", app.id));
        }
        if let Some(focus) = &app.focus {
            if !FOCUS_STRATEGIES.contains(&focus.strategy.as_str()) {
                return Err(format!(
                    "unsupported focus strategy for {}: {}",
                    app.id, focus.strategy
                ));
            }
        }
    }
    Ok(())
}

fn parse_catalog(json: &str) -> Result<Vec<AppPreset>, String> {
    let catalog: AppPresetCatalog =
        serde_json::from_str(json).map_err(|error| format!("invalid app preset json: {error}"))?;
    validate(&catalog.apps)?;
    Ok(catalog.apps)
}

/// Replace the runtime catalog from JSON (editable `config/app-presets.json`).
pub fn install_from_json(json: &str) -> Result<(), String> {
    let apps = parse_catalog(json)?;
    let mut slot = catalog_slot()
        .write()
        .map_err(|_| "app preset catalog lock poisoned".to_owned())?;
    *slot = apps;
    Ok(())
}

/// Install the embedded fallback copy (used by dev runs and tests).
pub fn install_defaults() -> Result<(), String> {
    install_from_json(DEFAULT_APP_PRESETS_JSON)
}

fn snapshot() -> Vec<AppPreset> {
    catalog_slot()
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// All entries in file order (including legacy hidden ones).
pub fn apps() -> Vec<AppPreset> {
    snapshot()
}

/// Entries shown in the picker's 推荐 list (file order, `hidden == false`).
pub fn visible_apps() -> Vec<AppPreset> {
    snapshot().into_iter().filter(|app| !app.hidden).collect()
}

pub fn preset(id: &str) -> Option<AppPreset> {
    snapshot().into_iter().find(|app| app.id == id)
}

pub fn aumid(id: &str) -> Option<String> {
    preset(id).and_then(|app| app.aumid)
}

/// Focus configuration; entries without one use the product default (`autoComposer`).
pub fn focus(id: &str) -> AppFocusConfig {
    preset(id)
        .and_then(|app| app.focus)
        .unwrap_or_else(|| AppFocusConfig {
            strategy: "autoComposer".to_owned(),
            shortcut: Vec::new(),
            click: None,
        })
}

pub fn preset_portable_bundle(id: &str) -> Option<String> {
    preset(id).and_then(|app| app.portable_bundle)
}

pub fn preset_for_portable_bundle(bundle: &str) -> Option<String> {
    snapshot()
        .into_iter()
        .find(|app| app.portable_bundle.as_deref() == Some(bundle))
        .map(|app| app.id)
}

pub fn default_catalog_json() -> &'static str {
    DEFAULT_APP_PRESETS_JSON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_keeps_the_published_order_and_hidden_entries() {
        let apps = parse_catalog(DEFAULT_APP_PRESETS_JSON).expect("embedded catalog parses");
        let visible: Vec<&str> = apps
            .iter()
            .filter(|app| !app.hidden)
            .map(|app| app.id.as_str())
            .collect();
        assert_eq!(
            visible,
            vec![
                "shengjian",
                "wechat",
                "netease_music",
                "chrome",
                "chatgpt",
                "workbuddy",
                "dimagent",
                "vokie",
                "notepad",
                "qq"
            ]
        );
        assert_eq!(apps.len(), 17);
        assert!(apps.iter().any(|app| app.id == "calc" && app.hidden));
        assert!(apps.iter().any(|app| app.id == "edge" && app.hidden));
    }

    #[test]
    fn focus_metadata_is_data_driven_and_defaults_to_auto_composer() {
        let apps = parse_catalog(DEFAULT_APP_PRESETS_JSON).expect("catalog parses");
        let chrome = apps.iter().find(|app| app.id == "chrome").unwrap();
        let focus = chrome.focus.as_ref().expect("chrome focus");
        assert_eq!(focus.strategy, "appShortcut");
        assert_eq!(focus.shortcut, vec!["left_control", "l"]);

        let netease = apps
            .iter()
            .find(|app| app.id == "netease_music")
            .and_then(|app| app.focus.as_ref())
            .and_then(|focus| focus.click)
            .expect("netease click point");
        assert_eq!((netease.x, netease.y), (410.0, 37.0));

        assert!(apps
            .iter()
            .find(|app| app.id == "wechat")
            .is_some_and(|app| app.focus.is_none()));
    }

    #[test]
    fn aumid_and_portable_bundles_live_in_the_catalog() {
        let apps = parse_catalog(DEFAULT_APP_PRESETS_JSON).expect("catalog parses");
        let calc = apps.iter().find(|app| app.id == "calc").unwrap();
        assert_eq!(
            calc.aumid.as_deref(),
            Some("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App")
        );
        let wechat = apps.iter().find(|app| app.id == "wechat").unwrap();
        assert_eq!(
            wechat.portable_bundle.as_deref(),
            Some("com.tencent.xinWeChat")
        );
    }

    #[test]
    fn invalid_catalogs_are_rejected() {
        assert!(parse_catalog("not json").is_err());
        assert!(parse_catalog(r#"{"apps":[]}"#).is_err());
        assert!(
            parse_catalog(r#"{"apps":[{"id":"a","name":"A"},{"id":"a","name":"B"}]}"#).is_err()
        );
        assert!(
            parse_catalog(r#"{"apps":[{"id":"a","name":"A","focus":{"strategy":"bogus"}}]}"#)
                .is_err()
        );
    }
}
