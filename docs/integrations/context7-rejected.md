# Integration Intake: Upstash Context7

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/upstash/context7 |
| **License** | MIT |
| **Date** | 2026-07-01 |
| **Proposed track** | 4 — Rejected |
| **Status** | **Rejected** |

## Summary

MCP server for live library documentation. Valuable concept but **ContextCrush** vulnerability (indirect prompt injection via poisoned registry docs) documented in user analysis.

## Evaluation scores (1–5)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Zero-trust security | 2 | External doc content treated as trusted |
| **Average** | **2.0** | Fails security floor |

## Decision

- [x] **Rejected** — revisit only with signed doc bundles or local mirror + hash verification

## Alternative

Track 3 skill pointing to pinned local docs in `docs/`; Track 2 fetch for specific known URLs only.