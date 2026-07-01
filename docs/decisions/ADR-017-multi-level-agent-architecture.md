# ADR-017: Multi-Level Agent Architecture

**Date:** 2026-07-01  
**Status:** **Accepted**  
**Related:** ADR-010 (nervous/body), ADR-014 (native-first), ADR-015 (CoreIntent v2), Sprint 3

---

## Context

Sprint 1–2 delivered manifest-driven tools, agent definitions, and a flat router. RMNG-OS must evolve into a **multi-agent operating system** where agents operate at different stack depths — from kernel/hardware interaction to high-level orchestration — without breaking the nervous/body separation.

## Decision

Adopt a **four-layer agent model (L1–L4)** with:

1. **Layer field** on every `agents/definitions/*.yaml` manifest
2. **`LayerAgent` trait** — layer, privilege, handoff rules
3. **Downward-only handoffs** — L4 may delegate to L3/L2/L1; L1 never delegates upward
4. **Session store** at `~/.rmng/sessions/<id>.json` — active agents, shared context, handoff history
5. **Layer-aware router** — `ask_routed()`, `handoff()`, L4 orchestration auto-delegates execution intents

### Layer definitions

| Layer | Name | Responsibility | Privilege | Execution |
|-------|------|----------------|-----------|-----------|
| **L1** | Core / Hardware | Kernel, devices, low-level ops | High | Native Rust only |
| **L2** | Runtime / Execution | Tool dispatch, MCP proxy, audit | Medium-high | Native + MCP via rmngd |
| **L3** | Integration / Domain | Domain workflows (git, GitHub, research) | Medium | Skills + allowlisted tools |
| **L4** | Orchestration / Swarm | Planning, decomposition, delegation | Low | `plan.only` or handoff — **no direct execution** |

### Communication boundaries

```text
User / CLI
    │
    ▼
L4 Orchestrator (swarm-coordinator)
    │ handoff (session recorded)
    ▼
L3 Domain Agent (repo-keeper) ──► L2 runtime-executor (optional)
    │                                   │
    ▼                                   ▼
L1 Specialist (kernel-engineer, system-health)
    │
    ▼
CoreIntent v2 ──IPC──► rmngd (Body) ──► PermissionGate ──► execute
```

**Invariant (ADR-010):** No layer executes outside `rmngd`. LLM/nervous system emits intents only.

### Session store schema

```json
{
  "id": "uuid",
  "active_agents": { "L3": { "agent_id": "repo-keeper", "layer": "L3" } },
  "shared_context": {},
  "task_state": { "status": "open", "current_prompt": "..." },
  "handoff_history": [{ "from_agent": "swarm-coordinator", "to_agent": "repo-keeper", ... }]
}
```

## Extensibility

- **L3/L4 agents:** YAML-only — no Rust changes
- **L2/L1 with new tools:** manifest + handler (existing Sprint 1 pipeline)
- **Registration:** `AgentRegistry::load()` scans `agents/definitions/*.yaml`

## Consequences

### Positive

- Clear privilege boundaries per layer
- Session persistence enables future swarm workflows
- Orchestrator can delegate without embedding tool logic

### Negative / deferred

- No hot-reload of agent definitions (restart not required — loaded per request)
- Full swarm consensus / queen election deferred to Sprint 4+
- L4 auto-delegation uses tool-name heuristics in mock mode; production LLM must emit `plan.only` + explicit handoff for complex flows

## Examples

| Agent | Layer | Role |
|-------|-------|------|
| `system-health` | L1 | `kernel.status` only |
| `kernel-engineer` | L1 | Full kernel lab |
| `runtime-executor` | L2 | Multi-tool execution delegate |
| `repo-keeper` | L3 | Git/GitHub domain |
| `swarm-coordinator` | L4 | Task decomposition + handoff |
