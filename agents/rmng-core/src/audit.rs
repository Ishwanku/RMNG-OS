use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub intent_id: Uuid,
    pub action: String,
    pub outcome: String,
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
    fn appends_jsonl_line() {
        let dir = std::env::temp_dir().join(format!("rmng-audit-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");
        let log = AuditLog::new(&path);
        log.append(&AuditEntry {
            timestamp: Utc::now(),
            intent_id: Uuid::new_v4(),
            action: "test".into(),
            outcome: "ok".into(),
            detail: None,
        })
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"action\":\"test\""));
        let _ = std::fs::remove_dir_all(dir);
    }
}
