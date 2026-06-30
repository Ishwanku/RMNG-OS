---
name: git-workflow
description: RMNG-OS repository hygiene — commits, WSL git push, gh auth, branch policy, and safe changes within the project structure. Use for any RMNG-OS repo work.
---

# Git Workflow Skill

## When to use

- Committing scripts, docs, patches, skills, or agent tooling in RMNG-OS
- Pushing to `main` on GitHub (`Ishwanku/RMNG-OS`)
- Verifying what belongs in the repo vs local-only paths
- Using `gh` for auth, issues, or PR context during development

## Repository scope

**In RMNG-OS repo (`~/dev/projects/RMNG-OS`):**

- `scripts/`, `docs/`, `patches/`, `config/`, `skills/`, `agents/`, `integrations/`

**Never commit:**

- `~/build/kernel` — out-of-tree build artifacts
- `~/dev/kernel/linux` — separate kernel clone (GPLv2, upstream)
- `~/.rmng/` — runtime config, sockets, audit logs
- `~/.cursor/`, `~/.config/rmng/mcp-dev.json` — IDE and dev MCP config (may contain tokens)
- Local-only dev tooling paths listed in `.gitignore`

## Branch policy

- **`main` only** on GitHub — no `master`, no long-lived feature branches unless explicitly requested
- One logical change per commit; message describes *what* and *why*

## Constraints

- Work in WSL Ubuntu for git operations against this repo
- Use `gh auth login` (or token via `gh auth setup-git`) before push — hung pushes usually mean missing WSL auth
- Do not disclose which external AI tools were used to build the project in commit messages or public docs

## Commit standards

```
<type>: <short summary>

<body — why this change, what phase it supports>
```

Types: `docs`, `feat`, `fix`, `scripts`, `kernel`, `agents`, `skills`, `config`

Examples:

- `skills: add kernel-config and git-workflow skills (Phase 6a)`
- `docs: document dev MCP servers in daily-workflow`
- `scripts: add flock to rebuild-with-patches.sh`

## Safe push workflow

```bash
cd ~/dev/projects/RMNG-OS
git status
git diff
git add -p                    # stage intentionally, not bulk
git commit -m "describe change"
gh auth status                # must show logged in
git push origin main
```

If push hangs: run `gh auth login` in WSL, then retry.

## Tool intents (via rmngd)

| Intent tool | When |
|-------------|------|
| `git.status` | Check working tree before commit or after changes |

Production git operations flow through `rmng send` / `rmngd` — not through IDE MCP git servers.

## gh integration

```bash
gh auth status
gh repo view Ishwanku/RMNG-OS
gh issue list                 # optional, dev context
```

For GitHub MCP in the IDE, use a token from `gh auth token` — never commit tokens to the repo.

## Validation

- `git status` clean after push (or only expected local-only files ignored)
- Changes limited to RMNG-OS paths — no kernel tree or build dir files staged
- Remote `main` matches local after `git push origin main`