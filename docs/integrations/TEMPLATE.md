# Integration Intake: `<REPO_NAME>`

| Field | Value |
|-------|-------|
| **Repository** | `<url>` |
| **License** | |
| **Date** | YYYY-MM-DD |
| **Proposed track** | 1 Native / 2 MCP / 3 Skill / 4 Rejected |
| **Status** | Evaluating |

## Summary

One paragraph: what the project does and why RMNG-OS might need it.

## Evaluation scores (1–5)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Execution plane isolation | | |
| Structural determinism | | |
| Zero-trust security | | |
| Architectural fit (ADR-010) | | |
| **Average** | | |

## Threat model

- Prompt injection surface:
- Filesystem access:
- Network egress:
- Credential handling:

## Implementation plan (if accepted)

### Track 1 — Native Core
- [ ] `integrations/<domain>/<tool>.json`
- [ ] Rust handler in `rmng-core`
- [ ] `PermissionGate` allow entry
- [ ] Intent schema + test

### Track 2 — MCP Proxy
- [ ] `./scripts/register-mcp-tool.sh …`
- [ ] Example intent in `agents/schemas/`
- [ ] `systemctl --user restart rmngd`
- [ ] Audit log verification

### Track 3 — Skill
- [ ] `skills/<name>/SKILL.md`
- [ ] Entry in `skills/INDEX.md`

## Rollback

How to disable/remove without breaking rmngd.

## Decision

- [ ] Accepted
- [ ] Deferred — reason:
- [ ] Rejected — reason: