# Integration Intake: MCP Fetch Server

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/modelcontextprotocol/servers (src/fetch) |
| **License** | MIT |
| **Date** | 2026-07-01 |
| **Proposed track** | 2 — MCP Proxy |
| **Status** | **Active** |

## Summary

Official MCP `fetch` server retrieves URL content as markdown/text for LLM context. Read-only HTTP egress — ideal first expansion beyond git/github.

## Evaluation scores (1–5)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Execution plane isolation | 5 | Subprocess, no in-process Python in rmngd |
| Structural determinism | 5 | Single `fetch` tool, schema-stable |
| Zero-trust security | 4 | Network egress — restrict to HTTPS; audit all calls |
| Architectural fit (ADR-010) | 5 | Pure `mcp.proxy` body path |
| **Average** | **4.75** | |

## Threat model

- Prompt injection surface: fetched content returned to LLM — treat as untrusted input
- Filesystem access: none
- Network egress: yes (user-controlled URLs)
- Credential handling: none required

## Implementation

### Track 2 — MCP Proxy
- [x] Example in `config/mcp-allowlist.toml.example`
- [x] Example intent `agents/schemas/mcp-fetch.intent.json`
- [x] `web-researcher` agent definition
- [ ] User registration: `./scripts/register-mcp-tool.sh fetch npx -y @modelcontextprotocol/server-fetch --tools fetch`
- [ ] E2E test when rmngd running

## Rollback

Set `[servers.fetch] enabled = false` or remove server block; restart rmngd.

## Decision

- [x] Accepted — Track 2 production path