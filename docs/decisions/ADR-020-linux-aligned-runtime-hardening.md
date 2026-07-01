# ADR-020: Linux-Aligned Runtime Hardening

**Date:** 2026-07-01  
**Status:** **Accepted**  
**Related:** ADR-010 (nervous/body), ADR-014 (MCP), ADR-019 (licensing/layering), Sprint 10

---

## Context

RMNG-OS multi-LLM workflows (Sprints 7–9) added fallback chains, cost telemetry, and catalog sync. As the runtime matures toward daily use, we adopt **Linux kernel design principles** in userspace:

| Principle | RMNG mapping |
|-----------|--------------|
| Thin core | `rmng-core` + `rmngd` — policy, audit, dispatch only |
| Strong boundaries | JSON schema at nervous→body IPC; versioned interfaces |
| Deep observability | Structured audit, `rmng observe`, session metrics |
| Auditability | Tamper-evident hash-chained audit log |
| Isolation | cgroup v2 + rlimits for MCP subprocesses |

WSL2 limits full kernel-style isolation (no custom seccomp BPF in v1). Sprint 10 ships **pragmatic** boundaries that work on WSL and improve on bare-metal Linux.

---

## Decision

### 1. Tamper-evident audit log (schema v3)

- Monotonic `seq`, `prev_hash`, `entry_hash` (SHA-256 chain)
- State file: `~/.rmng/logs/audit.chain`
- Structured `category` field: `native`, `mcp`, `llm`, `handoff`, `circuit`, `plan`, `system`
- Query-friendly fields: `session_id`, `agent_id`, `cost_usd`, `tokens_*`, `mcp_pid`
- `AuditLog::verify_chain()` for integrity checks

Legacy entries without v3 fields remain readable; new entries are sealed on append.

### 2. Subprocess isolation for MCP

Global defaults in `~/.rmng/config.toml` `[isolation]`; per-server overrides in `mcp-allowlist.toml`.

| Control | Mechanism |
|---------|-----------|
| Memory cap | `RLIMIT_AS` (best-effort) |
| PID cap | `RLIMIT_NPROC` |
| CPU / memory cgroup | cgroup v2 under user slice when delegated |
| Privilege boundary | `PR_SET_NO_NEW_PRIVS` when enabled |
| Session isolation | `setsid()` for new process group |

Set `RMNG_CGROUP_BASE` when auto-detection fails (common in minimal WSL).

### 3. Interface stability

- Core intent schema version `2` — reject mismatched `schema_version` at validator
- Audit schema version `3` — independent versioning
- MCP wire protocol unchanged (`2024-11-05`)

---

## Consequences

**Positive:** Forensic-grade audit trail; reduced blast radius for MCP tools; clearer ops via `rmng observe`.

**Negative:** cgroup delegation varies by WSL/systemd setup; hash chain does not sign entries (no HSM) — detects tamper, not authorship.

**WSL trade-offs:** Full seccomp profiles deferred; network namespace isolation not in v1; cgroup paths may require manual `RMNG_CGROUP_BASE`.

---

## References

- `agents/rmng-core/src/audit.rs`
- `agents/rmng-mcp/src/isolation.rs`
- `docs/llm-configuration.md`
- `config/rmng-config.toml.example` · `config/mcp-allowlist.toml.example`