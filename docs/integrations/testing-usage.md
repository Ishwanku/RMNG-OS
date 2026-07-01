# Testing & Validation Workflows (Sprint 18)

Composable Track 3 skills that chain research, memory, evaluation, and E2B sandbox execution into bounded test loops.

## Skills

| Skill | Role | Executes tools? |
|-------|------|----------------|
| `run-tests` | Emit `e2b:run_code` test harness (max 2/request) | yes (sandbox) |
| `validate-output` | Grade sandbox stdout/logs (plan.only) | no |
| `test-coverage-check` | Breadth rubric before/after tests (plan.only) | no |
| `regression-check` | Compare vs session/Mem0 baseline (plan.only) | optional search |

Existing skills used in the same loop: `memory-management`, `self-critique`, `output-validation`, `improvement-loop`, `code-execution`.

## Primary workflow: research → code → test → validate

```bash
rmng session new
SID=<session-id>

# 1. Recall context
rmng ask --agent research-curator --session $SID \
  "search memory for prior integration test lessons"

# 2. Research
rmng ask --agent research-curator --session $SID \
  "list open issues about sandbox testing"

# 3. Critique draft (plan.only)
rmng ask --agent research-curator --session $SID \
  "self-critique the synthesis before testing"

# 4. Coverage rubric (plan.only)
rmng ask --agent research-curator --session $SID \
  "test-coverage-check for the snippet we will verify"

# 5. Run tests (e2b — requires opt-in)
rmng ask --agent research-curator --session $SID \
  "run tests in sandbox for the algorithm under review"

# 6. Validate execution output (plan.only)
rmng ask --agent research-curator --session $SID \
  "validate-output on the last sandbox run"

# 7. Regression vs session history (plan.only)
rmng ask --agent research-curator --session $SID \
  "regression check against prior test results"

# 8. Deliverable gate + persist lesson
rmng ask --agent research-curator --session $SID \
  "output-validation then remember the test lesson if pass"
```

## Repo-keeper variant

```bash
rmng ask --agent repo-keeper --session $SID \
  "run tests in sandbox after reviewing git diff summary"

rmng ask --agent repo-keeper --session $SID \
  "validate-output on sandbox results"
```

## Anti-overuse guards (enforced by skills)

| Limit | Value |
|-------|-------|
| `run_code` per request | 2 |
| `validate-output` per request | 2 |
| `test-coverage-check` per request | 1 |
| `regression-check` per request | 1 |
| Full test retry cycles | 2 (via improvement-loop) |

## Agents with testing skills

`repo-keeper`, `research-curator` only (opt-in per agent YAML).

## Prerequisites

- E2B enabled: see [sandbox-usage.md](sandbox-usage.md)
- Evaluation skills: see [evaluation-usage.md](evaluation-usage.md)

## Tests

```bash
cd agents && cargo test -p rmng-nervous --test testing_workflow_e2e -- --nocapture
cd agents && cargo test -p rmng-nervous agent::tests::l3_testing_agents -- --nocapture
```
