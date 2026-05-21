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
    status.happiness = clamp(status.happiness as i32 + 3);
    status.love = clamp(status.love as i32 + 1);
    status.last_update = now_secs();
    update_mood(&mut status);
    save_status(&status);
    status
}

#[tauri::command]
pub fn pet_chatted() -> PetStatus {
    let mut status = load_status();
    status.happiness = clamp(status.happiness as i32 + 5);
    status.love = clamp(status.love as i32 + 5);
    status.last_update = now_secs();
    update_mood(&mut status);
    save_status(&status);
    status
}

#[tauri::command]
pub fn pet_angry() -> PetStatus {
    let mut status = load_status();
    status.happiness = clamp(status.happiness as i32 - 15);
    status.last_update = now_secs();
    status.mood = "angry".to_string();
    save_status(&status);
    status
}

#[tauri::command]
pub fn pet_fed() -> PetStatus {
    let mut status = load_status();
    status.hunger = clamp(status.hunger as i32 + 30);
    status.last_update = now_secs();
    update_mood(&mut status);
    save_status(&status);
    status
}

#[tauri::command]
pub fn pet_rested() -> PetStatus {
    let mut status = load_status();
    status.energy = clamp(status.energy as i32 + 15);
    status.last_update = now_secs();
    update_mood(&mut status);
    save_status(&status);
    status
}

#[allow(dead_code)]
pub fn get_context() -> String {
    let status = load_status();
    format!(
        "Pet mood: {} (happiness:{}, energy:{}, hunger:{}, love:{})",
        status.mood, status.happiness, status.energy, status.hunger, status.love
    )
}
