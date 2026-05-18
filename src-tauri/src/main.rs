#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod config;
mod modules;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let pet_window = app.get_webview_window("main").unwrap();
            let _ = pet_window.set_shadow(false);
            let _ = pet_window.set_position(tauri::Position::Logical(
                tauri::LogicalPosition::new(100.0, 100.0),
            ));
            let _ = pet_window.set_focus();

            let show = MenuItem::with_id(app, "show", "Show Pet", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Hide Pet", true, None::<&str>)?;
            let status = MenuItem::with_id(app, "status", "Meow Status", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show, &hide, &status, &settings, &quit])?;

            let icon = tauri::include_image!("icons/tray-iconTemplate@2x.png");

            TrayIconBuilder::new()
                .icon(icon)
                .icon_as_template(true)
                .menu(&menu)
                .tooltip("ClaudeMeow")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.hide();
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
