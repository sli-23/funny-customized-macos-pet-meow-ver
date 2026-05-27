
pub fn build_context_key(app_name: &str, time_bucket: &str) -> String {
    let app_short = app_name.split_whitespace().next().unwrap_or(app_name);
    format!("{}:{}", app_short, time_bucket)
}

pub fn get_time_bucket(hour: u32) -> &'static str {
    match hour {
        0..=5 => "night",
        6..=11 => "morning",
        12..=17 => "afternoon",
        18..=23 => "evening",
        _ => "unknown",
    }
}

pub fn check_message_cache(_context_key: &str) -> Option<String> {
    // Disabled: caching causes repetitive messages. Always generate fresh.
    None
}

pub fn save_to_message_cache(_context_key: &str, _message: &str) {
    // No-op: caching disabled to prevent repetition
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_context_key_uses_first_word() {
        assert_eq!(build_context_key("Visual Studio Code", "morning"), "Visual:morning");
    }

    #[test]
    fn test_build_context_key_single_word() {
        assert_eq!(build_context_key("Safari", "evening"), "Safari:evening");
    }

    #[test]
    fn test_get_time_bucket_night() {
        assert_eq!(get_time_bucket(3), "night");
    }

    #[test]
    fn test_get_time_bucket_morning() {
        assert_eq!(get_time_bucket(9), "morning");
    }

    #[test]
    fn test_get_time_bucket_afternoon() {
        assert_eq!(get_time_bucket(14), "afternoon");
    }

    #[test]
    fn test_get_time_bucket_evening() {
        assert_eq!(get_time_bucket(20), "evening");
    }
}
