#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod bedrock;
mod config;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let pet_window = app.get_webview_window("main").unwrap();
            let _ = pet_window.set_shadow(false);
            let _ = pet_window.set_position(tauri::Position::Logical(
                tauri::LogicalPosition::new(100.0, 100.0),
            ));
            let _ = pet_window.set_focus();

            let show = MenuItem::with_id(app, "show", "Show Pet", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Hide Pet", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show, &hide, &settings, &quit])?;

            let icon = tauri::include_image!("icons/icon.png");

            TrayIconBuilder::new()
                .icon(icon)
                .icon_as_template(false)
                .menu(&menu)
                .tooltip("ClaudeMeowPet")
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

            if let Some(settings_window) = app.get_webview_window("settings") {
                let sw = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = sw.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config::get_config,
            config::set_config,
            activity::get_active_window,
            bedrock::generate_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
