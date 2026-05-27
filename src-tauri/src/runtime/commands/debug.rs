use super::{dev_log, is_dev_mode};
use base64::Engine;
use tauri::Emitter;

#[tauri::command]
pub fn emit_dev_log(app: tauri::AppHandle, tag: String, tag_class: String, message: String) {
    if is_dev_mode(&app) {
        dev_log(&app, &tag, &tag_class, &message);
    }
}

#[tauri::command]
pub fn emit_cat_status(app: tauri::AppHandle, status: String) {
    let _ = app.emit("cat-status", serde_json::json!({ "status": status }));
}

#[tauri::command]
pub fn trigger_meme(app: tauri::AppHandle) -> Result<String, String> {
    if let Some(meme_path) = pick_random_meme() {
        let data = std::fs::read(&meme_path).map_err(|e| e.to_string())?;
        let ext = std::path::Path::new(&meme_path)
            .extension().unwrap_or_default().to_string_lossy().to_lowercase();
        let mime = match ext.as_str() {
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "image/jpeg",
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
        let data_url = format!("data:{};base64,{}", mime, b64);
        let _ = app.emit("meme-reaction", serde_json::json!({ "data_url": data_url }));
        Ok(meme_path)
    } else {
        Err("No memes found in modules/memes/images/".to_string())
    }
}

fn pick_random_meme() -> Option<String> {
    use crate::runtime::module_loader::default_modules_dir;
    let memes_dir = default_modules_dir().join("memes").join("images");
    let entries = std::fs::read_dir(&memes_dir).ok()?;
    let images: Vec<_> = entries
        .flatten()
        .filter(|e| {
            let ext = e.path().extension().unwrap_or_default().to_string_lossy().to_lowercase();
            matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp")
        })
        .collect();
    if images.is_empty() {
        return None;
    }
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize) % images.len();
    Some(images[idx].path().to_string_lossy().to_string())
}
