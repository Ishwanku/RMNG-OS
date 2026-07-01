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

## CLI reference

```bash
rmng llm show
rmng llm providers
rmng llm models [--provider google] [--specialized]
rmng llm use <profile>
rmng llm setup
rmng llm health
rmng llm matrix
```

See also: [llm-providers.md](./llm-providers.md), [llm-provider-matrix.md](./llm-provider-matrix.md).