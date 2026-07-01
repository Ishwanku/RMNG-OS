---
name: improvement-loop
description: Bounded revise cycle — memory, critique, validation. Max 2 iterations; plan.only for grading.
---

# Improvement Loop

Self-improvement without external eval frameworks.

## When to use

- Critique or validation failed
- User asks to improve a prior answer
- Before Mem0 persist of research lessons

## Skip when

- Validation passed (score >= 0.8)
- Simple tool status output
- User wants a quick draft

## Loop (max 2 iterations)

1. **Recall** — optional `mem0:search_memories` + session context
2. **Produce** — tools + draft synthesis
3. **Self-critique** — plan.only rubric (threshold 0.75)
4. **Output-validation** — threshold 0.8
5. **Pass** — deliver; optional one `add_memory` lesson
6. **Fail** — revise; repeat if iteration < 2

## Mem0 after pass

Store one-sentence **lesson learned**, not full draft.

## promptfoo patterns adopted

| promptfoo | RMNG |
|-----------|------|
| llm-rubric | self-critique JSON |
| threshold | 0.75 / 0.8 |
| not-llm-rubric | negation checks |
| Full CLI | Not imported (Track 3) |

## Limits

- Max 2 critique+validation cycles per request
- Max 1 Mem0 add after loop
- repo-keeper: read-only Mem0 in loop unless user requests add