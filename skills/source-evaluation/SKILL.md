---
name: source-evaluation
description: >-
  Evaluate credibility and integration fit of external sources/repos/docs
  before RMNG adoption recommendations.
---

# Source Evaluation

Use when assessing GitHub repos, docs pages, or whitepapers for RMNG integration.

## Scoring rubric (aligns with INTEGRATION-STRATEGY.md)

Rate 1–5 each:

| Dimension | Question |
|-----------|----------|
| Isolation | Subprocess/MCP/skill-only, or in-process runtime? |
| Determinism | Schema-stable I/O? |
| Security | Filesystem/network scope? Known CVEs? |
| Fit | Respects CoreIntent v2 and ADR-010? |

## Track recommendation

| Score pattern | Track |
|---------------|-------|
| Small auditable tool | 1 Native |
| MCP server, sparse use | 2 MCP Proxy |
| Methodology / prompts | 3 Skill |
| Full agent loop / unbounded tools | 4 Rejected |

## Output

Emit `plan.only` with: repo/url, proposed track, scores table, threat notes, rollback plan.

## Red flags (auto Track 4)

- `allowed_tools = ["*"]` or dynamic tool discovery without allowlist
- Unvalidated remote doc registries (e.g. context7-style injection)
- In-process Python agent runtime inside rmngd