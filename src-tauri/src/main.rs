#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod config;
mod modules;
mod paths;
pub mod platform;
mod runtime;
mod util;

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

#[tauri::command]
fn scan_secret_meow() -> Result<serde_json::Value, String> {
    let profiles_dir = crate::paths::profiles_dir();

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
    use platform::Platform;
    let plat = platform::native();
    plat.open_system_settings(&url);
}

#[tauri::command]
fn check_permissions_status() -> std::collections::HashMap<String, bool> {
    use platform::Platform;
    let plat = platform::native();
    plat.check_permissions_status()
}

fn check_permissions() {
    use platform::Platform;
    let plat = platform::native();

    let status = plat.check_permissions_status();
    let accessibility_ok = status.get("accessibility").copied().unwrap_or(false);

    if !accessibility_ok {
        plat.run_applescript(r#"display dialog "ClaudeMeow needs Accessibility permission to detect your active app and provide contextual reactions.

Please grant access in:
System Settings → Privacy & Security → Accessibility

Then restart ClaudeMeow." with title "ClaudeMeow — Permission Required" with icon caution buttons {"Open Settings", "Later"} default button "Open Settings""#);

        plat.open_system_settings("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility");
    }

    if plat.is_process_running("Google Chrome") {
        let browser_ok = status.get("browser_url").copied().unwrap_or(false);
        if !browser_ok {
            plat.run_applescript(r#"display dialog "ClaudeMeow needs Automation permission to read your browser URL for contextual reactions.

Please grant access in:
System Settings → Privacy & Security → Automation

Allow ClaudeMeow to control Google Chrome." with title "ClaudeMeow — Browser Access" with icon caution buttons {"Open Settings", "Later"} default button "Open Settings""#);

            plat.open_system_settings("x-apple.systempreferences:com.apple.preference.security?Privacy_Automation");
        }
    }
}

fn restart_app(app: &tauri::AppHandle) {
    let current_exe = std::env::current_exe().expect("failed to get current exe path");
    let exe_path = current_exe.to_string_lossy().to_string();

    if cfg!(debug_assertions) || exe_path.contains("/target/") {
        // Dev mode: re-run via cargo run in the src-tauri directory
        let src_tauri_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::process::Command::new("cargo")
            .arg("run")
            .current_dir(&src_tauri_dir)
            .spawn()
            .expect("failed to restart via cargo run");
    } else {
        // Release / .app bundle: relaunch the .app via `open`
        // The exe lives at Something.app/Contents/MacOS/binary
        let app_bundle = current_exe
            .parent() // MacOS/
            .and_then(|p| p.parent()) // Contents/
            .and_then(|p| p.parent()); // Something.app

        if let Some(bundle_path) = app_bundle {
            std::process::Command::new("open")
                .arg("-n")
                .arg(bundle_path)
                .spawn()
                .expect("failed to restart app bundle");
        } else {
            // Fallback: just relaunch the binary directly
            std::process::Command::new(&current_exe)
                .spawn()
                .expect("failed to restart binary");
        }
    }

    app.exit(0);
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            paths::migrate_legacy_dirs();
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

            let data_dir = crate::paths::data_dir();
            let activity_logger = ActivityLogger::new(data_dir.clone());
            activity_logger.prune();
            ai::history::ChatHistory::new(data_dir).prune();

            let runtime_state = RuntimeState {
                event_bus: event_bus.clone(),
                state_machine: state_machine.clone(),
                module_loader: Arc::new(RwLock::new(loader)),
                reaction_engine: ReactionEngine::new(event_bus.clone()),
                activity_logger,
                context_engine: Arc::new(RwLock::new(ContextEngine::new())),
            };
            app.manage(runtime_state);

            // Load encrypted .meow profile (before UI opens)
            let meow_state = runtime::secret_meow::SecretMeowState::try_load();
            eprintln!("[ClaudeMeow] SecretMeow loaded={}, nicknames={:?}", meow_state.loaded, meow_state.nicknames);
            runtime::secret_meow::init_global_context(&meow_state);

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
                runtime::poller::start_polling(app_handle).await;
            });

            let pet_window = app.get_webview_window("main").unwrap();
            let _ = pet_window.set_shadow(false);
            let _ = pet_window.set_position(tauri::Position::Logical(
                tauri::LogicalPosition::new(100.0, 100.0),
            ));
            let _ = pet_window.set_focus();

            let toggle_pet = MenuItem::with_id(app, "toggle_pet", "Hide Pet", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Control Panel", true, None::<&str>)?;
            let dev_label = if config.dev_mode { "Hide Dev Console" } else { "Show Dev Console" };
            let dev = MenuItem::with_id(app, "dev", dev_label, true, None::<&str>)?;
            let restart = MenuItem::with_id(app, "restart", "Restart", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&toggle_pet, &settings, &dev, &restart, &quit])?;

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
                    "restart" => {
                        restart_app(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyC);
            if let Some(chat_window) = app.get_webview_window("main") {
                app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, _event| {
                    let _ = chat_window.show();
                    let _ = chat_window.set_focus();
                    let _ = chat_window.emit("open-chat", ());
                })?;
            }

            let settings_shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyP);
            if let Some(settings_win) = app.get_webview_window("settings") {
                app.global_shortcut().on_shortcut(settings_shortcut, move |_app, _shortcut, _event| {
                    let _ = settings_win.show();
                    let _ = settings_win.set_focus();
                })?;
            }

            if let Some(settings_window) = app.get_webview_window("settings") {
                let sw = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = sw.hide();
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
            runtime::commands::module::get_modules,
            runtime::commands::module::refresh_modules,
            runtime::commands::module::toggle_module,
            runtime::commands::module::create_module,
            runtime::commands::module::delete_user_module,
            runtime::commands::module::emit_test_event,
            runtime::commands::module::get_priority_overrides,
            runtime::commands::module::set_priority_overrides,
            runtime::commands::state::get_activity_log,
            runtime::commands::state::log_status_change,
            runtime::commands::state::get_screen_time,
            runtime::commands::state::context_on_rage,
            runtime::commands::state::context_on_chat,
            runtime::commands::state::get_pet_context,
            runtime::commands::state::get_system_stats,
            runtime::commands::state::get_meow_nicknames,
            runtime::commands::state::get_meow_public_info,
            runtime::commands::state::reload_secret_meow,
            runtime::commands::state::open_external_url,
            runtime::commands::state::clear_chat_history,
            runtime::commands::state::clear_activity_log,
            runtime::commands::state::export_logs,
            runtime::commands::state::open_data_folder,
            runtime::commands::debug::emit_dev_log,
            runtime::commands::debug::emit_cat_status,
            runtime::commands::debug::trigger_meme,
            runtime::commands::ai::trigger_cr_comment,
            runtime::commands::ai::trigger_targeted_gossip,
            runtime::commands::ai::get_team_stats,
            runtime::commands::ai::get_team_activity,
            runtime::commands::ai::get_gossip_history,
            runtime::commands::ai::clear_gossip_memory,
            runtime::commands::ai::refresh_team_stats,
            runtime::mcp_runner::is_mcp_module_present,
            runtime::mcp_runner::call_module_mcp,
            runtime::mcp_runner::get_module_mcp_config,
            runtime::mcp_runner::save_module_mcp_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
