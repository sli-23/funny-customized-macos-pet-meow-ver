# ClaudeMeow

<p align="center">
    <img src="assets/image_5_v11.png" alt="alt text" width="100%">
</p>

**ClaudeMeow** is the cat version of Claude mascot. It is a lightweight macOS desktop pet. Inspired by the original Claude mascot, ClaudeMeow reimagines the design with cat-like features and behaviors while preserving its recognizable charm. It quietly lives on your desktop, bringing a little personality, comfort, and chaos to your workspace.

<p align="center">
    <img src="assets/image_3_v11.png" alt="alt text" width="40%">
    <img src="assets/image_4_v11.png" alt="alt text" width="55%">
</p>

## Download

**[Download ClaudeMeow.dmg](https://github.com/sli-23/funny-customized-macos-pet-meow-ver/releases/latest)** (macOS, Apple Silicon)

1. Download the `.dmg` from Releases
2. Open it and drag ClaudeMeow to Applications
3. Launch ClaudeMeow
4. Grant Accessibility permission when prompted (System Settings > Privacy & Security > Accessibility)
5. Set your Bedrock API key in Settings

Updates are built-in — check for new versions in Settings > Updates.

---

## Features (v1.3)

- **Module System** — drop-in reaction packs like game mods
- **Browser URL Detection** — reacts to GitHub, YouTube, Amazon internal sites
- **Context Engine** — pet adapts personality based on time, activity streaks, mood
- **4-Level Priority System** — Critical / Important / Normal / Low
- **Health Reminders** — posture, water, Monster drinks, breaks (every 45 min)
- **Floating Score Numbers** — pixel font +3/-15 above pet
- **Dev Console** — floating terminal-style event log
- **Screen Time Tracking** — warns after 4h on same app
- **10 Modules** — Coding, Slack, Zoom, Spotify, GitHub, YouTube, Social, Weather, Health + external
- **SecretMeow** — .meow files from friends with secret messages
- **Auto-Update** — check for updates in Settings
- **AI Personality** — powered by Claude via Amazon Bedrock

## Modules

Drop module folders into `modules/` to add new reactions:

```
modules/
  coding/         <- reacts to IDEs
  slack/          <- reacts to Slack
  zoom/           <- reacts to Zoom/Chime
  spotify/        <- reacts to Spotify
  github/         <- reacts to GitHub URLs
  youtube/        <- reacts to YouTube
  social/         <- reacts to Reddit/Twitter/Bilibili
  weather/        <- weather comments
  health/         <- posture/water reminders
  your-module/    <- create your own!
```

Each module is a folder with `manifest.json` + `reactions.json`. See [documentations/](documentations/) for details.

---

## Development

### Prerequisites

- macOS (Apple Silicon)
- Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Tauri CLI: `cargo install tauri-cli`

### Run (dev mode)

```bash
git clone https://github.com/sli-23/funny-customized-macos-pet-meow-ver.git
cd funny-customized-macos-pet-meow-ver/src-tauri
cargo run
```

### Build .dmg

```bash
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/ClaudeMeow.key)"
cargo tauri build
```

Output: `src-tauri/target/release/bundle/dmg/ClaudeMeow_x.x.x_aarch64.dmg`

### Tests

```bash
cd src-tauri && cargo test
```

---

## Architecture

```
src-tauri/src/
  runtime/           <- event-driven runtime (v1.3)
    event_bus.rs
    state_machine.rs
    module_loader.rs
    reaction_engine.rs
    context_engine.rs
    activity_log.rs
    commands.rs
  modules/           <- Rust data collectors
  ai/                <- Claude AI integration
modules/             <- JSON reaction packs (drop-in)
ui/                  <- Frontend (HTML/JS)
documentations/      <- System docs
```

## License

MIT
