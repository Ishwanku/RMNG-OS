# Integration Intake: Microsoft Playwright MCP

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/microsoft/playwright-mcp |
| **License** | Apache-2.0 |
| **Date** | 2026-07-01 |
| **Proposed track** | 2 — MCP Proxy |
| **Status** | **Active** (disabled by default in example allowlist) |

## Summary

Exposes Playwright accessibility tree over MCP — DOM-first navigation without vision tokens. Enables research agents to interact with web UIs deterministically.

## Evaluation scores (1–5)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Execution plane isolation | 4 | Subprocess; browser child processes |
| Structural determinism | 4 | Tool set evolves — pin version |
| Zero-trust security | 3 | Full browser capability — enable per-agent only |
| Architectural fit (ADR-010) | 5 | MCP proxy only |
| **Average** | **4.0** | |

## Threat model

- Prompt injection surface: page content → LLM
- Filesystem access: browser downloads possible — use isolated profile
- Network egress: unrestricted web
- Credential handling: may access logged-in sessions if browser profile shared

## Implementation

### Track 2
- [x] Example allowlist entry (disabled by default)
- [x] `web-researcher` agent with limited tool allowlist
- [ ] Register: `npx -y @playwright/mcp@latest`
- [ ] Pin version after first successful E2E
- [ ] Isolation: recommend `[servers.playwright.isolation] memory_mb = 1024`

## Rollback

`enabled = false` on server block.

## Decision

- [x] Accepted with **opt-in enable** — not on by default