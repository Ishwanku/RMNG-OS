# Live LLM chain quirks (Sprint 25)

Model-specific behavior when emitting `handoff_chain` and orchestration metadata.

## Parser fallbacks (all providers)

`parse_core_intent` normalizes before strict deserialize:

| Mistake | Normalized to |
|---------|----------------|
| `handoff_chain: "a, b, c"` | JSON array |
| `handoff_chain: "a -> b -> c"` | JSON array |
| `handoff_chain: "[\"a\",\"b\"]"` | JSON array |
| `action: "plan_only"` | `plan.only` |
| Top-level `handoff_chain` | Hoisted into `metadata` |
| Trailing commas in JSON | Stripped |

## Provider notes

### Groq (`GROQ_API_KEY`, `llama-3.3-70b-versatile`)

- Generally follows JSON array `handoff_chain` when examples are in prompt.
- Occasionally adds trailing commas; parser strips them.

### Grok / xAI (`XAI_API_KEY`, `grok-3-mini`)

- Often emits comma-separated string instead of array; parser fixes, prompt stresses JSON array only.
- May wrap JSON in markdown fences on first attempt; repair pass usually succeeds.

### Ollama (local)

- Smaller models may use `plan_only` action; normalized to `plan.only`.

## Testing

```bash
cargo test -p rmng-nervous --test chain_parse_e2e
cargo test -p rmng-nervous --test live_llm_chain_e2e -- --nocapture
```

## Prompt path

`build_reasoning_prompt` adds provider hints via `LlmReasonContext.provider_id` during fallback chain execution.
