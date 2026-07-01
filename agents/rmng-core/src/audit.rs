use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

/// Execution plane track for audit correlation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditTrack {
    Native,
    Mcp,
    Plan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub intent_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<AuditTrack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server: Option<String>,
    pub action: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone)]
pub struct AuditLog {
    path: std::path::PathBuf,
}

impl AuditLog {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn default_path() -> std::path::PathBuf {
        dirs_fallback().join("audit.jsonl")
    }

    pub fn append(&self, entry: &AuditEntry) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let line = serde_json::to_string(entry).expect("audit entry serializes");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{line}")?;
        Ok(())
    }
}

fn dirs_fallback() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home).join(".rmng").join("logs");
    }
    std::path::PathBuf::from("/tmp/rmng/logs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_jsonl_line_with_v2_fields() {
        let dir = std::env::temp_dir().join(format!("rmng-audit-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");
        let log = AuditLog::new(&path);
        log.append(&AuditEntry {
            timestamp: Utc::now(),
            intent_id: Uuid::new_v4(),
            trace_id: Some(Uuid::new_v4().to_string()),
            skill_name: Some("git-workflow".into()),
            track: Some(AuditTrack::Native),
            duration_ms: Some(12),
            mcp_server: None,
            action: "git.status".into(),
            outcome: "ok".into(),
            detail: None,
        })
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"track\":\"native\""));
        assert!(content.contains("\"duration_ms\":12"));
        let _ = std::fs::remove_dir_all(dir);
    }
}