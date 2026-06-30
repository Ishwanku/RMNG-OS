# Agents — Rust Runtime (Phase 5)

**Status:** Scaffold · Implementation in progress

## Architecture (ADR-009, ADR-010)

| Layer | Component | Role |
|-------|-----------|------|
| Nervous system | `nervous/` | LLM adapters — emit JSON intents only |
| Heart + Brains | `rmng-core/`, `rmngd/` | Local runtime — execute tools, permissions, memory |
| Interface | `rmng-cli/` | CLI-first entry point (ADR-011) |

External LLMs never receive raw terminal or system access.

## Workspace layout

```
agents/
├── Cargo.toml          # Rust workspace
├── rmng-core/          # Runtime library
├── rmngd/              # System daemon
├── rmng-cli/           # CLI binary (`rmng`)
├── schemas/            # JSON intent schemas (versioned)
└── nervous/            # Ollama + external API adapters
```

## Build

```bash
cd ~/dev/projects/RMNG-OS/agents
cargo build
cargo test
```

## Specs

- [REQUIREMENTS.md](../docs/REQUIREMENTS.md)
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md)
- [DECISIONS.md](../docs/DECISIONS.md) — ADR-009–012
