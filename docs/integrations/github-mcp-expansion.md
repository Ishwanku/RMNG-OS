# GitHub MCP Expansion (Sprint 4)

**Track:** 2 — MCP Proxy  
**Layer:** L3 (`research-curator`)  
**Date:** 2026-07-01

## Evaluation (INTEGRATION-STRATEGY.md)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Execution isolation | 5 | Subprocess via rmng-mcp; no in-process code |
| Structural determinism | 4 | JSON-RPC; explicit allowlist |
| Zero-trust | 4 | `search_issues` read-only; scoped queries |
| Architectural fit | 5 | ADR-010 compliant |

**Average:** 4.5 — **Approved**

## Integrated tools

| MCP tool | Server | Agent |
|----------|--------|-------|
| `search_issues` | `github` | `research-curator` |
| `get_issue` | `github` | `research-curator` (existing) |

## Registration

```bash
# Already in ~/.rmng/mcp-allowlist.toml after Sprint 4
systemctl --user restart rmngd
```

## Verification

```bash
rmng observe   # github server lists search_issues
```
