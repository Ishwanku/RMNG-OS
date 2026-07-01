---
name: test-coverage-check
description: >-
  Lightweight checklist rubric for test completeness — plan.only only.
  No coverage tooling; composable with run-tests.
---

# Test Coverage Check

Mental **coverage rubric** before or after `run-tests`. RMNG has no coverage MCP — this skill prevents "one happy-path assert" false confidence.

## When to use

| Use | Skip |
|-----|------|
| Before first `run_code` — design test matrix | Trivial `print(1)` smoke |
| After `validate-output` pass — confirm breadth | User asked for quick yes/no only |
| Research-curator validating integration snippets | repo-keeper git status checks |

**Budget guard:** Max **1** coverage check per request.

## Checklist (plan.only)

Score each 0–1; require average ≥ 0.7 before declaring adequate coverage:

| Item | Question |
|------|----------|
| Happy path | Does at least one test exercise the main success case? |
| Edge case | Zero, empty, or boundary input tested? |
| Error path | Invalid input or expected failure handled? |
| Grounding | Tests match claims from research / tool_results? |
| Minimality | No redundant duplicate asserts wasting sandbox budget? |

```json
{
  "adequate": true,
  "score": 0.8,
  "gaps": [],
  "suggested_tests": []
}
```

If `adequate: false`, add `suggested_tests` and run **one** expanded `run-tests` harness — not a third sandbox run unless the first never executed.

## Composition

```
research → test-coverage-check → run-tests → validate-output
```

Or post-hoc: `validate-output` pass → `test-coverage-check` → optional Mem0 lesson if gaps found for future work.

## Anti-overuse

- Do not demand exhaustive coverage for exploratory research
- Do not block delivery on coverage when user asked for a single smoke test
- One checklist per request — no recursive coverage-on-coverage
