use crate::RmngError;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One integration domain manifest (`integrations/<domain>/*.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct IntegrationManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub tools: Vec<ToolManifest>,
}

/// Native tool definition loaded from integration manifests.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

/// Registry of all native tools discovered under `integrations/**/*.json`.
#[derive(Debug, Clone)]
pub struct IntegrationRegistry {
    root: PathBuf,
    manifests: Vec<IntegrationManifest>,
    tools: HashMap<String, ToolManifest>,
}

impl IntegrationRegistry {
    /// Load all integration manifests from the project `integrations/` tree.
    pub fn load() -> Result<Self, RmngError> {
        let root = integrations_root();
        Self::load_from(&root)
    }

    pub fn load_from(root: impl AsRef<Path>) -> Result<Self, RmngError> {
        let root = root.as_ref().to_path_buf();
        let mut manifests = Vec::new();
        let mut tools = HashMap::new();

        if !root.is_dir() {
            tracing::warn!(
                path = %root.display(),
                "integrations directory missing — PermissionGate will have no native tools"
            );
            return Ok(Self {
                root,
                manifests,
                tools,
            });
        }

        let mut files = Vec::new();
        collect_json_files(&root, &mut files).map_err(|e| {
            RmngError::InvalidIntent(format!("walk integrations tree: {e}"))
        })?;

        for path in files {
            let raw = std::fs::read_to_string(&path)
                .map_err(|e| RmngError::InvalidIntent(format!("read {}: {e}", path.display())))?;
            let manifest: IntegrationManifest = serde_json::from_str(&raw).map_err(|e| {
                RmngError::InvalidIntent(format!("parse {}: {e}", path.display()))
            })?;
            for tool in &manifest.tools {
                if tools.contains_key(&tool.name) {
                    return Err(RmngError::InvalidIntent(format!(
                        "duplicate tool name '{}' in integrations manifests",
                        tool.name
                    )));
                }
                tools.insert(tool.name.clone(), tool.clone());
            }
            manifests.push(manifest);
        }

        manifests.sort_by(|a, b| a.name.cmp(&b.name));
        tracing::info!(
            path = %root.display(),
            manifests = manifests.len(),
            tools = tools.len(),
            "integration registry loaded"
        );
        Ok(Self {
            root,
            manifests,
            tools,
        })
    }

    pub fn integrations_root(&self) -> &Path {
        &self.root
    }

    pub fn manifests(&self) -> &[IntegrationManifest] {
        &self.manifests
    }

    pub fn tool(&self, name: &str) -> Option<&ToolManifest> {
        self.tools.get(name)
    }

    pub fn allowed_tool_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

/// Resolve `integrations/` relative to RMNG-OS repo root.
pub fn integrations_root() -> PathBuf {
    if let Ok(root) = std::env::var("RMNG_PROJECT_ROOT") {
        let path = PathBuf::from(root).join("integrations");
        if path.is_dir() {
            return path;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join("dev/projects/RMNG-OS/integrations");
        if path.is_dir() {
            return path;
        }
    }
    PathBuf::from("integrations")
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "json") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../integrations")
    }

    #[test]
    fn loads_dev_integrations() {
        let reg = IntegrationRegistry::load_from(fixture_root()).expect("load fixtures");
        assert!(reg.has_tool("kernel.status"));
        assert!(reg.has_tool("git.status"));
        assert!(reg.has_tool("git.diff"));
        assert!(reg.has_tool("github.pr_status"));
        let names = reg.allowed_tool_names();
        assert!(names.len() >= 6);
    }

    #[test]
    fn tool_has_parameters_schema() {
        let reg = IntegrationRegistry::load_from(fixture_root()).unwrap();
        let git = reg.tool("git.status").unwrap();
        assert_eq!(git.parameters.get("type").and_then(|v| v.as_str()), Some("object"));
    }
}