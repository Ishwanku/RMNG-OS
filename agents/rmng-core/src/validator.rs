use crate::intent::{CoreIntent, CORE_INTENT_SCHEMA_VERSION};
use crate::registry::IntegrationRegistry;
use crate::RmngError;
use jsonschema::Validator;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Validates v2 `CoreIntent` envelopes and per-tool parameter schemas.
#[derive(Clone)]
pub struct IntentValidator {
    core_schema: Arc<Validator>,
    registry: IntegrationRegistry,
    tool_schemas: HashMap<String, Arc<Validator>>,
}

impl IntentValidator {
    pub fn new(registry: IntegrationRegistry) -> Result<Self, RmngError> {
        let schema_path = core_intent_schema_path();
        let raw = std::fs::read_to_string(&schema_path).map_err(|e| {
            RmngError::InvalidIntent(format!(
                "read core intent schema {}: {e}",
                schema_path.display()
            ))
        })?;
        let schema_value: Value = serde_json::from_str(&raw)?;
        let core_schema = Arc::new(
            jsonschema::validator_for(&schema_value).map_err(|e| {
                RmngError::InvalidIntent(format!("compile core intent schema: {e}"))
            })?,
        );

        let mut tool_schemas = HashMap::new();
        for name in registry.allowed_tool_names() {
            let Some(tool) = registry.tool(&name) else {
                continue;
            };
            if tool.parameters.is_null() {
                continue;
            }
            let validator = jsonschema::validator_for(&tool.parameters).map_err(|e| {
                RmngError::InvalidIntent(format!("compile parameters schema for {name}: {e}"))
            })?;
            tool_schemas.insert(name, Arc::new(validator));
        }

        Ok(Self {
            core_schema,
            registry,
            tool_schemas,
        })
    }

    /// Validate parsed intent: core envelope schema, then per-tool parameters when applicable.
    pub fn validate(&self, intent: &CoreIntent) -> Result<(), RmngError> {
        let json = serde_json::to_value(intent)?;
        self.validate_value(&json)?;
        if let CoreIntent::ToolExecute {
            target,
            parameters,
            ..
        } = intent
        {
            self.validate_tool_parameters(target, parameters)?;
        }
        Ok(())
    }

    pub fn registry(&self) -> &IntegrationRegistry {
        &self.registry
    }

    pub fn validate_tool_parameters(
        &self,
        tool_name: &str,
        parameters: &Value,
    ) -> Result<(), RmngError> {
        if !self.registry.has_tool(tool_name) {
            return Err(RmngError::InvalidIntent(format!(
                "unknown native tool '{tool_name}' (not in integration manifests)"
            )));
        }
        let Some(schema) = self.tool_schemas.get(tool_name) else {
            return Ok(());
        };
        let messages: Vec<String> = schema
            .iter_errors(parameters)
            .map(|e| e.to_string())
            .collect();
        if !messages.is_empty() {
            return Err(RmngError::InvalidIntent(format!(
                "tool '{tool_name}' parameters invalid: {}",
                messages.join("; ")
            )));
        }
        Ok(())
    }

    fn validate_value(&self, value: &Value) -> Result<(), RmngError> {
        if let Some(ver) = value.get("schema_version").and_then(|v| v.as_str()) {
            if ver != CORE_INTENT_SCHEMA_VERSION {
                return Err(RmngError::InvalidIntent(format!(
                    "unsupported core intent schema_version '{ver}' (expected {CORE_INTENT_SCHEMA_VERSION})"
                )));
            }
        }
        let messages: Vec<String> = self
            .core_schema
            .iter_errors(value)
            .map(|e| e.to_string())
            .collect();
        if !messages.is_empty() {
            return Err(RmngError::InvalidIntent(format!(
                "core intent schema validation failed: {}",
                messages.join("; ")
            )));
        }
        Ok(())
    }
}

fn core_intent_schema_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join("../schemas/core-intent.schema.json");
    if path.is_file() {
        return path;
    }
    if let Ok(root) = std::env::var("RMNG_PROJECT_ROOT") {
        let path = PathBuf::from(root).join("agents/schemas/core-intent.schema.json");
        if path.is_file() {
            return path;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home)
            .join("dev/projects/RMNG-OS/agents/schemas/core-intent.schema.json");
        if path.is_file() {
            return path;
        }
    }
    PathBuf::from("agents/schemas/core-intent.schema.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::CoreIntent;
    use crate::registry::IntegrationRegistry;

    fn validator() -> IntentValidator {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../integrations");
        let registry = IntegrationRegistry::load_from(root).unwrap();
        IntentValidator::new(registry).unwrap()
    }

    #[test]
    fn accepts_valid_tool_execute() {
        let v = validator();
        let intent = CoreIntent::ToolExecute {
            target: "git.status".into(),
            parameters: serde_json::json!({}),
            metadata: None,
        };
        assert!(v.validate(&intent).is_ok());
    }

    #[test]
    fn rejects_unknown_tool() {
        let v = validator();
        let intent = CoreIntent::ToolExecute {
            target: "system.rm_rf".into(),
            parameters: serde_json::json!({}),
            metadata: None,
        };
        assert!(v.validate(&intent).is_err());
    }

    #[test]
    fn rejects_invalid_git_diff_parameters() {
        let v = validator();
        let intent = CoreIntent::ToolExecute {
            target: "git.diff".into(),
            parameters: serde_json::json!({ "staged": "yes" }),
            metadata: None,
        };
        assert!(v.validate(&intent).is_err());
    }
}