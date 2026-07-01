use rmng_core::LlmProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogMeta {
    pub version: String,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelEntry {
    pub id: String,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub specialized: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderEntry {
    pub label: String,
    pub api_style: String,
    #[serde(default)]
    pub default_endpoint: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub key_prefix_hint: Option<String>,
    #[serde(default)]
    pub docs_url: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmCatalogFile {
    pub catalog: CatalogMeta,
    #[serde(default)]
    pub providers: HashMap<String, ProviderEntry>,
}

#[derive(Debug, Clone)]
pub struct LlmCatalog {
    pub path: PathBuf,
    pub file: LlmCatalogFile,
}

static CATALOG: OnceLock<LlmCatalog> = OnceLock::new();

pub fn catalog_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let user = PathBuf::from(&home).join(".rmng/llm-catalog.toml");
        if user.is_file() {
            return user;
        }
    }
    if let Ok(root) = std::env::var("RMNG_PROJECT_ROOT") {
        let repo = PathBuf::from(root).join("config/llm-catalog.toml");
        if repo.is_file() {
            return repo;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let repo = PathBuf::from(home).join("dev/projects/RMNG-OS/config/llm-catalog.toml");
        if repo.is_file() {
            return repo;
        }
    }
    PathBuf::from("config/llm-catalog.toml")
}

pub fn load_catalog() -> &'static LlmCatalog {
    CATALOG.get_or_init(|| {
        let path = catalog_path();
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        let file: LlmCatalogFile = toml::from_str(&raw).unwrap_or_else(|e| {
            tracing::warn!(error = %e, path = %path.display(), "llm catalog parse failed; using empty catalog");
            LlmCatalogFile {
                catalog: CatalogMeta {
                    version: "0".into(),
                    updated: None,
                    notes: Some("catalog missing or invalid".into()),
                },
                providers: HashMap::new(),
            }
        });
        LlmCatalog { path, file }
    })
}

pub fn provider_id(provider: LlmProvider) -> &'static str {
    match provider {
        LlmProvider::None => "none",
        LlmProvider::Ollama => "ollama",
        LlmProvider::OpenAi => "openai",
        LlmProvider::Grok => "grok",
        LlmProvider::Anthropic => "anthropic",
        LlmProvider::Google => "google",
        LlmProvider::Groq => "groq",
        LlmProvider::Together => "together",
        LlmProvider::Fireworks => "fireworks",
        LlmProvider::DeepSeek => "deepseek",
        LlmProvider::NvidiaNim => "nvidia_nim",
        LlmProvider::Custom => "custom",
    }
}

pub fn catalog_endpoint(provider: LlmProvider) -> Option<String> {
    let id = provider_id(provider);
    load_catalog()
        .file
        .providers
        .get(id)
        .and_then(|p| p.default_endpoint.clone())
}

pub fn catalog_api_key_env(provider: LlmProvider) -> Option<String> {
    let id = provider_id(provider);
    load_catalog()
        .file
        .providers
        .get(id)
        .and_then(|p| p.api_key_env.clone())
}

pub fn catalog_default_model(provider: LlmProvider) -> Option<String> {
    let id = provider_id(provider);
    let entry = load_catalog().file.providers.get(id)?;
    entry
        .models
        .iter()
        .find(|m| m.default && !m.specialized)
        .or_else(|| entry.models.iter().find(|m| m.default))
        .map(|m| m.id.clone())
        .or_else(|| entry.models.first().map(|m| m.id.clone()))
}

pub fn list_catalog_models(provider: LlmProvider, include_specialized: bool) -> Vec<ModelEntry> {
    let id = provider_id(provider);
    load_catalog()
        .file
        .providers
        .get(id)
        .map(|p| {
            p.models
                .iter()
                .filter(|m| include_specialized || !m.specialized)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

pub fn list_all_providers() -> Vec<(String, ProviderEntry)> {
    let mut out: Vec<_> = load_catalog()
        .file
        .providers
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

pub fn user_catalog_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".rmng/llm-catalog.toml");
    }
    PathBuf::from("/tmp/rmng/llm-catalog.toml")
}

/// Merge live-only model ids into the user catalog (Sprint 9).
pub fn apply_live_models(provider_key: &str, live_only: &[String]) -> Result<(PathBuf, Vec<String>), String> {
    let path = user_catalog_path();
    if !path.is_file() {
        return Err(format!(
            "catalog not found at {} — run `rmng llm setup` first",
            path.display()
        ));
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut file: LlmCatalogFile = toml::from_str(&raw).map_err(|e| e.to_string())?;
    let provider = file
        .providers
        .get_mut(provider_key)
        .ok_or_else(|| format!("provider '{provider_key}' not in catalog"))?;
    let mut added = Vec::new();
    for id in live_only {
        if provider.models.iter().any(|m| m.id == *id) {
            continue;
        }
        provider.models.push(ModelEntry {
            id: id.clone(),
            tier: Some("discovered".into()),
            description: Some("Added by rmng llm sync-catalog --apply".into()),
            default: false,
            specialized: false,
        });
        added.push(id.clone());
    }
    if added.is_empty() {
        return Ok((path, added));
    }
    file.catalog.updated = Some(chrono::Utc::now().format("%Y-%m-%d").to_string());
    let out = toml::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok((path, added))
}

pub fn install_user_catalog(from: &Path) -> std::io::Result<PathBuf> {
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"));
    let dest_dir = home.join(".rmng");
    std::fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join("llm-catalog.toml");
    std::fs::copy(from, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_gemini_default_from_catalog() {
        std::env::set_var(
            "RMNG_PROJECT_ROOT",
            format!(
                "{}/dev/projects/RMNG-OS",
                std::env::var("HOME").unwrap_or_else(|_| "/home/saini".into())
            ),
        );
        let _ = load_catalog();
        let model = catalog_default_model(LlmProvider::Google).unwrap_or_default();
        assert!(
            model.starts_with("gemini-3") || model.starts_with("gemini-2"),
            "expected modern gemini default, got {model}"
        );
    }
}