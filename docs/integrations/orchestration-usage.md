# Multi-hop orchestration usage (Sprint 23–30)

Live LLM workflows: [live-llm-orchestration.md](live-llm-orchestration.md) · model quirks: [live-llm-chain-quirks.md](live-llm-chain-quirks.md)

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

## Auto-continue foundation (Sprint 25)

Continuation state lives at `orchestration.continuation` (daemon-resumable):

| Field | Purpose |
|-------|---------|
| `continuation.enabled` | Loop active |
| `continuation.step` / `max_steps` | Progress cursor |
| `continuation.active_agent` | Next agent to ask |
| `awaiting_continuation` | Set after handoff chain completes — tool dispatch may follow |
| `history[]` | Last 5 finalized orchestration snapshots |

### Daemon auto-continue (Sprint 26)

Send to rmngd Unix socket:

```json
{"action":"orchestration.continue","session_id":"<sid>"}
```

After successful tool dispatch, rmngd also triggers continuation when `orchestration.continuation` is active or `awaiting_continuation` is true.

Config (`~/.rmng/config.toml`):

```toml
[auto_continue]
max_steps = 3
timeout_secs = 600
default_failure_policy = "abort"
```

Env override: `RMNG_AUTO_CONTINUE_MAX_STEPS` (wins over config `max_steps` when set).

Post-dispatch continuation runs **in the background** so the IPC response stays a normal `HandleResponse` (CLI-compatible). Use explicit `orchestration.continue` to await the full loop result.

CLI `--auto-continue` uses the same `AutoContinueLoop` in `rmng-nervous`; rmngd logs `daemon background auto-continue finished` with `steps` and `status`.

### Production safety (Sprint 27)

- **Per-session lock** — only one `continue_session` runs per session; background triggers are skipped when a loop is already in progress (`daemon background auto-continue skipped`).
- **Timeout cleanup** — when `timeout_secs` elapses, rmngd calls `finalize_orchestration` so `continuation.status` is not left as `running` (`status: timed_out`).
- **CLI defaults** — omit `--max-steps` to use `[auto_continue].max_steps` from config (env `RMNG_AUTO_CONTINUE_MAX_STEPS` still overrides).

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

## Live LLM notes (see [live-llm-chain-quirks.md](./live-llm-chain-quirks.md))

- **Groq** (`GROQ_API_KEY`): tends to follow JSON array `handoff_chain` when prompted explicitly.
- **Grok** (`XAI_API_KEY`): may need explicit "JSON array not comma string" in prompt.
- Parser normalizes comma/arrow/JSON-string `handoff_chain` and `plan_only` action aliases.
- Provider-specific hints via `LlmReasonContext.provider_id`.

Run live tests: `cargo test -p rmng-nervous --test live_llm_chain_e2e -- --nocapture`
