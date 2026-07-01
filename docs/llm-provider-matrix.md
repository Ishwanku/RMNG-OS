# LLM Provider Test Matrix (Sprint 6)

Structured validation for production providers. Keys come from **environment variables only**.

## Run

```bash
export XAI_API_KEY=...        # xAI Grok (xai- prefix)
export OPENAI_API_KEY=...     # OpenAI
export GROQ_API_KEY=...       # Groq (gsk_ prefix)
export GOOGLE_API_KEY=...     # Gemini

./scripts/llm-provider-matrix.sh
# or
rmng llm matrix
```

Integration tests (ignored by default):

```bash
cd agents
cargo test -p rmng-nervous provider_matrix -- --ignored --nocapture
```

## Matrix columns

| Column | Meaning |
|--------|---------|
| `key` | Env var resolved and non-empty |
| `health` | Endpoint reachable (`/models` or minimal completion) |
| `json` | Model returns parseable v2 `CoreIntent` |
| `env` | Expected env var name |

## Provider quirks

### Grok (xAI)

- OpenAI-compatible `/v1/chat/completions`
- Default model: `grok-4.3`
- JSON mode supported; occasional markdown fences (stripped by parser)
- 401 = invalid `XAI_API_KEY`

### OpenAI

- Default model: `gpt-4o` (matrix may use `gpt-4o-mini` for cost)
- Strong JSON adherence with `json_object` response format
- 429 common under load — retried automatically

### Groq

- Keys use `gsk_` prefix — **not** xAI Grok
- Fast inference; JSON reliable on Llama models
- Rate limits stricter than OpenAI

### Google Gemini

- Auth via `X-goog-api-key` header (Sprint 6)
- Default model: `gemini-2.0-flash`
- `responseMimeType: application/json` in generation config

### Ollama (local)

- No API key; requires `ollama serve` on `127.0.0.1:11434`
- JSON quality varies by model — `llama3.2` recommended
- Health = `/api/tags` or generate probe

## Autonomous handoff (related)

When the LLM sets `metadata.handoff_to` to a valid agent id and a session is active, the router calls `handoff()` automatically. Prompt guidance is in `SESSION_ORCHESTRATION_GUIDE` (`skill.rs`).