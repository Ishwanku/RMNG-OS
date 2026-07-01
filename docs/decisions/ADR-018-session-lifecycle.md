# ADR-018: Session lifecycle and retention

**Date:** 2026-07-01  
**Status:** Accepted  
**Related:** ADR-017

## Decision

Sessions persist as JSON at `~/.rmng/sessions/<uuid>.json` with no automatic expiry in Sprint 4.

- **Create:** `rmng session new`
- **Shared context:** `rmng session set-context <id> <key> <json-value>`
- **Handoffs:** recorded in `handoff_history`; injected into nervous prompts when `--session` is active
- **Retention:** `rmng session prune --older-than-days N` (Sprint 4b); automatic TTL still deferred
- **Write-back:** successful `rmngd` dispatches with session metadata append to `shared_context.tool_results`

## Consequences

Operators should periodically prune stale sessions (`rmng session list --verbose` shows active vs stale). Future work: automatic TTL enforcement on load.
