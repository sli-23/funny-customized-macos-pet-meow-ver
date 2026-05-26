use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const MAX_FILE_SIZE: u64 = 524_288; // 512 KB
const RETENTION_SECS: u64 = 7_200;  // 2 hours

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatTurn {
    pub ts: u64,
    pub role: String,  // "user" | "assistant"
    pub text: String,
}

pub struct ChatHistory {
    path: PathBuf,
}

impl ChatHistory {
    pub fn new(data_dir: PathBuf) -> Self {
        fs::create_dir_all(&data_dir).ok();
        Self {
            path: data_dir.join("chat_history.jsonl"),
        }
    }

    pub fn append(&self, turn: &ChatTurn) {
        if let Ok(line) = serde_json::to_string(turn) {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                writeln!(file, "{}", line).ok();
            }
        }
        self.check_size_limit();
    }

    pub fn read_recent(&self, n: usize) -> Vec<ChatTurn> {
        if !self.path.exists() {
            return Vec::new();
        }
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let mut turns: Vec<ChatTurn> = BufReader::new(file)
            .lines()
            .flatten()
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();
        let start = turns.len().saturating_sub(n);
        turns.drain(..start);
        turns
    }

    pub fn format_for_prompt(&self, n: usize) -> String {
        let turns = self.read_recent(n);
        format_turns(&turns)
    }

    pub fn prune(&self) {
        if !self.path.exists() {
            return;
        }
        let cutoff = now_secs().saturating_sub(RETENTION_SECS);
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let kept: Vec<String> = BufReader::new(file)
            .lines()
            .flatten()
            .filter(|line| {
                serde_json::from_str::<ChatTurn>(line)
                    .map(|t| t.ts >= cutoff)
                    .unwrap_or(false)
            })
            .collect();
        let content = if kept.is_empty() {
            String::new()
        } else {
            kept.join("\n") + "\n"
        };
        fs::write(&self.path, content).ok();
    }

    fn check_size_limit(&self) {
        if let Ok(meta) = fs::metadata(&self.path) {
            if meta.len() > MAX_FILE_SIZE {
                self.truncate_oldest_half();
            }
        }
    }

    fn truncate_oldest_half(&self) {
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let lines: Vec<String> = BufReader::new(file).lines().flatten().collect();
        let half = lines.len() / 2;
        fs::write(&self.path, lines[half..].join("\n") + "\n").ok();
    }
}

/// Pure helper — exposed for testing prompt-building logic in chat.rs
pub(crate) fn format_turns(turns: &[ChatTurn]) -> String {
    if turns.is_empty() {
        return String::new();
    }
    let mut lines = vec!["[Recent conversation]".to_string()];
    for turn in turns {
        let label = if turn.role == "user" { "You" } else { "ClaudeMeow" };
        lines.push(format!("{}: {}", label, turn.text));
    }
    lines.join("\n")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, ChatHistory) {
        let tmp = tempfile::tempdir().unwrap();
        let h = ChatHistory::new(tmp.path().to_path_buf());
        (tmp, h)
    }

    fn turn(role: &str, text: &str) -> ChatTurn {
        ChatTurn { ts: now_secs(), role: role.into(), text: text.into() }
    }

    fn turn_at(ts: u64, role: &str, text: &str) -> ChatTurn {
        ChatTurn { ts, role: role.into(), text: text.into() }
    }

    // ── Checkpoint 1: format_turns (pure, no disk) ────────────────────────────

    #[test]
    fn test_chat_turn_serializes_to_json() {
        let t = turn_at(1000, "user", "hello");
        let s = serde_json::to_string(&t).unwrap();
        assert!(s.contains("\"role\":\"user\""));
        assert!(s.contains("\"text\":\"hello\""));
    }

    #[test]
    fn test_chat_turn_deserializes_from_json() {
        let json = r#"{"ts":1000,"role":"assistant","text":"meow"}"#;
        let t: ChatTurn = serde_json::from_str(json).unwrap();
        assert_eq!(t.role, "assistant");
        assert_eq!(t.text, "meow");
    }

    #[test]
    fn test_format_turns_empty_returns_empty_string() {
        assert_eq!(format_turns(&[]), "");
    }

    #[test]
    fn test_format_turns_includes_header() {
        let turns = vec![turn("user", "hi")];
        assert!(format_turns(&turns).starts_with("[Recent conversation]"));
    }

    #[test]
    fn test_format_turns_user_labeled_you() {
        let turns = vec![turn("user", "hello")];
        assert!(format_turns(&turns).contains("You: hello"));
    }

    #[test]
    fn test_format_turns_assistant_labeled_claudemeow() {
        let turns = vec![turn("assistant", "meow~")];
        assert!(format_turns(&turns).contains("ClaudeMeow: meow~"));
    }

    #[test]
    fn test_format_turns_one_full_exchange() {
        let turns = vec![
            turn("user", "hi"),
            turn("assistant", "hello!"),
        ];
        let s = format_turns(&turns);
        assert!(s.contains("You: hi"));
        assert!(s.contains("ClaudeMeow: hello!"));
    }

    // ── Checkpoint 2: disk I/O ────────────────────────────────────────────────

    #[test]
    fn test_append_writes_jsonl_line() {
        let (_tmp, h) = setup();
        h.append(&turn("user", "test"));
        let data = fs::read_to_string(&h.path).unwrap();
        assert!(data.contains("\"text\":\"test\""));
    }

    #[test]
    fn test_read_recent_empty_if_no_file() {
        let (_tmp, h) = setup();
        assert!(h.read_recent(10).is_empty());
    }

    #[test]
    fn test_roundtrip_append_and_read() {
        let (_tmp, h) = setup();
        h.append(&turn("user", "hi"));
        h.append(&turn("assistant", "hello!"));
        let turns = h.read_recent(10);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].role, "user");
        assert_eq!(turns[1].text, "hello!");
    }

    #[test]
    fn test_read_recent_returns_last_n() {
        let (_tmp, h) = setup();
        for i in 0..6u64 {
            h.append(&turn_at(i, "user", &format!("msg{}", i)));
        }
        let turns = h.read_recent(3);
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[0].text, "msg3");
        assert_eq!(turns[2].text, "msg5");
    }

    #[test]
    fn test_prune_removes_turns_older_than_2h() {
        let (_tmp, h) = setup();
        let now = now_secs();
        h.append(&turn_at(now - RETENTION_SECS - 1, "user", "old"));
        h.append(&turn_at(now, "user", "new"));
        h.prune();
        let turns = h.read_recent(10);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].text, "new");
    }

    #[test]
    fn test_prune_keeps_recent_turns() {
        let (_tmp, h) = setup();
        h.append(&turn("user", "recent"));
        h.prune();
        assert_eq!(h.read_recent(10).len(), 1);
    }

    #[test]
    fn test_prune_on_missing_file_does_not_panic() {
        let (_tmp, h) = setup();
        h.prune(); // no file exists yet — should not panic
    }

    #[test]
    fn test_size_cap_truncates_oldest_half() {
        let (_tmp, h) = setup();
        let big = "x".repeat(5000);
        for i in 0u64..120 {
            h.append(&turn_at(i, "user", &big));
        }
        let meta = fs::metadata(&h.path).unwrap();
        assert!(meta.len() <= MAX_FILE_SIZE + 10_000);
    }

    #[test]
    fn test_format_for_prompt_respects_n_limit() {
        let (_tmp, h) = setup();
        for i in 0..6u64 {
            h.append(&turn_at(i, "user", &format!("m{}", i)));
        }
        let s = h.format_for_prompt(2);
        assert!(s.contains("m4"));
        assert!(s.contains("m5"));
        assert!(!s.contains("m3"));
    }
}
