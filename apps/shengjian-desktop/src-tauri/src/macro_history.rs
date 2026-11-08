use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const HISTORY_LIMIT: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroHistoryEntry {
    pub macro_id: String,
    pub name: String,
    pub result: String,
    pub elapsed_ms: u64,
    pub at_unix: u64,
    pub steps: usize,
}

fn history_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Shengjian")
        .join("macro-history.json")
}

fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn load_unlocked() -> Vec<MacroHistoryEntry> {
    let path = history_path();
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<MacroHistoryEntry>>(&contents)
        .map(|mut entries| {
            entries.truncate(HISTORY_LIMIT);
            entries
        })
        .unwrap_or_default()
}

fn write_unlocked(entries: &[MacroHistoryEntry]) -> Result<(), String> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let contents = serde_json::to_string_pretty(entries).map_err(|error| error.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents).map_err(|error| error.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|error| error.to_string())
}

pub fn load() -> Result<Vec<MacroHistoryEntry>, String> {
    let _guard = lock().lock().map_err(|error| error.to_string())?;
    Ok(load_unlocked())
}

fn normalize_entry(mut entry: MacroHistoryEntry) -> MacroHistoryEntry {
    if entry.at_unix == 0 {
        entry.at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
    entry.name = entry.name.chars().take(80).collect();
    entry.result = entry.result.chars().take(80).collect();
    entry
}

fn insert_entry(entries: &mut Vec<MacroHistoryEntry>, entry: MacroHistoryEntry) {
    entries.insert(0, entry);
    entries.truncate(HISTORY_LIMIT);
}

pub fn append(entry: MacroHistoryEntry) -> Result<Vec<MacroHistoryEntry>, String> {
    let _guard = lock().lock().map_err(|error| error.to_string())?;
    let entry = normalize_entry(entry);
    let mut entries = load_unlocked();
    insert_entry(&mut entries, entry);
    write_unlocked(&entries)?;
    Ok(entries)
}

pub fn clear() -> Result<(), String> {
    let _guard = lock().lock().map_err(|error| error.to_string())?;
    write_unlocked(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: usize) -> MacroHistoryEntry {
        MacroHistoryEntry {
            macro_id: format!("macro.{index}"),
            name: format!("name-{index}"),
            result: "completed".to_owned(),
            elapsed_ms: 10,
            at_unix: index as u64 + 1,
            steps: 1,
        }
    }

    #[test]
    fn normalizes_long_name_and_result() {
        let mut item = entry(1);
        item.name = "名".repeat(100);
        item.result = "r".repeat(100);
        let normalized = normalize_entry(item);
        assert_eq!(normalized.name.chars().count(), 80);
        assert_eq!(normalized.result.chars().count(), 80);
    }

    #[test]
    fn fills_missing_timestamp() {
        let mut item = entry(1);
        item.at_unix = 0;
        assert!(normalize_entry(item).at_unix > 0);
    }

    #[test]
    fn keeps_newest_fifty_entries() {
        let mut entries = Vec::new();
        for index in 0..60 {
            insert_entry(&mut entries, entry(index));
        }
        assert_eq!(entries.len(), HISTORY_LIMIT);
        assert_eq!(entries[0].macro_id, "macro.59");
        assert_eq!(entries[49].macro_id, "macro.10");
    }
}
