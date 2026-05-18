# ClaudeMeow v1.1

A pixel art cat desktop pet powered by Amazon Bedrock.

## Prerequisites

- macOS
- [Rust](https://rustup.rs/)
- [Tauri CLI](https://tauri.app/): `cargo install tauri-cli`
- An Amazon Bedrock API key (ABSK format)

## Run

```bash
cargo tauri dev
```

## Setup

1. Launch the app — the cat appears on your desktop
2. Right-click the tray icon → **Settings**
3. Paste your Bedrock API key → click **Test**
4. If successful, settings auto-save
5. Press **Ctrl+Cmd+C** anywhere to chat with your pet
