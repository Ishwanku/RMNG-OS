use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Resource limits for MCP / tool subprocesses (Sprint 10).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IsolationLimits {
    /// Max resident memory (MB) via RLIMIT_AS where supported.
    #[serde(default)]
    pub memory_mb: Option<u32>,
    /// CPU weight hint (stored for cgroup cpu.max when available).
    #[serde(default)]
    pub cpu_percent: Option<u32>,
    /// Max child processes (RLIMIT_NPROC).
    #[serde(default)]
    pub pids_max: Option<u32>,
    /// New session / process group (setsid) — basic isolation.
    #[serde(default)]
    pub new_session: bool,
    /// Attempt cgroup v2 limits under user slice (Linux/WSL when delegated).
    #[serde(default)]
    pub cgroup: bool,
    /// Drop ambient privileges where possible (no_new_privs on Linux).
    #[serde(default)]
    pub no_new_privs: bool,
}

impl IsolationLimits {
    pub fn merge(base: &Self, override_: Option<&Self>) -> Self {
        let Some(o) = override_ else {
            return base.clone();
        };
        Self {
            memory_mb: o.memory_mb.or(base.memory_mb),
            cpu_percent: o.cpu_percent.or(base.cpu_percent),
            pids_max: o.pids_max.or(base.pids_max),
            new_session: o.new_session || base.new_session,
            cgroup: o.cgroup || base.cgroup,
            no_new_privs: o.no_new_privs || base.no_new_privs,
        }
    }

    pub fn is_active(&self) -> bool {
        self.memory_mb.is_some()
            || self.pids_max.is_some()
            || self.cpu_percent.is_some()
            || self.new_session
            || self.cgroup
            || self.no_new_privs
    }
}

/// Result of applying isolation — for audit / observe.
#[derive(Debug, Clone, Default)]
pub struct IsolationReport {
    pub cgroup_path: Option<PathBuf>,
    pub rlimit_as: Option<u32>,
    pub rlimit_nproc: Option<u32>,
    pub new_session: bool,
    pub no_new_privs: bool,
    pub warnings: Vec<String>,
}

#[cfg(unix)]
pub mod unix {
    use super::{IsolationLimits, IsolationReport};
    use nix::unistd::setsid;
    use std::fs::{create_dir_all, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use tokio::process::Command as AsyncCommand;
    use uuid::Uuid;

    pub fn prepare_cgroup(limits: &IsolationLimits) -> (Option<PathBuf>, Vec<String>) {
        if !limits.cgroup {
            return (None, Vec::new());
        }
        let base = std::env::var("RMNG_CGROUP_BASE")
            .map(PathBuf::from)
            .ok()
            .or_else(detect_user_cgroup);
        let Some(base) = base else {
            return (
                None,
                vec!["cgroup: no user delegation path (set RMNG_CGROUP_BASE)".into()],
            );
        };
        let id = Uuid::new_v4().to_string();
        let path = base.join(format!("rmng-mcp-{id}"));
        if create_dir_all(&path).is_err() {
            return (None, vec![format!("cgroup: cannot create {}", path.display())]);
        }
        if let Some(mb) = limits.memory_mb {
            let _ = std::fs::write(path.join("memory.max"), format!("{}M", mb));
        }
        if let Some(pct) = limits.cpu_percent {
            let quota = (pct as u64).saturating_mul(1000);
            let _ = std::fs::write(path.join("cpu.max"), format!("{quota} 100000"));
        }
        if let Some(pids) = limits.pids_max {
            let _ = std::fs::write(path.join("pids.max"), pids.to_string());
        }
        (Some(path), Vec::new())
    }

    fn detect_user_cgroup() -> Option<PathBuf> {
        let uid = unsafe { libc::getuid() };
        let candidates = [
            format!("/sys/fs/cgroup/user.slice/user-{uid}.slice"),
            format!("/sys/fs/cgroup/user.slice/user-{uid}.slice/user@{uid}.service"),
            "/sys/fs/cgroup".into(),
        ];
        candidates
            .into_iter()
            .map(PathBuf::from)
            .find(|p| p.join("cgroup.procs").exists())
    }

    pub fn attach_pid(cgroup: &PathBuf, pid: u32) -> Result<(), String> {
        let mut f = OpenOptions::new()
            .write(true)
            .open(cgroup.join("cgroup.procs"))
            .map_err(|e| e.to_string())?;
        writeln!(f, "{pid}").map_err(|e| e.to_string())
    }

    pub fn apply_pre_exec(limits: &IsolationLimits) -> std::io::Result<()> {
        if limits.new_session {
            setsid().map_err(|e| std::io::Error::other(e.to_string()))?;
        }
        if limits.no_new_privs {
            nix::sys::prctl::set_no_new_privs()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }
        if let Some(mb) = limits.memory_mb {
            let bytes = (mb as u64).saturating_mul(1024 * 1024);
            let lim = libc::rlimit {
                rlim_cur: bytes,
                rlim_max: bytes,
            };
            unsafe {
                let _ = libc::setrlimit(libc::RLIMIT_AS, &lim);
            }
        }
        if let Some(nproc) = limits.pids_max {
            let lim = libc::rlimit {
                rlim_cur: nproc as u64,
                rlim_max: nproc as u64,
            };
            unsafe {
                let _ = libc::setrlimit(libc::RLIMIT_NPROC, &lim);
            }
        }
        Ok(())
    }

    pub fn configure_command(cmd: &mut AsyncCommand, limits: &IsolationLimits) {
        let limits = limits.clone();
        unsafe {
            cmd.pre_exec(move || apply_pre_exec(&limits));
        }
    }

    pub fn build_report(limits: &IsolationLimits, cgroup: Option<PathBuf>, warnings: Vec<String>) -> IsolationReport {
        IsolationReport {
            cgroup_path: cgroup,
            rlimit_as: limits.memory_mb,
            rlimit_nproc: limits.pids_max,
            new_session: limits.new_session,
            no_new_privs: limits.no_new_privs,
            warnings,
        }
    }
}

#[cfg(not(unix))]
pub mod unix {
    use super::{IsolationLimits, IsolationReport};
    use std::path::PathBuf;
    use tokio::process::Command as AsyncCommand;

    pub fn prepare_cgroup(_limits: &IsolationLimits) -> (Option<PathBuf>, Vec<String>) {
        (None, vec!["isolation: Unix-only (WSL/Linux required)".into()])
    }

    pub fn attach_pid(_cgroup: &PathBuf, _pid: u32) -> Result<(), String> {
        Ok(())
    }

    pub fn configure_command(_cmd: &mut AsyncCommand, _limits: &IsolationLimits) {}

    pub fn build_report(_limits: &IsolationLimits, _cgroup: Option<PathBuf>, warnings: Vec<String>) -> IsolationReport {
        IsolationReport { warnings, ..Default::default() }
    }
}

pub use unix::{attach_pid, build_report, configure_command, prepare_cgroup};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_prefers_override() {
        let base = IsolationLimits {
            memory_mb: Some(512),
            cpu_percent: Some(50),
            ..Default::default()
        };
        let over = IsolationLimits {
            memory_mb: Some(256),
            new_session: true,
            ..Default::default()
        };
        let m = IsolationLimits::merge(&base, Some(&over));
        assert_eq!(m.memory_mb, Some(256));
        assert_eq!(m.cpu_percent, Some(50));
        assert!(m.new_session);
    }
}