//! Integration test: real subprocess peak RSS via wait4 (Linux/WSL).

use rmng_mcp::harvest_child_resources;
use std::process::Stdio;
use tokio::process::Command;

#[tokio::test]
async fn subprocess_reports_nonzero_peak_rss() {
    if !cfg!(unix) {
        return;
    }
    let child = Command::new("python3")
        .args([
            "-c",
            "buf = 'x' * (8 * 1024 * 1024); print(len(buf))",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn python3");

    let pid = child.id().expect("pid");
    // wait4 reaps the child and reads ru_maxrss; tokio Child::wait() must not run first.
    let metrics = harvest_child_resources(pid);
    drop(child);

    let peak = metrics.peak_rss_kb.unwrap_or(0);
    assert!(
        peak > 0,
        "expected peak_rss_kb > 0 from allocating subprocess, got {metrics:?}"
    );
}
