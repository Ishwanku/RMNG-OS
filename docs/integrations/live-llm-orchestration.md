# Live LLM multi-hop orchestration (Sprint 30)

Guide for running multi-agent `handoff_chain` and `handoff_return_to` workflows with real LLM providers.

## Quick start

```bash
export RMNG_PROJECT_ROOT=~/dev/projects/RMNG-OS
rmng session new
SID=<session-id>

# L4 orchestrator — multi-hop chain
rmng ask --session "$SID" --agent swarm-coordinator \
  "Coordinate git hygiene: repo-keeper inspects, runtime-executor runs checks"

# Auto-continue through hops
rmng ask --session "$SID" --agent swarm-coordinator --auto-continue --max-steps 5 \
  "Run the git workflow end-to-end"
```

## Prompt path (what the model sees)

1. `SESSION_ORCHESTRATION_GUIDE` — decision tree + strict JSON rules
2. `orchestration_prompt::session_chain_hints` — recipes, few-shot examples, error recovery
3. `build_reasoning_prompt` — provider-specific hints + chain format rules + examples

Provider hints are injected via `LlmReasonContext.provider_id` during LLM calls.

## Required metadata

| Field | Who | Format |
|-------|-----|--------|
| `handoff_chain` | L4 | JSON array, min 2 agent ids, first = your id |
| `handoff_to` | Any | Single agent id string |
| `handoff_return_to` | L3/L2 | Usually `swarm-coordinator` |
| `chain_id` | L4 | Recommended = `session_id` |
| `hop_failure_policy` | L4 | `retry` \| `skip` \| `abort` |

## Model-specific patterns

See [live-llm-chain-quirks.md](live-llm-chain-quirks.md) for parser fallbacks and per-provider notes.

| Provider | Chain emission tip |
|----------|-------------------|
| **Groq** | Usually follows JSON array; occasional trailing commas (parser strips) |
| **Grok/xAI** | Often comma-string on first try — parser normalizes; repair pass helps |
| **Ollama** | Use compact JSON; may alias `plan_only` → `plan.only` |
| **OpenAI / Anthropic / Google** | Raw JSON only; array required for multi-hop |

## Error recovery behavior

When a hop fails, circuit opens, or budget denies:

- **L3/L2** should emit `plan.only` + `handoff_return_to` with summary — not restart the full chain.
- **L4** should synthesize or emit a shorter chain with `hop_failure_policy: "skip"`.
- Prompts include `ERROR_RECOVERY_HINTS` when session orchestration state is present.

## Testing

```bash
# Offline parser normalization
cargo test -p rmng-nervous --test chain_parse_e2e

# Live chain emission (needs API keys / Ollama)
cargo test -p rmng-nervous --test live_llm_chain_e2e -- --nocapture

# Strict mode (default): requires HandoffChain len >= 2
RMNG_CHAIN_STRICT=0 cargo test -p rmng-nervous --test live_llm_chain_e2e -- --nocapture

# Full provider matrix (ignored by default)
cargo test -p rmng-nervous --test live_llm_chain_matrix -- --ignored --nocapture
```

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `Direct` plan.only instead of chain | Model used `handoff_to` or invalid chain | Strengthen prompt; check parser dropped chain (< 2 ids) |
| Parse retry in audit | Fences, comma-string, or `plan_only` alias | Normal — repair pass or parser handles |
| Chain rejected at router | Unknown agent id or layer violation | Use ids from `orchestration_prompt::KNOWN_AGENT_IDS` |
| Single-hop only from Grok | Common quirk | Parser normalizes comma-string; prompt stresses array |

## References

- [orchestration-usage.md](orchestration-usage.md) — hop policies, auto-continue, daemon IPC
- [live-llm-chain-quirks.md](live-llm-chain-quirks.md) — parser table + provider notes
- `agents/rmng-nervous/src/orchestration_prompt.rs` — recipes and few-shot examples