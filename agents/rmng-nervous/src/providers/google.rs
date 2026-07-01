use super::prompt::build_reasoning_prompt;
use super::backoff::retry_delay;
use super::types::{parse_core_intent, LlmReasonContext, LlmRequest, LlmResponse, LlmUsage, ProviderError};
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
    #[serde(rename = "usageMetadata", default)]
    usage_metadata: Option<GoogleUsageMetadata>,
}

#[derive(Deserialize)]
struct GoogleUsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount", default)]
    candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount", default)]
    total_token_count: Option<u32>,
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

    pub async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = format!(
            "{}/v1beta/models",
            self.base_url.trim_end_matches('/')
        );
        let resp = self
            .client
            .get(&url)
            .header("X-goog-api-key", &self.api_key)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ProviderError::api("google", status.as_u16(), &message));
        }
        #[derive(Deserialize)]
        struct ModelsResponse {
            models: Option<Vec<GoogleModelEntry>>,
        }
        #[derive(Deserialize)]
        struct GoogleModelEntry {
            name: String,
        }
        let parsed: ModelsResponse = resp.json().await?;
        let mut ids: Vec<String> = parsed
            .models
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| m.name.strip_prefix("models/").map(str::to_string))
            .collect();
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    pub async fn health(&self) -> Result<bool, ProviderError> {
        self.complete_once("Respond with {}").await.map(|_| true).or_else(|e| {
            if matches!(e, ProviderError::Api { status: 401 | 403, .. }) {
                Ok(false)
            } else {
                Err(e)
            }
        })
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
            ProviderError::Misconfigured("google retries exhausted".into())
        }))
    }

    async fn complete_once(&self, prompt: &str) -> Result<LlmResponse, ProviderError> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url.trim_end_matches('/'),
            self.model
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
        let resp = self
            .client
            .post(&url)
            .header("X-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ProviderError::api("google", status.as_u16(), &message));
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
        let usage = parsed
            .usage_metadata
            .as_ref()
            .map(|u| {
                let mut usage =
                    LlmUsage::from_counts(u.prompt_token_count, u.candidates_token_count);
                if let Some(t) = u.total_token_count {
                    usage.total_tokens = Some(t);
                }
                usage
            })
            .unwrap_or_default();
        Ok(LlmResponse {
            content,
            provider_id: "google",
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