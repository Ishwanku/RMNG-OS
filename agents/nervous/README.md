# Nervous System (BYO-LLM)

The nervous system emits JSON intents only. Implementation: `rmng-nervous` crate.

## Configuration

`~/.rmng/config.toml` — see `config/rmng-config.toml.example`

| `llm_provider` | Behavior |
|----------------|----------|
| `none` (default) | Mock intents — no network calls |
| `ollama` | Local Ollama `/api/generate` |
| `openai` / `grok` | OpenAI-compatible chat completions |
| `anthropic` / `google` | Native Messages / Gemini APIs |
| `groq` / `together` / `fireworks` / `deepseek` / `nvidia_nim` | OpenAI-compatible |
| `custom` | Self-hosted OpenAI-compatible (vLLM, etc.) |

See [docs/llm-providers.md](../../docs/llm-providers.md).

## Usage

```bash
rmng ask "show git status" --dry-run    # mock → git.status intent
rmng ask "plan next phase" --dry-run    # mock → plan intent
```
