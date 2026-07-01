use super::prompt::build_reasoning_prompt;
use super::types::{parse_core_intent, LlmReasonContext, LlmRequest, LlmResponse, ProviderError};
use rmng_core::CoreIntent;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct GenerateRequest<'a> {
    contents: Vec<Content<'a>>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content<'a> {
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    response_mime_type: &'static str,
    temperature: f32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Option<Vec<CandidatePart>>,
}

#[derive(Deserialize)]
struct CandidatePart {
    text: Option<String>,
}

pub struct GoogleProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
    max_retries: u32,
}

impl GoogleProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
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
            api_key: api_key.into(),
            model: model.into(),
            client,
            max_retries,
        }
    }

    pub fn id(&self) -> &'static str {
        "google"
    }

    pub async fn health(&self) -> Result<bool, ProviderError> {
        Ok(!self.api_key.is_empty())
    }

    pub async fn complete(&self, req: LlmRequest<'_>) -> Result<LlmResponse, ProviderError> {
        let prompt = build_reasoning_prompt(req.assembled_prompt, &req.ctx);
        let mut last_err = None;
        for attempt in 0..=self.max_retries {
            match self.complete_once(&prompt).await {
                Ok(resp) => return Ok(resp),
                Err(e) if e.is_retryable() && attempt < self.max_retries => {
                    last_err = Some(e);
                    tokio::time::sleep(Duration::from_millis(800 * (attempt as u64 + 1))).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            ProviderError::Misconfigured("google retries exhausted".into())
        }))
    }

    async fn complete_once(&self, prompt: &str) -> Result<LlmResponse, ProviderError> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            self.model,
            self.api_key
        );
        let body = GenerateRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt }],
            }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json",
                temperature: 0.0,
            },
        };
        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                provider: "google".into(),
                status: status.as_u16(),
                message,
            });
        }
        let parsed: GenerateResponse = resp.json().await?;
        let content = parsed
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.as_ref())
            .and_then(|p| p.first())
            .and_then(|p| p.text.clone())
            .ok_or_else(|| ProviderError::Misconfigured("google empty response".into()))?;
        Ok(LlmResponse {
            content,
            provider_id: "google",
            model: self.model.clone(),
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