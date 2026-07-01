use super::prompt::build_reasoning_prompt;
use super::types::{parse_core_intent, LlmReasonContext, LlmRequest, LlmResponse, ProviderError};
use rmng_core::CoreIntent;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageOut,
}

#[derive(Deserialize)]
struct ChatMessageOut {
    content: Option<String>,
}

/// OpenAI-compatible chat completions (OpenAI, Grok, Groq, Together, Fireworks, DeepSeek, NIM, custom).
pub struct OpenAiCompatProvider {
    provider_id: &'static str,
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
    max_retries: u32,
    temperature: f32,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
}

impl OpenAiCompatProvider {
    pub fn new(
        provider_id: &'static str,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        timeout_secs: u64,
        max_retries: u32,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        top_p: Option<f32>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("http client");
        Self {
            provider_id,
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            client,
            max_retries,
            temperature: temperature.unwrap_or(0.0),
            max_tokens,
            top_p,
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = format!("{}/models", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ProviderError::api(
                self.provider_id,
                status.as_u16(),
                &message,
            ));
        }
        let body = resp.text().await?;
        Self::parse_models_body(&body)
    }

    fn parse_models_body(body: &str) -> Result<Vec<String>, ProviderError> {
        #[derive(Deserialize)]
        struct ModelsResponse {
            data: Option<Vec<ModelEntry>>,
        }
        #[derive(Deserialize)]
        struct ModelEntry {
            id: String,
        }
        let parsed: ModelsResponse = serde_json::from_str(body).map_err(|e| {
            ProviderError::Misconfigured(format!("models list parse error: {e}"))
        })?;
        let mut ids: Vec<String> = parsed
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.id)
            .collect();
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    pub fn id(&self) -> &'static str {
        self.provider_id
    }

    pub async fn health(&self) -> Result<bool, ProviderError> {
        let url = format!("{}/models", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        if resp.status().is_success() {
            return Ok(true);
        }
        let status = resp.status().as_u16();
        let message = resp.text().await.unwrap_or_default();
        if status == 404 {
            return self.complete_once("Respond with {}").await.map(|_| true);
        }
        Err(ProviderError::api(self.provider_id, status, &message))
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
            ProviderError::Misconfigured(format!("{} retries exhausted", self.provider_id))
        }))
    }

    async fn complete_once(&self, prompt: &str) -> Result<LlmResponse, ProviderError> {
        let url = format!(
            "{}/chat/completions",
            self.base_url.trim_end_matches('/')
        );
        let body = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage {
                role: "user",
                content: prompt,
            }],
            response_format: Some(ResponseFormat {
                kind: "json_object",
            }),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            top_p: self.top_p,
        };
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(ProviderError::api(
                self.provider_id,
                status.as_u16(),
                &message,
            ));
        }
        let parsed: ChatResponse = resp.json().await?;
        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| {
                ProviderError::Misconfigured(format!(
                    "{} returned empty completion",
                    self.provider_id
                ))
            })?;
        Ok(LlmResponse {
            content,
            provider_id: self.provider_id,
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