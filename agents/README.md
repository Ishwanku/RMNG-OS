# Agents — Local Rust Runtime (Phase 5)

**Status:** Specification locked · Implementation not started

## Role in the architecture (ADR-009, ADR-010)

The `agents/` tree hosts the **Heart + Brains** of RMNG-OS — entirely local, never delegated to external LLMs.

| Component | Responsibility |
|-----------|----------------|
| `rmngd` daemon | Long-running orchestrator |
| Intent parser | Validates JSON schemas from nervous-system layer |
| Permission gate | Authorizes or denies each tool dispatch |
| Memory store | Session + persistent state (local ownership) |
| Multi-agent router | Delegates to specialist agents (local processes) |

**External LLMs** connect only as the **Nervous System** — they emit intents; they never execute tools or hold authoritative state.

## Planned layout

```
agents/
├── rmng-core/        # Runtime library (Rust)
├── rmngd/            # System daemon
├── rmng-cli/         # CLI binary (ADR-011)
├── schemas/          # JSON intent schemas (versioned)
└── nervous/          # LLM adapters (Ollama, OpenAI, Anthropic)
```

## Specs

- [REQUIREMENTS.md](../docs/REQUIREMENTS.md) — FR-L4-*
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md) — Layer 4, biological separation
- [DECISIONS.md](../docs/DECISIONS.md) — ADR-009, ADR-010, ADR-011