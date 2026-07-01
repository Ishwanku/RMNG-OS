# Integration: GitHub MCP Server

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/github/github-mcp-server |
| **License** | MIT |
| **Track** | **2 — MCP Proxy Plane** |
| **Status** | **Active** (Sprint 14 expanded) |

## Role

Read-only GitHub issue intelligence for `research-curator`.  
Write tools (`create_issue`) are **not** allowlisted.

## Allowed tools (Sprint 14)

| Tool | Purpose |
|------|---------|
| `search_issues` | Cross-repo search queries |
| `list_issues` | Enumerate issues in a repo |
| `get_issue` | Single issue detail |

## Configuration

```bash
./scripts/register-mcp-tool.sh github npx -y @github/github-mcp-server \
  --tools search_issues,list_issues,get_issue
```

- Production: `~/.rmng/mcp-allowlist.toml` → `[servers.github]`
- Example intents: `agents/schemas/mcp-github-list-issues.intent.json`, `mcp-github-get-issue.intent.json`

## Security

- Requires `gh auth login` or token — never commit tokens
- Explicit tool enumeration; no wildcards
- Agent policy: `research-curator` only

## Rollback

Set `enabled = false` under `[servers.github]`; restart `rmngd`.