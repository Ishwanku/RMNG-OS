use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Current audit entry schema (Sprint 10 — tamper-evident chain).
pub const AUDIT_SCHEMA_VERSION: u32 = 3;

/// Genesis hash when the chain starts empty.
pub const AUDIT_GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Execution plane track for audit correlation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditTrack {
    Native,
    Mcp,
    Plan,
}

/// High-level category for filtering (`jq`, `rmng observe`, cost queries).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Native,
    Mcp,
    Llm,
    Handoff,
    Circuit,
    Plan,
    System,
}

impl AuditCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Mcp => "mcp",
            Self::Llm => "llm",
            Self::Handoff => "handoff",
            Self::Circuit => "circuit",
            Self::Plan => "plan",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub seq: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prev_hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub entry_hash: String,
    pub timestamp: DateTime<Utc>,
    pub intent_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<AuditCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<AuditTrack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_peak_rss_kb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_cpu_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_prompt: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_completion: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_index: Option<u32>,
    pub action: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

fn default_schema_version() -> u32 {
    AUDIT_SCHEMA_VERSION
}

impl AuditEntry {
    /// Builder for structured nervous/body events (Sprint 10).
    pub fn new(action: impl Into<String>, outcome: impl Into<String>) -> Self {
        Self {
            schema_version: AUDIT_SCHEMA_VERSION,
            seq: 0,
            prev_hash: String::new(),
            entry_hash: String::new(),
            timestamp: Utc::now(),
            intent_id: Uuid::new_v4(),
            category: None,
            trace_id: None,
            session_id: None,
            agent_id: None,
            llm_profile: None,
            skill_name: None,
            track: None,
            duration_ms: None,
            mcp_server: None,
            mcp_pid: None,
            mcp_peak_rss_kb: None,
            mcp_cpu_time_ms: None,
            cost_usd: None,
            tokens_prompt: None,
            tokens_completion: None,
            fallback_index: None,
            action: action.into(),
            outcome: outcome.into(),
            detail: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ChainState {
    seq: u64,
    last_hash: String,
}

impl ChainState {
    fn load(audit_path: &Path) -> Self {
        let state_path = chain_state_path(audit_path);
        if !state_path.is_file() {
            return Self {
                seq: 0,
                last_hash: AUDIT_GENESIS_HASH.into(),
            };
        }
        let raw = std::fs::read_to_string(&state_path).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_else(|_| Self {
            seq: 0,
            last_hash: AUDIT_GENESIS_HASH.into(),
        })
    }

    fn save(&self, audit_path: &Path) -> std::io::Result<()> {
        let state_path = chain_state_path(audit_path);
        if let Some(parent) = state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string(self).expect("chain state serializes");
        std::fs::write(state_path, raw)
    }
}

fn chain_state_path(audit_path: &Path) -> PathBuf {
    audit_path.with_file_name("audit.chain")
}

#[derive(Clone)]
pub struct AuditLog {
    path: PathBuf,
}

/// Aggregate stats for `rmng audit verify --stats` and CI reporting.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditStats {
    pub total_entries: u64,
    pub by_category: std::collections::HashMap<String, u64>,
    pub llm_calls: u64,
    pub mcp_calls: u64,
    pub circuit_events: u64,
    pub handoff_events: u64,
    pub spent_today_usd: f64,
    pub spent_total_usd: f64,
    pub mcp_peak_rss_kb_max: u64,
    pub mcp_cpu_time_ms_total: u64,
    pub mcp_runtime_ms_total: u64,
}

pub fn compute_audit_stats(entries: &[AuditEntry]) -> AuditStats {
    use chrono::Utc;
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut stats = AuditStats {
        total_entries: entries.len() as u64,
        ..Default::default()
    };
    for e in entries {
        let cat = e
            .category
            .map(|c| c.as_str().to_string())
            .unwrap_or_else(|| "unknown".into());
        *stats.by_category.entry(cat).or_default() += 1;
        if e.category == Some(AuditCategory::Llm) {
            stats.llm_calls += 1;
            if let Some(c) = e.cost_usd {
                stats.spent_total_usd += c;
                if e.timestamp.format("%Y-%m-%d").to_string() == today {
                    stats.spent_today_usd += c;
                }
            }
        }
        if e.category == Some(AuditCategory::Mcp) {
            stats.mcp_calls += 1;
            if let Some(rss) = e.mcp_peak_rss_kb {
                stats.mcp_peak_rss_kb_max = stats.mcp_peak_rss_kb_max.max(rss);
            }
            if let Some(cpu) = e.mcp_cpu_time_ms {
                stats.mcp_cpu_time_ms_total += cpu;
            }
            if let Some(ms) = e.duration_ms {
                stats.mcp_runtime_ms_total += ms;
            }
        }
        if e.category == Some(AuditCategory::Circuit) {
            stats.circuit_events += 1;
        }
        if e.category == Some(AuditCategory::Handoff) {
            stats.handoff_events += 1;
        }
    }
    stats
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainVerifyResult {
    pub entries: u64,
    pub valid: bool,
    pub first_break_seq: Option<u64>,
    pub message: String,
}

impl AuditLog {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn default_path() -> PathBuf {
        dirs_fallback().join("audit.jsonl")
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append a sealed entry (hash chain + monotonic sequence).
    pub fn append(&self, entry: &AuditEntry) -> std::io::Result<AuditEntry> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut state = ChainState::load(&self.path);
        let mut sealed = entry.clone();
        sealed.schema_version = AUDIT_SCHEMA_VERSION;
        sealed.seq = state.seq + 1;
        sealed.prev_hash = state.last_hash.clone();
        let payload = canonical_payload(&sealed);
        sealed.entry_hash = sha256_hex(&format!("{}{}", sealed.prev_hash, payload));

        let line = serde_json::to_string(&sealed).expect("audit entry serializes");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{line}")?;

        state.seq = sealed.seq;
        state.last_hash = sealed.entry_hash.clone();
        state.save(&self.path)?;
        Ok(sealed)
    }

    /// Verify hash chain integrity over the full log (or tail).
    pub fn verify_chain(&self) -> std::io::Result<ChainVerifyResult> {
        if !self.path.is_file() {
            return Ok(ChainVerifyResult {
                entries: 0,
                valid: true,
                first_break_seq: None,
                message: "empty log".into(),
            });
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut prev_hash = AUDIT_GENESIS_HASH.to_string();
        let mut count = 0u64;
        let mut first_break = None;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };
            count += 1;

            if entry.schema_version >= 3 && !entry.entry_hash.is_empty() {
                let payload = canonical_payload(&entry);
                let expected = sha256_hex(&format!("{}{}", entry.prev_hash, payload));
                if entry.entry_hash != expected || entry.prev_hash != prev_hash {
                    if first_break.is_none() {
                        first_break = Some(entry.seq.max(count));
                    }
                }
                prev_hash = entry.entry_hash.clone();
            }
        }

        let valid = first_break.is_none();
        let message = if valid {
            format!("{count} entries, chain intact")
        } else {
            format!("chain break at seq {:?}", first_break)
        };
        Ok(ChainVerifyResult {
            entries: count,
            valid,
            first_break_seq: first_break,
            message,
        })
    }

    /// Read all entries in file order.
    pub fn read_all(&self) -> std::io::Result<Vec<AuditEntry>> {
        if !self.path.is_file() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str(&line) {
                out.push(entry);
            }
        }
        Ok(out)
    }

    /// Read recent entries (newest last in returned vec).
    pub fn tail(&self, n: usize) -> std::io::Result<Vec<AuditEntry>> {
        if !self.path.is_file() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)?;
        let lines: Vec<String> = BufReader::new(file).lines().collect::<Result<_, _>>()?;
        let tail: Vec<AuditEntry> = lines
            .iter()
            .rev()
            .take(n)
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(tail.into_iter().rev().collect())
    }
}

fn canonical_payload(entry: &AuditEntry) -> String {
    let mut clone = entry.clone();
    clone.entry_hash.clear();
    serde_json::to_string(&clone).unwrap_or_default()
}

fn sha256_hex(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn dirs_fallback() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".rmng").join("logs");
    }
    PathBuf::from("/tmp/rmng/logs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_sealed_chain_entries() {
        let dir = std::env::temp_dir().join(format!("rmng-audit-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");
        let log = AuditLog::new(&path);

        let e1 = log
            .append(&AuditEntry::new("git.status", "ok"))
            .unwrap();
        assert_eq!(e1.seq, 1);
        assert_eq!(e1.prev_hash, AUDIT_GENESIS_HASH);
        assert!(!e1.entry_hash.is_empty());

        let e2 = log
            .append(&AuditEntry::new("nervous.llm_call", "success"))
            .unwrap();
        assert_eq!(e2.seq, 2);
        assert_eq!(e2.prev_hash, e1.entry_hash);

        let verify = log.verify_chain().unwrap();
        assert!(verify.valid);
        assert_eq!(verify.entries, 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn detects_tampered_entry() {
        let dir = std::env::temp_dir().join(format!("rmng-audit-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");
        let log = AuditLog::new(&path);
        log.append(&AuditEntry::new("test.action", "ok")).unwrap();

        let mut content = std::fs::read_to_string(&path).unwrap();
        content = content.replace("\"ok\"", "\"tampered\"");
        std::fs::write(&path, content).unwrap();

        let verify = log.verify_chain().unwrap();
        assert!(!verify.valid);
        let _ = std::fs::remove_dir_all(dir);
    }
}