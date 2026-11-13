//! Bundled, editable voice-input tool catalog.
//!
//! The authoritative data lives in `config/voice-tools.json` at the repository root
//! (copied into the installer resources by `scripts/prepare-config.ps1`). The same
//! file is embedded here as a fallback for tests and `tauri dev`.
//!
//! The catalog decides the user-visible list, the display label and the default
//! hotkey. Windows-specific behavior (IME healing, BLE injection, status reporting)
//! stays in code and is selected by the `kind` field:
//! `wechat` / `doubao` / `vokie` / `custom`.

use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};

/// Embedded fallback copy of `config/voice-tools.json`.
pub const DEFAULT_VOICE_TOOLS_JSON: &str = include_str!("../../../config/voice-tools.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceToolCatalog {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub tools: Vec<VoiceToolConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceToolConfig {
    pub id: String,
    pub label: String,
    /// `wechat` | `doubao` | `vokie` | `chatterfly` | `custom` (behavior adapter).
    pub kind: String,
    /// `KeyboardEvent.code` sequence used as the default push-to-talk hotkey.
    #[serde(default)]
    pub default_hotkey: Vec<String>,
    /// 可选：该工具本身是 TSF 输入法时的身份（固定 GUID 或按显示名发现）。
    #[serde(default)]
    pub ime: Option<VoiceToolImeConfig>,
}

/// 一个输入法工具的 TSF 身份。
///
/// 优先级：`clsid` + `profile` 固定 GUID；固定配置缺失/失效时用 `names`
/// 按显示名在已安装输入法里自动发现（只匹配简体中文键盘配置）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VoiceToolImeConfig {
    #[serde(default)]
    pub clsid: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    /// 输入法显示名（大小写/空白不敏感；用于自动发现）。
    #[serde(default)]
    pub names: Vec<String>,
}

impl VoiceToolImeConfig {
    /// 是否配置了完整的固定 GUID 对。
    pub fn fixed_profile(&self) -> Option<(&str, &str)> {
        let clsid = self
            .clsid
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())?;
        let profile = self
            .profile
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())?;
        Some((clsid, profile))
    }

    /// 是否配置了可用于自动发现的显示名。
    pub fn discovery_names(&self) -> Vec<String> {
        self.names
            .iter()
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect()
    }
}

static CATALOG: OnceLock<RwLock<Vec<VoiceToolConfig>>> = OnceLock::new();

const TOOL_KINDS: [&str; 5] = ["wechat", "doubao", "vokie", "chatterfly", "custom"];

fn catalog_slot() -> &'static RwLock<Vec<VoiceToolConfig>> {
    CATALOG.get_or_init(|| {
        let parsed = serde_json::from_str::<VoiceToolCatalog>(DEFAULT_VOICE_TOOLS_JSON)
            .map(|catalog| catalog.tools)
            .unwrap_or_default();
        RwLock::new(parsed)
    })
}

fn validate(tools: &[VoiceToolConfig]) -> Result<(), String> {
    if tools.is_empty() {
        return Err("voice tool catalog is empty".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    for tool in tools {
        if tool.id.trim().is_empty() || tool.label.trim().is_empty() {
            return Err("voice tool entries need a non-empty id and label".to_owned());
        }
        if !seen.insert(tool.id.to_ascii_lowercase()) {
            return Err(format!("duplicate voice tool id: {}", tool.id));
        }
        if !TOOL_KINDS.contains(&tool.kind.as_str()) {
            return Err(format!("unsupported voice tool kind: {}", tool.kind));
        }
        if let Some(ime) = &tool.ime {
            if ime.fixed_profile().is_none() && ime.discovery_names().is_empty() {
                return Err(format!(
                    "voice tool {} ime config needs clsid+profile or names",
                    tool.id
                ));
            }
        }
    }
    Ok(())
}

fn parse_catalog(json: &str) -> Result<Vec<VoiceToolConfig>, String> {
    let catalog: VoiceToolCatalog =
        serde_json::from_str(json).map_err(|error| format!("invalid voice tool json: {error}"))?;
    validate(&catalog.tools)?;
    Ok(catalog.tools)
}

/// Replace the runtime catalog from JSON (editable `config/voice-tools.json`).
pub fn install_from_json(json: &str) -> Result<(), String> {
    let tools = parse_catalog(json)?;
    let mut slot = catalog_slot()
        .write()
        .map_err(|_| "voice tool catalog lock poisoned".to_owned())?;
    *slot = tools;
    Ok(())
}

/// Install the embedded fallback copy (used by dev runs and tests).
pub fn install_defaults() -> Result<(), String> {
    install_from_json(DEFAULT_VOICE_TOOLS_JSON)
}

pub fn tools() -> Vec<VoiceToolConfig> {
    catalog_slot()
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

pub fn tool(id: &str) -> Option<VoiceToolConfig> {
    tools().into_iter().find(|tool| tool.id == id)
}

pub fn kind(id: &str) -> Option<String> {
    tool(id).map(|tool| tool.kind)
}

pub fn default_hotkey(id: &str) -> Vec<String> {
    tool(id).map(|tool| tool.default_hotkey).unwrap_or_default()
}

/// 某个行为 kind 的 TSF 身份配置（同一 kind 的多个工具取第一条）。
pub fn ime_config(kind: &str) -> Option<VoiceToolImeConfig> {
    tools()
        .into_iter()
        .find(|tool| tool.kind == kind)
        .and_then(|tool| tool.ime)
}

pub fn default_catalog_json() -> &'static str {
    DEFAULT_VOICE_TOOLS_JSON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_keeps_the_four_published_tools_and_hotkeys() {
        let tools = parse_catalog(DEFAULT_VOICE_TOOLS_JSON).expect("embedded catalog parses");
        let ids: Vec<&str> = tools.iter().map(|tool| tool.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["wechat", "doubao", "vokie", "chatterfly", "other"]
        );
        assert_eq!(tools[0].default_hotkey, vec!["ControlLeft", "MetaLeft"]);
        assert_eq!(tools[1].default_hotkey, vec!["AltRight"]);
        assert_eq!(tools[3].kind, "chatterfly");
        assert_eq!(tools[3].default_hotkey, vec!["ControlLeft", "MetaLeft"]);
        let ime = tools[3].ime.as_ref().expect("chatterfly ime identity");
        assert!(
            ime.fixed_profile().is_some(),
            "chatterfly should ship fixed TSF GUIDs"
        );
        assert!(ime
            .discovery_names()
            .iter()
            .any(|name| name == "Chatterfly"));
        assert_eq!(tools[4].kind, "custom");
    }

    #[test]
    fn invalid_catalogs_are_rejected() {
        assert!(parse_catalog("{}").is_err());
        assert!(parse_catalog(r#"{"tools":[{"id":"a","label":"A","kind":"unknown"}]}"#).is_err());
        assert!(parse_catalog(
            r#"{"tools":[{"id":"a","label":"A","kind":"custom"},{"id":"a","label":"B","kind":"custom"}]}"#
        )
        .is_err());
        // ime 身份既没有固定 GUID 对、也没有发现名 → 拒绝。
        assert!(parse_catalog(
            r#"{"tools":[{"id":"a","label":"A","kind":"chatterfly","ime":{}}]}"#
        )
        .is_err());
        // 只有 half 的 GUID 对 + 没有名字 → 拒绝。
        assert!(parse_catalog(
            r#"{"tools":[{"id":"a","label":"A","kind":"chatterfly","ime":{"clsid":"604A99E3-6D90-4571-824D-2639BD572F6C"}}]}"#
        )
        .is_err());
    }
}
