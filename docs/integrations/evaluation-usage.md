# Evaluation & Self-Improvement — Usage Guide (Sprint 16)

Track 3 skills only — no extra MCP servers. Grading uses `plan.only` intents.

## Skills

| Skill | Role |
|-------|------|
| `self-critique` | LLM-as-judge rubric (one pass per request) |
| `output-validation` | Deterministic checks + threshold 0.8 |
| `improvement-loop` | Bounded recall → produce → critique → validate |

## Example: research with memory

```bash
rmng session new
# 1. Recall prior work
rmng ask --agent research-curator --session <id> \
  "search memory for RMNG integration decisions"

# 2. Research + synthesize (github MCP, plan.only draft)
rmng ask --agent research-curator --session <id> \
  "list open issues and summarize integration gaps"

# 3. Self-improve (skills guide plan.only critique — no extra tools)
rmng ask --agent research-curator --session <id> \
  "run output-validation on your last synthesis; revise if below threshold"

# 4. Persist lesson after pass
rmng ask --agent research-curator --session <id> \
  "remember the key integration lesson from this session"
```

## Example: web research loop

```bash
rmng ask --agent web-researcher --session <id> \
  "fetch https://example.com and summarize"

rmng ask --agent web-researcher --session <id> \
  "self-critique the summary for grounding and source citations"
```

## Anti-patterns (overuse)

- Do not run critique+validation more than **twice** per user request
- Do not call Mem0 add before validation passes
- Do not import promptfoo CLI inside rmngd

## Agents with evaluation skills

`research-curator`, `web-researcher`, `repo-keeper`, `browser-researcher`

## Reference

[promptfoo-patterns.md](promptfoo-patterns.md) · [skills/self-critique/SKILL.md](../../skills/self-critique/SKILL.md)