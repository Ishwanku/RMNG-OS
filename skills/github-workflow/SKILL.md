---
name: github-workflow
description: GitHub PR and issue context for RMNG-OS via native tools and allowlisted MCP. Use for pull requests, PR status, and git history beyond porcelain status.
---

# GitHub Workflow Skill

## When to use

- Checking open pull request status on `Ishwanku/RMNG-OS`
- Reviewing recent commits via MCP `git.log` when native `git.diff` is insufficient
- Validating `gh auth status` before GitHub operations

## Native tools (preferred)

| Tool | Use |
|------|-----|
| `github.pr_status` | Current PR state via `gh pr status` |
| `git.status` | Working tree hygiene |
| `git.diff` | Local changes before commit |

## MCP proxy (allowlisted)

| Server | Tool | Use |
|--------|------|-----|
| `git` | `git.log` | Recent commit history |

## Rules

- Never store PATs in the repo; use `gh auth login` in WSL
- Emit `tool.execute` or `mcp.proxy` intents only — no shell commands
- Repo-keeper agent scope: `git.*`, `github.pr_status`, `git:git.log` MCP only