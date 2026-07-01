---
name: regression-check
description: >-
  Compare current test results to session history or Mem0 baselines. plan.only;
  optional mem0:search_memories for prior pass markers.
---

# Regression Check

Detect whether current sandbox results **regressed** vs earlier evidence in the session or long-term memory.

## When to use

| Use | Skip |
|-----|------|
| After `validate-output` pass on a changed snippet | First-ever run with no baseline |
| User asks "did we break anything?" | No prior `e2b.run_code` or stored baseline |
| Before `memory-management` add of "test passed" lesson | Single-run smoke with no history |

**Budget guard:** Max **1** regression check per request. Max **1** `search_memories` call to fetch baselines.

## Baseline sources (priority order)

1. Earlier `e2b.run_code` entries in **same session** `tool_results`
2. Mem0 hits containing `test_pass` or `baseline:` markers (`search_memories`)
3. User-stated expected output in the prompt

## plan.only format

```json
{
  "regression": false,
  "baseline_source": "session.tool_results[2]",
  "current_source": "session.tool_results[5]",
  "diff": "Both report pass:true with tests:3",
  "action": "none"
}
```

On regression:

```json
{
  "regression": true,
  "diff": "Baseline pass:3 tests; current pass:false failed:['edge']",
  "action": "run-tests once with fix, then validate-output"
}
```

## Composition

```
run-tests → validate-output → regression-check → output-validation → add_memory (lesson)
```

`repo-keeper`: read-only Mem0 for baseline search unless user requests storing a new baseline.

## Anti-overuse

- Do not regression-check without a current validated pass/fail result
- Do not auto-store baselines in Mem0 — only after user confirms or `output-validation` passes
- Skip when session has only one sandbox run
