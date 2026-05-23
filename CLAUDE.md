# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**ClaudeMeow** is a macOS desktop pet app (Tauri 2 + Rust backend, HTML/JS frontend) powered by Claude AI via Amazon Bedrock. The cat reacts to what the user is doing — active app, browser URLs, time of day, and more.

## Commands

All Rust/Tauri commands run from `src-tauri/`:

```bash
# Dev run
cd src-tauri && cargo run

# Build distributable .dmg
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/ClaudeMeow.key)"
cargo tauri build
# Output: src-tauri/target/release/bundle/dmg/ClaudeMeow_x.x.x_aarch64.dmg

# Run tests
cd src-tauri && cargo test

# Run a single test
cd src-tauri && cargo test <test_name>
```

## Architecture

### Runtime (event-driven core, `src-tauri/src/runtime/`)

The runtime wires together all subsystems at startup (`main.rs → RuntimeState`):

- **`event_bus.rs`** — tokio broadcast channel carrying `MeowEvent` structs. Events are typed via `EventType` enum (e.g. `ActiveAppChanged`, `BrowserUrlChanged`). Everything communicates through this bus.
- **`state_machine.rs`** — tracks `PetState` (Idle/Coding/Social/Meeting/Sleepy/Excited/Angry) and `PetStats` (happiness, energy, hunger, love 0–100). Transitions fire on events from the bus.
- **`module_loader.rs`** — scans `modules/` at startup, reads `manifest.json` + `reactions.json` per folder, and stores parsed `ModuleManifest` / `Reaction` structs. Modules can be toggled at runtime.
- **`reaction_engine.rs`** — matches incoming `MeowEvent`s against loaded reactions (trigger + condition), applies priority + cooldown, selects a message.
- **`context_engine.rs`** — builds contextual state (time of day, activity streaks, mood) consumed by AI calls.
- **`activity_log.rs`** — appends `ActivityEntry` records to disk under `~/Library/Application Support/ClaudeMeow/`; prunes old entries on startup.
- **`commands.rs`** — Tauri command handlers exposed to the frontend. Holds `RuntimeState` (all subsystems wrapped in `Arc<RwLock<>>`). Background polling loop (`start_polling`) runs continuously in `tauri::async_runtime`.

### Data Collectors (`src-tauri/src/modules/`)

These are Rust modules that collect system data, not to be confused with the JSON reaction packs:

- `activity.rs` — reads active app name via AppleScript
- `system.rs` — CPU/memory metrics
- `spotify.rs` — current track via Spotify AppleScript API
- `weather.rs` — weather via `wttr.in`
- `zoom.rs` — Zoom/Chime meeting detection
- `status.rs` — pet stat read/write (touched, chatted, angry, fed, rested)
- `user_profile.rs` — reads/writes `~/Library/Application Support/claude-meow-pet/profiles/*.meow` (SecretMeow)
- `performance.rs`, `quotes.rs` — supplementary context

### AI (`src-tauri/src/ai/`)

- **`mod.rs`** — `send_to_bedrock()`: core function that calls Amazon Bedrock (or direct API key path). Supports both `auth_mode: "bedrock"` and `auth_mode: "apikey"`.
- **`periodic.rs`** — `generate_message` command: fires on a timer, builds context from all modules, sends to Claude, returns a short cat message.
- **`chat.rs`** — `chat_message` command: handles direct user chat (triggered by Ctrl+Cmd+C global shortcut).

### Config (`src-tauri/src/config.rs`)

Stored at `~/Library/Application Support/claude-meow-pet/config.json`. Key fields: `auth_mode` (apikey vs bedrock), `api_key`, `aws_region`, `model_id`, `persona`, `interval_minutes`, `dev_mode`.

Default model: `us.anthropic.claude-haiku-4-5-20251001-v1:0`.

### Module System (`modules/` JSON packs)

Each module folder contains:
- `manifest.json` — id, name, version, permissions, eventSubscriptions
- `reactions.json` — array of reactions with trigger (event + condition), response (messages array, priority 1–10, cooldown_minutes)

Conditions match on `field` (e.g. `app_name`, `url`) using `contains`, `equals`, or `not_contains`. Priority is 4-level: Critical(9-10) / Important(7-8) / Normal(4-6) / Low(1-3).

Modules are loaded from the app bundle's `modules/` folder by default (`default_modules_dir()` in `module_loader.rs`).

### UI (`ui/`)

Four HTML pages, each corresponds to a Tauri webview window:
- `index.html` — main pet window (the cat sprite, chat interface)
- `settings.html` — API key, persona, interval config
- `status.html` — pet stats display
- `dev.html` — floating dev console (only shown when `dev_mode: true`)

Windows are defined in `src-tauri/tauri.conf.json`. Close events are intercepted to hide rather than destroy windows.

### macOS Permissions

The app requires Accessibility and Automation (Chrome URL reading) permissions. `check_permissions()` in `main.rs` runs at startup and shows native macOS dialogs if missing.

## Key Conventions

- Tauri commands are registered in `main.rs` `invoke_handler!` and implemented as `#[tauri::command]` fns in their respective modules.
- `RuntimeState` is stored as Tauri managed state; access it in commands via `State<'_, RuntimeState>`.
- Use `eprintln!("[ClaudeMeow] ...")` for debug output (not `println!`). In commands, use `dev_log()` to emit to the dev console window.
- Config persists to disk on every `set_config` call — no batching.
- The `modules/` JSON packs ship inside the app bundle and are copied to the user's data dir on first run — do not depend on the working directory.
