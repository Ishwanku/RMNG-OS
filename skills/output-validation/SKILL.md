---
name: output-validation
description: Lightweight output gate — deterministic checks plus rubric threshold. plan.only only.
---

# Output Validation

Final gate before delivery. Complements self-critique.

## When to use

- After self-critique when score >= 0.75
- Before mem0:add_memory
- Skip for raw single-tool passthrough

Budget: One validation pass per deliverable.

## Layer 1 — Deterministic checks

- Valid CoreIntent v2 if tools proposed
- Tools within agent allowlist
- Claims cite session tool_results or memory hits
- No secrets in Mem0 payloads

## Layer 2 — Rubric (threshold 0.8)

pass and score >= 0.8 required. Include assertions for grounded, scoped, actionable.

## On failure

Emit plan.only with revision plan; invoke improvement-loop (max one extra cycle).
