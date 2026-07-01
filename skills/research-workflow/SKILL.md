---
name: research-workflow
description: >-
  Research and documentation curation for RMNG-OS — gather context, summarize
  findings, and emit plan.only or read-only MCP queries. No direct execution.
---

# Research Workflow

Use when the user asks to research, summarize, or document external material.

## Rules

1. Prefer `plan.only` for synthesis and recommendations.
2. Use `github.search_issues` MCP only when GitHub search is explicitly needed.
3. Never emit shell commands or bypass rmngd.
4. Record key findings in session shared context when a session is active.

## Example intents

- `plan.only` — summarize evaluation of an external repo
- `mcp.proxy` — `github.search_issues` with query scoped to RMNG-OS org

## Evaluation

After synthesis, apply `self-critique` then `output-validation` before handoff.
