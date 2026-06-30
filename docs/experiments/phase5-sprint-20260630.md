# Phase 5 Sprint — BYO-LLM + Execution Plane

**Date:** 2026-06-30

## Completed

- systemd user unit `config/rmngd.service`
- `scripts/install-rmng.sh` — build, config, systemd enable
- `git.status` tool (direct `git` exec, no shell)
- `integrations/dev/git.json`
- `rmng-core::config` — `~/.rmng/config.toml` BYO-LLM schema
- `rmng ask` — config-driven; defaults to mock (no network)

## Verify

```bash
~/dev/projects/RMNG-OS/scripts/install-rmng.sh
systemctl --user status rmngd
rmng send -f agents/schemas/git-status.intent.json
rmng ask "git status" --dry-run
```
