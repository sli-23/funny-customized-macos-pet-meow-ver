use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CrMemory {
    week_start: String,
    commented_ids: Vec<String>,
}

fn cr_memory_path() -> PathBuf {
    crate::paths::memory_dir().join("cr_commented.json")
}

fn load_cr_memory() -> CrMemory {
    fs::read_to_string(cr_memory_path())
        .ok()
        .and_then(|d| serde_json::from_str(&d).ok())
        .unwrap_or_default()
}

fn save_cr_memory(mem: &CrMemory) {
    let _ = fs::write(cr_memory_path(), serde_json::to_string(mem).unwrap_or_default());
}

fn cr_reset_days() -> u32 {
    fs::read_to_string(crate::paths::amazon_data_dir().join("config.json"))
        .ok()
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .and_then(|c| c["cr_reset_days"].as_u64())
        .unwrap_or(3) as u32
}

fn current_period_start() -> String {
    use chrono::{Local, NaiveDate};
    let today = Local::now().date_naive();
    let reset = cr_reset_days();
    let epoch = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let days_since_epoch = (today - epoch).num_days().unsigned_abs() as u32;
    let period_offset = days_since_epoch % reset;
    let start = today - chrono::Duration::days(period_offset as i64);
    start.format("%Y-%m-%d").to_string()
}

pub fn get_commented_ids() -> Vec<String> {
    let mut mem = load_cr_memory();
    let period = current_period_start();

    if mem.week_start != period {
        mem.week_start = period;
        mem.commented_ids.clear();
        save_cr_memory(&mem);
    }

    mem.commented_ids
}

pub fn clear_commented_ids() {
    let mut mem = load_cr_memory();
    mem.commented_ids.clear();
    save_cr_memory(&mem);
}

pub fn mark_commit_commented(commit_id: &str) {
    let mut mem = load_cr_memory();
    let period = current_period_start();

    if mem.week_start != period {
        mem.week_start = period;
        mem.commented_ids.clear();
    }

    if !mem.commented_ids.contains(&commit_id.to_string()) {
        mem.commented_ids.push(commit_id.to_string());
    }

    save_cr_memory(&mem);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_period_start_is_valid_date() {
        let period = current_period_start();
        assert!(period.len() == 10); // "YYYY-MM-DD"
        assert!(period.contains('-'));
    }

    #[test]
    fn test_cr_reset_days_default() {
        // With no config file, defaults to 3
        assert_eq!(cr_reset_days(), 3);
    }
}
