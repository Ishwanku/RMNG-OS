# Live LLM chain quirks (Sprint 25–30)

Model-specific behavior when emitting `handoff_chain` and orchestration metadata.

## Parser fallbacks (all providers)

`parse_core_intent` normalizes before strict deserialize:

| Mistake | Normalized to |
|---------|----------------|
| `handoff_chain: "a, b, c"` | JSON array |
| `handoff_chain: "a; b; c"` | JSON array |
| `handoff_chain: "a -> b -> c"` | JSON array |
| `handoff_chain: "[\"a\",\"b\"]"` | JSON array |
| `handoff_chain: ["a", "", "b"]` | `["a","b"]` (empty filtered) |
| `handoff_chain: ["a"]` | **dropped** (< 2 agents; logged) |
| `action: "plan_only"` | `plan.only` |
| Top-level `handoff_chain` | Hoisted into `metadata` |
| Trailing commas in JSON | Stripped |

Invalid chains are removed with a `tracing::warn` — router may fall through to `handoff_to` or `Direct`.

## Provider notes

### Groq (`GROQ_API_KEY`, `llama-3.3-70b-versatile`)

- Generally follows JSON array `handoff_chain` when few-shot examples are in prompt.
- Occasionally adds trailing commas; parser strips them.
- **Prompt hint:** strict JSON array example with WRONG/RIGHT comparison.

### Grok / xAI (`XAI_API_KEY`, `grok-3-mini`)

- Often emits comma-separated string instead of array; parser fixes on first pass.
- May wrap JSON in markdown fences on first attempt; repair pass usually succeeds.
- **Prompt hint:** inline JSON example; explicit "no fences" rule.

### Ollama (local, `llama3.2`)

- Smaller models may use `plan_only` action; normalized to `plan.only`.
- May emit single `handoff_to` instead of chain — use `RMNG_CHAIN_STRICT=0` in tests while tuning prompts.

### OpenAI / Anthropic / Google

- Generally compliant when given raw-JSON-only instruction.
- Anthropic may add preamble — `extract_json_payload` handles prose wrappers.

## Prompt layers (Sprint 30)

| Layer | Source |
|-------|--------|
| Decision tree | `skill.rs` `SESSION_ORCHESTRATION_GUIDE` |
| Few-shot + recipes | `orchestration_prompt.rs` |
| Provider hints | `providers/prompt.rs` `provider_chain_hints()` |
| Repair pass | `providers/reason.rs` `REPAIR_SUFFIX` |

## Testing

```bash
cargo test -p rmng-nervous --test chain_parse_e2e
cargo test -p rmng-nervous --test live_llm_chain_e2e -- --nocapture
cargo test -p rmng-nervous --test live_llm_chain_matrix -- --ignored --nocapture
```

See [live-llm-orchestration.md](live-llm-orchestration.md) for operator workflows.