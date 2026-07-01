use super::prompt::build_reasoning_prompt;
use super::backoff::retry_delay;
use super::types::{parse_core_intent, LlmReasonContext, LlmRequest, LlmResponse, LlmUsage, ProviderError};
use rmng_core::CoreIntent;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

pub struct OllamaProvider {
    base_url: String,
    model: String,
    client: reqwest::Client,
    max_retries: u32,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new("http://127.0.0.1:11434", "llama3.2", 120, 2)
    }
}

impl OllamaProvider {
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        timeout_secs: u64,
        max_retries: u32,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("http client");
        Self {
            base_url: base_url.into(),
            model: model.into(),
            client,
            max_retries,
        }
    }

    pub fn id(&self) -> &'static str {
        "ollama"
    }

    pub async fn health(&self) -> Result<bool, ProviderError> {
        let url = format!("{}/api/tags", self.base_url.trim_end_matches('/'));
        Ok(self.client.get(&url).send().await?.status().is_success())
    }

    pub async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = format!("{}/api/tags", self.base_url.trim_end_matches('/'));
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ProviderError::api("ollama", status.as_u16(), &message));
        }
        #[derive(Deserialize)]
        struct TagsResponse {
            models: Option<Vec<TagEntry>>,
        }
        #[derive(Deserialize)]
        struct TagEntry {
            name: String,
        }
        let parsed: TagsResponse = resp.json().await?;
        let mut ids: Vec<String> = parsed
            .models
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.name.split(':').next().unwrap_or(&m.name).to_string())
            .collect();
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    pub async fn complete(&self, req: LlmRequest<'_>) -> Result<LlmResponse, ProviderError> {
        let prompt = build_reasoning_prompt(req.assembled_prompt, &req.ctx);
        let mut last_err = None;
        for attempt in 0..=self.max_retries {
            match self.complete_once(&prompt).await {
                Ok(resp) => return Ok(resp),
                Err(e) if e.is_retryable() && attempt < self.max_retries => {
                    last_err = Some(e);
                    tokio::time::sleep(retry_delay(attempt)).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            ProviderError::Misconfigured("ollama retries exhausted".into())
        }))
    }

    async fn complete_once(&self, prompt: &str) -> Result<LlmResponse, ProviderError> {
        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let body = GenerateRequest {
            model: &self.model,
            prompt,
            stream: false,
            format: "json",
        };
        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                provider: "ollama".into(),
                status: status.as_u16(),
                message,
            });
        }
        let parsed: GenerateResponse = resp.json().await?;
        let usage = LlmUsage::from_counts(parsed.prompt_eval_count, parsed.eval_count);
        Ok(LlmResponse {
            content: parsed.response,
            provider_id: "ollama",
            model: self.model.clone(),
            usage,
        })
    }

    pub async fn reason_core(
        &self,
        assembled: &str,
        ctx: &LlmReasonContext<'_>,
    ) -> Result<CoreIntent, ProviderError> {
        let resp = self
            .complete(LlmRequest {
                assembled_prompt: assembled,
                ctx: ctx.clone(),
            })
            .await?;
        parse_core_intent(&resp.content)
    }
}