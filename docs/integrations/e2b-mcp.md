# Integration Intake: E2B MCP

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/e2b-dev/mcp-server |
| **Package** | `@e2b/mcp-server` (npx) |
| **License** | Apache-2.0 |
| **Date** | 2026-07-02 |
| **Track** | 2 MCP Proxy + 3 Skill |
| **Status** | Active (opt-in) |

## Summary

E2B runs Python in cloud-isolated sandboxes via the `run_code` MCP tool (Jupyter-style cells). RMNG wires the **stdio** subprocess (`npx -y @e2b/mcp-server`) through `mcp.proxy` — code never executes on the host.

**Note:** The upstream repo is archived (2026-04); the npm package remains published. E2B's newer [HTTP MCP gateway](https://e2b.dev/docs/mcp) inside sandboxes is **deferred** until rmng-mcp supports HTTP transport.

## Alternatives evaluated

| Candidate | Verdict | Reason |
|-----------|---------|--------|
| **@e2b/mcp-server** (stdio) | **Accepted** | True remote isolation; stdio fits Track 2; single tool `run_code` |
| E2B HTTP MCP gateway | Deferred | rmng-mcp is subprocess JSON-RPC only |
| **mcp-run-python** (Pyodide/Deno) | Rejected | Archived; Pyodide can escape to host JS/filesystem |
| **mcp-run-python** local | Rejected | Runs on host — violates isolation constraint |
| Docker `mcp/e2b` | Equivalent | Same `run_code` tool; npx preferred for allowlist parity |

## Evaluation scores (1–5)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Execution plane isolation | 5 | Code runs in E2B cloud sandbox, not rmngd/host |
| Structural determinism | 4 | Single tool; `code` string in / JSON out |
| Zero-trust security | 4 | Opt-in; API key external; network egress in sandbox |
| Architectural fit (ADR-010) | 5 | Nervous plans; body proxies via PermissionGate |
| **Average** | **4.5** | Meets 3.5 threshold; security ≥ 3 |

## Threat model

- **Prompt injection:** Untrusted code in sandbox — treat stdout as untrusted input
- **Filesystem:** Host FS not exposed; sandbox ephemeral
- **Network egress:** E2B sandbox may reach internet — scope code to verification only
- **Credentials:** `E2B_API_KEY` in rmngd env only — never in repo or agent prompts

## Allowed tools

| Tool | Params | Description |
|------|--------|-------------|
| `run_code` | `code` (string) | Python/Jupyter syntax in E2B sandbox |

## Register

```bash
export E2B_API_KEY="e2b_..."
./scripts/register-mcp-tool.sh e2b npx -y @e2b/mcp-server --tools run_code
# Edit ~/.rmng/mcp-allowlist.toml: enabled = true
systemctl --user restart rmngd
```

## Rollback

Set `[servers.e2b] enabled = false` and restart rmngd. Remove `e2b:run_code` from agent YAML if needed.

## Decision

Accepted opt-in Track 2. HTTP gateway deferred.


## Security (Sprint 21)

> **High risk** — code runs in E2B cloud, but the local MCP subprocess still needs hardening. Use `seccomp_profile = "e2b"` and `drop_capabilities = true`. See [security-mcp-usage.md](security-mcp-usage.md).
