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

## Chain failure behavior

Failed hops set `shared_context.orchestration.status = failed` with `failed_hop`, `error`. Audit: `nervous.handoff_chain_hop` outcome `failed`.

## Live LLM notes

- **Groq** (`GROQ_API_KEY`): tends to follow JSON array `handoff_chain` when prompted explicitly.
- **Grok** (`XAI_API_KEY`): may need explicit "JSON array not comma string" in prompt.
- Parser normalizes comma-separated `handoff_chain` strings as fallback.

Run live tests: `cargo test -p rmng-nervous --test live_llm_chain_e2e -- --nocapture`
