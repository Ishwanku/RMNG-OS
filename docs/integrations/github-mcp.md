# Integration: GitHub MCP Server

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/github/github-mcp-server |
| **License** | MIT |
| **Track** | **2 — MCP Proxy Plane** |
| **Status** | **Active** |

## Role

IDE and production proxy for GitHub API operations (`get_issue`, `create_issue`).  
**Not** a replacement for future native `github.*` tools (Track 1).

## Configuration

- Production: `~/.rmng/mcp-allowlist.toml` → `[servers.github]`
- Dev IDE: `~/.config/rmng/mcp-dev.json` via `setup-dev-mcp.sh`
- Register/update: `./scripts/register-mcp-tool.sh github npx -y @github/github-mcp-server --tools get_issue,create_issue`

## Security

- Requires `gh auth login` or token in environment — never commit tokens
- Allowlist enumerates tools explicitly; no wildcard

## Rollback

Set `enabled = false` under `[servers.github]` or remove section; restart `rmngd`.