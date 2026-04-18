// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod db;
mod engine;
mod observability;
mod rules;

use ai::AIClassificationResult;
use engine::TransactionManifest;
use log::{error, info, warn};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub extension: Option<String>,
    pub parent_folder: Option<String>,
    pub modified_unix_ms: Option<u64>,
    pub created_unix_ms: Option<u64>,
    pub content_snippet: Option<String>,
    pub category: Option<String>,
    pub suggested_folder: Option<String>,
    pub matched_rule_id: Option<String>,
    pub matched_rule_name: Option<String>,
    pub planned_action: Option<String>,
    pub ai_confidence: Option<f32>,
    pub ai_reason: Option<String>,
    pub ai_top_level_category: Option<String>,
    pub ai_semantic_subfolder: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScanOptions {
    pub recursive: Option<bool>,
    pub max_depth: Option<usize>,
    pub include_hidden: Option<bool>,
    pub exclude_patterns: Option<Vec<String>>,
    pub enable_ai: Option<bool>,
    pub ai_first_with_fallback: Option<bool>,
    pub complete_ai_sorting: Option<bool>,
    pub progress_log_every: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ScanMetrics {
    pub total_entries_seen: usize,
    pub files_seen: usize,
    pub files_classified_by_rules: usize,
    pub files_classified_by_ai: usize,
    pub files_unknown: usize,
    pub files_with_content_snippet: usize,
    pub files_metadata_fallback: usize,
    pub elapsed_ms: u128,
    pub throughput_files_per_sec: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScanResponse {
    pub files: Vec<FileNode>,
    pub metrics: ScanMetrics,
    pub conflicts: Vec<rules::RuleConflict>,
}

fn should_skip_entry(name: &str, include_hidden: bool, exclude_patterns: &[String]) -> bool {
    if !include_hidden && name.starts_with('.') {
        return true;
    }

    let lower = name.to_ascii_lowercase();
    exclude_patterns
        .iter()
        .map(|p| p.to_ascii_lowercase())
        .any(|pattern| !pattern.trim().is_empty() && lower.contains(&pattern))
}

const TOP_LEVEL_CATEGORIES: [&str; 8] = [
    "Work",
    "Personal",
    "Finance",
    "Media",
    "Code",
    "Academic",
    "Projects",
    "Unknown",
];

const AI_LOW_CONFIDENCE_THRESHOLD: f32 = 0.55;
const CONTENT_SNIPPET_MAX_BYTES: usize = 4096;
const CONTENT_SNIPPET_MAX_FILE_SIZE: u64 = 2 * 1024 * 1024;

fn to_unix_ms(value: std::io::Result<SystemTime>) -> Option<u64> {
    value
        .ok()
        .and_then(|ts| ts.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
}

fn extract_parent_folder(path: &Path) -> Option<String> {
    path.parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
}

fn should_extract_text_snippet(extension: Option<&str>) -> bool {
    let ext = extension.unwrap_or_default().to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "txt"
            | "md"
            | "csv"
            | "json"
            | "toml"
            | "yaml"
            | "yml"
            | "xml"
            | "log"
            | "rs"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "py"
            | "java"
            | "kt"
            | "go"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "cs"
            | "swift"
            | "rb"
            | "php"
            | "sh"
            | "ps1"
    )
}

fn extract_light_content_snippet(path: &Path, extension: Option<&str>, size: u64) -> Option<String> {
    if size == 0 || size > CONTENT_SNIPPET_MAX_FILE_SIZE || !should_extract_text_snippet(extension) {
        return None;
    }

    let mut file = File::open(path).ok()?;
    let mut buffer = vec![0u8; CONTENT_SNIPPET_MAX_BYTES];
    let bytes_read = file.read(&mut buffer).ok()?;
    if bytes_read == 0 {
        return None;
    }

    buffer.truncate(bytes_read);
    if buffer.contains(&0) {
        return None;
    }

    let snippet = String::from_utf8_lossy(&buffer);
    let compact = snippet
        .split_whitespace()
        .take(200)
        .collect::<Vec<_>>()
        .join(" ");
    if compact.is_empty() {
        None
    } else {
        Some(compact)
    }
}

fn normalize_top_level_category(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Unknown".to_string();
    }

    for category in TOP_LEVEL_CATEGORIES {
        if category.eq_ignore_ascii_case(trimmed) {
            return category.to_string();
        }
    }

    "Unknown".to_string()
}

fn sanitize_folder_segment(value: &str, fallback: &str) -> String {
    let filtered = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();

    let collapsed = filtered
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    if collapsed.is_empty() {
        fallback.to_string()
    } else {
        collapsed
    }
}

fn semantic_subfolder_for_result(result: &AIClassificationResult) -> String {
    if result.confidence < AI_LOW_CONFIDENCE_THRESHOLD {
        return "Needs Review".to_string();
    }

    let raw = result
        .semantic_subfolder
        .as_deref()
        .or_else(|| {
            if result.suggested_folder_name.trim().is_empty() {
                None
            } else {
                Some(result.suggested_folder_name.as_str())
            }
        })
        .unwrap_or("General");

    sanitize_folder_segment(raw, "General")
}

fn apply_ai_results(files: &mut [FileNode], ai_results: Vec<AIClassificationResult>) -> usize {
    let mut mapping: HashMap<String, Vec<AIClassificationResult>> = HashMap::new();
    for res in ai_results {
        mapping
            .entry(res.filename.to_ascii_lowercase())
            .or_default()
            .push(res);
    }
    let mut count = 0usize;

    for file in files.iter_mut() {
        if file.is_dir || file.category.is_some() {
            continue;
        }
        if let Some(decisions) = mapping.get_mut(&file.name.to_ascii_lowercase()) {
            let Some(ai_decision) = decisions.pop() else {
                continue;
            };
            let top_category = normalize_top_level_category(
                ai_decision
                    .top_level_category
                    .as_deref()
                    .unwrap_or(ai_decision.category.as_str()),
            );
            let semantic_subfolder = semantic_subfolder_for_result(&ai_decision);

            file.category = Some(top_category.clone());
            file.suggested_folder = Some(format!("{}/{}", top_category, semantic_subfolder));
            file.matched_rule_id = Some("ai-generated".to_string());
            file.matched_rule_name = Some("AI Semantic Planner".to_string());
            file.planned_action = Some("move".to_string());
            file.ai_confidence = Some(ai_decision.confidence);
            file.ai_reason = ai_decision.reason;
            file.ai_top_level_category = Some(top_category);
            file.ai_semantic_subfolder = Some(semantic_subfolder);
            count += 1;
        }
    }

    count
}

fn build_ai_folder_hints(
    files: &[FileNode],
    config: &rules::RuleConfig,
    include_rule_hints: bool,
) -> HashSet<String> {
    let mut folder_hints: HashSet<String> = files
        .iter()
        .filter_map(|f| f.suggested_folder.clone())
        .collect();
    folder_hints.insert(config.unknown_folder.clone());
    if include_rule_hints {
        for rule in &config.rules {
            if !rule.destination_folder.trim().is_empty() {
                folder_hints.insert(rule.destination_folder.clone());
            }
        }
    }
    folder_hints
}

async fn classify_unresolved_with_ai(
    files: &mut [FileNode],
    config: &rules::RuleConfig,
    ai_settings: &ai::AISettings,
    include_rule_hints: bool,
) -> usize {
    let mut ai_candidates: Vec<FileNode> = files
        .iter()
        .filter(|f| !f.is_dir && f.category.is_none())
        .cloned()
        .collect();

    if ai_candidates.is_empty() {
        return 0;
    }

    let mut folder_hints = build_ai_folder_hints(files, config, include_rule_hints);
    let mut files_classified_by_ai = 0usize;
    let chunk_size = 50usize;

    for chunk in ai_candidates.chunks_mut(chunk_size) {
        let folder_vec: Vec<String> = folder_hints.iter().cloned().collect();
        match ai::classify_files_with_ai(chunk, &folder_vec, ai_settings).await {
            Ok(results) => {
                for result in &results {
                    folder_hints.insert(result.suggested_folder_name.clone());
                }
                files_classified_by_ai += apply_ai_results(files, results);
            }
            Err(err) => {
                warn!("ai_classification_chunk_failed error='{}'", err);
            }
        }
    }

    files_classified_by_ai
}

async fn scan_internal(
    app: &tauri::AppHandle,
    path: &str,
    options: Option<ScanOptions>,
) -> Result<ScanResponse, String> {
    let started = Instant::now();

    let opts = options.unwrap_or(ScanOptions {
        recursive: Some(false),
        max_depth: Some(1),
        include_hidden: Some(false),
        exclude_patterns: Some(vec!["node_modules".to_string(), "target".to_string()]),
        enable_ai: Some(true),
        ai_first_with_fallback: Some(true),
        complete_ai_sorting: Some(false),
        progress_log_every: Some(5000),
    });

    let recursive = opts.recursive.unwrap_or(false);
    let max_depth = if recursive {
        opts.max_depth.unwrap_or(usize::MAX)
    } else {
        1
    };
    let include_hidden = opts.include_hidden.unwrap_or(false);
    let exclude_patterns = opts.exclude_patterns.unwrap_or_default();
    let ai_settings = ai::load_or_init_ai_settings(app)?;
    let enable_ai = opts.enable_ai.unwrap_or(true) && ai_settings.enabled;
    let ai_first_with_fallback = opts
        .ai_first_with_fallback
        .unwrap_or(ai_settings.ai_first_with_fallback);
    let complete_ai_sorting = opts
        .complete_ai_sorting
        .unwrap_or(ai_settings.complete_ai_sorting);
    let progress_every = opts.progress_log_every.unwrap_or(5000).max(1);

    let config = rules::load_or_init_rule_config(app)?;
    let conflicts = rules::detect_conflicts(&config);
    if !conflicts.is_empty() {
        warn!(
            "Detected {} overlapping extension conflict(s) in rule configuration",
            conflicts.len()
        );
    }
    let engine = rules::RuleEngine::new(&config);

    info!(
        "scan_started path='{}' recursive={} max_depth={} include_hidden={} enable_ai={} ai_first_with_fallback={} complete_ai_sorting={}",
        path,
        recursive,
        max_depth,
        include_hidden,
        enable_ai,
        ai_first_with_fallback,
        complete_ai_sorting
    );

    let entries: Vec<_> = WalkDir::new(path)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| {
            let file_name = entry.file_name().to_string_lossy();
            !should_skip_entry(&file_name, include_hidden, &exclude_patterns)
        })
        .collect();

    let total_entries_seen = entries.len();
    info!("scan_entries_collected count={}", total_entries_seen);

    let mut files: Vec<FileNode> = entries
        .into_par_iter()
        .filter_map(|entry| {
            if entry.path().to_string_lossy() == path {
                return None;
            }

            let metadata = entry.metadata().ok()?;
            let extension = entry.path().extension().map(|e| e.to_string_lossy().to_string());
            let parent_folder = extract_parent_folder(entry.path());
            let modified_unix_ms = to_unix_ms(metadata.modified());
            let created_unix_ms = to_unix_ms(metadata.created());
            let content_snippet = if metadata.is_file() {
                extract_light_content_snippet(entry.path(), extension.as_deref(), metadata.len())
            } else {
                None
            };
            let node = FileNode {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry.path().to_string_lossy().to_string(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                extension,
                parent_folder,
                modified_unix_ms,
                created_unix_ms,
                content_snippet,
                category: None,
                suggested_folder: None,
                matched_rule_id: None,
                matched_rule_name: None,
                planned_action: None,
                ai_confidence: None,
                ai_reason: None,
                ai_top_level_category: None,
                ai_semantic_subfolder: None,
            };
            Some(node)
        })
        .collect();

    let files_seen = files.iter().filter(|f| !f.is_dir).count();
    let files_classified_by_rules = 0usize;

    if files_seen > 0 {
        info!(
            "scan_progress files_seen={} classified_by_rules={} interval={}",
            files_seen,
            files_classified_by_rules,
            progress_every
        );
    }

    let mut files_classified_by_ai = 0usize;
    if enable_ai && (ai_first_with_fallback || complete_ai_sorting) {
        files_classified_by_ai +=
            classify_unresolved_with_ai(&mut files, &config, &ai_settings, !complete_ai_sorting)
                .await;
    }

    if !complete_ai_sorting || !enable_ai {
        for file in files.iter_mut() {
            if file.is_dir || file.category.is_some() {
                continue;
            }
            if let Some(rule_match) = engine.evaluate(&file.name, file.size) {
                file.category = Some(rule_match.category_path.clone());
                file.suggested_folder = Some(rule_match.destination_folder.clone());
                file.matched_rule_id = Some(rule_match.rule_id.clone());
                file.matched_rule_name = Some(rule_match.rule_name.clone());
                file.planned_action = Some(format!("{:?}", rule_match.action).to_ascii_lowercase());
            }
        }
    }

    if enable_ai && !ai_first_with_fallback && !complete_ai_sorting {
        files_classified_by_ai += classify_unresolved_with_ai(&mut files, &config, &ai_settings, true).await;
    }

    for file in files.iter_mut() {
        if !file.is_dir && file.category.is_none() {
            file.category = Some("Unknown".to_string());
            file.suggested_folder = Some(config.unknown_folder.clone());
            file.matched_rule_name = Some("No Matching Rule".to_string());
            file.planned_action = Some("move".to_string());
        }
    }

    let files_unknown = files
        .iter()
        .filter(|f| !f.is_dir && f.matched_rule_name.as_deref() == Some("No Matching Rule"))
        .count();

    let files_classified_by_rules = files
        .iter()
        .filter(|f| {
            !f.is_dir
                && f.matched_rule_id.is_some()
                && f.matched_rule_id.as_deref() != Some("ai-generated")
        })
        .count();

    let files_with_content_snippet = files
        .iter()
        .filter(|f| !f.is_dir && f.content_snippet.is_some())
        .count();
    let files_metadata_fallback = files_seen.saturating_sub(files_with_content_snippet);

    files.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let elapsed_ms = started.elapsed().as_millis();
    let throughput_files_per_sec = if elapsed_ms == 0 {
        files_seen as f64
    } else {
        (files_seen as f64) / (elapsed_ms as f64 / 1000.0)
    };

    let metrics = ScanMetrics {
        total_entries_seen,
        files_seen,
        files_classified_by_rules,
        files_classified_by_ai,
        files_unknown,
        files_with_content_snippet,
        files_metadata_fallback,
        elapsed_ms,
        throughput_files_per_sec,
    };

    info!(
        "scan_completed files_seen={} classified_by_rules={} classified_by_ai={} unknown={} elapsed_ms={} throughput={:.2}",
        metrics.files_seen,
        metrics.files_classified_by_rules,
        metrics.files_classified_by_ai,
        metrics.files_unknown,
        metrics.elapsed_ms,
        metrics.throughput_files_per_sec
    );

    Ok(ScanResponse {
        files,
        metrics,
        conflicts,
    })
}

#[tauri::command]
async fn execute_moves(
    app: tauri::AppHandle,
    path: String,
    files: Vec<FileNode>,
    dry_run: Option<bool>,
) -> Result<TransactionManifest, String> {
    let manifest = engine::generate_manifest(&path, &files);
    let is_dry_run = dry_run.unwrap_or(false);
    match engine::execute_manifest(manifest, is_dry_run) {
        Ok(completed_manifest) => {
            if let Err(e) = db::save_manifest(&app, &completed_manifest) {
                error!("Failed to save manifest: {}", e);
            }
            Ok(completed_manifest)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn undo_moves(
    app: tauri::AppHandle,
    manifest: TransactionManifest,
) -> Result<TransactionManifest, String> {
    // Reverse operations on the file system
    let reversed = engine::undo_manifest(manifest)?;

    // Update the DB to record the rollback state
    if let Err(e) = db::save_manifest(&app, &reversed) {
        error!("Failed to update manifest state on undo: {}", e);
    }

    Ok(reversed)
}

#[tauri::command]
async fn fetch_history(app: tauri::AppHandle) -> Result<Vec<TransactionManifest>, String> {
    db::fetch_history(&app).map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_directory(app: tauri::AppHandle, path: String) -> Result<Vec<FileNode>, String> {
    let response = scan_internal(&app, &path, None).await?;
    Ok(response.files)
}

#[tauri::command]
async fn scan_directory_advanced(
    app: tauri::AppHandle,
    path: String,
    options: Option<ScanOptions>,
) -> Result<ScanResponse, String> {
    scan_internal(&app, &path, options).await
}

#[tauri::command]
async fn get_rule_config(app: tauri::AppHandle) -> Result<rules::RuleConfig, String> {
    rules::load_or_init_rule_config(&app)
}

#[tauri::command]
async fn get_ai_settings(app: tauri::AppHandle) -> Result<ai::AISettingsEnvelope, String> {
    ai::get_ai_settings_view(&app)
}

#[tauri::command]
async fn save_ai_settings_cmd(
    app: tauri::AppHandle,
    request: ai::SaveAISettingsRequest,
) -> Result<ai::AISettingsEnvelope, String> {
    ai::save_ai_settings_view(&app, request)
}

#[tauri::command]
async fn list_ai_models(
    app: tauri::AppHandle,
    request: ai::ProviderModelsRequest,
) -> Result<Vec<String>, String> {
    ai::list_provider_models(&app, request).await
}

#[tauri::command]
async fn validate_ai_provider(
    app: tauri::AppHandle,
    request: ai::ProviderValidationRequest,
) -> Result<ai::ProviderValidationResult, String> {
    ai::validate_provider_credentials(&app, request).await
}

#[tauri::command]
async fn save_rule_config_cmd(
    app: tauri::AppHandle,
    config: rules::RuleConfig,
) -> Result<rules::RuleConfig, String> {
    rules::save_rule_config(&app, &config)?;
    rules::load_or_init_rule_config(&app)
}

#[tauri::command]
async fn preview_rule_matches(
    app: tauri::AppHandle,
    path: String,
    rule_id: String,
    max_results: Option<usize>,
) -> Result<Vec<String>, String> {
    let config = rules::load_or_init_rule_config(&app)?;
    let engine = rules::RuleEngine::new(&config);
    let limit = max_results.unwrap_or(100);

    let mut matches = Vec::new();
    for entry in WalkDir::new(&path)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().is_dir() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let size = match entry.metadata() {
            Ok(m) => m.len(),
            Err(_) => continue,
        };
        if let Some(rule_match) = engine.evaluate(&file_name, size) {
            if rule_match.rule_id == rule_id {
                matches.push(entry.path().to_string_lossy().to_string());
                if matches.len() >= limit {
                    break;
                }
            }
        }
    }

    Ok(matches)
}

fn main() {
    let mut updater_plugin_builder = tauri_plugin_updater::Builder::new();
    if let Some(pubkey) = option_env!("TAURI_UPDATER_PUBLIC_KEY") {
        if !pubkey.trim().is_empty() {
            updater_plugin_builder = updater_plugin_builder.pubkey(pubkey);
        }
    } else {
        warn!(
            "TAURI_UPDATER_PUBLIC_KEY not set at build time; updater checks will fail until a public key is configured"
        );
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(updater_plugin_builder.build())
        .setup(|app| {
            if let Err(e) = db::init_db(app.handle()) {
                eprintln!("Failed to initialize DB: {}", e);
            }
            if let Err(e) = observability::init_logging(app.handle()) {
                eprintln!("Failed to initialize logging: {}", e);
            } else if let Some(path) = observability::log_path(app.handle()) {
                info!("logger_initialized path='{}'", path.display());
            }

            if let Err(e) = rules::load_or_init_rule_config(app.handle()) {
                eprintln!("Failed to initialize rules config: {}", e);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_directory,
            scan_directory_advanced,
            execute_moves,
            undo_moves,
            fetch_history,
            get_rule_config,
            get_ai_settings,
            save_ai_settings_cmd,
            list_ai_models,
            validate_ai_provider,
            save_rule_config_cmd,
            preview_rule_matches
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
