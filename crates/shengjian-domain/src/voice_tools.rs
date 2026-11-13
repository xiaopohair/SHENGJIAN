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
    /// `wechat` | `doubao` | `vokie` | `custom` (behavior adapter).
    pub kind: String,
    /// `KeyboardEvent.code` sequence used as the default push-to-talk hotkey.
    #[serde(default)]
    pub default_hotkey: Vec<String>,
}

static CATALOG: OnceLock<RwLock<Vec<VoiceToolConfig>>> = OnceLock::new();

const TOOL_KINDS: [&str; 4] = ["wechat", "doubao", "vokie", "custom"];

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
        assert_eq!(ids, vec!["wechat", "doubao", "vokie", "other"]);
        assert_eq!(tools[0].default_hotkey, vec!["ControlLeft", "MetaLeft"]);
        assert_eq!(tools[1].default_hotkey, vec!["AltRight"]);
        assert_eq!(tools[3].kind, "custom");
    }

    #[test]
    fn invalid_catalogs_are_rejected() {
        assert!(parse_catalog("{}").is_err());
        assert!(parse_catalog(r#"{"tools":[{"id":"a","label":"A","kind":"unknown"}]}"#).is_err());
        assert!(parse_catalog(
            r#"{"tools":[{"id":"a","label":"A","kind":"custom"},{"id":"a","label":"B","kind":"custom"}]}"#
        )
        .is_err());
    }
}
