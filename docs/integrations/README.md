# Integration Documentation

Per-repository evaluation records for open-source AI/agent/MCP projects considered for RMNG-OS.

**Governance:** [INTEGRATION-STRATEGY.md](../INTEGRATION-STRATEGY.md)

## Index

| Document | Repo | Track | Status |
|----------|------|-------|--------|
| [github-mcp.md](github-mcp.md) | `@github/github-mcp-server` | 2 — MCP Proxy | **Active** (allowlisted) |
| [git-mcp.md](git-mcp.md) | `mcp-server-git` | 2 — MCP Proxy | **Active** (allowlisted) |
| [mcp-fetch.md](mcp-fetch.md) | `@modelcontextprotocol/server-fetch` | 2 — MCP Proxy | **Active** (Sprint 12) |
| [playwright-mcp.md](playwright-mcp.md) | `@playwright/mcp` | 2 — MCP Proxy | **Active** (opt-in) |
| [superpowers-skill.md](superpowers-skill.md) | `obra/superpowers` | 3 — Skill | **Active** (adapted) |
| [context7-rejected.md](context7-rejected.md) | `upstash/context7` | 4 — Rejected | **Rejected** |
| _future_ | Use [TEMPLATE.md](TEMPLATE.md) | — | Intake |

**Roadmap:** [INTEGRATION-ROADMAP.md](../INTEGRATION-ROADMAP.md)

## Quick commands

```bash
# Register new MCP server (Track 2)
./scripts/register-mcp-tool.sh <name> <cmd> [args...] --tools t1,t2

# Native tool (Track 1) — manual: integrations/ + rmng-core + PermissionGate
# Skill (Track 3) — add skills/<name>/SKILL.md
```

## Status legend

| Status | Meaning |
|--------|---------|
| **Active** | In production or dev path, documented |
| **Evaluating** | Intake complete, not yet wired |
| **Deferred** | Good fit later; blocked by phase |
| **Rejected** | Fails evaluation framework |