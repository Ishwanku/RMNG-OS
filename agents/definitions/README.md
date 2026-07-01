# Agent Definitions

YAML manifests for RMNG multi-level agents (ADR-017).

## Layout

```
agents/definitions/
  kernel-engineer.yaml    # L1 — kernel lab
  system-health.yaml      # L1 — minimal health probe
  runtime-executor.yaml   # L2 — execution plane delegate
  repo-keeper.yaml        # L3 — git/GitHub domain
  swarm-coordinator.yaml  # L4 — orchestration / handoff
```

## Required fields

| Field | Description |
|-------|-------------|
| `id` | Unique agent identifier |
| `layer` | `L1` \| `L2` \| `L3` \| `L4` |
| `description` | Human-readable role |
| `skills` | Skill names under `skills/` |
| `allowed_native_tools` | Native tool allowlist (`prefix.*` wildcards) |
| `allowed_mcp_tools` | MCP entries as `server:tool` or `server:*` |
| `delegates_to` | Explicit handoff targets (required for L4 orchestrators) |

## Layer model

| Layer | Role | Privilege |
|-------|------|-----------|
| L1 | Core / hardware | High |
| L2 | Runtime / execution | Medium-high |
| L3 | Integration / domain | Medium |
| L4 | Orchestration / swarm | Low (coordination only) |

Handoffs flow **downward only** (L4 → L3 → L2 → L1).

## Usage

```bash
export RMNG_PROJECT_ROOT=~/dev/projects/RMNG-OS
rmng ask --agent repo-keeper "check git status" --dry-run
rmng ask --agent swarm-coordinator "check git status" --session <id> --dry-run
rmng session new
```

## Adding a new agent (minimal friction)

1. Add `agents/definitions/<name>.yaml` — no Rust changes required for L3/L4
2. Add skills under `skills/<name>/` if needed
3. For new native tools: manifest + handler (Sprint 1 pipeline)
4. `cargo test` and `rmng observe`
