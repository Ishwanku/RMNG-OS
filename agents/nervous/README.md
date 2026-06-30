# Nervous System (BYO-LLM)

The nervous system emits JSON intents only. Implementation: `rmng-nervous` crate.

## Configuration

`~/.rmng/config.toml` — see `config/rmng-config.toml.example`

| `llm_provider` | Behavior |
|----------------|----------|
| `none` (default) | Mock intents — no network calls |
| `ollama` | Live Ollama `/api/generate` |
| `openai` / `anthropic` / `custom` | Scaffolded — not wired yet |

## Usage

```bash
rmng ask "show git status" --dry-run    # mock → git.status intent
rmng ask "plan next phase" --dry-run    # mock → plan intent
```
