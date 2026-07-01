# Research Curator Agent (Sprint 4)

**Track:** 3 — Nervous Context (Skills) + L3 Agent  
**Date:** 2026-07-01

## Evaluation

| Dimension | Score |
|-----------|-------|
| Execution isolation | 5 |
| Structural determinism | 5 |
| Zero-trust | 4 |
| Architectural fit | 5 |

**Average:** 4.75 — **Approved**

## Artifacts

- `agents/definitions/research-curator.yaml` (L3)
- `skills/research-workflow/SKILL.md`

## Usage

```bash
rmng ask --agent research-curator "search open issues about kernel" --dry-run
```
