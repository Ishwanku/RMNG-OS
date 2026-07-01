# Integration: mcp-server-git

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/modelcontextprotocol/servers (git) |
| **License** | MIT |
| **Track** | **2 — MCP Proxy Plane** |
| **Status** | **Active** |

## Role

Rich git log/history via MCP for production proxy (`git.log`).  
Complements native `git.status` (Track 1) — does not replace it.

## Configuration

```bash
./scripts/register-mcp-tool.sh git uvx mcp-server-git \
  --repository ~/dev/projects/RMNG-OS \
  --tools git.log
```

Wire name mapping: `git.log` (allowlist) → `git_log` (MCP wire) via `rmng-mcp::wire_tool_name`.

## Security

- Scoped to explicit `--repository` path
- Read-oriented tool only in current allowlist

## Rollback

Disable `[servers.git]` in allowlist; restart `rmngd`.