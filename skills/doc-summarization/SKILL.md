---
name: doc-summarization
description: >-
  Summarize fetched or ingested content into structured plan.only outputs
  with citations and session write-back awareness.
---

# Document Summarization

## Workflow

1. **Ingest** — prior `fetch.fetch` or `markitdown.convert_to_markdown` result in session `tool_results`.
2. **Synthesize** — emit `plan.only` with sections: Summary, Key Points, Sources, Open Questions.
3. **Cite** — reference tool id and URL/URI from prior result metadata.

## Session awareness

When `metadata.session_id` is set, read `tool_results` from session context before summarizing. Do not re-fetch the same URL unless the user requests an update.

## Output shape (plan.only reasoning)

```
## Summary
(2-4 sentences)

## Key points
- bullet list

## Sources
- fetch.fetch: <url> OR markitdown: <uri>

## Open questions
- items needing follow-up
```

## Rules

- Never invent content not present in tool output.
- Flag low-confidence or truncated content (`...(truncated)` in output).