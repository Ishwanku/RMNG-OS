use serde::{Deserialize, Serialize};

/// Lightweight per-subprocess resource summary (Sprint 20).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ResourceMetrics {
    /// Peak resident set size in kilobytes (Linux wait4 ru_maxrss).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_rss_kb: Option<u64>,
    /// User + system CPU time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_time_ms: Option<u64>,
    /// Wall-clock runtime in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_ms: Option<u64>,
}

impl ResourceMetrics {
    pub fn with_runtime(mut self, ms: u64) -> Self {
        self.runtime_ms = Some(ms);
        self
    }

    pub fn has_usage(&self) -> bool {
        self.peak_rss_kb.is_some() || self.cpu_time_ms.is_some()
    }
}

#[cfg(unix)]
pub fn harvest_child_resources(pid: u32) -> ResourceMetrics {
    use libc::{rusage, wait4};

    let mut status: i32 = 0;
    let mut usage: rusage = unsafe { std::mem::zeroed() };
    let waited = unsafe { wait4(pid as i32, &mut status, 0, &mut usage) };
    if waited < 0 {
        return ResourceMetrics::default();
    }

    let cpu_ms = timeval_to_ms(&usage.ru_utime) + timeval_to_ms(&usage.ru_stime);
    let peak_rss_kb = normalize_maxrss(usage.ru_maxrss as u64);

    ResourceMetrics {
        peak_rss_kb: if peak_rss_kb > 0 { Some(peak_rss_kb) } else { None },
        cpu_time_ms: if cpu_ms > 0 { Some(cpu_ms) } else { None },
        runtime_ms: None,
    }
}

#[cfg(unix)]
fn timeval_to_ms(tv: &libc::timeval) -> u64 {
    (tv.tv_sec as u64).saturating_mul(1000) + (tv.tv_usec as u64 / 1000)
}

#[cfg(unix)]
fn normalize_maxrss(raw: u64) -> u64 {
    #[cfg(target_os = "macos")]
    {
        raw / 1024
    }
    #[cfg(not(target_os = "macos"))]
    {
        raw
    }
}

#[cfg(not(unix))]
pub fn harvest_child_resources(_pid: u32) -> ResourceMetrics {
    ResourceMetrics::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_metrics_serialize_optional_fields() {
        let m = ResourceMetrics {
            peak_rss_kb: Some(1024),
            cpu_time_ms: Some(12),
            runtime_ms: Some(50),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("peak_rss_kb"));
        let empty = ResourceMetrics::default();
        let json2 = serde_json::to_string(&empty).unwrap();
        assert_eq!(json2, "{}");
    }
}
