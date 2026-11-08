//! Host-side macro library file store.
//!
//! The core domain owns schema/validation; this module only handles the
//! desktop file (`%LOCALAPPDATA%\Shengjian\macros.json`), backups and
//! atomic replacement.

use shengjian_domain::{validate_library, MacroLibraryDocument, MACRO_LIBRARY_FORMAT_VERSION};
use std::path::{Path, PathBuf};

const MAX_LIBRARY_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug)]
pub struct MacroStoreError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
}

impl MacroStoreError {
    fn new(code: &'static str, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }
}

pub fn default_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Shengjian")
        .join("macros.json")
}

/// Load a library. Missing file means an empty library; malformed content is
/// backed up next to the original and returned as an error.
pub fn load(path: &Path) -> Result<MacroLibraryDocument, MacroStoreError> {
    if !path.exists() {
        return Ok(MacroLibraryDocument {
            format_version: MACRO_LIBRARY_FORMAT_VERSION,
            definitions: Vec::new(),
        });
    }
    let metadata = std::fs::metadata(path)
        .map_err(|_| MacroStoreError::new("macro-store-read", "cannot stat macros file", true))?;
    if metadata.len() > MAX_LIBRARY_BYTES {
        backup(path);
        return Err(MacroStoreError::new(
            "macro-store-too-large",
            "macros file is too large",
            false,
        ));
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|_| MacroStoreError::new("macro-store-read", "cannot read macros file", true))?;
    let document = serde_json::from_str::<MacroLibraryDocument>(&contents).map_err(|_| {
        backup(path);
        MacroStoreError::new(
            "macro-store-invalid",
            "macros file is not valid JSON",
            false,
        )
    })?;
    let document = document.normalized();
    if let Err(issues) = validate_library(&document) {
        backup(path);
        return Err(MacroStoreError::new(
            "macro-invalid",
            issue_message(&issues),
            false,
        ));
    }
    Ok(document)
}

/// Validate and persist a library, then return the normalized document.
pub fn save(
    path: &Path,
    document: &MacroLibraryDocument,
) -> Result<MacroLibraryDocument, MacroStoreError> {
    let document = document.clone().normalized();
    if let Err(issues) = validate_library(&document) {
        return Err(MacroStoreError::new(
            "macro-invalid",
            issue_message(&issues),
            false,
        ));
    }
    let encoded = serde_json::to_string_pretty(&document).map_err(|_| {
        MacroStoreError::new("macro-store-encode", "cannot encode macros file", false)
    })?;
    if encoded.len() as u64 > MAX_LIBRARY_BYTES {
        return Err(MacroStoreError::new(
            "macro-store-too-large",
            "macros file is too large",
            false,
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| {
            MacroStoreError::new("macro-store-write", "cannot create macros folder", true)
        })?;
    }
    let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
    std::fs::write(&temp, encoded)
        .map_err(|_| MacroStoreError::new("macro-store-write", "cannot write macros file", true))?;
    if path.exists() {
        let backup = path.with_extension("json.bak");
        let _ = std::fs::copy(path, backup);
    }
    std::fs::rename(&temp, path).map_err(|_| {
        MacroStoreError::new("macro-store-write", "cannot replace macros file", true)
    })?;
    Ok(document)
}

fn issue_message(issues: &[shengjian_domain::MacroIssue]) -> String {
    let codes: Vec<String> = issues.iter().take(8).map(|issue| issue.code()).collect();
    format!("macro validation failed: {}", codes.join(", "))
}

fn backup(path: &Path) {
    let backup = path.with_extension("json.bak");
    let _ = std::fs::copy(path, backup);
}

#[cfg(test)]
mod tests {
    use super::*;
    use shengjian_domain::{MacroDefinition, MacroStep};

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "shengjian-macro-store-{}-{name}.json",
            std::process::id()
        ))
    }

    fn definition() -> MacroDefinition {
        MacroDefinition {
            schema_version: shengjian_domain::MACRO_SCHEMA_VERSION.to_owned(),
            macro_id: "macro-12345678".to_owned(),
            version: "1.0.0".to_owned(),
            name: "Test".to_owned(),
            summary: None,
            steps: vec![MacroStep {
                step_id: "step-1".to_owned(),
                action: shengjian_domain::MacroAction::FocusFrontmost,
                timeout_ms: None,
                parameters: Default::default(),
            }],
        }
    }

    #[test]
    fn missing_file_is_empty_library() {
        let path = temp_path("missing");
        let _ = std::fs::remove_file(&path);
        let document = load(&path).expect("load");
        assert!(document.definitions.is_empty());
        assert_eq!(document.format_version, MACRO_LIBRARY_FORMAT_VERSION);
    }

    #[test]
    fn round_trip_and_backup() {
        let path = temp_path("roundtrip");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("json.bak"));
        let document = MacroLibraryDocument {
            format_version: MACRO_LIBRARY_FORMAT_VERSION,
            definitions: vec![definition()],
        };
        save(&path, &document).expect("save");
        let loaded = load(&path).expect("load");
        assert_eq!(loaded, document);
        std::fs::write(&path, b"not json").expect("write");
        assert!(load(&path).is_err());
        assert!(path.with_extension("json.bak").exists());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("json.bak"));
    }

    #[test]
    fn rejects_invalid_library() {
        let path = temp_path("invalid");
        let document = MacroLibraryDocument {
            format_version: MACRO_LIBRARY_FORMAT_VERSION,
            definitions: vec![MacroDefinition {
                name: "".to_owned(),
                ..definition()
            }],
        };
        assert!(save(&path, &document).is_err());
    }
}
