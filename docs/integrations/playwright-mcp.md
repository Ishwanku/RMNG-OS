# Integration Intake: Microsoft Playwright MCP

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/microsoft/playwright-mcp |
| **License** | Apache-2.0 |
| **Date** | 2026-07-02 |
| **Proposed track** | 2 — MCP Proxy |
| **Status** | **Active** (disabled by default) |

## Summary

Exposes Playwright accessibility tree over MCP — DOM-first navigation without vision tokens.

## Evaluation scores (1–5)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Execution plane isolation | 4 | Subprocess + browser children; cgroup limits |
| Structural determinism | 4 | Pin version after E2E |
| Zero-trust security | 3 | Full browser — opt-in only |
| Architectural fit (ADR-010) | 5 | MCP proxy only |
| **Average** | **4.0** | |

## Threat model

- Prompt injection: page content → LLM
- Filesystem: browser downloads — isolated profile required
- Network: unrestricted web egress
- Credentials: never share logged-in browser profiles

## Implementation (Sprint 14)

### Track 2
- [x] Example allowlist entry (`enabled = false`)
- [x] Isolation: `memory_mb=1024`, `pids_max=128`, cgroup, `new_session`, `no_new_privs`
- [x] `browser-researcher` agent (tight tool scope)
- [x] E2E: `agents/rmng-nervous/tests/playwright_e2e.rs`
- [x] Example intent: `agents/schemas/mcp-playwright-navigate.intent.json`
- [x] Usage: [browser-research-usage.md](browser-research-usage.md)
- [ ] Pin `@playwright/mcp` version after first live E2E

### Register (opt-in)

```bash
./scripts/register-mcp-tool.sh playwright npx -y @playwright/mcp@latest \
  --tools browser_navigate,browser_snapshot,browser_click
# Then set enabled = true in ~/.rmng/mcp-allowlist.toml
```

## Rollback

`enabled = false` under `[servers.playwright]`; restart `rmngd`.

## Decision

- [x] Accepted with **opt-in enable** — separate `browser-researcher` agent, not `web-researcher`