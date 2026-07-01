# LLM Providers (Sprint 6)

RMNG-OS nervous system uses a **pluggable provider abstraction** in `rmng-nervous/src/providers/`. All providers emit the same v2 `CoreIntent` JSON — the body (`rmngd`) never changes.

## Architecture

```
rmng ask / router
    → NervousConnector
        → assemble_prompt_full (session + agent context)
        → LlmBackend::from_config
            → OllamaProvider | OpenAiCompatProvider | AnthropicProvider | GoogleProvider
        → parse_core_intent → CoreIntent
    → rmngd dispatch (unchanged)
```

| Component | Role |
|-----------|------|
| `LlmRequest` / `LlmResponse` | Standard request/response envelope |
| `build_reasoning_prompt` | Shared prompt + session hints + JSON examples |
| `LlmBackend` | Factory + dispatch (no orchestration changes per provider) |
| `OpenAiCompatProvider` | OpenAI, Grok, Groq, Together, Fireworks, DeepSeek, NVIDIA NIM, custom |

## Configuration

File: `~/.rmng/config.toml`

```toml
[llm]
llm_provider = "grok"          # see table below
model = "grok-2-latest"
api_key_env_var = "XAI_API_KEY" # preferred
max_retries = 2
timeout_secs = 120
```

### Supported providers

| `llm_provider` | Default endpoint | Default model | API key env |
|----------------|------------------|---------------|-------------|
| `none` | — | mock | — |
| `ollama` | `http://127.0.0.1:11434` | `llama3.2` | — |
| `openai` | `https://api.openai.com/v1` | `gpt-4o` | `OPENAI_API_KEY` |
| `grok` | `https://api.x.ai/v1` | `grok-2-latest` | `XAI_API_KEY` |
| `anthropic` | `https://api.anthropic.com` | `claude-3-5-sonnet-20241022` | `ANTHROPIC_API_KEY` |
| `google` | `https://generativelanguage.googleapis.com` | `gemini-2.0-flash` | `GOOGLE_API_KEY` |
| `groq` | `https://api.groq.com/openai/v1` | `llama-3.3-70b-versatile` | `GROQ_API_KEY` |
| `together` | `https://api.together.xyz/v1` | `meta-llama/Llama-3-8b-chat-hf` | `TOGETHER_API_KEY` |
| `fireworks` | `https://api.fireworks.ai/inference/v1` | (see defaults) | `FIREWORKS_API_KEY` |
| `deepseek` | `https://api.deepseek.com/v1` | `deepseek-chat` | `DEEPSEEK_API_KEY` |
| `nvidia_nim` | `https://integrate.api.nvidia.com/v1` | `meta/llama3-8b-instruct` | `NVIDIA_API_KEY` |
| `custom` | **you must set** `endpoint_url` | `gpt-4o` | `RMNG_LLM_API_KEY` |

Override any default with `endpoint_url`, `model`, or `api_key_env_var` in config.

## Quick start: Ollama (local)

```bash
cp config/rmng-config.toml.example ~/.rmng/config.toml
# Edit: llm_provider = "ollama"
ollama serve
rmng llm health
rmng ask --agent repo-keeper "check git status" --dry-run
```

## Getting started with Grok (xAI)

Grok uses the **xAI** API (`https://api.x.ai/v1`). Keys start with `xai-` (not `gsk_` — that prefix is **Groq**).

```bash
export XAI_API_KEY="xai-..."
cat >> ~/.rmng/config.toml <<'EOF'
[llm]
llm_provider = "grok"
model = "grok-2-latest"
api_key_env_var = "XAI_API_KEY"
EOF
rmng llm health
rmng llm matrix    # optional: probe all configured env keys
rmng ask "plan next sprint task" --dry-run
```

### Grok quirks (Sprint 6 matrix)

| Area | Behavior |
|------|----------|
| JSON mode | `response_format: json_object` — reliable for `tool.execute` / `plan.only` |
| Tool calling | Native tool APIs unused; RMNG uses JSON intent envelope |
| Rate limits | 429 retried with backoff (`max_retries` in config) |
| Invalid key | 401 with hint in error message |

## Groq vs Grok

| Provider | Key prefix | Env var | Endpoint |
|----------|------------|---------|----------|
| **Grok** (xAI) | `xai-` | `XAI_API_KEY` | `api.x.ai` |
| **Groq** | `gsk_` | `GROQ_API_KEY` | `api.groq.com` |

If you have a `gsk_` key, use `llm_provider = "groq"` not `grok`.

## Quick start: OpenAI

```bash
export OPENAI_API_KEY="sk-..."
cat >> ~/.rmng/config.toml <<'EOF'
[llm]
llm_provider = "openai"
model = "gpt-4o"
api_key_env_var = "OPENAI_API_KEY"
EOF
rmng llm health
rmng ask --agent swarm-coordinator "check git status" --dry-run
```

## Custom / self-hosted (vLLM, NVIDIA NIM, etc.)

Any **OpenAI-compatible** `/v1/chat/completions` endpoint:

```toml
[llm]
llm_provider = "custom"   # or nvidia_nim, groq, etc.
endpoint_url = "http://localhost:8000/v1"
model = "my-local-model"
api_key_env_var = "RMNG_LLM_API_KEY"
```

## Secure API key management

- **Never** commit API keys to `config.toml` or the repo.
- Prefer `api_key_env_var` pointing at shell env or a secrets manager.
- Use separate env vars per provider (`XAI_API_KEY`, `OPENAI_API_KEY`, `GROQ_API_KEY`).
- Rotate keys immediately if exposed in chat, logs, or CI output.
- `rmng llm health` reports `key_set` without printing the key value.

## Sprint 6 reliability features

- **Auto-retry**: invalid JSON from the model triggers one repair prompt; logged as `nervous.llm_retry` in `~/.rmng/logs/audit.jsonl`.
- **Autonomous handoff**: LLM may set `metadata.handoff_to` (agent id); router auto-handoffs when a session is active.
- **Rate limits**: 429 responses retry with exponential backoff.
- **Provider matrix**: `rmng llm matrix` or `./scripts/llm-provider-matrix.sh`.

See also: [llm-provider-matrix.md](./llm-provider-matrix.md).

## CLI commands

```bash
rmng llm list      # all supported providers
rmng llm health    # detailed probe of configured provider
rmng llm matrix    # validate Grok/OpenAI/Groq/Google/Ollama via env keys
rmng observe       # includes llm health summary
```

## Adding a new provider

1. If OpenAI-compatible: add variant to `LlmProviderKind` in `rmng-core/src/config.rs` and defaults in `providers/defaults.rs` — `OpenAiCompatProvider` handles the rest.
2. If native API (like Anthropic): add `providers/yourprovider.rs` implementing `health`, `complete`, `reason_core`.
3. Wire in `LlmBackend::from_config` in `providers/factory.rs`.
4. Add tests in `tests/providers.rs`.

No changes to router, rmngd, or session logic required.