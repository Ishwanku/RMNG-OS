use super::prompt::build_reasoning_prompt;
use super::types::{parse_core_intent, LlmReasonContext, LlmRequest, LlmResponse, ProviderError};
use rmng_core::CoreIntent;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

pub struct AnthropicProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
    max_retries: u32,
}

impl AnthropicProvider {
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
        "anthropic"
    }

    pub async fn health(&self) -> Result<bool, ProviderError> {
        if self.api_key.is_empty() {
            return Ok(false);
        }
        // Key-presence check only — avoids token spend on every `rmng llm health`.
        // Use `scripts/probe-anthropic-minimal.py` for a ~20-token live probe.
        Ok(true)
    }

    pub async fn complete(&self, req: LlmRequest<'_>) -> Result<LlmResponse, ProviderError> {
        let user_prompt = build_reasoning_prompt(req.assembled_prompt, &req.ctx);
        let mut last_err = None;
        for attempt in 0..=self.max_retries {
            match self.complete_once(&user_prompt).await {
                Ok(resp) => return Ok(resp),
                Err(e) if e.is_retryable() && attempt < self.max_retries => {
                    last_err = Some(e);
                    tokio::time::sleep(Duration::from_millis(800 * (attempt as u64 + 1))).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            ProviderError::Misconfigured("anthropic retries exhausted".into())
        }))
    }

    async fn complete_once(&self, user_prompt: &str) -> Result<LlmResponse, ProviderError> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let body = MessagesRequest {
            model: &self.model,
            max_tokens: 1024, // intent JSON only — keep API spend low
            system: "You output only valid JSON for RMNG core-intent v2. No markdown.",
            messages: vec![AnthropicMessage {
                role: "user",
                content: user_prompt,
            }],
        };
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                provider: "anthropic".into(),
                status: status.as_u16(),
                message,
            });
        }
        let parsed: MessagesResponse = resp.json().await?;
        let content = parsed
            .content
            .first()
            .and_then(|b| b.text.clone())
            .ok_or_else(|| ProviderError::Misconfigured("anthropic empty response".into()))?;
        Ok(LlmResponse {
            content,
            provider_id: "anthropic",
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