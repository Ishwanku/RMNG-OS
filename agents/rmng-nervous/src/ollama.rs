use rmng_core::CoreIntent;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_URL: &str = "http://127.0.0.1:11434";
const DEFAULT_MODEL: &str = "llama3.2";

#[derive(Debug, thiserror::Error)]
pub enum NervousError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("ollama error: {0}")]
    Ollama(String),
    #[error("invalid intent from model: {0}")]
    InvalidIntent(#[from] rmng_core::RmngError),
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    format: &'static str,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

pub struct OllamaAdapter {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl Default for OllamaAdapter {
    fn default() -> Self {
        Self::new(DEFAULT_URL, DEFAULT_MODEL)
    }
}

impl OllamaAdapter {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("http client");
        Self {
            base_url: base_url.into(),
            model: model.into(),
            client,
        }
    }

    pub async fn health(&self) -> Result<bool, NervousError> {
        let url = format!("{}/api/tags", self.base_url.trim_end_matches('/'));
        Ok(self.client.get(&url).send().await?.status().is_success())
    }

    pub async fn reason_core(
        &self,
        assembled_prompt: &str,
        skill_name: Option<&str>,
    ) -> Result<CoreIntent, NervousError> {
        let skill_note = skill_name
            .map(|n| format!("\nInclude metadata.skill_name = \"{n}\" when appropriate."))
            .unwrap_or_default();
        let prompt = format!(
            "{assembled_prompt}{skill_note}\n\nRespond with a single JSON object for core-intent v2."
        );
        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let body = GenerateRequest {
            model: &self.model,
            prompt: &prompt,
            stream: false,
            format: "json",
        };
        let resp: GenerateResponse = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| NervousError::Ollama(e.to_string()))?
            .json()
            .await?;
        CoreIntent::parse(&resp.response).map_err(NervousError::InvalidIntent)
    }
}