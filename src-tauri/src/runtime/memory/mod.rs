mod message_cache;
mod theme_tracker;
mod cr_memory;

pub use message_cache::{build_context_key, check_message_cache, get_time_bucket, save_to_message_cache};
pub use theme_tracker::{get_blocked_themes_for_prompt, is_theme_blocked, record_theme};
pub use cr_memory::{get_commented_ids, mark_commit_commented};

pub fn amazon_data_dir() -> std::path::PathBuf {
    crate::paths::amazon_data_dir()
}
