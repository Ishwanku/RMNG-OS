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


## Session + handoff (Sprint 4)

```bash
SID=$(rmng session new | grep '^session:' | awk '{print $2}')
rmng session set-context "$SID" repo '"'"'RMNG-OS'"'"'
rmng ask --agent swarm-coordinator "check git status" --session "$SID" --dry-run
rmng handoff --session "$SID" --from swarm-coordinator --to repo-keeper --reason "explicit" "check git status" --dry-run
```

### Shared context flow (Sprint 4b — bidirectional)

| Direction | When | What |
|-----------|------|------|
| **In** | `rmng ask` / `rmng handoff` with `--session` | `shared_context`, handoff history, active agents → nervous prompt |
| **Out** | After successful `rmngd` dispatch with session metadata | Tool name, parameters, output, timestamp, success → `shared_context.tool_results[]` |

```bash
# Multi-hop chain (L4 → L3 → L2); each hop recorded in handoff_history
rmng handoff --session "$SID" \
  --chain swarm-coordinator,repo-keeper,runtime-executor \
  --reason "delegate execution" "check git status"

# Session hygiene
rmng session list --verbose    # active vs stale (7-day window)
rmng session prune --older-than-days 30 --dry-run
rmng session prune --older-than-days 30
```

When `--session` is set, tool results from `rmngd` are automatically appended to the session JSON so the next agent hop can reason over prior execution output.

## Live LLM orchestration (Sprint 4c)

Configure Ollama in `~/.rmng/config.toml`:

```toml
[llm]
llm_provider = "ollama"
endpoint_url = "http://127.0.0.1:11434"
model = "llama3.2"
```

With a live provider, the nervous system injects **session orchestration guidance** plus `recent_tool_results` and handoff history into every `--session` prompt. The LLM is instructed to:

1. Read prior tool results before acting
2. Emit `plan.only` when the task is complete
3. Emit `tool.execute` / `mcp.proxy` for the next step (within agent allowlist)
4. Include `metadata.session_id` on every intent

### Multi-agent workflow example (live LLM)

```bash
export RMNG_PROJECT_ROOT=~/dev/projects/RMNG-OS
SID=$(rmng session new | grep '^session:' | awk '{print $2}')

# L4 orchestrator delegates git work to L3
rmng ask --agent swarm-coordinator "check git status" --session "$SID"

# L3 executes; result written to shared_context.tool_results
rmng session show "$SID"

# Research via MCP (requires github MCP in allowlist + rmngd)
rmng handoff --session "$SID" --from swarm-coordinator --to research-curator \
  --reason "research" "search open issues in RMNG-OS"

# Follow-up — LLM sees prior MCP output in session context
rmng ask --agent research-curator "summarize previous search results" --session "$SID" --dry-run
```

Session lifecycle: `active` (<1h), `idle` (<7d), `stale` (≥7d). Expired sessions (default 90d TTL) are removed on load; override with `RMNG_SESSION_TTL_DAYS=0` to disable.
