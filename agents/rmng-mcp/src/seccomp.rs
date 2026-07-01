//! Seccomp BPF profiles for high-risk MCP subprocesses (Sprint 21).
//! Blocklist-oriented filters: default ALLOW, deny dangerous syscalls.

use std::convert::TryInto;

/// Known profile names (configurable per MCP server).
pub const PROFILE_BASIC: &str = "basic";
pub const PROFILE_PLAYWRIGHT: &str = "playwright";
pub const PROFILE_E2B: &str = "e2b";

pub fn is_high_risk_mcp_server(server: &str) -> bool {
    matches!(server, "playwright" | "e2b")
}

pub fn normalize_profile(name: &str) -> Option<&'static str> {
    match name.trim().to_ascii_lowercase().as_str() {
        "" | "off" | "none" | "false" => None,
        "basic" => Some(PROFILE_BASIC),
        "playwright" => Some(PROFILE_PLAYWRIGHT),
        "e2b" => Some(PROFILE_E2B),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
const FILTER_BASIC: &str = r#"{
  "default": {
  "mismatch_action": "allow",
  "match_action": {"errno": 1},
  "filter": [
    {"syscall": "mount"}, {"syscall": "umount2"}, {"syscall": "pivot_root"},
    {"syscall": "chroot"}, {"syscall": "kexec_load"}, {"syscall": "kexec_file_load"},
    {"syscall": "init_module"}, {"syscall": "finit_module"}, {"syscall": "delete_module"},
    {"syscall": "reboot"}, {"syscall": "swapon"}, {"syscall": "swapoff"},
    {"syscall": "bpf"}, {"syscall": "userfaultfd"}, {"syscall": "perf_event_open"},
    {"syscall": "sethostname"}, {"syscall": "setdomainname"},
    {"syscall": "settimeofday"}, {"syscall": "clock_settime"}, {"syscall": "adjtimex"},
    {"syscall": "acct"}, {"syscall": "iopl"}, {"syscall": "ioperm"}, {"syscall": "quotactl"},
    {"syscall": "setns"}, {"syscall": "unshare"}, {"syscall": "keyctl"},
    {"syscall": "add_key"}, {"syscall": "request_key"}
  ]
  }
}"#;

#[cfg(target_os = "linux")]
const FILTER_PLAYWRIGHT: &str = r#"{
  "default": {
  "mismatch_action": "allow",
  "match_action": {"errno": 1},
  "filter": [
    {"syscall": "mount"}, {"syscall": "umount2"}, {"syscall": "pivot_root"},
    {"syscall": "chroot"}, {"syscall": "kexec_load"}, {"syscall": "kexec_file_load"},
    {"syscall": "init_module"}, {"syscall": "finit_module"}, {"syscall": "delete_module"},
    {"syscall": "reboot"}, {"syscall": "bpf"}, {"syscall": "iopl"}, {"syscall": "ioperm"},
    {"syscall": "swapon"}, {"syscall": "swapoff"}
  ]
  }
}"#;

#[cfg(target_os = "linux")]
const FILTER_E2B: &str = FILTER_BASIC;

#[cfg(target_os = "linux")]
fn filter_json(profile: &str) -> Option<&'static str> {
    match profile {
        PROFILE_BASIC => Some(FILTER_BASIC),
        PROFILE_PLAYWRIGHT => Some(FILTER_PLAYWRIGHT),
        PROFILE_E2B => Some(FILTER_E2B),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
pub fn apply_profile(profile: &str) -> Result<(), String> {
    use seccompiler::BpfMap;

    let json = filter_json(profile).ok_or_else(|| format!("unknown seccomp profile: {profile}"))?;
    let arch = std::env::consts::ARCH
        .try_into()
        .map_err(|_| format!("unsupported arch for seccomp: {}", std::env::consts::ARCH))?;
    let map: BpfMap = seccompiler::compile_from_json(json.as_bytes(), arch)
        .map_err(|e| format!("compile seccomp: {e}"))?;
    let prog = map
        .get("default")
        .ok_or_else(|| "missing default seccomp filter".to_string())?;
    seccompiler::apply_filter(prog).map_err(|e| format!("apply seccomp: {e}"))
}

#[cfg(not(target_os = "linux"))]
pub fn apply_profile(_profile: &str) -> Result<(), String> {
    Err("seccomp: Linux only".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_profile_names() {
        assert_eq!(normalize_profile("Playwright"), Some(PROFILE_PLAYWRIGHT));
        assert_eq!(normalize_profile("off"), None);
        assert_eq!(normalize_profile("bogus"), None);
    }

    #[test]
    fn high_risk_servers() {
        assert!(is_high_risk_mcp_server("playwright"));
        assert!(is_high_risk_mcp_server("e2b"));
        assert!(!is_high_risk_mcp_server("git"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn compiles_profiles() {
        use seccompiler::BpfMap;
        let arch = std::env::consts::ARCH.try_into().expect("arch");
        for p in [PROFILE_BASIC, PROFILE_PLAYWRIGHT, PROFILE_E2B] {
            let json = filter_json(p).expect("json");
            let map: BpfMap =
                seccompiler::compile_from_json(json.as_bytes(), arch).expect("compile");
            assert!(map.get("default").is_some(), "{p}");
        }
    }
}
