# Agents — Rust Runtime (Phase 5)

**Status:** In progress

## Architecture (ADR-009, ADR-010)

| Layer | Component | Role |
|-------|-----------|------|
| Nervous system | `rmng-nervous` | Ollama → JSON intents only |
| Heart + Brains | `rmng-core`, `rmngd` | Permission gate, tool dispatch, audit |
| Interface | `rmng-cli` | `rmng` CLI (ADR-011) |

## Workspace

```
agents/
├── rmng-core/       # Intent, permissions, tools, audit
├── rmng-nervous/    # Ollama adapter
├── rmng-cli/        # rmng binary
├── rmngd/           # Unix socket daemon
└── schemas/         # JSON intent schemas
```

## Commands

```bash
cd ~/dev/projects/RMNG-OS/agents
cargo build

rmng status
rmng tools
rmng run -f schemas/kernel-status.intent.json
rmng ask "check kernel environment" --dry-run   # needs Ollama
```

## Specs

- [REQUIREMENTS.md](../docs/REQUIREMENTS.md)
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md)
- [DECISIONS.md](../docs/DECISIONS.md)
