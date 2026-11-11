use crate::{parse_flag, require_flag};
use serde::de::DeserializeOwned;
use shengjian_app::{SettingsDocument, SettingsStorePort};
use shengjian_contract::{ButtonMappingSet, ButtonProfilesDocument};
use shengjian_domain::AppSettings;
use shengjian_windows::send_input::ButtonMappings;
use shengjian_windows_adapter::{
    migrate_button_mappings, migrate_button_profiles_document, LegacyProfiles, WindowsSettingsStore,
};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(require_flag(args, "--out-dir")?);
    fs::create_dir_all(&out_dir)?;
    let mut converted = 0_u32;

    if let Some(path) = parse_flag(args, "--legacy-settings") {
        let legacy: AppSettings = read_json(&path)?;
        let document = SettingsDocument::from_flat(legacy).normalized();
        let store = WindowsSettingsStore::new(out_dir.join("settings.json"));
        store.save_document(&document)?;
        println!("settings -> {}", out_dir.join("settings.json").display());
        converted += 1;
    }

    if let Some(path) = parse_flag(args, "--legacy-mappings") {
        let legacy: ButtonMappings = read_json(&path)?;
        let migrated: ButtonMappingSet = migrate_button_mappings(&legacy);
        write_json(&out_dir.join("button-mappings.json"), &migrated)?;
        println!(
            "mappings -> {}",
            out_dir.join("button-mappings.json").display()
        );
        converted += 1;
    }

    if let Some(path) = parse_flag(args, "--legacy-profiles") {
        let legacy: LegacyProfiles = read_json(&path)?;
        let migrated: ButtonProfilesDocument = migrate_button_profiles_document(&legacy);
        write_json(&out_dir.join("button-profiles.json"), &migrated)?;
        println!(
            "profiles -> {}",
            out_dir.join("button-profiles.json").display()
        );
        converted += 1;
    }

    if converted == 0 {
        return Err("no legacy input specified".into());
    }
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &str) -> Result<T, Box<dyn Error>> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), Box<dyn Error>> {
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shengjian_windows::send_input::ButtonMappings;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "shengjian-windows-host-migrate-test-{}",
            std::process::id()
        ))
    }

    #[test]
    fn migrates_legacy_inputs_to_core_documents() {
        let dir = temp_dir();
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        let legacy_settings = dir.join("legacy-settings.json");
        fs::write(
            &legacy_settings,
            serde_json::to_string(&AppSettings::default()).expect("settings json"),
        )
        .expect("write settings");
        let legacy_mappings = dir.join("legacy-mappings.json");
        fs::write(
            &legacy_mappings,
            serde_json::to_string(&ButtonMappings::default()).expect("mappings json"),
        )
        .expect("write mappings");
        let legacy_profiles = dir.join("legacy-profiles.json");
        fs::write(
            &legacy_profiles,
            serde_json::to_string(&LegacyProfiles::default()).expect("profiles json"),
        )
        .expect("write profiles");

        let out = dir.join("out");
        run(&[
            "migrate".to_owned(),
            "--out-dir".to_owned(),
            out.to_string_lossy().into_owned(),
            "--legacy-settings".to_owned(),
            legacy_settings.to_string_lossy().into_owned(),
            "--legacy-mappings".to_owned(),
            legacy_mappings.to_string_lossy().into_owned(),
            "--legacy-profiles".to_owned(),
            legacy_profiles.to_string_lossy().into_owned(),
        ])
        .expect("migrate");

        let settings: SettingsDocument =
            serde_json::from_str(&fs::read_to_string(out.join("settings.json")).expect("settings"))
                .expect("core settings");
        assert_eq!(settings.schema_version, 1);
        let mappings: ButtonMappingSet = serde_json::from_str(
            &fs::read_to_string(out.join("button-mappings.json")).expect("mappings"),
        )
        .expect("core mappings");
        assert!(mappings.actions.is_empty());
        let profiles: ButtonProfilesDocument = serde_json::from_str(
            &fs::read_to_string(out.join("button-profiles.json")).expect("profiles"),
        )
        .expect("core profiles");
        // Normalization always adds the global profile.
        assert_eq!(profiles.profiles.len(), 1);
        assert_eq!(profiles.profiles[0].id, "global");
        let _ = fs::remove_dir_all(dir);
    }
}
