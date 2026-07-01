# Code Workflow — GitHub + Git MCP Usage (Sprint 14)

Read-only MCP expansion for issue intelligence and repository inspection.

## GitHub issue tools

| Tool | Agent | Intent schema |
|------|-------|---------------|
| `search_issues` | research-curator | (prompt-driven) |
| `list_issues` | research-curator | `mcp-github-list-issues.intent.json` |
| `get_issue` | research-curator | `mcp-github-get-issue.intent.json` |

`create_issue` is **not** allowlisted — write ops denied at gate.

```bash
rmng send -f agents/schemas/mcp-github-list-issues.intent.json
rmng send -f agents/schemas/mcp-github-get-issue.intent.json
```

Requires `gh auth login` or token in environment.

## Git repository tools

| Tool | Agent | Native fallback |
|------|-------|-----------------|
| `git.log` | repo-keeper | — |
| `git.diff` | repo-keeper | native `git.diff` |
| `git.status` | repo-keeper | native `git.status` |

```bash
rmng send -f agents/schemas/mcp-git-diff.intent.json
rmng send -f agents/schemas/mcp-git-status.intent.json
```

Mock LLM routes MCP git tools when prompt includes `mcp` (e.g. "show mcp git diff").

## Register expanded allowlist

```bash
./scripts/register-mcp-tool.sh github npx -y @github/github-mcp-server \
  --tools search_issues,list_issues,get_issue

./scripts/register-mcp-tool.sh git uvx mcp-server-git \
  --repository ~/dev/projects/RMNG-OS \
  --tools git.log,git.diff,git.status

systemctl --user restart rmngd
```

## Sandbox verification (Sprint 17)

After git/issue research and evaluation, use opt-in E2B sandbox for script verification. See [sandbox-usage.md](sandbox-usage.md).

## Tests

```bash
cd agents && cargo test -p rmng-nervous --test sandbox_e2e -- --nocapture
cd agents && cargo test -p rmng-nervous --test mcp_e2e -- --nocapture
cd agents && cargo test -p rmng-core permission -- --nocapture
```