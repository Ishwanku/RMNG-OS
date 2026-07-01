#!/usr/bin/env bash
# Sprint 6 — run live LLM provider validation matrix.
# Keys via environment only (never commit to config.toml).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export RMNG_PROJECT_ROOT="$ROOT"

echo "=== RMNG LLM Provider Matrix ==="
echo "Keys checked: XAI_API_KEY, OPENAI_API_KEY, GROQ_API_KEY, GOOGLE_API_KEY, Ollama (local)"
echo ""

if command -v rmng >/dev/null 2>&1; then
  rmng llm matrix
else
  cd "$ROOT/agents"
  cargo run -p rmng-cli -- llm matrix
fi

echo ""
echo "Ignored integration tests (per-provider):"
cd "$ROOT/agents"
cargo test -p rmng-nervous provider_matrix -- --ignored --nocapture 2>&1 || true