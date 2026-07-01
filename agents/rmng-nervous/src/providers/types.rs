use crate::nervous_audit::log_nervous_event;
use rmng_core::CoreIntent;
use serde_json::Value;
use serde::{Deserialize, Serialize};

/// Session/agent context passed to every provider adapter.
#[derive(Debug, Clone, Default)]
pub struct LlmReasonContext<'a> {
    pub session_id: Option<&'a str>,
    pub agent_id: Option<&'a str>,
    pub skill_name: Option<&'a str>,
    /// Active provider id for model-specific chain hints (Sprint 25).
    pub provider_id: Option<&'a str>,
}

/// Standard nervous-system request — all providers receive the same shape.
#[derive(Debug, Clone)]
pub struct LlmRequest<'a> {
    pub assembled_prompt: &'a str,
    pub ctx: LlmReasonContext<'a>,
}

/// Token usage and optional cost estimate (Sprint 9).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LlmUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
    /// `provider` (billing API), `estimate` (catalog/heuristic), or `none`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_source: Option<String>,
}

impl LlmUsage {
    pub fn from_counts(prompt: Option<u32>, completion: Option<u32>) -> Self {
        let total = prompt
            .zip(completion)
            .map(|(p, c)| p.saturating_add(c))
            .or(prompt)
            .or(completion);
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: total,
            ..Default::default()
        }
    }

    pub fn merge(&mut self, other: &LlmUsage) {
        self.prompt_tokens = sum_opt(self.prompt_tokens, other.prompt_tokens);
        self.completion_tokens = sum_opt(self.completion_tokens, other.completion_tokens);
        self.total_tokens = sum_opt(self.total_tokens, other.total_tokens);
        self.estimated_cost_usd = sum_cost(self.estimated_cost_usd, other.estimated_cost_usd);
        if other.cost_source.is_some() {
            self.cost_source = other.cost_source.clone();
        }
    }
}

fn sum_opt(a: Option<u32>, b: Option<u32>) -> Option<u32> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.saturating_add(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

fn sum_cost(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x + y),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

/// Result of nervous reasoning — intent plus aggregated usage across retries.
#[derive(Debug, Clone)]
pub struct ReasonResult {
    pub intent: CoreIntent,
    pub usage: LlmUsage,
}

/// Standard nervous-system response — raw JSON text from the model.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub provider_id: &'static str,
    pub model: String,
    pub usage: LlmUsage,
}

/// Classified failure kind for health/matrix reporting (Sprint 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorKind {
    Misconfigured,
    InvalidKey,
    Billing,
    ModelNotFound,
    RateLimit,
    Network,
    InvalidIntent,
    Other,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("provider misconfigured: {0}")]
    Misconfigured(String),
    #[error("{provider} API error ({status}): {message}")]
    Api {
        provider: String,
        status: u16,
        message: String,
    },
    #[error("invalid intent from model: {0}")]
    InvalidIntent(#[from] rmng_core::RmngError),
    #[error("provider not implemented: {0}")]
    NotImplemented(String),
}

impl ProviderError {
    pub fn kind(&self) -> ProviderErrorKind {
        match self {
            Self::Misconfigured(_) => ProviderErrorKind::Misconfigured,
            Self::InvalidIntent(_) => ProviderErrorKind::InvalidIntent,
            Self::NotImplemented(_) => ProviderErrorKind::Other,
            Self::Http(e) if e.is_timeout() || e.is_connect() => ProviderErrorKind::Network,
            Self::Http(_) => ProviderErrorKind::Network,
            Self::Api { status, message, .. } => classify_api_error(*status, message),
        }
    }

    pub fn api(provider: &str, status: u16, message: &str) -> Self {
        let hint = match classify_api_error(status, message) {
            ProviderErrorKind::InvalidKey => " — invalid or expired API key",
            ProviderErrorKind::Billing => {
                " — billing/credits issue (add credits or check account status)"
            }
            ProviderErrorKind::ModelNotFound => " — model not found or not enabled for this key",
            ProviderErrorKind::RateLimit => " — rate limited; retry with backoff",
            _ => "",
        };
        let trimmed = if message.len() > 240 {
            format!("{}…", &message[..240])
        } else {
            message.to_string()
        };
        Self::Api {
            provider: provider.to_string(),
            status,
            message: format!("{trimmed}{hint}"),
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(e) => e.is_timeout() || e.is_connect() || e.is_request(),
            Self::Api { status, .. } => matches!(*status, 429 | 500 | 502 | 503 | 504),
            Self::InvalidIntent(_) => true,
            _ => false,
        }
    }

    /// Whether the nervous layer should try the next profile in `llm_fallback` (Sprint 8).
    pub fn warrants_provider_fallback(&self) -> bool {
        matches!(
            self.kind(),
            ProviderErrorKind::RateLimit
                | ProviderErrorKind::Network
                | ProviderErrorKind::Billing
                | ProviderErrorKind::ModelNotFound
                | ProviderErrorKind::Other
        )
    }
}

fn classify_api_error(status: u16, message: &str) -> ProviderErrorKind {
    let lower = message.to_lowercase();
    if status == 401 {
        return ProviderErrorKind::InvalidKey;
    }
    if status == 429 {
        return ProviderErrorKind::RateLimit;
    }
    if status == 404 && (lower.contains("model") || lower.contains("not found")) {
        return ProviderErrorKind::ModelNotFound;
    }
    if status == 403 || status == 402 {
        if lower.contains("credit")
            || lower.contains("billing")
            || lower.contains("quota")
            || lower.contains("payment")
            || lower.contains("license")
            || lower.contains("spend")
        {
            return ProviderErrorKind::Billing;
        }
        return ProviderErrorKind::InvalidKey;
    }
    if lower.contains("model") && (lower.contains("not found") || lower.contains("does not exist")) {
        return ProviderErrorKind::ModelNotFound;
    }
    ProviderErrorKind::Other
}

fn strip_trailing_commas(json: &str) -> String {
    let mut s = json.to_string();
    loop {
        let next = s.replace(",}", "}").replace(",]", "]");
        if next == s {
            break;
        }
        s = next;
    }
    s
}

/// Split model-emitted agent lists (comma, arrow, or JSON-array string).
pub fn parse_agent_id_list(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.starts_with('[') {
        if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(trimmed) {
            return arr
                .iter()
                .filter_map(|v| v.as_str().map(str::trim))
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
    }
    let splitter = if trimmed.contains("->") {
        "->"
    } else if trimmed.contains('→') {
        "→"
    } else if trimmed.contains(';') {
        ";"
    } else if trimmed.contains('|') {
        "|"
    } else {
        ","
    };
    trimmed
        .split(splitter)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn filter_agent_array(arr: Vec<Value>) -> Vec<Value> {
    arr.into_iter()
        .filter_map(|v| {
            let s = v.as_str()?.trim();
            if s.is_empty() {
                None
            } else {
                Some(Value::String(s.to_string()))
            }
        })
        .collect()
}

fn normalize_handoff_chain_value(chain: Value) -> Option<Value> {
    match chain {
        Value::Array(mut arr) => {
            arr = filter_agent_array(arr);
            if arr.len() == 1 {
                if let Some(s) = arr.pop().and_then(|v| v.as_str().map(str::to_string)) {
                    let ids = parse_agent_id_list(&s);
                    if ids.len() >= 2 {
                        return Some(Value::Array(
                            ids.into_iter().map(Value::String).collect(),
                        ));
                    }
                }
            }
            if arr.len() >= 2 {
                Some(Value::Array(arr))
            } else {
                None
            }
        }
        Value::String(s) => {
            let ids = parse_agent_id_list(&s);
            if ids.len() >= 2 {
                Some(Value::Array(ids.into_iter().map(Value::String).collect()))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract JSON object from LLM output (fences, prose wrappers).
pub fn extract_json_payload(content: &str) -> Option<String> {
    let mut trimmed = content.trim();
    if trimmed.starts_with("```") {
        trimmed = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```JSON")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
    }
    let candidates = [trimmed.to_string(), strip_trailing_commas(trimmed)];
    for cand in &candidates {
        if serde_json::from_str::<Value>(cand).is_ok() {
            return Some(cand.clone());
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end > start {
        let slice = strip_trailing_commas(&trimmed[start..=end]);
        if serde_json::from_str::<Value>(&slice).is_ok() {
            return Some(slice);
        }
    }
    None
}

/// Coerce common LLM mistakes in handoff metadata before strict parse.
pub fn normalize_intent_value(value: &mut Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    if let Some(action) = obj.get("action").and_then(|v| v.as_str()) {
        let alias = match action.to_ascii_lowercase().as_str() {
            "plan_only" | "plan-only" | "planonly" => Some("plan.only"),
            "tool_execute" | "tool-execute" | "toolexecute" => Some("tool.execute"),
            "mcp_proxy" | "mcp-proxy" | "mcpproxy" => Some("mcp.proxy"),
            _ => None,
        };
        if let Some(normalized) = alias {
            log_nervous_event(
                "nervous.parse_normalize",
                "action_alias",
                Some(&format!("{action} -> {normalized}")),
            );
            obj.insert("action".into(), Value::String(normalized.to_string()));
        }
    }
    if !obj.contains_key("metadata") {
        obj.insert("metadata".into(), Value::Object(serde_json::Map::new()));
    }
    let mut hoisted: Vec<(&str, Value)> = Vec::new();
    for key in [
        "handoff_chain",
        "handoff_to",
        "handoff_return_to",
        "chain_id",
        "hop_failure_policy",
        "hop_retry_max",
    ] {
        if let Some(v) = obj.remove(key) {
            hoisted.push((key, v));
        }
    }
    let Some(meta) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) else {
        return;
    };
    for (key, v) in hoisted {
        meta.entry(key.to_string()).or_insert(v);
    }
    if let Some(chain) = meta.get("handoff_chain").cloned() {
        match normalize_handoff_chain_value(chain.clone()) {
            Some(normalized) => {
                if normalized != chain {
                    log_nervous_event(
                        "nervous.parse_normalize",
                        "handoff_chain",
                        Some(&format!("coerced {chain} -> {normalized}")),
                    );
                }
                meta.insert("handoff_chain".into(), normalized);
            }
            None => {
                tracing::warn!(
                    handoff_chain = %chain,
                    "dropped invalid handoff_chain (need >= 2 agent ids after normalization)"
                );
                log_nervous_event(
                    "nervous.parse_normalize",
                    "handoff_chain_dropped",
                    Some(&format!("invalid chain: {chain}")),
                );
                meta.remove("handoff_chain");
            }
        }
    }
    for key in ["handoff_to", "handoff_return_to", "handoff_from", "chain_id"] {
        if let Some(v) = meta.get_mut(key) {
            if let Some(s) = v.as_str() {
                *v = Value::String(s.trim().to_string());
            }
        }
    }
    if let Some(v) = meta.get_mut("hop_failure_policy") {
        if let Some(s) = v.as_str() {
            *v = Value::String(s.trim().to_ascii_lowercase());
        }
    }
}

/// Parse provider output into a v2 CoreIntent (Sprint 24 — robust extraction).
pub fn parse_core_intent(content: &str) -> Result<CoreIntent, ProviderError> {
    use rmng_core::RmngError;
    let json_str = extract_json_payload(content).ok_or_else(|| {
        ProviderError::InvalidIntent(RmngError::InvalidIntent(
            "no JSON object found in LLM output".into(),
        ))
    })?;
    let mut value: Value = serde_json::from_str(&json_str).map_err(|e| {
        ProviderError::InvalidIntent(RmngError::InvalidIntent(e.to_string()))
    })?;
    normalize_intent_value(&mut value);
    serde_json::from_value(value).map_err(|e| {
        ProviderError::InvalidIntent(RmngError::InvalidIntent(e.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warrants_fallback_on_rate_limit_and_billing() {
        let rate = ProviderError::api("groq", 429, "rate limit exceeded");
        assert!(rate.warrants_provider_fallback());
        assert_eq!(rate.kind(), ProviderErrorKind::RateLimit);

        let billing = ProviderError::api("grok", 403, "insufficient credits for billing");
        assert!(billing.warrants_provider_fallback());
        assert_eq!(billing.kind(), ProviderErrorKind::Billing);
    }

    #[test]
    fn parses_handoff_chain_from_comma_string() {
        let raw = r#"{"action":"plan.only","reasoning":"chain","metadata":{"handoff_chain":"swarm-coordinator, repo-keeper, runtime-executor"}}"#;
        let intent = parse_core_intent(raw).expect("parse");
        let chain = intent
            .metadata()
            .and_then(|m| m.handoff_chain.as_ref())
            .expect("chain");
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[1], "repo-keeper");
    }

    #[test]
    fn extracts_json_from_prose_wrapper() {
        let raw = r#"Here is the intent:
```json
{"action":"plan.only","reasoning":"done","metadata":{"session_id":"abc"}}
```
"#;
        let intent = parse_core_intent(raw).expect("parse");
        assert!(matches!(intent, CoreIntent::PlanOnly { .. }));
    }

    #[test]
    fn parses_handoff_chain_from_arrow_string() {
        let raw = r#"{"action":"plan.only","reasoning":"chain","metadata":{"handoff_chain":"swarm-coordinator -> repo-keeper -> runtime-executor"}}"#;
        let intent = parse_core_intent(raw).expect("parse");
        let chain = intent.metadata().unwrap().handoff_chain.as_ref().unwrap();
        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn parses_handoff_chain_from_json_array_string() {
        let raw = "{\"action\":\"plan.only\",\"reasoning\":\"chain\",\"metadata\":{\"handoff_chain\":\"[\\\"swarm-coordinator\\\",\\\"repo-keeper\\\"]\"}}";
        let intent = parse_core_intent(raw).expect("parse");
        assert_eq!(intent.metadata().unwrap().handoff_chain.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn normalizes_plan_only_action_alias() {
        let raw = r#"{"action":"plan_only","reasoning":"x","metadata":{}}"#;
        let intent = parse_core_intent(raw).expect("parse");
        assert!(matches!(intent, CoreIntent::PlanOnly { .. }));
    }

    #[test]
    fn hoists_top_level_handoff_chain_into_metadata() {
        let raw = r#"{"action":"plan.only","reasoning":"x","handoff_chain":["swarm-coordinator","repo-keeper"]}"#;
        let intent = parse_core_intent(raw).expect("parse");
        assert!(intent.metadata().unwrap().handoff_chain.is_some());
    }

    #[test]
    fn strips_trailing_commas_before_parse() {
        let raw = r#"{"action":"plan.only","reasoning":"x","metadata":{"session_id":"s1",},}"#;
        let intent = parse_core_intent(raw).expect("parse");
        assert!(matches!(intent, CoreIntent::PlanOnly { .. }));
    }

    #[test]
    fn invalid_key_does_not_warrant_fallback() {
        let key = ProviderError::api("openai", 401, "invalid api key");
        assert!(!key.warrants_provider_fallback());
        assert_eq!(key.kind(), ProviderErrorKind::InvalidKey);
    }
}