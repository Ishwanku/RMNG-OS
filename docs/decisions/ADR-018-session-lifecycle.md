# ADR-018: Session lifecycle and retention

**Date:** 2026-07-01  
**Status:** Accepted  
**Related:** ADR-017

## Decision

Sessions persist as JSON at `~/.rmng/sessions/<uuid>.json` with no automatic expiry in Sprint 4.

- **Create:** `rmng session new`
- **Shared context:** `rmng session set-context <id> <key> <json-value>`
- **Handoffs:** recorded in `handoff_history`; injected into nervous prompts when `--session` is active
- **Retention:** `rmng session prune --older-than-days N` (Sprint 4b)
- **TTL on load:** sessions older than `RMNG_SESSION_TTL_DAYS` (default 90) are auto-deleted (Sprint 4c); set `0` to disable
- **Lifecycle:** `active` (<1h since update), `idle` (<7d), `stale` (≥7d) — shown in `rmng session list --verbose`
- **Write-back:** successful `rmngd` dispatches with session metadata append to `shared_context.tool_results`
- **Live LLM:** session orchestration guide + `recent_tool_results` injected into Ollama prompts when `--session` is active

## Consequences

Operators should periodically prune stale sessions. Expired sessions are rejected on load. Live LLM workflows require `~/.rmng/config.toml` with a configured provider.
