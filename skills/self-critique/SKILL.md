---
name: self-critique
description: >-
  LLM-as-judge self-critique via plan.only — rubric scoring adapted from promptfoo
  llm-rubric. Use once per deliverable before handoff; composable with output-validation.
---

# Self-Critique

Critique your **own** draft output before presenting it to the user or persisting to Mem0. Track 3 only — emit `plan.only`, never a new tool call for grading.

## When to use

| Use | Skip |
|-----|------|
| Research summaries, integration recommendations, multi-source synthesis | Single-line confirmations, raw tool passthrough |
| After fetch/markitdown/github MCP results are synthesized | Before any work is done |
| User asks for review or quality check | Task already passed `output-validation` with `pass: true` |

**Budget guard:** At most **one** self-critique pass per user request unless the user explicitly asks for another round.

## Rubric format (promptfoo `llm-rubric` adapted)

Structure your critique as JSON inside `plan.only` reasoning:

```json
{
  "pass": true,
  "score": 0.85,
  "reason": "Concise analysis against rubric criteria",
  "criteria": [
    { "name": "accuracy", "score": 0.9, "note": "Claims match tool_results" },
    { "name": "completeness", "score": 0.8, "note": "Missing rollback step" },
    { "name": "actionability", "score": 0.85, "note": "Clear next steps" }
  ],
  "fixes": ["Add rollback command", "Cite fetch source URL"]
}
```

- `score`: 0.0–1.0 aggregate; use `threshold: 0.75` mentally — below threshold → revise before responding
- `pass`: explicit boolean; do not assume pass when score is low
- `fixes`: ordered list of concrete edits (not vague "improve clarity")

## Default rubric criteria

Adapt weights to task type:

| Criterion | Research | Web/doc | Repo/git |
|-----------|----------|---------|----------|
| Grounded in evidence | Required | Required | Required |
| No hallucinated tools/paths | Required | Required | Required |
| Appropriate scope (no scope creep) | Required | Required | Required |
| Concise structure | Preferred | Preferred | Preferred |

## Negation checks (promptfoo `not-llm-rubric` pattern)

Explicitly fail if the draft:

- Recommends tools not in agent `allowed_mcp_tools` / `allowed_native_tools`
- Stores secrets or full raw MCP blobs in Mem0
- Claims sprint/roadmap status without citing `docs/ROADMAP.md` evidence
- Apologizes excessively or hedges without adding information

## Composition

```
memory-management (search) → work → self-critique → output-validation → [revise] → memory-management (add)
```

Pair with `improvement-loop` when revision is needed; hand off to `output-validation` for the final gate.

## Output intent

Always `plan.only` — the critique is reasoning for the nervous system, not an executable action:

```json
{
  "action": "plan.only",
  "reasoning": "<JSON rubric block above + revised draft if score < threshold>"
}
```