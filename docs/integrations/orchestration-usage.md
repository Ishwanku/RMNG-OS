# Multi-hop orchestration usage (Sprint 23–24)

## LLM metadata fields

| Field | Who emits | Purpose |
|-------|-----------|---------|
| `handoff_chain` | L4 orchestrator | Ordered multi-agent sequence |
| `handoff_to` | Any | Single-hop delegation |
| `handoff_return_to` | L3/L2 specialist | Return summary to orchestrator |

## Auto-continue (Sprint 24)

Reduces manual follow-up `rmng ask` calls:

```bash
rmng session new
rmng ask --agent swarm-coordinator --session <sid> --auto-continue --max-steps 3 \
  "check git status and report back"
```

Loop: ask → dispatch executable intent → re-ask final agent with continuation prompt → until `plan.only` or max steps.

## Hop failure policy (Sprint 25)

Set on `plan.only` metadata when emitting `handoff_chain`:

| Field | Values | Default |
|-------|--------|---------|
| `hop_failure_policy` | `retry`, `skip`, `abort` | `abort` |
| `hop_retry_max` | integer ≥ 1 | `2` (when policy is `retry`) |

**Policies:**
- **abort** — stop chain, set `orchestration.status = failed` (backward compatible).
- **retry** — retry the same hop up to `hop_retry_max`, then abort.
- **skip** — record skipped hop, attempt shortcut from current agent to the agent after the failed target (e.g. L4→L3 fails → try L4→L2).

Audit events: `nervous.handoff_chain_policy` (decision), `nervous.handoff_chain_hop` with outcomes `retry` / `skipped` / `failed`.

Session `shared_context.orchestration` gains `hop_decisions[]` and `skipped_hops[]` on recovery paths.

## Chain error recovery (Sprint 25)

Failed and skipped hops are persisted for orchestrator re-planning:

| Field | Purpose |
|-------|---------|
| `hop_errors[]` | Structured errors (`hop_index`, `from_agent`, `to_agent`, `error`, `action`) |
| `error` / `failed_hop` | Terminal failure when policy is `abort` |
| `skipped_hops[]` | Hops bypassed by `skip` policy |

The orchestrator sees `Chain error recovery` in `prompt_context` and orchestration prompts when status is `failed` or `completed_with_skips`. CLI prints `--- chain recovery context ---` on router/handoff errors.

## Chain failure behavior

Failed hops set `shared_context.orchestration.status = failed` with `failed_hop`, `error`. Audit: `nervous.handoff_chain_hop` outcome `failed`.

## Live LLM notes

- **Groq** (`GROQ_API_KEY`): tends to follow JSON array `handoff_chain` when prompted explicitly.
- **Grok** (`XAI_API_KEY`): may need explicit "JSON array not comma string" in prompt.
- Parser normalizes comma-separated `handoff_chain` strings as fallback.

Run live tests: `cargo test -p rmng-nervous --test live_llm_chain_e2e -- --nocapture`
