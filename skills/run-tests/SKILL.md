---
name: run-tests
description: >-
  Compose E2B sandbox runs into bounded test harnesses. Max 2 run_code per request;
  pair with validate-output after each run.
---

# Run Tests

Shape **one** `e2b:run_code` intent per test batch. Track 3 only — the skill guides intent emission; execution stays in the sandbox (`code-execution`).

## When to use

| Use | Skip |
|-----|------|
| Verify a Python snippet after research synthesis | Host `cargo test` / `pytest` on RMNG machine |
| Smoke-test algorithm or transform logic | User only asked for a plan or summary |
| Re-run after `validate-output` reports failure | E2B not enabled in allowlist |
| Regression repro from `regression-check` findings | Same tests already passed in this session |

**Budget guard:** Max **2** `run_code` intents per user request. Batch assertions into one script when possible.

## Preconditions

1. `output-validation` or `self-critique` passed on the draft under test (score ≥ 0.75)
2. `[servers.e2b] enabled = true` — otherwise emit `plan.only` explaining opt-in requirement
3. Code is pasted into sandbox — no host file paths

## Test harness template

Emit a single `run_code` with inline assertions and a clear pass marker:

```python
# rmng-test-harness
errors = []

def check(name, cond):
    if not cond:
        errors.append(name)

# --- tests ---
check("sum", sum([1, 2, 3]) == 6)

if errors:
    print({"pass": False, "failed": errors})
else:
    print({"pass": True, "tests": 1})
```

## Wire format

```json
{
  "action": "mcp.proxy",
  "server": "e2b",
  "tool": "run_code",
  "params": { "code": "<harness above>" }
}
```

## Composition

```
memory-management (search) → research → self-critique → run-tests (run_code)
  → validate-output (plan.only) → [regression-check] → memory-management (add lesson)
```

After `run_code`, always follow with `validate-output` before declaring success.

## Anti-overuse

- Do not run tests before research/evaluation completes
- Do not split one logical test suite into multiple sandbox sessions to bypass the 2-run cap
- Do not embed network calls or secrets in harness code
- If both runs fail validation, stop and report — do not loop via `improvement-loop` more than once for tests
