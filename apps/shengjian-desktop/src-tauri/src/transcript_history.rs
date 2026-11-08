use crate::host::TranscriptRecord;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const HISTORY_LIMIT: usize = 200;
const MAX_TEXT_CHARS: usize = 20_000;

fn history_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Shengjian")
        .join("transcript-history.json")
}

fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn load_unlocked() -> Vec<TranscriptRecord> {
    let Ok(contents) = std::fs::read_to_string(history_path()) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<TranscriptRecord>>(&contents)
        .map(|mut records| {
            records.truncate(HISTORY_LIMIT);
            records
        })
        .unwrap_or_default()
}

fn write_unlocked(records: &[TranscriptRecord]) -> Result<(), String> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let contents = serde_json::to_string_pretty(records).map_err(|error| error.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents).map_err(|error| error.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|error| error.to_string())
}

pub fn load() -> Result<Vec<TranscriptRecord>, String> {
    let _guard = lock().lock().map_err(|error| error.to_string())?;
    Ok(load_unlocked())
}

fn normalize_record(mut record: TranscriptRecord) -> TranscriptRecord {
    record.text = record.text.chars().take(MAX_TEXT_CHARS).collect();
    record
}

fn insert_record(records: &mut Vec<TranscriptRecord>, record: TranscriptRecord) -> bool {
    if records
        .iter()
        .any(|existing| existing.session_id == record.session_id)
    {
        return false;
    }
    records.insert(0, record);
    records.truncate(HISTORY_LIMIT);
    true
}

pub fn append(record: TranscriptRecord) -> Result<Vec<TranscriptRecord>, String> {
    let _guard = lock().lock().map_err(|error| error.to_string())?;
    let record = normalize_record(record);
    let mut records = load_unlocked();
    if insert_record(&mut records, record) {
        write_unlocked(&records)?;
    }
    Ok(records)
}

pub fn clear() -> Result<(), String> {
    let _guard = lock().lock().map_err(|error| error.to_string())?;
    write_unlocked(&[])
}

pub fn save(records: &[TranscriptRecord]) -> Result<(), String> {
    let _guard = lock().lock().map_err(|error| error.to_string())?;
    let mut records = records.to_vec();
    records.truncate(HISTORY_LIMIT);
    for record in &mut records {
        record.text = record.text.chars().take(MAX_TEXT_CHARS).collect();
    }
    write_unlocked(&records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(session: &str, text: &str) -> TranscriptRecord {
        TranscriptRecord {
            session_id: session.to_owned(),
            started_at_ms: 1,
            ended_at_ms: 2,
            local_date_key: "2026-11-07".to_owned(),
            application_key: "notepad".to_owned(),
            application_name: "Notepad".to_owned(),
            source: "voice".to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn truncates_oversized_text() {
        let normalized = normalize_record(record("a", &"x".repeat(MAX_TEXT_CHARS + 10)));
        assert_eq!(normalized.text.chars().count(), MAX_TEXT_CHARS);
    }

    #[test]
    fn skips_duplicate_session_id() {
        let mut records = Vec::new();
        assert!(insert_record(&mut records, record("a", "first")));
        assert!(!insert_record(&mut records, record("a", "second")));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].text, "first");
    }

    #[test]
    fn keeps_newest_two_hundred_records() {
        let mut records = Vec::new();
        for index in 0..220 {
            insert_record(&mut records, record(&format!("session-{index}"), "text"));
        }
        assert_eq!(records.len(), HISTORY_LIMIT);
        assert_eq!(records[0].session_id, "session-219");
    }
}
