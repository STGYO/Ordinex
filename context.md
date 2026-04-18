# FileSorter Context Snapshot

This file is a generated working context for coding sessions. It is meant to avoid re-discovering project structure in every prompt.

Generated: 2026-04-18 18:10:00 +05:30
Scope: all files under repository root, excluding generated artifacts in src-tauri/target, node_modules, dist, and .git
Total scoped files: 108

## Commands

- dev: vite
- build: tsc && vite build
- preview: vite preview
- tauri: tauri

## Frontend Stack

- react: ^19.1.0
- react-dom: ^19.1.0
- @tauri-apps/api: ^2.10.1
- @tauri-apps/plugin-dialog: ^2.6.0
- recharts: ^3.7.0
- tailwindcss-animate: ^1.0.7
- typescript: ~5.8.3
- vite: ^7.0.4
- @vitejs/plugin-react: ^4.6.0
- @tauri-apps/cli: ^2
- tailwindcss: ^3.4.19

## Backend Stack (Cargo.toml)

- tauri
- tauri-plugin-opener
- tauri-plugin-dialog
- tokio
- reqwest
- rayon
- walkdir
- rusqlite
- sha2
- simplelog

## README Highlights

- New Architecture Features
- Rule Configuration
- Advanced Scan Options
- Prerequisites
- Install
- Run in development
- Build frontend bundle
- Run backend tests
- Package desktop app
- AI classification setup
- Notes

## Architecture Summary

- Desktop app built with Tauri 2: React TypeScript frontend + Rust backend commands.
- Frontend invokes backend commands for scan, rule config, execution, and rollback.
- Frontend also invokes AI settings commands for provider config, model listing, and provider validation.
- Rule engine and file move transaction logic live in src-tauri/src/rules.rs and src-tauri/src/engine.rs.
- AI classification supports multiple providers (Gemini, OpenAI, Anthropic, Ollama, OpenRouter-compatible) with provider-specific credentials stored in OS credential vault.
- Scan flow is AI-first with per-file rules fallback, then Unknown fallback for unmatched files.
- History persists to SQLite via src-tauri/src/db.rs; logs via src-tauri/src/observability.rs.

## File Inventory

### .github/copilot-instructions.md

- type: text
- lines: 32
- first_non_empty: # Project Guidelines
- key_items: # Project Guidelines, # Architecture, # Build And Test, # Conventions, # Context Snapshot Maintenance

### .github/hooks/pre-commit-context-check.json

- type: text
- lines: 11
- first_non_empty: {
- key_items: key:hooks

### .github/hooks/scripts/pre-commit-context-check.js

- type: text
- lines: 157
- first_non_empty: #!/usr/bin/env node
- key_items: readStdin, parseJson, safeString, getCommand, emitDecision, runGit, isSourcePath, main

### .github/prompts/update-context.prompt.md

- type: text
- lines: 21
- first_non_empty: ---

### .gitignore

- type: text
- lines: 25
- first_non_empty: # Logs

### .vscode/extensions.json

- type: text
- lines: 4
- first_non_empty: {
- key_items: key:recommendations

### .vscode/settings.json

- type: text
- lines: 5
- first_non_empty: {
- key_items: key:chat.tools.terminal.autoApprove

### components.json

- type: text
- lines: 24
- first_non_empty: {
- key_items: key:$schema, key:style, key:rsc, key:tsx, key:tailwind, key:iconLibrary, key:rtl, key:aliases

### context.md

- type: text
- lines: 368
- first_non_empty: # FileSorter Context Snapshot
- key_items: # FileSorter Context Snapshot, # Commands, # Frontend Stack, # Backend Stack (Cargo.toml), # README Highlights, # Architecture Summary, # File Inventory, # .github/copilot-instructions.md

### index.html

- type: text
- lines: 15
- first_non_empty: &lt;!doctype html&gt;

### Ordinex.png

- type: binary-asset
- bytes: 185924

### Ordinex.svg

- type: text
- lines: 30
- first_non_empty: &lt;svg xmlns="<http://www.w3.org/2000/svg>" width="250" height="250" fill="none" viewBox="0 0 250 250"&gt;

### package.json

- type: text
- lines: 43
- first_non_empty: {
- key_items: key:name, key:private, key:version, key:type, key:scripts, key:dependencies, key:devDependencies

### package-lock.json

- type: text
- lines: 4041
- first_non_empty: {

### postcss.config.js

- type: text
- lines: 7
- first_non_empty: export default {

### public/tauri.svg

- type: text
- lines: 7

### public/vite.svg

- type: text
- lines: 1

### README.md

- type: text
- lines: 96
- first_non_empty: # FileSorter AI (Tauri + React + Rust)
- key_items: # FileSorter AI (Tauri + React + Rust), # New Architecture Features, # Rule Configuration, # Advanced Scan Options, # Prerequisites, # Install, # Run in development, # Build frontend bundle

### src/App.css

- type: text
- lines: 117
- first_non_empty: .logo.vite:hover {

### src/App.tsx

- type: text
- lines: 947
- first_non_empty: import { Fragment, lazy, Suspense, useEffect, useState } from "react";
- key_items: AnalyticsCharts

### src/assets/react.svg

- type: text
- lines: 1

### src/components/analytics-charts.tsx

- type: text
- lines: 108
- first_non_empty: import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
- key_items: AnalyticsCharts, COLORS

### src/components/ui/button.tsx

- type: text
- lines: 58
- first_non_empty: import * as React from "react"
- key_items: buttonVariants, Button

### src/components/ui/card.tsx

- type: text
- lines: 77
- first_non_empty: import * as React from "react"
- key_items: Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter

### src/components/ui/dialog.tsx

- type: text
- lines: 121
- first_non_empty: import * as React from "react"
- key_items: Dialog, DialogTrigger, DialogPortal, DialogClose, DialogOverlay, DialogContent, DialogHeader, DialogFooter

### src/components/ui/input.tsx

- type: text
- lines: 23
- first_non_empty: import * as React from "react"
- key_items: Input

### src/components/ui/label.tsx

- type: text
- lines: 25
- first_non_empty: import * as React from "react"
- key_items: labelVariants, Label

### src/components/ui/scroll-area.tsx

- type: text
- lines: 47
- first_non_empty: import * as React from "react"
- key_items: ScrollArea, ScrollBar

### src/components/ui/separator.tsx

- type: text
- lines: 32
- first_non_empty: "use client"
- key_items: Separator

### src/components/ui/table.tsx

- type: text
- lines: 121
- first_non_empty: import * as React from "react"
- key_items: Table, TableHeader, TableBody, TableFooter, TableRow, TableHead, TableCell, TableCaption

### src/index.css

- type: text
- lines: 92
- first_non_empty: @tailwind base;

### src/lib/utils.ts

- type: text
- lines: 7
- first_non_empty: import { clsx, type ClassValue } from "clsx"
- key_items: cn

### src/main.tsx

- type: text
- lines: 11
- first_non_empty: import React from "react";

### src/vite-env.d.ts

- type: text
- lines: 2
- first_non_empty: /// &lt;reference types="vite/client" /&gt;

### src-tauri/.gitignore

- type: text
- lines: 8
- first_non_empty: # Generated by Cargo

### src-tauri/build.rs

- type: text
- lines: 4
- first_non_empty: fn main() {
- key_items: main

### src-tauri/capabilities/default.json

- type: text
- lines: 12
- first_non_empty: {
- key_items: key:$schema, key:identifier, key:description, key:windows, key:permissions

### src-tauri/Cargo.lock

- type: text
- lines: 5925
- first_non_empty: # This file is automatically @generated by Cargo.

### src-tauri/Cargo.toml

- type: text
- lines: 39
- first_non_empty: [package]
- key_items: section:package, section:lib, section:build-dependencies, section:dependencies, section:dev-dependencies

### src-tauri/gen/schemas/acl-manifests.json

- type: text
- lines: 1
- first_non_empty: {"core":{"default_permission":{"identifier":"default","description":"Default core plugins set.","permissions":["core:pat...
- key_items: key:core, key:core:app, key:core:event, key:core:image, key:core:menu, key:core:path, key:core:resources, key:core:tray

### src-tauri/gen/schemas/capabilities.json

- type: text
- lines: 1
- first_non_empty: {"default":{"identifier":"default","description":"Capability for the main window","local":true,"windows":["main"],"permi...
- key_items: key:default

### src-tauri/gen/schemas/desktop-schema.json

- type: text
- lines: 2543
- first_non_empty: {
- key_items: key:$schema, key:title, key:description, key:anyOf, key:definitions

### src-tauri/gen/schemas/windows-schema.json

- type: text
- lines: 2543
- first_non_empty: {
- key_items: key:$schema, key:title, key:description, key:anyOf, key:definitions

### src-tauri/icons/128x128.png

- type: binary-asset
- bytes: 3512

### src-tauri/icons/128x128@2x.png

- type: binary-asset
- bytes: 7012

### src-tauri/icons/32x32.png

- type: binary-asset
- bytes: 974

### src-tauri/icons/64x64.png

- type: binary-asset
- bytes: 2164

### src-tauri/icons/android/

- type: generated-directory
- files: 17

### src-tauri/icons/icon.icns

- type: binary-asset
- bytes: 98451

### src-tauri/icons/icon.ico

- type: binary-asset
- bytes: 86642

### src-tauri/icons/icon.png

- type: binary-asset
- bytes: 14183

### src-tauri/icons/ios/

- type: generated-directory
- files: 18

### src-tauri/icons/Square107x107Logo.png

- type: binary-asset
- bytes: 2863

### src-tauri/icons/Square142x142Logo.png

- type: binary-asset
- bytes: 3858

### src-tauri/icons/Square150x150Logo.png

- type: binary-asset
- bytes: 3966

### src-tauri/icons/Square284x284Logo.png

- type: binary-asset
- bytes: 7737

### src-tauri/icons/Square30x30Logo.png

- type: binary-asset
- bytes: 903

### src-tauri/icons/Square310x310Logo.png

- type: binary-asset
- bytes: 8591

### src-tauri/icons/Square44x44Logo.png

- type: binary-asset
- bytes: 1299

### src-tauri/icons/Square71x71Logo.png

- type: binary-asset
- bytes: 2011

### src-tauri/icons/Square89x89Logo.png

- type: binary-asset
- bytes: 2468

### src-tauri/icons/StoreLogo.png

- type: binary-asset
- bytes: 1523

### src-tauri/src/ai.rs

- type: text
- lines: 184
- first_non_empty: use crate::FileNode;
- key_items: classify_files_with_ai, Part, Content, SystemInstruction, GenerationConfig, GeminiRequest, AIClassificationResult, GeminiResponse

### src-tauri/src/db.rs

- type: text
- lines: 77
- first_non_empty: use crate::engine::TransactionManifest;
- key_items: init_db, save_manifest, fetch_history

### src-tauri/src/engine.rs

- type: text
- lines: 457
- first_non_empty: use serde::{Deserialize, Serialize};
- key_items: is_protected, move_file_with_fallback, hash_file, generate_manifest, execute_manifest, undo_manifest, MoveOperation, ExecutionSummary

### src-tauri/src/lib.rs

- type: text
- lines: 15
- first_non_empty: // Learn more about Tauri commands in this inline module comment.
- key_items: greet, run

### src-tauri/src/main.rs

- type: text
- lines: 434
- first_non_empty: // Prevents additional console window on Windows in release, DO NOT REMOVE!!
- key_items: should_skip_entry, apply_ai_results, scan_internal, execute_moves, undo_moves, fetch_history, scan_directory, scan_directory_advanced

### src-tauri/src/observability.rs

- type: text
- lines: 53
- first_non_empty: use simplelog::{CombinedLogger, ConfigBuilder, LevelFilter, WriteLogger};
- key_items: init_logging, log_path

### src-tauri/src/rules.rs

- type: text
- lines: 480
- first_non_empty: use serde::{Deserialize, Serialize};
- key_items: default_enabled, default_action, default_unknown_folder, default_stop_on_match, norm_extension, lower_vec, extract_extensions, detect_conflicts

### src-tauri/tauri.conf.json

- type: text
- lines: 36
- first_non_empty: {
- key_items: key:$schema, key:productName, key:version, key:identifier, key:build, key:app, key:bundle

### tailwind.config.js

- type: text
- lines: 61
- first_non_empty: /** @type {import('tailwindcss').Config} */

### tsconfig.json

- type: text
- lines: 38
- first_non_empty: {

### tsconfig.node.json

- type: text
- lines: 11
- first_non_empty: {
- key_items: key:compilerOptions, key:include

### tsconfig.path.json

- type: text
- lines: 2
- first_non_empty: {" "compilerOptions:{baseUrl:.,paths:{@/*:[./src/*]}}}

### vite.config.ts

- type: text
- lines: 39
- first_non_empty: import { defineConfig } from "vite";
- key_items: host

## Update Policy

- This file should be updated whenever source, config, or architectural behavior changes.
- Keep generated-artifact directories excluded unless explicitly needed for debugging.
