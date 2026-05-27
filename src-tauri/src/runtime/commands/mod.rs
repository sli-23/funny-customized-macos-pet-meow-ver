pub mod module;
pub mod debug;
pub mod state;
pub mod ai;

use super::activity_log::ActivityLogger;
use super::context_engine::ContextEngine;
use super::event_bus::EventBus;
use super::module_loader::ModuleLoader;
use super::reaction_engine::ReactionEngine;
use super::state_machine::StateMachine;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RuntimeState {
    pub event_bus: EventBus,
    pub state_machine: StateMachine,
    pub module_loader: Arc<RwLock<ModuleLoader>>,
    pub reaction_engine: ReactionEngine,
    pub activity_logger: ActivityLogger,
    pub context_engine: Arc<RwLock<ContextEngine>>,
}

pub(crate) fn dev_log(app: &tauri::AppHandle, tag: &str, tag_class: &str, msg: &str) {
    let _ = tauri::Emitter::emit(app, "dev-log", serde_json::json!({
        "tag": tag,
        "tag_class": tag_class,
        "message": msg,
    }));
}

pub(crate) fn is_dev_mode(app: &tauri::AppHandle) -> bool {
    use tauri::Manager;
    let config_on = crate::config::load_config().dev_mode;
    let window_visible = app.get_webview_window("dev")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);
    config_on || window_visible
}
