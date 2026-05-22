#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod config;
mod modules;
mod runtime;

use runtime::activity_log::ActivityLogger;
use runtime::commands::RuntimeState;
use runtime::context_engine::ContextEngine;
use runtime::event_bus::EventBus;
use runtime::module_loader::{default_modules_dir, ModuleLoader};
use runtime::reaction_engine::ReactionEngine;
use runtime::state_machine::StateMachine;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tokio::sync::RwLock;
use std::process::Command;

#[tauri::command]
fn scan_secret_meow() -> Result<serde_json::Value, String> {
    let profiles_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("claude-meow-pet")
        .join("profiles");

    if !profiles_dir.exists() {
        return Ok(serde_json::json!({ "found": false, "message": "No profiles folder found" }));
    }

    let mut found_files: Vec<String> = Vec::new();
    let mut from_littleshrimp = false;

    if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("meow") {
                let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                found_files.push(filename);
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let lower = content.to_lowercase();
                    if lower.contains("littleshrimp") || lower.contains("little_shrimp") || lower.contains("小虾") {
                        from_littleshrimp = true;
                    }
                }
            }
        }
    }

    if found_files.is_empty() {
        Ok(serde_json::json!({ "found": false, "message": "No .meow files detected" }))
    } else {
        Ok(serde_json::json!({
            "found": true,
            "from_littleshrimp": from_littleshrimp,
            "files": found_files,
            "message": if from_littleshrimp {
                "Secret file from LittleShrimp loaded successfully! 🦐✨"
            } else {
                "Secret .meow file(s) detected and loaded!"
            }
        }))
    }
}

#[tauri::command]
fn open_system_settings(url: String) {
    Command::new("open").arg(&url).spawn().ok();
}

#[tauri::command]
fn check_permissions_status() -> std::collections::HashMap<String, bool> {
    let mut result = std::collections::HashMap::new();

    let accessibility = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to get name of first application process whose frontmost is true"#)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    result.insert("accessibility".to_string(), accessibility);

    let window_title = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events"
            set frontApp to name of first application process whose frontmost is true
            tell process frontApp
                set wTitle to name of front window
            end tell
            return wTitle
        end tell"#)
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false);
    result.insert("window_titles".to_string(), window_title);

    let chrome_running = Command::new("pgrep")
        .arg("-x")
        .arg("Google Chrome")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if chrome_running {
        let chrome_url = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "Google Chrome" to get URL of active tab of front window"#)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        result.insert("browser_url".to_string(), chrome_url);
    } else {
        result.insert("browser_url".to_string(), false);
    }

    result
}

fn check_permissions() {

    // Test accessibility: can we read window titles?
    let accessibility_ok = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to get name of first application process whose frontmost is true"#)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !accessibility_ok {
        // Show native macOS dialog
        let _ = Command::new("osascript")
            .arg("-e")
            .arg(r#"display dialog "ClaudeMeow needs Accessibility permission to detect your active app and provide contextual reactions.

Please grant access in:
System Settings → Privacy & Security → Accessibility

Then restart ClaudeMeow." with title "ClaudeMeow — Permission Required" with icon caution buttons {"Open Settings", "Later"} default button "Open Settings""#)
            .output()
            .and_then(|o| {
                let result = String::from_utf8_lossy(&o.stdout).to_string();
                if result.contains("Open Settings") {
                    Command::new("open")
                        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                        .spawn()
                        .ok();
                }
                Ok(o)
            });
    }

    // Test browser automation: can we read Chrome URL?
    let has_chrome = Command::new("pgrep")
        .arg("-x")
        .arg("Google Chrome")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_chrome {
        let automation_ok = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "Google Chrome" to get title of active tab of front window"#)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !automation_ok {
            let _ = Command::new("osascript")
                .arg("-e")
                .arg(r#"display dialog "ClaudeMeow needs Automation permission to read your browser URL for contextual reactions.

Please grant access in:
System Settings → Privacy & Security → Automation

Allow ClaudeMeow to control Google Chrome." with title "ClaudeMeow — Browser Access" with icon caution buttons {"Open Settings", "Later"} default button "Open Settings""#)
                .output()
                .and_then(|o| {
                    let result = String::from_utf8_lossy(&o.stdout).to_string();
                    if result.contains("Open Settings") {
                        Command::new("open")
                            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Automation")
                            .spawn()
                            .ok();
                    }
                    Ok(o)
                });
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            check_permissions();

            // Initialize runtime
            let event_bus = EventBus::new(256);
            let state_machine = StateMachine::new(event_bus.clone());

            let modules_dir = default_modules_dir();
            let mut loader = ModuleLoader::new(modules_dir);
            let errors = loader.scan_and_load();
            if !errors.is_empty() {
                eprintln!("[ClaudeMeow] Module load errors: {:?}", errors);
            }
            let module_count = loader.get_modules().len();
            eprintln!("[ClaudeMeow] Loaded {} modules", module_count);

            let data_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("ClaudeMeow");
            let activity_logger = ActivityLogger::new(data_dir);
            activity_logger.prune();

            let runtime_state = RuntimeState {
                event_bus: event_bus.clone(),
                state_machine: state_machine.clone(),
                module_loader: Arc::new(RwLock::new(loader)),
                reaction_engine: ReactionEngine::new(event_bus.clone()),
                activity_logger,
                context_engine: Arc::new(RwLock::new(ContextEngine::new())),
            };
            app.manage(runtime_state);

            // Show dev console if dev mode is enabled
            let config = config::load_config();
            if config.dev_mode {
                if let Some(w) = app.get_webview_window("dev") {
                    let _ = w.show();
                }
            }

            // Start background polling
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                runtime::commands::start_polling(app_handle).await;
            });

            let pet_window = app.get_webview_window("main").unwrap();
            let _ = pet_window.set_shadow(false);
            let _ = pet_window.set_position(tauri::Position::Logical(
                tauri::LogicalPosition::new(100.0, 100.0),
            ));
            let _ = pet_window.set_focus();

            let toggle_pet = MenuItem::with_id(app, "toggle_pet", "Hide Pet", true, None::<&str>)?;
            let status = MenuItem::with_id(app, "status", "Meow Status", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let dev_label = if config.dev_mode { "Hide Dev Console" } else { "Show Dev Console" };
            let dev = MenuItem::with_id(app, "dev", dev_label, true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&toggle_pet, &status, &settings, &dev, &quit])?;

            let icon = tauri::include_image!("icons/tray-iconTemplate@2x.png");

            let toggle_pet_ref = toggle_pet.clone();
            let dev_ref = dev.clone();

            TrayIconBuilder::new()
                .icon(icon)
                .icon_as_template(true)
                .menu(&menu)
                .tooltip("ClaudeMeow")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "toggle_pet" => {
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                                let _ = toggle_pet_ref.set_text("Show Pet");
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                                let _ = toggle_pet_ref.set_text("Hide Pet");
                            }
                        }
                    }
                    "status" => {
                        if let Some(w) = app.get_webview_window("status") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "settings" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "dev" => {
                        if let Some(w) = app.get_webview_window("dev") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                                let _ = dev_ref.set_text("Show Dev Console");
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                                let _ = dev_ref.set_text("Hide Dev Console");
                            }
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyC);
            let chat_window = app.get_webview_window("main").unwrap();
            app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, _event| {
                let _ = chat_window.show();
                let _ = chat_window.set_focus();
                let _ = chat_window.emit("open-chat", ());
            })?;

            if let Some(settings_window) = app.get_webview_window("settings") {
                let sw = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = sw.hide();
                    }
                });
            }

            if let Some(status_window) = app.get_webview_window("status") {
                let stw = status_window.clone();
                status_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = stw.hide();
                    }
                });
            }

            if let Some(dev_window) = app.get_webview_window("dev") {
                let dw = dev_window.clone();
                dev_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = dw.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config::get_config,
            config::set_config,
            modules::activity::get_active_window,
            modules::get_activity_context,
            modules::get_all_context,
            modules::user_profile::get_user_profile,
            modules::user_profile::set_user_profile,
            modules::user_profile::get_meow_profile,
            modules::user_profile::import_meow_profile,
            modules::status::get_pet_status,
            modules::status::pet_touched,
            modules::status::pet_chatted,
            modules::status::pet_angry,
            modules::status::pet_fed,
            modules::status::pet_rested,
            ai::periodic::generate_message,
            ai::chat::chat_message,
            ai::test_api,
            check_permissions_status,
            open_system_settings,
            scan_secret_meow,
            runtime::commands::get_modules,
            runtime::commands::refresh_modules,
            runtime::commands::toggle_module,
            runtime::commands::get_activity_log,
            runtime::commands::emit_test_event,
            runtime::commands::log_status_change,
            runtime::commands::emit_dev_log,
            runtime::commands::get_screen_time,
            runtime::commands::get_priority_overrides,
            runtime::commands::set_priority_overrides,
            runtime::commands::context_on_rage,
            runtime::commands::context_on_chat,
            runtime::commands::get_pet_context,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
