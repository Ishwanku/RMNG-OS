# LLM Configuration (generic, catalog-driven)

RMNG-OS separates **engine wiring** (Rust adapters) from **model/provider settings** (editable config files). When Google, xAI, or Ollama ship new models, update the catalog — no rebuild required.

## Two files to know

| File | Purpose |
|------|---------|
| `~/.rmng/config.toml` | Active provider, model, keys env var, profiles |
| `config/llm-catalog.toml` | Model ids, endpoints, docs links (repo default) |
| `~/.rmng/llm-catalog.toml` | User override (optional, takes precedence) |

Secrets: `~/.rmng/secrets.env` — never commit API keys.

## Quick setup

```bash
cp config/rmng-config.toml.example ~/.rmng/config.toml
rmng llm setup                    # copy catalog to ~/.rmng/
rmng llm providers                # list all providers from catalog
rmng llm models --provider google # Gemini model ids
rmng llm models --live --provider groq  # compare API vs catalog
rmng llm show                     # active resolved config
rmng llm health
```

## Switching provider or model

**Option A — edit config**

```toml
[llm]
llm_provider = "google"
model = "gemini-3.5-flash"
api_key_env_var = "GOOGLE_API_KEY"
```

Note: `profile = "..."` must be at the **root** of the file (before `[llm]`), not inside the `[llm]` table.

**Option B — named profiles**

```toml
profile = "gemini-reasoning"

[[profiles]]
name = "gemini-reasoning"
llm_provider = "google"
model = "gemini-3.1-pro-preview"
api_key_env_var = "GOOGLE_API_KEY"
```

```bash
rmng llm use gemini-reasoning
```

**Option C — one-off CLI override**

```bash
rmng ask --provider google --model gemini-2.5-flash "check git status" --dry-run
rmng ask --profile anthropic-economy "summarize session" --dry-run
```

## Google Gemini (mid-2026)

Text models for RMNG core-intent JSON:

| Model id | Use case |
|----------|----------|
| `gemini-3.5-flash` | **Default** — fast agentic workflows |
| `gemini-3.1-pro-preview` | Deep reasoning |
| `gemini-3.1-flash-lite` | Budget / high volume |
| `gemini-2.5-pro` | Stable premium |
| `gemini-2.5-flash` | Stable price/performance |
| `gemini-2.5-flash-lite` | Budget repetitive ops |

Specialized (not used for intent JSON by default): image, live audio, TTS, embeddings — see catalog with `rmng llm models --provider google --specialized`.

Auth: `GOOGLE_API_KEY` + `X-goog-api-key` header (wired in provider).

## When models change

1. Edit `~/.rmng/llm-catalog.toml` (or repo `config/llm-catalog.toml`)
2. Add/update `[[providers.<id>.models]]` entries
3. Set `default = true` on the new production default
4. `rmng llm models --provider <id>` to verify

No Rust changes unless the **API style** changes (e.g. new auth scheme).

## API styles (engine — rarely changes)

| `api_style` | Providers |
|-------------|-----------|
| `google` | Gemini `generateContent` |
| `openai_compat` | OpenAI, Grok, Groq, Together, Fireworks, DeepSeek, NIM, custom |
| `anthropic` | Claude Messages API |
| `ollama` | Local `/api/generate` |
| `mock` | `none` provider |

## Per-agent model selection (Sprint 7)

Agents in `agents/definitions/*.yaml` can override the global `[llm]` config:

```yaml
# agents/definitions/swarm-coordinator.yaml
llm_profile: groq-fast          # named [[profiles]] from ~/.rmng/config.toml

# agents/definitions/kernel-engineer.yaml
llm_profile: gemini-reasoning

# Or inline overrides (when llm_profile is unset):
llm_provider: grok
model: grok-4.3
```

Resolution order: global `[llm]` → active `profile` → agent `llm_profile` → agent `llm_provider` / `model`.

## Provider fallback chains (Sprint 8)

When the primary LLM fails with a retryable error (rate limit, billing, model not found, network), the nervous layer automatically tries the next profile in the fallback chain — transparent to the caller when a session is active.

**Global fallback** in `~/.rmng/config.toml`:

```toml
llm_fallback = ["grok-frontier", "local-ollama"]

[llm]
llm_provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env_var = "GROQ_API_KEY"

[[profiles]]
name = "grok-frontier"
llm_provider = "grok"
model = "grok-4.3"
api_key_env_var = "XAI_API_KEY"

[[profiles]]
name = "local-ollama"
llm_provider = "ollama"
endpoint_url = "http://127.0.0.1:11434"
model = "llama3.2"
```

**Per-agent override** in `agents/definitions/*.yaml`:

```yaml
llm_profile: groq-fast
llm_fallback:
  - grok-frontier
  - local-ollama
```

Per-agent `llm_fallback` replaces the global list when non-empty. Invalid API keys do **not** trigger fallback (fix the key instead). Fallback attempts are logged as `nervous.llm_fallback` and shown in `rmng observe`.

## Handoff pre-validation (Sprint 8)

Before any handoff (CLI, router, or LLM-suggested `metadata.handoff_to` / `handoff_chain`), RMNG validates:

1. Session exists and is loadable
2. Every agent id in the chain exists in the registry
3. Layer rules and `delegates_to` constraints are satisfied

Failures return a clear error before runtime dispatch — e.g. `handoff 'repo-keeper' → 'swarm-coordinator' rejected`. Invalid chains from LLM metadata are logged and skipped rather than partially executed.

## Generation parameters

Optional in `[llm]` or `[[profiles]]`:

```toml
temperature = 0.0
max_tokens = 4096
top_p = 1.0
```

OpenAI-compatible providers honor all three; others use provider defaults where unsupported.

## CLI reference

```bash
rmng llm show
rmng llm providers
rmng llm models [--provider google] [--specialized] [--live]
rmng llm use <profile>
rmng llm setup
rmng llm health
rmng llm matrix
rmng llm sync-catalog [--specialized]
```

`rmng observe` shows global fallback chain and per-session LLM call latency when metrics are recorded.

See also: [llm-providers.md](./llm-providers.md), [llm-provider-matrix.md](./llm-provider-matrix.md).