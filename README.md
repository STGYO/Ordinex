# FileSorter AI (Tauri + React + Rust)

Desktop app to scan a folder, classify files (AI-first + rules fallback), preview moves, execute moves, and rollback with history.

## New Architecture Features

- External JSON rule engine stored in app data as `rules.json` (auto-created on first run)
- Priority-based conflict resolution (`stop_on_match` / first-match-wins)
- Compound extension support (for example `.tar.gz`)
- Rule preview command (`preview_rule_matches`) for simulation of rule scope
- Advanced scan command (`scan_directory_advanced`) with metrics + conflict reporting
- Dry-run execution (`execute_moves` with optional `dry_run: true`) that logs simulated actions
- Structured file action logging in app data log file (`filesorter.log`)

## Rule Configuration

- Load config: Tauri command `get_rule_config`
- Save config: Tauri command `save_rule_config_cmd`
- Rule fields include:
  - `id`, `name`, `enabled`, `priority`
  - `category_path`, `destination_folder`, `action`
  - `conditions.extensions`, `conditions.filename_keywords`, `conditions.min_size_bytes`, `conditions.max_size_bytes`
- Toggle rules on/off using `enabled`.
- Resolve overlaps by reordering `priority`.

## Advanced Scan Options

Use `scan_directory_advanced` with optional payload:

- `recursive: bool`
- `max_depth: number`
- `include_hidden: bool`
- `exclude_patterns: string[]`
- `enable_ai: bool`
- `progress_log_every: number`

Returns:

- `files` (classification preview / dry-run plan)
- `metrics` (throughput, elapsed, classified counts)
- `conflicts` (overlapping extension rules)

## Prerequisites

- Node.js 20+
- Rust stable toolchain (`rustup`)
- Tauri v2 prerequisites for Windows

## Install

```powershell
npm install
```

## Run in development

```powershell
npm.cmd run tauri dev
```

## Build frontend bundle

```powershell
npm.cmd run build
```

## Run backend tests

```powershell
cd src-tauri
cargo test
```

## Package desktop app

```powershell
npm.cmd run tauri build
```

## Theme behavior

- The app now defaults to `dark` theme on first launch.
- Users can switch between dark/light themes from the top header theme toggle.
- Theme preference is persisted locally under `ordinex-theme`.
- Warning UI (including dry-run safety notices) uses semantic warning tokens for readability in both themes.

## Automatic updater workflow

- On startup, the app checks for updates from GitHub Releases:
  - `https://github.com/STGYO/Ordinex/releases/latest/download/latest.json`
- If an update is found, it is downloaded automatically in the background.
- Once download completes, the UI prompts the user to install/restart.
- Updater permissions are enabled through Tauri capabilities (`updater:default`).

## Updater signing and release setup

Tauri updater signatures are mandatory. Generate keys once, then configure CI secrets.

Generate keys (run locally once):

```powershell
npm run tauri signer generate -- -w "$env:USERPROFILE\\.tauri\\ordinex.key"
```

Configure these GitHub repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY`: private key contents (or path content)
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: private key password (if set)
- `TAURI_UPDATER_PUBLIC_KEY`: public key contents to embed at build time

Release automation:

- Workflow file: `.github/workflows/release-tauri.yml`
- Trigger on tag pushes (`v*`) or manual dispatch.
- Builds signed artifacts and uploads updater metadata (`latest.json`) to GitHub Releases.

## AI classification setup

AI configuration is now done inside the app:

- Use the top header AI indicator button on any page.
- Green indicator: selected provider validated successfully.
- Red indicator: selected provider not validated or unavailable.
- Configure:
  - `Model Provider` (Gemini, OpenAI, Anthropic, Ollama, OpenRouter-Compatible)
  - `API Key` (stored per provider in OS credential vault)
  - `Model Name` (loaded live from provider API)
  - `Base URL` (for Ollama and OpenRouter-Compatible endpoints)

Sorting behavior:

- AI sorting runs first when enabled.
- Normal rules-based sorting is applied as fallback for files AI does not classify.
- Remaining unmatched files still fall back to `Unknown` / `Needs Sorting`.

## Notes

- Action history is stored in the app data folder (`history.db`).
- AI settings are stored in app data (`ai_settings.json`) and provider secrets in OS credential vault.
- Rule configuration is stored in app data (`rules.json`).
- App updates preserve app data directory contents (`history.db`, `ai_settings.json`, `rules.json`).
- Undo updates existing transaction history entries rather than duplicating IDs.
- Move operations handle cross-drive scenarios with copy+delete fallback.
- Logs are stored in the app data folder (`filesorter.log`).
