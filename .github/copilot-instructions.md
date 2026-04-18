# Project Guidelines

## Architecture
- This repository is a Tauri 2 desktop app with a React + TypeScript frontend and a Rust backend.
- Frontend source lives in src and uses Tauri invoke calls to backend commands.
- Backend source lives in src-tauri/src and is split by responsibility:
  - main.rs: command handlers and scan pipeline wiring
  - rules.rs: rule engine and conflict detection
  - engine.rs: manifest generation, move execution, rollback
  - db.rs: SQLite history persistence
  - ai.rs: optional Gemini classification
  - observability.rs: file logging setup

## Build And Test
- Install dependencies: npm install
- Frontend dev server: npm run dev
- Full desktop dev app: npm run tauri dev
- Frontend build: npm run build
- Desktop package build: npm run tauri build
- Backend tests: cd src-tauri && cargo test

## Conventions
- Use the existing UI and utility patterns in src/components/ui and src/lib/utils.ts.
- Keep TypeScript imports on the @ alias when files are under src.
- Keep Rust command names stable unless frontend invoke call sites are updated in the same change.
- Treat src-tauri/target, dist, and node_modules as generated output; do not treat them as source of truth.

## Context Snapshot Maintenance
- context.md is the repository context snapshot and must be kept in sync with source and config changes.
- For every code change, update context.md in the same task before finishing.
- When adding, removing, or renaming files, update the File Inventory section in context.md.
- When behavior, commands, or architecture change, update the relevant summary sections in context.md.