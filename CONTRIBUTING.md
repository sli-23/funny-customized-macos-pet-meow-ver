# Contributing to ClaudeMeow

## Prerequisites

- macOS (Apple Silicon or Intel)
- Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Tauri CLI: `cargo install tauri-cli`

## Development

```bash
git clone https://github.com/sli-23/funny-customized-macos-pet-meow-ver.git
cd funny-customized-macos-pet-meow-ver/src-tauri
cargo run
```

## Testing

```bash
cd src-tauri
cargo test              # default features
cargo test --features amazon-internal  # all features
cargo clippy            # lint
cargo fmt -- --check    # formatting
```

## Project Structure

```
src-tauri/src/
  main.rs               # App entry, window setup, tray menu
  paths.rs              # Unified filesystem paths
  util.rs               # Shared helpers (now_secs, now_ms)
  config.rs             # AppConfig load/save
  runtime/
    poller.rs           # Background polling loop (5s interval)
    browser.rs          # URL extraction + site classification
    commands.rs         # Tauri command handlers
    event_bus.rs        # Broadcast event system
    state_machine.rs    # Pet state transitions
    module_loader.rs    # JSON module loading from disk
    reaction_engine.rs  # Event → reaction matching + cooldowns
    context_engine.rs   # Time/activity/mood context builder
    activity_log.rs     # JSONL activity persistence
    memory/             # Split memory subsystems
      message_cache.rs  # AI message LRU cache
      theme_tracker.rs  # Repetition avoidance
      cr_memory.rs      # CR comment dedup (amazon-internal feature)
    mcp_runner.rs       # Python MCP script runner
  ai/                   # Claude AI integration (multi-provider)
  modules/              # Rust data collectors (activity, spotify, etc.)
modules/                # JSON reaction packs (drop-in)
ui/                     # Frontend (HTML/JS/CSS)
```

## Adding a Module

See [docs/module-development.md](docs/module-development.md) for the full guide.

Quick version: create a folder in `modules/` with `manifest.json` + `reactions.json`.

## Code Style

- `cargo fmt` for Rust formatting
- No comments unless the WHY is non-obvious
- Use `crate::paths::*` for all filesystem paths
- Use `crate::util::{now_secs, now_ms}` for timestamps
- `eprintln!("[ClaudeMeow] ...")` for debug output

## Amazon-Internal Module

The `amazon-internal` module (CR gossip, calendar, team info) is not included in the repo. It's shared separately as a drop-in folder. To use it:

1. Get the `modules/amazon-internal/` folder from the module author
2. Place it in `modules/amazon-internal/`
3. Restart ClaudeMeow — the Amazon tab appears in Settings automatically

No compile flags needed. The app detects the module at runtime via `is_mcp_module_present`.

## Pull Requests

- Run `cargo test && cargo clippy` before submitting
- Keep PRs focused on one change
- Include test coverage for new logic
