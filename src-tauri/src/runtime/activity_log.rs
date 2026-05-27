use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const MAX_FILE_SIZE: u64 = 1_048_576; // 1MB
const RETENTION_MS: u64 = 86_400_000; // 24 hours

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub ts: u64,
    pub source: String,
    pub event: String,
    pub detail: String,
    #[serde(default)]
    pub delta: Option<i32>,
}

pub struct ActivityLogger {
    log_path: PathBuf,
}

impl ActivityLogger {
    pub fn new(data_dir: PathBuf) -> Self {
        fs::create_dir_all(&data_dir).ok();
        Self {
            log_path: data_dir.join("activity.jsonl"),
        }
    }


    #[cfg(test)]
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    pub fn append(&self, entry: &ActivityEntry) {
        if let Ok(line) = serde_json::to_string(entry) {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_path)
            {
                writeln!(file, "{}", line).ok();
            }
        }
        self.check_size_limit();
    }

    pub fn read_recent(&self, limit: usize) -> Vec<ActivityEntry> {
        if !self.log_path.exists() {
            return Vec::new();
        }

        let file = match fs::File::open(&self.log_path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let reader = BufReader::new(file);
        let mut entries: Vec<ActivityEntry> = reader.lines()
            .flatten()
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();

        let start = entries.len().saturating_sub(limit);
        entries.drain(..start);
        entries
    }

    pub fn prune(&self) {
        if !self.log_path.exists() {
            return;
        }

        let cutoff = now_ms().saturating_sub(RETENTION_MS);

        let file = match fs::File::open(&self.log_path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let reader = BufReader::new(file);
        let kept: Vec<String> = reader.lines()
            .flatten()
            .filter(|line| {
                if let Ok(entry) = serde_json::from_str::<ActivityEntry>(line) {
                    entry.ts >= cutoff
                } else {
                    false
                }
            })
            .collect();

        if let Err(e) = fs::write(&self.log_path, kept.join("\n") + if kept.is_empty() { "" } else { "\n" }) {
            eprintln!("[ClaudeMeow] activity log prune failed: {}", e);
        }
    }

    fn check_size_limit(&self) {
        if let Ok(meta) = fs::metadata(&self.log_path) {
            if meta.len() > MAX_FILE_SIZE {
                self.truncate_oldest_half();
            }
        }
    }

    fn truncate_oldest_half(&self) {
        let file = match fs::File::open(&self.log_path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().flatten().collect();
        let half = lines.len() / 2;
        let kept = &lines[half..];
        if let Err(e) = fs::write(&self.log_path, kept.join("\n") + "\n") {
            eprintln!("[ClaudeMeow] activity log truncate failed: {}", e);
        }
    }
}

use crate::util::now_ms;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, ActivityLogger) {
        let tmp = tempfile::tempdir().unwrap();
        let logger = ActivityLogger::new(tmp.path().to_path_buf());
        (tmp, logger)
    }

    #[test]
    fn test_append_and_read() {
        let (_tmp, logger) = setup();

        let entry = ActivityEntry {
            ts: 1000,
            source: "test".to_string(),
            event: "user_typing".to_string(),
            detail: "VSCode".to_string(),
            delta: None,
        };

        logger.append(&entry);

        let entries = logger.read_recent(10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "test");
        assert_eq!(entries[0].event, "user_typing");
    }

    #[test]
    fn test_read_recent_limit() {
        let (_tmp, logger) = setup();

        for i in 0..20 {
            logger.append(&ActivityEntry {
                ts: i,
                source: "test".to_string(),
                event: "e".to_string(),
                detail: format!("entry_{}", i),
                delta: Some(i as i32),
            });
        }

        let entries = logger.read_recent(5);
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].detail, "entry_15");
        assert_eq!(entries[4].detail, "entry_19");
    }

    #[test]
    fn test_prune_old_entries() {
        let (_tmp, logger) = setup();

        let now = now_ms();
        let old = now - RETENTION_MS - 1000;

        logger.append(&ActivityEntry {
            ts: old,
            source: "old".to_string(),
            event: "e".to_string(),
            detail: "should be pruned".to_string(),
            delta: None,
        });
        logger.append(&ActivityEntry {
            ts: now,
            source: "new".to_string(),
            event: "e".to_string(),
            detail: "should remain".to_string(),
            delta: None,
        });

        logger.prune();

        let entries = logger.read_recent(10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "new");
    }

    #[test]
    fn test_empty_read() {
        let (_tmp, logger) = setup();
        let entries = logger.read_recent(10);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_delta_field() {
        let (_tmp, logger) = setup();

        logger.append(&ActivityEntry {
            ts: 1000,
            source: "status".to_string(),
            event: "pet_status_changed".to_string(),
            detail: "happiness".to_string(),
            delta: Some(10),
        });

        logger.append(&ActivityEntry {
            ts: 2000,
            source: "status".to_string(),
            event: "pet_status_changed".to_string(),
            detail: "energy".to_string(),
            delta: Some(-5),
        });

        let entries = logger.read_recent(10);
        assert_eq!(entries[0].delta, Some(10));
        assert_eq!(entries[1].delta, Some(-5));
    }

    #[test]
    fn test_size_limit_truncation() {
        let (_tmp, logger) = setup();

        // Write enough to exceed 1MB
        let big_detail = "x".repeat(10000);
        for i in 0..120 {
            logger.append(&ActivityEntry {
                ts: i,
                source: "bulk".to_string(),
                event: "e".to_string(),
                detail: big_detail.clone(),
                delta: None,
            });
        }

        let meta = fs::metadata(logger.log_path()).unwrap();
        assert!(meta.len() <= MAX_FILE_SIZE + 20000);
    }

    #[test]
    fn test_prune_empty_file() {
        let (_tmp, logger) = setup();
        // Should not panic on empty/missing file
        logger.prune();
        let entries = logger.read_recent(10);
        assert!(entries.is_empty());
    }
}
