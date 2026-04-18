use crate::FileNode;
use keyring::Entry;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::Manager;

const KEYRING_SERVICE_NAME: &str = "ordinex.ai";
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
const OPENAI_API_BASE: &str = "https://api.openai.com/v1";
const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
const OLLAMA_API_BASE: &str = "http://localhost:11434";
const OPENROUTER_API_BASE: &str = "https://openrouter.ai/api/v1";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    Gemini,
    Openai,
    Anthropic,
    Ollama,
    Openrouter,
}

impl AIProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            AIProvider::Gemini => "gemini",
            AIProvider::Openai => "openai",
            AIProvider::Anthropic => "anthropic",
            AIProvider::Ollama => "ollama",
            AIProvider::Openrouter => "openrouter",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            AIProvider::Gemini => "Gemini",
            AIProvider::Openai => "OpenAI",
            AIProvider::Anthropic => "Anthropic",
            AIProvider::Ollama => "Ollama",
            AIProvider::Openrouter => "OpenRouter-Compatible",
        }
    }
}

impl Default for AIProvider {
    fn default() -> Self {
        AIProvider::Gemini
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AISettings {
    #[serde(default = "default_ai_settings_version")]
    pub version: u32,
    #[serde(default = "default_ai_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub selected_provider: AIProvider,
    #[serde(default)]
    pub selected_model: String,
    #[serde(default)]
    pub custom_base_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AISettingsEnvelope {
    pub settings: AISettings,
    pub api_key_present: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveAISettingsRequest {
    pub settings: AISettings,
    pub api_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProviderModelsRequest {
    pub provider: Option<AIProvider>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProviderValidationRequest {
    pub provider: Option<AIProvider>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProviderValidationResult {
    pub available: bool,
    pub message: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AIClassificationResult {
    pub filename: String,
    pub category: String,
    #[serde(alias = "suggested_folder", alias = "folder", alias = "suggestedFolderName")]
    pub suggested_folder_name: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default)]
    pub is_temporary_or_cleanup: bool,
}

fn default_confidence() -> f32 {
    0.5
}

fn default_ai_settings_version() -> u32 {
    1
}

fn default_ai_enabled() -> bool {
    true
}

fn default_model_for(provider: AIProvider) -> &'static str {
    match provider {
        AIProvider::Gemini => "gemini-2.5-flash",
        AIProvider::Openai => "gpt-4o-mini",
        AIProvider::Anthropic => "claude-3-5-sonnet-latest",
        AIProvider::Ollama => "llama3.1",
        AIProvider::Openrouter => "openai/gpt-4o-mini",
    }
}

fn normalize_settings(mut settings: AISettings) -> AISettings {
    settings.version = default_ai_settings_version();
    if settings.selected_model.trim().is_empty() {
        settings.selected_model = default_model_for(settings.selected_provider).to_string();
    }
    if let Some(base_url) = &settings.custom_base_url {
        let trimmed = base_url.trim();
        if trimmed.is_empty() {
            settings.custom_base_url = None;
        } else {
            settings.custom_base_url = Some(trimmed.trim_end_matches('/').to_string());
        }
    }
    settings
}

pub fn default_ai_settings() -> AISettings {
    normalize_settings(AISettings {
        version: default_ai_settings_version(),
        enabled: default_ai_enabled(),
        selected_provider: AIProvider::Gemini,
        selected_model: default_model_for(AIProvider::Gemini).to_string(),
        custom_base_url: None,
    })
}

fn settings_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app_data_dir: {}", e))?;
    fs::create_dir_all(&path).map_err(|e| format!("Could not create app data directory: {}", e))?;
    path.push("ai_settings.json");
    Ok(path)
}

pub fn load_or_init_ai_settings(app_handle: &tauri::AppHandle) -> Result<AISettings, String> {
    let path = settings_path(app_handle)?;
    if !path.exists() {
        let default = default_ai_settings();
        save_ai_settings(app_handle, &default)?;
        return Ok(default);
    }

    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Could not read AI settings at {}: {}", path.display(), e))?;
    let settings: AISettings = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid AI settings JSON at {}: {}", path.display(), e))?;
    let normalized = normalize_settings(settings);
    Ok(normalized)
}

pub fn save_ai_settings(app_handle: &tauri::AppHandle, settings: &AISettings) -> Result<(), String> {
    let path = settings_path(app_handle)?;
    let normalized = normalize_settings(settings.clone());
    let raw = serde_json::to_string_pretty(&normalized)
        .map_err(|e| format!("Could not serialize AI settings: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("Could not write AI settings: {}", e))
}

fn provider_key_account(provider: AIProvider) -> String {
    format!("provider.{}", provider.as_str())
}

fn provider_requires_api_key(provider: AIProvider) -> bool {
    !matches!(provider, AIProvider::Ollama)
}

fn read_provider_api_key(provider: AIProvider) -> Result<Option<String>, String> {
    let entry = Entry::new(KEYRING_SERVICE_NAME, &provider_key_account(provider)).map_err(|e| {
        format!(
            "Failed to access credential vault for {}: {}",
            provider.display_name(),
            e
        )
    })?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!(
            "Failed to read API key from credential vault for {}: {}",
            provider.display_name(),
            e
        )),
    }
}

fn store_provider_api_key(provider: AIProvider, api_key: Option<&str>) -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE_NAME, &provider_key_account(provider)).map_err(|e| {
        format!(
            "Failed to access credential vault for {}: {}",
            provider.display_name(),
            e
        )
    })?;
    match api_key {
        Some(value) if !value.trim().is_empty() => entry.set_password(value.trim()).map_err(|e| {
            format!(
                "Failed to store API key in credential vault for {}: {}",
                provider.display_name(),
                e
            )
        }),
        _ => match entry.delete_password() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!(
                "Failed to remove API key from credential vault for {}: {}",
                provider.display_name(),
                e
            )),
        },
    }
}

fn resolve_api_key(provider: AIProvider, api_key_override: Option<String>) -> Result<Option<String>, String> {
    if let Some(key) = api_key_override {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed));
        }
    }

    let stored = read_provider_api_key(provider)?;
    if provider_requires_api_key(provider) && stored.is_none() {
        return Err(format!(
            "No API key is configured for {}.",
            provider.display_name()
        ));
    }
    Ok(stored)
}

fn resolve_base_url(provider: AIProvider, maybe_url: Option<String>) -> String {
    if let Some(url) = maybe_url {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            return trimmed.trim_end_matches('/').to_string();
        }
    }

    match provider {
        AIProvider::Gemini => GEMINI_API_BASE.to_string(),
        AIProvider::Openai => OPENAI_API_BASE.to_string(),
        AIProvider::Anthropic => ANTHROPIC_API_BASE.to_string(),
        AIProvider::Ollama => OLLAMA_API_BASE.to_string(),
        AIProvider::Openrouter => OPENROUTER_API_BASE.to_string(),
    }
}

fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

pub fn get_ai_settings_view(app_handle: &tauri::AppHandle) -> Result<AISettingsEnvelope, String> {
    let settings = load_or_init_ai_settings(app_handle)?;
    let api_key_present = read_provider_api_key(settings.selected_provider)?.is_some();
    Ok(AISettingsEnvelope {
        settings,
        api_key_present,
    })
}

pub fn save_ai_settings_view(
    app_handle: &tauri::AppHandle,
    request: SaveAISettingsRequest,
) -> Result<AISettingsEnvelope, String> {
    let settings = normalize_settings(request.settings);
    save_ai_settings(app_handle, &settings)?;

    if request.api_key.is_some() {
        store_provider_api_key(settings.selected_provider, request.api_key.as_deref())?;
    }

    get_ai_settings_view(app_handle)
}

async fn list_models_internal(
    provider: AIProvider,
    api_key: Option<String>,
    base_url: String,
) -> Result<Vec<String>, String> {
    match provider {
        AIProvider::Gemini => list_gemini_models(api_key, base_url).await,
        AIProvider::Openai => list_openai_models(api_key, base_url).await,
        AIProvider::Anthropic => list_anthropic_models(api_key, base_url).await,
        AIProvider::Ollama => list_ollama_models(api_key, base_url).await,
        AIProvider::Openrouter => list_openrouter_models(api_key, base_url).await,
    }
}

pub async fn list_provider_models(
    app_handle: &tauri::AppHandle,
    request: ProviderModelsRequest,
) -> Result<Vec<String>, String> {
    let settings = load_or_init_ai_settings(app_handle)?;
    let provider = request.provider.unwrap_or(settings.selected_provider);
    let base_url = resolve_base_url(provider, request.base_url.or(settings.custom_base_url));
    let api_key = resolve_api_key(provider, request.api_key)?;

    let mut models = list_models_internal(provider, api_key, base_url).await?;
    models.sort();
    models.dedup();
    Ok(models)
}

pub async fn validate_provider_credentials(
    app_handle: &tauri::AppHandle,
    request: ProviderValidationRequest,
) -> Result<ProviderValidationResult, String> {
    let settings = load_or_init_ai_settings(app_handle)?;
    let provider = request.provider.unwrap_or(settings.selected_provider);
    let base_url = resolve_base_url(provider, request.base_url.or(settings.custom_base_url));

    let api_key = match resolve_api_key(provider, request.api_key) {
        Ok(key) => key,
        Err(err) => {
            return Ok(ProviderValidationResult {
                available: false,
                message: err,
            });
        }
    };

    let result = list_models_internal(provider, api_key, base_url).await;
    match result {
        Ok(models) => {
            if models.is_empty() {
                Ok(ProviderValidationResult {
                    available: true,
                    message: format!(
                        "{} connected, but no models were returned.",
                        provider.display_name()
                    ),
                })
            } else {
                Ok(ProviderValidationResult {
                    available: true,
                    message: format!(
                        "{} connected successfully ({} models available).",
                        provider.display_name(),
                        models.len()
                    ),
                })
            }
        }
        Err(err) => Ok(ProviderValidationResult {
            available: false,
            message: err,
        }),
    }
}

fn build_file_payload(unclassified_files: &[FileNode]) -> Value {
    let mut files_json = serde_json::json!([]);
    for f in unclassified_files {
        let mut obj = serde_json::Map::new();
        obj.insert("name".to_string(), serde_json::Value::String(f.name.clone()));
        obj.insert(
            "size_bytes".to_string(),
            serde_json::Value::Number(f.size.into()),
        );
        if let Some(ext) = &f.extension {
            obj.insert("ext".to_string(), serde_json::Value::String(ext.clone()));
        }
        files_json
            .as_array_mut()
            .expect("files_json initialized as array")
            .push(serde_json::Value::Object(obj));
    }
    files_json
}

fn classification_prompt(unclassified_files: &[FileNode], existing_folders: &[String]) -> (String, String) {
    let files_json = build_file_payload(unclassified_files);
    let context_str = if existing_folders.is_empty() {
        String::new()
    } else {
        format!(
            "\nYou have previously created these folders: {:?}. Reuse one if it semantically fits; otherwise create a concise, specific folder name.",
            existing_folders
        )
    };

    let prompt = format!("Classify these files:\n{}{}", files_json, context_str);
    let sys_prompt = "You are a specialized file classification engine for Windows.
Return ONLY a strict JSON array of objects, one per input file, with keys:
- filename (exact match to input name)
- category (Work, Personal, Finance, Media, Code, Academic, Projects, Unknown)
- suggested_folder_name
- confidence (0.0 to 1.0)
- is_temporary_or_cleanup (boolean)
Do not include markdown fences or any extra text.";

    (prompt, sys_prompt.to_string())
}

fn sanitize_model_for_gemini(model: &str) -> String {
    model.trim().trim_start_matches("models/").to_string()
}

fn normalize_base(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn bearer_headers(api_key: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(&format!("Bearer {}", api_key))
        .map_err(|e| format!("Invalid API key header value: {}", e))?;
    headers.insert(AUTHORIZATION, value);
    Ok(headers)
}

async fn extract_text_response(response: reqwest::Response, provider_name: &str) -> Result<String, String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read {} response body: {}", provider_name, e))?;
    if !status.is_success() {
        return Err(format!(
            "{} request failed with status {}: {}",
            provider_name,
            status,
            body
        ));
    }
    Ok(body)
}

fn extract_openai_like_content(value: &Value) -> String {
    if let Some(content) = value
        .get("choices")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("message"))
        .and_then(|v| v.get("content"))
    {
        if let Some(as_str) = content.as_str() {
            return as_str.to_string();
        }
        if let Some(parts) = content.as_array() {
            for part in parts {
                if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                    return text.to_string();
                }
            }
        }
    }
    String::new()
}

async fn classify_with_gemini(
    api_key: String,
    model: &str,
    prompt: &str,
    sys_prompt: &str,
) -> Result<String, String> {
    let model_id = sanitize_model_for_gemini(model);
    let url = format!(
        "{}/models/{}:generateContent?key={}",
        GEMINI_API_BASE, model_id, api_key
    );
    let body = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "system_instruction": { "parts": [{ "text": sys_prompt }] },
        "generation_config": {
            "temperature": 0.1,
            "response_mime_type": "application/json"
        }
    });
    let client = build_client()?;
    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to call Gemini API: {}", e))?;

    let text = extract_text_response(response, "Gemini").await?;
    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Gemini response wrapper: {}", e))?;

    let candidate = parsed
        .get("candidates")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("content"))
        .and_then(|v| v.get("parts"))
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    if candidate.trim().is_empty() {
        return Err("Gemini returned an empty classification payload.".to_string());
    }

    Ok(candidate)
}

async fn classify_with_openai_compatible(
    provider_label: &str,
    api_base: &str,
    api_key: String,
    model: &str,
    prompt: &str,
    sys_prompt: &str,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", normalize_base(api_base));
    let body = serde_json::json!({
        "model": model,
        "temperature": 0.1,
        "messages": [
            { "role": "system", "content": sys_prompt },
            { "role": "user", "content": prompt }
        ]
    });
    let client = build_client()?;
    let response = client
        .post(url)
        .headers(bearer_headers(&api_key)?)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to call {} API: {}", provider_label, e))?;

    let text = extract_text_response(response, provider_label).await?;
    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse {} response wrapper: {}", provider_label, e))?;
    let content = extract_openai_like_content(&parsed);
    if content.trim().is_empty() {
        return Err(format!("{} returned an empty classification payload.", provider_label));
    }
    Ok(content)
}

async fn classify_with_anthropic(
    api_base: &str,
    api_key: String,
    model: &str,
    prompt: &str,
    sys_prompt: &str,
) -> Result<String, String> {
    let url = format!("{}/messages", normalize_base(api_base));
    let body = serde_json::json!({
        "model": model,
        "system": sys_prompt,
        "max_tokens": 2048,
        "temperature": 0.1,
        "messages": [
            { "role": "user", "content": prompt }
        ]
    });
    let client = build_client()?;
    let response = client
        .post(url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to call Anthropic API: {}", e))?;

    let text = extract_text_response(response, "Anthropic").await?;
    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Anthropic response wrapper: {}", e))?;
    let content = parsed
        .get("content")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    if content.trim().is_empty() {
        return Err("Anthropic returned an empty classification payload.".to_string());
    }
    Ok(content)
}

async fn classify_with_ollama(
    api_base: &str,
    api_key: Option<String>,
    model: &str,
    prompt: &str,
    sys_prompt: &str,
) -> Result<String, String> {
    let url = format!("{}/api/chat", normalize_base(api_base));
    let body = serde_json::json!({
        "model": model,
        "stream": false,
        "format": "json",
        "messages": [
            { "role": "system", "content": sys_prompt },
            { "role": "user", "content": prompt }
        ],
        "options": { "temperature": 0.1 }
    });

    let client = build_client()?;
    let mut request = client.post(url).json(&body);
    if let Some(key) = api_key {
        if !key.trim().is_empty() {
            request = request.headers(bearer_headers(&key)?);
        }
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to call Ollama API: {}", e))?;
    let text = extract_text_response(response, "Ollama").await?;
    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Ollama response wrapper: {}", e))?;

    let content = parsed
        .get("message")
        .and_then(|v| v.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    if content.trim().is_empty() {
        return Err("Ollama returned an empty classification payload.".to_string());
    }
    Ok(content)
}

fn extract_json_segment(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        let mut cleaned = trimmed.replace("```json", "").replace("```JSON", "");
        cleaned = cleaned.replace("```", "");
        return cleaned.trim().to_string();
    }

    if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
        if end > start {
            return trimmed[start..=end].to_string();
        }
    }

    trimmed.to_string()
}

fn parse_classification_json(raw: &str) -> Result<Vec<AIClassificationResult>, String> {
    let payload = extract_json_segment(raw);

    if let Ok(results) = serde_json::from_str::<Vec<AIClassificationResult>>(&payload) {
        return Ok(results);
    }

    let parsed_value: Value = serde_json::from_str(&payload)
        .map_err(|e| format!("Failed to parse AI JSON payload: {} | payload: {}", e, payload))?;

    let list = parsed_value
        .get("classifications")
        .or_else(|| parsed_value.get("results"))
        .or_else(|| parsed_value.get("data"))
        .cloned()
        .ok_or_else(|| "AI JSON response did not include a classification list.".to_string())?;

    serde_json::from_value::<Vec<AIClassificationResult>>(list)
        .map_err(|e| format!("Failed to decode AI classification list: {}", e))
}

fn filter_results_to_input(
    input_files: &[FileNode],
    results: Vec<AIClassificationResult>,
) -> Vec<AIClassificationResult> {
    let allowed: HashSet<String> = input_files
        .iter()
        .map(|f| f.name.to_ascii_lowercase())
        .collect();

    results
        .into_iter()
        .filter(|r| allowed.contains(&r.filename.to_ascii_lowercase()))
        .collect()
}

pub async fn classify_files_with_ai(
    files: &[FileNode],
    existing_folders: &[String],
    settings: &AISettings,
) -> Result<Vec<AIClassificationResult>, String> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    if !settings.enabled {
        return Ok(Vec::new());
    }

    let provider = settings.selected_provider;
    let model = settings.selected_model.trim();
    let model_name = if model.is_empty() {
        default_model_for(provider).to_string()
    } else {
        model.to_string()
    };

    let base_url = resolve_base_url(provider, settings.custom_base_url.clone());
    let api_key = resolve_api_key(provider, None)?;
    let (prompt, sys_prompt) = classification_prompt(files, existing_folders);

    let raw = match provider {
        AIProvider::Gemini => {
            let key = api_key.ok_or_else(|| "Gemini API key was not found in credential vault.".to_string())?;
            classify_with_gemini(key, &model_name, &prompt, &sys_prompt).await?
        }
        AIProvider::Openai => {
            let key = api_key.ok_or_else(|| "OpenAI API key was not found in credential vault.".to_string())?;
            classify_with_openai_compatible("OpenAI", &base_url, key, &model_name, &prompt, &sys_prompt)
                .await?
        }
        AIProvider::Anthropic => {
            let key = api_key
                .ok_or_else(|| "Anthropic API key was not found in credential vault.".to_string())?;
            classify_with_anthropic(&base_url, key, &model_name, &prompt, &sys_prompt).await?
        }
        AIProvider::Ollama => classify_with_ollama(&base_url, api_key, &model_name, &prompt, &sys_prompt).await?,
        AIProvider::Openrouter => {
            let key = api_key
                .ok_or_else(|| "OpenRouter-compatible API key was not found in credential vault.".to_string())?;
            classify_with_openai_compatible(
                "OpenRouter-Compatible",
                &base_url,
                key,
                &model_name,
                &prompt,
                &sys_prompt,
            )
            .await?
        }
    };

    let parsed = parse_classification_json(&raw)?;
    Ok(filter_results_to_input(files, parsed))
}

async fn list_gemini_models(api_key: Option<String>, base_url: String) -> Result<Vec<String>, String> {
    let key = api_key.ok_or_else(|| "Gemini API key is required.".to_string())?;
    let url = format!("{}/models?key={}", normalize_base(&base_url), key);
    let client = build_client()?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to query Gemini models: {}", e))?;
    let text = extract_text_response(response, "Gemini").await?;

    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Gemini models response: {}", e))?;
    let mut models = Vec::new();

    if let Some(entries) = parsed.get("models").and_then(|v| v.as_array()) {
        for entry in entries {
            let supports_generate = entry
                .get("supportedGenerationMethods")
                .and_then(|v| v.as_array())
                .map(|methods| {
                    methods
                        .iter()
                        .filter_map(|m| m.as_str())
                        .any(|m| m.eq_ignore_ascii_case("generateContent"))
                })
                .unwrap_or(true);

            if !supports_generate {
                continue;
            }

            if let Some(name) = entry.get("name").and_then(|v| v.as_str()) {
                models.push(name.trim_start_matches("models/").to_string());
            }
        }
    }

    if models.is_empty() {
        return Err("Gemini did not return any usable models.".to_string());
    }
    Ok(models)
}

async fn list_openai_models(api_key: Option<String>, base_url: String) -> Result<Vec<String>, String> {
    let key = api_key.ok_or_else(|| "OpenAI API key is required.".to_string())?;
    let url = format!("{}/models", normalize_base(&base_url));
    let client = build_client()?;
    let response = client
        .get(url)
        .headers(bearer_headers(&key)?)
        .send()
        .await
        .map_err(|e| format!("Failed to query OpenAI models: {}", e))?;
    let text = extract_text_response(response, "OpenAI").await?;

    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse OpenAI models response: {}", e))?;
    let models = parsed
        .get("data")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("id").and_then(|v| v.as_str()))
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if models.is_empty() {
        return Err("OpenAI did not return any models.".to_string());
    }
    Ok(models)
}

async fn list_anthropic_models(api_key: Option<String>, base_url: String) -> Result<Vec<String>, String> {
    let key = api_key.ok_or_else(|| "Anthropic API key is required.".to_string())?;
    let url = format!("{}/models", normalize_base(&base_url));
    let client = build_client()?;
    let response = client
        .get(url)
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|e| format!("Failed to query Anthropic models: {}", e))?;
    let text = extract_text_response(response, "Anthropic").await?;

    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Anthropic models response: {}", e))?;
    let models = parsed
        .get("data")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("id").and_then(|v| v.as_str()))
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if models.is_empty() {
        return Err("Anthropic did not return any models.".to_string());
    }
    Ok(models)
}

async fn list_ollama_models(api_key: Option<String>, base_url: String) -> Result<Vec<String>, String> {
    let url = format!("{}/api/tags", normalize_base(&base_url));
    let client = build_client()?;
    let mut request = client.get(url);
    if let Some(key) = api_key {
        if !key.trim().is_empty() {
            request = request.headers(bearer_headers(&key)?);
        }
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to query Ollama models: {}", e))?;
    let text = extract_text_response(response, "Ollama").await?;

    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Ollama models response: {}", e))?;
    let models = parsed
        .get("models")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("name").and_then(|v| v.as_str()))
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if models.is_empty() {
        return Err("Ollama did not return any models. Ensure Ollama is running locally.".to_string());
    }
    Ok(models)
}

async fn list_openrouter_models(api_key: Option<String>, base_url: String) -> Result<Vec<String>, String> {
    let key = api_key.ok_or_else(|| "OpenRouter-compatible API key is required.".to_string())?;
    let url = format!("{}/models", normalize_base(&base_url));
    let client = build_client()?;
    let response = client
        .get(url)
        .headers(bearer_headers(&key)?)
        .send()
        .await
        .map_err(|e| format!("Failed to query OpenRouter-compatible models: {}", e))?;
    let text = extract_text_response(response, "OpenRouter-Compatible").await?;

    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse OpenRouter-compatible models response: {}", e))?;
    let models = parsed
        .get("data")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("id").and_then(|v| v.as_str()))
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if models.is_empty() {
        return Err("OpenRouter-compatible provider did not return any models.".to_string());
    }
    Ok(models)
}
