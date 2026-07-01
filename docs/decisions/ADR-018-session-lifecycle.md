# ADR-018: Session lifecycle and retention

**Date:** 2026-07-01  
**Status:** Accepted  
**Related:** ADR-017

## Decision

Sessions persist as JSON at `~/.rmng/sessions/<uuid>.json` with no automatic expiry in Sprint 4.

- **Create:** `rmng session new`
- **Shared context:** `rmng session set-context <id> <key> <json-value>`
- **Handoffs:** recorded in `handoff_history`; injected into nervous prompts when `--session` is active
- **Retention:** manual cleanup for now; automated pruning deferred

## Consequences

Operators should periodically remove stale sessions. Future work: TTL and `rmng session prune`.
