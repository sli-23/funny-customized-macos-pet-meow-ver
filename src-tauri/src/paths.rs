use std::fs;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claude-meow-pet");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn data_dir() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claude-meow-pet");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn memory_dir() -> PathBuf {
    let dir = data_dir().join("memory");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn amazon_data_dir() -> PathBuf {
    let dir = data_dir().join("amazon");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn profiles_dir() -> PathBuf {
    config_dir().join("profiles")
}

pub fn modules_user_dir() -> PathBuf {
    data_dir().join("modules")
}

pub fn migrate_legacy_dirs() {
    let legacy = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ClaudeMeow");

    if !legacy.exists() {
        return;
    }

    let marker = legacy.join(".migrated");
    if marker.exists() {
        return;
    }

    let target = data_dir();
    eprintln!("[ClaudeMeow] Migrating data from {:?} to {:?}", legacy, target);

    let files_to_migrate = ["activity.jsonl", "chat_history.jsonl"];
    for file in &files_to_migrate {
        let src = legacy.join(file);
        let dst = target.join(file);
        if src.exists() && !dst.exists() {
            fs::copy(&src, &dst).ok();
            eprintln!("[ClaudeMeow]   migrated {}", file);
        }
    }

    let legacy_modules = legacy.join("modules");
    let target_modules = target.join("modules");
    if legacy_modules.exists() && !target_modules.exists() {
        copy_dir_recursive(&legacy_modules, &target_modules);
        eprintln!("[ClaudeMeow]   migrated modules/");
    }

    fs::write(&marker, "migrated to claude-meow-pet").ok();
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) {
    fs::create_dir_all(dst).ok();
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());
            if path.is_dir() {
                copy_dir_recursive(&path.to_path_buf(), &dest_path);
            } else {
                fs::copy(&path, &dest_path).ok();
            }
        }
    }
}
