# Integration: promptfoo patterns (Track 3)

| Field | Value |
|-------|-------|
| Repository | https://github.com/promptfoo/promptfoo |
| Track | 3 Skill — patterns only |
| Status | Active (Sprint 16) |

## Summary

Adopt llm-rubric, threshold, and negation patterns via self-critique, output-validation, improvement-loop. Full promptfoo CLI not imported.

## Patterns

- llm-rubric -> self-critique JSON (pass, score, reason)
- threshold -> 0.75 critique / 0.8 validation
- not-llm-rubric -> negation checks
- Variables -> session tool_results + Mem0 hits

## Decision

Track 3 only. MCP/CLI deferred.
