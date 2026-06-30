# Agents — Rust Runtime (Phase 5)

**Status:** In progress

## Commands

```bash
cd ~/dev/projects/RMNG-OS/agents
cargo build

rmng status
rmng run -f schemas/kernel-status.intent.json    # local runtime
rmngd &                                            # start daemon
rmng send -f schemas/kernel-status.intent.json     # via daemon + audit
rmng ask "check kernel status" --dry-run           # Ollama → intent
```

Install to PATH: `~/dev/projects/RMNG-OS/scripts/install-rmng.sh`

## Crates

| Crate | Role |
|-------|------|
| `rmng-core` | Intent, permissions, tools, audit, IPC |
| `rmng-nervous` | Ollama adapter |
| `rmng-cli` | `rmng` binary |
| `rmngd` | Unix socket daemon (`~/.rmng/rmngd.sock`) |

## Specs

- [REQUIREMENTS.md](../docs/REQUIREMENTS.md)
- [integrations/dev/](../integrations/dev/) — tool manifests
