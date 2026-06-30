# Nervous System

LLM adapters live in the `rmng-nervous` crate (`../rmng-nervous/`).

## Ollama (default)

```bash
ollama serve
ollama pull llama3.2

# Dry-run: get intent JSON only
rmng ask "check kernel build status" --dry-run

# Full pipeline: intent → permission → tool
rmng ask "check kernel build status"
```

Environment: `OLLAMA_HOST` can be passed via `--ollama` flag.
