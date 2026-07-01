use rmng_core::CoreIntent;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_URL: &str = "http://127.0.0.1:11434";
const DEFAULT_MODEL: &str = "llama3.2";

#[derive(Debug, Clone, Default)]
pub struct LlmReasonContext<'a> {
    pub session_id: Option<&'a str>,
    pub agent_id: Option<&'a str>,
    pub skill_name: Option<&'a str>,
}

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
        ctx: &LlmReasonContext<'_>,
    ) -> Result<CoreIntent, NervousError> {
        let mut hints = Vec::new();
        if let Some(name) = ctx.skill_name {
            hints.push(format!(
                "Include metadata.skill_name = \"{name}\" when appropriate."
            ));
        }
        if let Some(sid) = ctx.session_id {
            hints.push(format!(
                "REQUIRED: include metadata.session_id = \"{sid}\" on the intent."
            ));
        }
        if let Some(agent) = ctx.agent_id {
            hints.push(format!(
                "You are agent \"{agent}\". Only emit tools listed in your Allowed tools section."
            ));
        }
        let hint_block = if hints.is_empty() {
            String::new()
        } else {
            format!("\n{}\n", hints.join("\n"))
        };

        let examples = r#"
Example intents (use exactly one):
{"action":"tool.execute","target":"git.status","parameters":{},"metadata":{"session_id":"<sid>"}}
{"action":"mcp.proxy","mcp_server":"github","mcp_tool":"search_issues","mcp_args":{"query":"repo:Ishwanku/RMNG-OS is:open"},"metadata":{"session_id":"<sid>"}}
{"action":"plan.only","reasoning":"Task complete. Summarize prior tool results.","metadata":{"session_id":"<sid>"}}
"#;

        let prompt = format!(
            "{assembled_prompt}{hint_block}{examples}\nRespond with a single JSON object for core-intent v2."
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