---
name: spec-compliance
description: Align RMNG skills and agent outputs with agentskills.io progressive disclosure and CoreIntent v2.
---

# Spec Compliance (anthropics/skills + agentskills reference)

## Progressive disclosure

1. Load skill **name + description** from index first
2. Activate full skill body only when task matches description
3. Do not merge unrelated skill instructions into one prompt

## CoreIntent v2 output

Always emit a single JSON object:

- `action`: `tool.execute` | `mcp.proxy` | `plan.only`
- `metadata.session_id` when session active
- `metadata.handoff_to` only for validated L4→L3 delegation

No markdown fences. No prose outside JSON for dispatch paths.

## Skill authoring in RMNG

- Path: `skills/<name>/SKILL.md`
- Register in `skills/INDEX.md`
- Reference from `agents/definitions/*.yaml` `skills:` list