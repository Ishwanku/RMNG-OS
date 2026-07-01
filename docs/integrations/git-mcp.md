# Integration: mcp-server-git

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/modelcontextprotocol/servers (git) |
| **License** | MIT |
| **Track** | **2 — MCP Proxy Plane** |
| **Status** | **Active** (Sprint 14 expanded) |

## Role

Repository inspection via MCP for `repo-keeper`. Complements native Track 1 tools.

## Allowed tools (Sprint 14)

| Tool | Wire name | Purpose |
|------|-----------|---------|
| `git.log` | `git_log` | Commit history |
| `git.diff` | `git_diff` | Working tree diff |
| `git.status` | `git_status` | Repository status |

## Configuration

```bash
./scripts/register-mcp-tool.sh git uvx mcp-server-git \
  --repository ~/dev/projects/RMNG-OS \
  --tools git.log,git.diff,git.status
```

Example intents: `agents/schemas/mcp-git-diff.intent.json`, `mcp-git-status.intent.json`

## Security

- Scoped to explicit `--repository` path
- Read-only tools only; `git.commit` denied at gate

## Rollback

Disable `[servers.git]` in allowlist; restart `rmngd`.