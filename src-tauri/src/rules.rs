use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Move,
    Copy,
    Delete,
    Ignore,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RuleConditions {
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub filename_keywords: Vec<String>,
    pub min_size_bytes: Option<u64>,
    pub max_size_bytes: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClassificationRule {
    pub id: String,
    pub name: String,
    pub category_path: String,
    pub destination_folder: String,
    pub priority: u32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_action")]
    pub action: RuleAction,
    #[serde(default)]
    pub conditions: RuleConditions,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleConfig {
    pub version: u32,
    #[serde(default = "default_stop_on_match")]
    pub stop_on_match: bool,
    #[serde(default = "default_unknown_folder")]
    pub unknown_folder: String,
    pub rules: Vec<ClassificationRule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleMatch {
    pub rule_id: String,
    pub rule_name: String,
    pub category_path: String,
    pub destination_folder: String,
    pub action: RuleAction,
    pub priority: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleConflict {
    pub rule_a: String,
    pub rule_b: String,
    pub shared_extensions: Vec<String>,
}

pub struct RuleEngine {
    rules: Vec<ClassificationRule>,
    extension_index: HashMap<String, Vec<usize>>,
    fallback_rules: Vec<usize>,
}

fn default_enabled() -> bool {
    true
}

fn default_action() -> RuleAction {
    RuleAction::Move
}

fn default_unknown_folder() -> String {
    "Needs Sorting".to_string()
}

fn default_stop_on_match() -> bool {
    true
}

fn norm_extension(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn lower_vec(values: &[String]) -> Vec<String> {
    values.iter().map(|v| v.to_ascii_lowercase()).collect()
}

pub fn extract_extensions(file_name: &str) -> Vec<String> {
    let lower = file_name.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('.').collect();
    if parts.len() <= 1 {
        return Vec::new();
    }

    let mut result = Vec::new();
    let last = parts.last().unwrap_or(&"").trim();
    if !last.is_empty() {
        result.push(last.to_string());
    }

    if parts.len() >= 3 {
        let compound = format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1]);
        result.insert(0, compound);
    }

    result
}

impl RuleEngine {
    pub fn new(config: &RuleConfig) -> Self {
        let mut rules: Vec<ClassificationRule> = config
            .rules
            .iter()
            .filter(|r| r.enabled)
            .cloned()
            .collect();

        rules.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.id.cmp(&b.id)));

        let mut extension_index: HashMap<String, Vec<usize>> = HashMap::new();
        let mut fallback_rules = Vec::new();

        for (idx, rule) in rules.iter().enumerate() {
            let extensions = lower_vec(&rule.conditions.extensions);
            if extensions.is_empty() {
                fallback_rules.push(idx);
            } else {
                for ext in extensions {
                    extension_index.entry(ext).or_default().push(idx);
                }
            }
        }

        RuleEngine {
            rules,
            extension_index,
            fallback_rules,
        }
    }

    fn matches_rule(rule: &ClassificationRule, file_name: &str, size: u64, exts: &[String]) -> bool {
        let rule_exts = lower_vec(&rule.conditions.extensions);
        if !rule_exts.is_empty() && !rule_exts.iter().any(|re| exts.contains(re)) {
            return false;
        }

        let lower_name = file_name.to_ascii_lowercase();
        let keywords = lower_vec(&rule.conditions.filename_keywords);
        if !keywords.is_empty() && !keywords.iter().any(|kw| lower_name.contains(kw)) {
            return false;
        }

        if let Some(min_size) = rule.conditions.min_size_bytes {
            if size < min_size {
                return false;
            }
        }

        if let Some(max_size) = rule.conditions.max_size_bytes {
            if size > max_size {
                return false;
            }
        }

        true
    }

    pub fn evaluate(&self, file_name: &str, size: u64) -> Option<RuleMatch> {
        let exts = extract_extensions(file_name);
        let mut candidate_indices: Vec<usize> = Vec::new();

        for ext in &exts {
            if let Some(indices) = self.extension_index.get(ext) {
                candidate_indices.extend(indices);
            }
        }
        candidate_indices.extend(&self.fallback_rules);
        candidate_indices.sort_unstable();
        candidate_indices.dedup();

        for idx in candidate_indices {
            let rule = &self.rules[idx];
            if Self::matches_rule(rule, file_name, size, &exts) {
                return Some(RuleMatch {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    category_path: rule.category_path.clone(),
                    destination_folder: rule.destination_folder.clone(),
                    action: rule.action.clone(),
                    priority: rule.priority,
                });
            }
        }

        None
    }
}

pub fn detect_conflicts(config: &RuleConfig) -> Vec<RuleConflict> {
    let enabled_rules: Vec<&ClassificationRule> = config.rules.iter().filter(|r| r.enabled).collect();
    let mut conflicts = Vec::new();

    for i in 0..enabled_rules.len() {
        for j in (i + 1)..enabled_rules.len() {
            let a = enabled_rules[i];
            let b = enabled_rules[j];

            let a_exts: HashSet<String> = a
                .conditions
                .extensions
                .iter()
                .map(|v| norm_extension(v))
                .collect();
            let b_exts: HashSet<String> = b
                .conditions
                .extensions
                .iter()
                .map(|v| norm_extension(v))
                .collect();

            if a_exts.is_empty() || b_exts.is_empty() {
                continue;
            }

            let overlap: Vec<String> = a_exts.intersection(&b_exts).cloned().collect();
            if !overlap.is_empty() {
                conflicts.push(RuleConflict {
                    rule_a: a.id.clone(),
                    rule_b: b.id.clone(),
                    shared_extensions: overlap,
                });
            }
        }
    }

    conflicts
}

fn config_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app_data_dir: {}", e))?;
    fs::create_dir_all(&path).map_err(|e| format!("Could not create app data directory: {}", e))?;
    path.push("rules.json");
    Ok(path)
}

pub fn validate_config(config: &RuleConfig) -> Result<(), String> {
    let mut ids = HashSet::new();
    for rule in &config.rules {
        if rule.id.trim().is_empty() {
            return Err("Rule id cannot be empty".to_string());
        }
        if !ids.insert(rule.id.clone()) {
            return Err(format!("Duplicate rule id detected: {}", rule.id));
        }
    }
    Ok(())
}

pub fn load_or_init_rule_config(app_handle: &tauri::AppHandle) -> Result<RuleConfig, String> {
    let path = config_path(app_handle)?;
    if !path.exists() {
        let default = get_default_rule_config();
        save_rule_config(app_handle, &default)?;
        return Ok(default);
    }

    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Could not read rules config at {}: {}", path.display(), e))?;
    let config: RuleConfig = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid rules config JSON at {}: {}", path.display(), e))?;
    validate_config(&config)?;
    Ok(config)
}

pub fn save_rule_config(app_handle: &tauri::AppHandle, config: &RuleConfig) -> Result<(), String> {
    validate_config(config)?;
    let path = config_path(app_handle)?;
    let raw = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Could not serialize rules config: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("Could not write rules config: {}", e))
}

pub fn get_default_rule_config() -> RuleConfig {
    RuleConfig {
        version: 1,
        stop_on_match: true,
        unknown_folder: "Needs Sorting".to_string(),
        rules: vec![
            ClassificationRule {
                id: "documents-general".to_string(),
                name: "Documents - General".to_string(),
                category_path: "Documents".to_string(),
                destination_folder: "Documents".to_string(),
                priority: 10,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec![
                        "txt", "md", "rtf", "doc", "docx", "odt", "pdf", "epub", "mobi",
                    ]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                    filename_keywords: vec!["report".to_string(), "invoice".to_string(), "draft".to_string()],
                    min_size_bytes: None,
                    max_size_bytes: None,
                },
            },
            ClassificationRule {
                id: "documents-spreadsheets".to_string(),
                name: "Documents - Spreadsheets".to_string(),
                category_path: "Documents/Spreadsheets".to_string(),
                destination_folder: "Documents/Spreadsheets".to_string(),
                priority: 11,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec!["xls", "xlsx", "ods"]
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ..Default::default()
                },
            },
            ClassificationRule {
                id: "documents-presentations".to_string(),
                name: "Documents - Presentations".to_string(),
                category_path: "Documents/Presentations".to_string(),
                destination_folder: "Documents/Presentations".to_string(),
                priority: 12,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec!["ppt", "pptx", "odp"]
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ..Default::default()
                },
            },
            ClassificationRule {
                id: "images".to_string(),
                name: "Images".to_string(),
                category_path: "Media/Images".to_string(),
                destination_folder: "Pictures".to_string(),
                priority: 20,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec![
                        "jpg", "jpeg", "jfif", "heic", "png", "gif", "webp", "tif", "tiff", "bmp", "svg", "eps", "cr2", "nef", "arw",
                    ]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                    filename_keywords: vec!["screenshot".to_string()],
                    ..Default::default()
                },
            },
            ClassificationRule {
                id: "audio".to_string(),
                name: "Audio".to_string(),
                category_path: "Media/Audio".to_string(),
                destination_folder: "Audio".to_string(),
                priority: 30,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec!["mp3", "wav", "flac", "aac", "m4a", "ogg", "wma"]
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                    filename_keywords: vec!["podcast".to_string()],
                    ..Default::default()
                },
            },
            ClassificationRule {
                id: "video".to_string(),
                name: "Video".to_string(),
                category_path: "Media/Video".to_string(),
                destination_folder: "Videos".to_string(),
                priority: 40,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec!["mp4", "avi", "mov", "mkv", "wmv", "flv", "3gp"]
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                    min_size_bytes: Some(1_000_000),
                    ..Default::default()
                },
            },
            ClassificationRule {
                id: "archives".to_string(),
                name: "Archives".to_string(),
                category_path: "Archives".to_string(),
                destination_folder: "Archives".to_string(),
                priority: 50,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec![
                        "zip", "rar", "7z", "tar", "tar.gz", "tgz", "tar.bz2", "gz", "iso", "dmg",
                    ]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                    ..Default::default()
                },
            },
            ClassificationRule {
                id: "installers".to_string(),
                name: "Executables & Installers".to_string(),
                category_path: "Programs/Installers".to_string(),
                destination_folder: "Software & Installers".to_string(),
                priority: 60,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec!["exe", "msi", "pkg", "deb", "rpm", "appimage", "jar", "dll"]
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                    ..Default::default()
                },
            },
            ClassificationRule {
                id: "source-code".to_string(),
                name: "Source Code & Scripts".to_string(),
                category_path: "Code".to_string(),
                destination_folder: "Code Projects".to_string(),
                priority: 70,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec![
                        "html", "htm", "css", "js", "jsx", "ts", "tsx", "php", "c", "cpp", "h", "java", "class", "py", "pyc", "ipynb", "cs", "go", "rs", "xml", "json", "yaml", "yml", "sql", "sh", "bash", "bat", "ps1", "sln",
                    ]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                    ..Default::default()
                },
            },
            ClassificationRule {
                id: "three-d-cad".to_string(),
                name: "3D Models & CAD".to_string(),
                category_path: "Design/3D-CAD".to_string(),
                destination_folder: "3D Models & CAD".to_string(),
                priority: 80,
                enabled: true,
                action: RuleAction::Move,
                conditions: RuleConditions {
                    extensions: vec![
                        "obj", "fbx", "stl", "amf", "gltf", "glb", "iges", "igs", "step", "stp", "dwg", "dxf", "sldprt", "sldasm", "rvt", "skp",
                    ]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                    ..Default::default()
                },
            },
        ],
    }
}
