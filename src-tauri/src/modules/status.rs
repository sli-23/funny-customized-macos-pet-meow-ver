use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetStatus {
    pub happiness: u32,
    pub energy: u32,
    pub hunger: u32,
    pub love: u32,
    pub mood: String,
    pub last_update: u64,
}

impl Default for PetStatus {
    fn default() -> Self {
        Self {
            happiness: 70,
            energy: 80,
            hunger: 60,
            love: 50,
            mood: "happy".to_string(),
            last_update: now_secs(),
        }
    }
}

fn status_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claude-meow-pet")
        .join("status.json")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn clamp(val: i32) -> u32 {
    val.max(0).min(100) as u32
}

pub fn load_status() -> PetStatus {
    let path = status_path();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(mut status) = serde_json::from_str::<PetStatus>(&data) {
                decay_stats(&mut status);
                return status;
            }
        }
    }
    let status = PetStatus::default();
    save_status(&status);
    status
}

fn save_status(status: &PetStatus) {
    let path = status_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(data) = serde_json::to_string_pretty(status) {
        fs::write(path, data).ok();
    }
}

fn decay_stats(status: &mut PetStatus) {
    let now = now_secs();

    // Reset stats to 100 at 9:00 AM daily (except energy)
    let today_9am = {
        let now_local = chrono::Local::now();
        now_local.date_naive()
            .and_hms_opt(9, 0, 0)
            .and_then(|dt| dt.and_local_timezone(chrono::Local).single())
            .map(|dt| dt.timestamp() as u64)
            .unwrap_or(0)
    };
    if status.last_update < today_9am && now >= today_9am {
        status.happiness = 100;
        status.hunger = 100;
        status.love = 100;
    }

    let elapsed_hours = (now - status.last_update) / 3600;
    if elapsed_hours > 0 {
        status.hunger = clamp(status.hunger as i32 - (elapsed_hours as i32 * 5));
        status.happiness = clamp(status.happiness as i32 - (elapsed_hours as i32 * 2));
        status.last_update = now;
    }
    // Energy is time-of-day based: high in morning, drains through work hours
    status.energy = clamp(energy_from_time_of_day() as i32);
    update_mood(status);
    save_status(status);
}

fn energy_from_time_of_day() -> u32 {
    let hour = chrono::Local::now().hour();
    match hour {
        6..=8 => 95,    // morning: fresh
        9..=11 => 80,   // mid-morning: productive
        12..=13 => 60,  // lunch slump
        14..=16 => 70,  // afternoon recovery
        17..=18 => 50,  // end of day drain
        19..=21 => 40,  // evening tired
        22..=23 => 25,  // late night
        0..=5 => 15,    // should be sleeping
        _ => 50,
    }
}

fn update_mood(status: &mut PetStatus) {
    if status.love > 90 {
        status.mood = "love".to_string();
    } else if status.happiness < 30 {
        status.mood = "sad".to_string();
    } else if status.energy < 20 {
        status.mood = "sleepy".to_string();
    } else if status.hunger < 20 {
        status.mood = "hungry".to_string();
    } else if status.happiness > 70 {
        status.mood = "happy".to_string();
    } else {
        status.mood = "neutral".to_string();
    }
}

#[tauri::command]
pub fn get_pet_status() -> PetStatus {
    load_status()
}

#[tauri::command]
pub fn pet_touched() -> PetStatus {
    let mut status = load_status();
    apply_touch(&mut status);
    status.last_update = now_secs();
    save_status(&status);
    status
}

#[tauri::command]
pub fn pet_chatted() -> PetStatus {
    let mut status = load_status();
    apply_chat(&mut status);
    status.last_update = now_secs();
    save_status(&status);
    status
}

#[tauri::command]
pub fn pet_angry() -> PetStatus {
    let mut status = load_status();
    apply_angry(&mut status);
    status.last_update = now_secs();
    save_status(&status);
    status
}

#[tauri::command]
pub fn pet_fed() -> PetStatus {
    let mut status = load_status();
    apply_fed(&mut status);
    status.last_update = now_secs();
    save_status(&status);
    status
}

#[tauri::command]
pub fn pet_rested() -> PetStatus {
    let mut status = load_status();
    apply_rested(&mut status);
    status.last_update = now_secs();
    save_status(&status);
    status
}

// ── Pure helpers extracted for testability ────────────────────────────────────

pub(crate) fn apply_touch(status: &mut PetStatus) {
    status.happiness = clamp(status.happiness as i32 + 3);
    status.love = clamp(status.love as i32 + 1);
    update_mood(status);
}

pub(crate) fn apply_chat(status: &mut PetStatus) {
    status.happiness = clamp(status.happiness as i32 + 5);
    status.love = clamp(status.love as i32 + 5);
    update_mood(status);
}

pub(crate) fn apply_angry(status: &mut PetStatus) {
    status.happiness = clamp(status.happiness as i32 - 15);
    status.mood = "angry".to_string();
}

pub(crate) fn apply_fed(status: &mut PetStatus) {
    status.hunger = clamp(status.hunger as i32 + 30);
    update_mood(status);
}

pub(crate) fn apply_rested(status: &mut PetStatus) {
    status.energy = clamp(status.energy as i32 + 15);
    update_mood(status);
}

// ── Disk I/O helpers with injectable path (used by tests via tempdir) ─────────

pub(crate) fn save_status_to(status: &PetStatus, path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(data) = serde_json::to_string_pretty(status) {
        fs::write(path, data).ok();
    }
}

pub(crate) fn load_status_from(path: &std::path::Path) -> PetStatus {
    if path.exists() {
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(status) = serde_json::from_str::<PetStatus>(&data) {
                return status;
            }
        }
    }
    PetStatus::default()
}

#[allow(dead_code)]
pub fn get_context() -> String {
    let status = load_status();
    format!(
        "Pet mood: {} (happiness:{}, energy:{}, hunger:{}, love:{})",
        status.mood, status.happiness, status.energy, status.hunger, status.love
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn status_at(happiness: u32, energy: u32, hunger: u32, love: u32) -> PetStatus {
        PetStatus { happiness, energy, hunger, love, mood: "neutral".to_string(), last_update: 0 }
    }

    // ── Checkpoint 1: clamp ───────────────────────────────────────────────────

    #[test]
    fn test_clamp_mid_value_unchanged() {
        assert_eq!(clamp(50), 50);
    }

    #[test]
    fn test_clamp_negative_returns_zero() {
        assert_eq!(clamp(-10), 0);
    }

    #[test]
    fn test_clamp_over_100_returns_100() {
        assert_eq!(clamp(150), 100);
    }

    // ── Checkpoint 1: update_mood ─────────────────────────────────────────────

    #[test]
    fn test_update_mood_love_above_90_is_love() {
        let mut s = status_at(70, 80, 60, 91);
        update_mood(&mut s);
        assert_eq!(s.mood, "love");
    }

    #[test]
    fn test_update_mood_happiness_below_30_is_sad() {
        let mut s = status_at(20, 80, 60, 50);
        update_mood(&mut s);
        assert_eq!(s.mood, "sad");
    }

    #[test]
    fn test_update_mood_energy_below_20_is_sleepy() {
        let mut s = status_at(70, 10, 60, 50);
        update_mood(&mut s);
        assert_eq!(s.mood, "sleepy");
    }

    #[test]
    fn test_update_mood_hunger_below_20_is_hungry() {
        let mut s = status_at(70, 80, 10, 50);
        update_mood(&mut s);
        assert_eq!(s.mood, "hungry");
    }

    #[test]
    fn test_update_mood_happiness_above_70_is_happy() {
        let mut s = status_at(80, 80, 60, 50);
        update_mood(&mut s);
        assert_eq!(s.mood, "happy");
    }

    #[test]
    fn test_update_mood_default_is_neutral() {
        let mut s = status_at(50, 50, 50, 50);
        update_mood(&mut s);
        assert_eq!(s.mood, "neutral");
    }

    // ── Checkpoint 1: apply_* stat changes ───────────────────────────────────

    #[test]
    fn test_apply_touch_increments_happiness_by_3() {
        let mut s = status_at(50, 80, 60, 50);
        apply_touch(&mut s);
        assert_eq!(s.happiness, 53);
    }

    #[test]
    fn test_apply_touch_increments_love_by_1() {
        let mut s = status_at(50, 80, 60, 50);
        apply_touch(&mut s);
        assert_eq!(s.love, 51);
    }

    #[test]
    fn test_apply_chat_increments_happiness_by_5() {
        let mut s = status_at(50, 80, 60, 50);
        apply_chat(&mut s);
        assert_eq!(s.happiness, 55);
    }

    #[test]
    fn test_apply_chat_increments_love_by_5() {
        let mut s = status_at(50, 80, 60, 50);
        apply_chat(&mut s);
        assert_eq!(s.love, 55);
    }

    #[test]
    fn test_apply_angry_decrements_happiness_by_15() {
        let mut s = status_at(50, 80, 60, 50);
        apply_angry(&mut s);
        assert_eq!(s.happiness, 35);
    }

    #[test]
    fn test_apply_angry_sets_mood_to_angry() {
        let mut s = status_at(50, 80, 60, 50);
        apply_angry(&mut s);
        assert_eq!(s.mood, "angry");
    }

    #[test]
    fn test_apply_fed_increments_hunger_by_30() {
        let mut s = status_at(50, 80, 50, 50);
        apply_fed(&mut s);
        assert_eq!(s.hunger, 80);
    }

    #[test]
    fn test_apply_rested_increments_energy_by_15() {
        let mut s = status_at(50, 50, 60, 50);
        apply_rested(&mut s);
        assert_eq!(s.energy, 65);
    }

    #[test]
    fn test_stats_clamped_to_100_on_overflow() {
        let mut s = status_at(98, 98, 80, 98);
        apply_touch(&mut s); // +3 happiness, +1 love → both cap at 100
        assert_eq!(s.happiness, 100);
        assert_eq!(s.love, 99);
    }

    #[test]
    fn test_stats_clamped_to_0_on_underflow() {
        let mut s = status_at(5, 80, 60, 50);
        apply_angry(&mut s); // -15 happiness → would go negative
        assert_eq!(s.happiness, 0);
    }

    // ── Checkpoint 2: disk I/O with tempdir ──────────────────────────────────

    fn tmp_path(dir: &TempDir) -> std::path::PathBuf {
        dir.path().join("status.json")
    }

    #[test]
    fn test_save_status_writes_parseable_json() {
        let dir = TempDir::new().unwrap();
        let s = status_at(70, 80, 60, 50);
        save_status_to(&s, &tmp_path(&dir));
        let data = std::fs::read_to_string(tmp_path(&dir)).unwrap();
        assert!(serde_json::from_str::<PetStatus>(&data).is_ok());
    }

    #[test]
    fn test_load_status_reads_saved_json() {
        let dir = TempDir::new().unwrap();
        let s = status_at(42, 77, 33, 88);
        save_status_to(&s, &tmp_path(&dir));
        let loaded = load_status_from(&tmp_path(&dir));
        assert_eq!(loaded.happiness, 42);
        assert_eq!(loaded.love, 88);
    }

    #[test]
    fn test_load_status_returns_default_if_missing() {
        let dir = TempDir::new().unwrap();
        let loaded = load_status_from(&tmp_path(&dir));
        assert_eq!(loaded.happiness, PetStatus::default().happiness);
    }

    #[test]
    fn test_load_status_returns_default_if_corrupt() {
        let dir = TempDir::new().unwrap();
        std::fs::write(tmp_path(&dir), b"not json at all").unwrap();
        let loaded = load_status_from(&tmp_path(&dir));
        assert_eq!(loaded.happiness, PetStatus::default().happiness);
    }

    #[test]
    fn test_save_and_load_roundtrip_preserves_fields() {
        let dir = TempDir::new().unwrap();
        let original = PetStatus {
            happiness: 55, energy: 66, hunger: 77, love: 88,
            mood: "happy".to_string(), last_update: 12345,
        };
        save_status_to(&original, &tmp_path(&dir));
        let loaded = load_status_from(&tmp_path(&dir));
        assert_eq!(loaded.happiness, 55);
        assert_eq!(loaded.energy, 66);
        assert_eq!(loaded.hunger, 77);
        assert_eq!(loaded.love, 88);
        assert_eq!(loaded.mood, "happy");
        assert_eq!(loaded.last_update, 12345);
    }
}
