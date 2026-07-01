# Operations & Production Hardening (Sprint 19–22)

Observability, circuit breakers, budgets, MCP resource metrics, and audit verification for CI/cron.

## Circuit breaker (persistent)

State file: `~/.rmng/circuit-state.json` — survives `rmngd` restarts and syncs across processes via mtime reload.

```bash
rmng observe --cost          # shows open circuits + MCP resources
rmng llm health              # circuits + budget summary
rmng llm health --json       # monitoring-friendly JSON
```

## Cost & budget observability

```bash
rmng observe --cost          # per-agent today + all-time rollups + MCP resources
rmng observe --json          # full JSON: cost rollups, resource rollups, budgets, circuits
```

### Per-profile budgets

Set in `~/.rmng/config.toml`:

```toml
[[profiles]]
name = "gemini-fast"
daily_budget_usd = 3.0

[llm_budget]
daily_usd = 5.0
enforce = "warn"
```

Profile spend is tracked via `llm_profile` on audit LLM entries.

### Per-agent budgets

Set `daily_budget_usd` in `agents/definitions/*.yaml`. Shown in `rmng observe --cost` agent section.

## MCP resource metrics (Sprint 20)

Each isolated MCP subprocess records lightweight usage after the call:

| Field | Source | Audit field |
|-------|--------|-------------|
| Peak RSS | `wait4` ru_maxrss (Linux/WSL) | `mcp_peak_rss_kb` |
| CPU time | user + system ms | `mcp_cpu_time_ms` |
| Runtime | wall clock | `duration_ms` |

Metrics appear in:

- **Audit log** — optional fields on `category=mcp` entries (backward compatible)
- **Session** — `tool_results[].peak_rss_kb`, `cpu_time_ms`, `runtime_ms`
- **`rmng observe`** — top consumers by agent, recent high-resource calls
- **`rmng observe --json`** — `resource_rollup` alongside `cost_rollup`

```bash
rmng observe --json | jq '.resource_rollup.top_consumers'
rmng observe --json | jq '.resource_rollup.recent_high_resource[:3]'
```

Example JSON fragment:

```json
{
  "resource_rollup": {
    "total_mcp_calls": 12,
    "peak_rss_kb_max": 8192,
    "cpu_time_ms_total": 450,
    "top_consumers": [
      { "id": "repo-keeper", "peak_rss_kb_max": 8192, "cpu_time_ms_total": 200, "mcp_calls": 5 }
    ],
    "recent_high_resource": [
      {
        "action": "mcp.proxy:github.get_issue",
        "agent_id": "repo-keeper",
        "peak_rss_kb": 8192,
        "cpu_time_ms": 120,
        "runtime_ms": 340
      }
    ]
  }
}
```

Collection is observability-only (no enforcement). On non-Unix hosts, runtime is still recorded; RSS/CPU may be absent.

## Audit verify (CI/cron)

```bash
rmng audit verify                    # exit 0=valid, 1=tampered, 2=error
rmng audit verify --stats            # + category stats + cost + resource rollups
rmng audit verify --json --stats     # CI JSON with exit_code field
```

## JSON monitoring examples

```bash
rmng observe --json | jq '.budgets.agents'
rmng observe --json | jq '.resource_rollup.by_agent_ranked'
rmng llm health --json | jq '.circuit_breakers'
rmng audit verify --json --stats | jq '.valid, .stats.spent_today_usd, .resource_rollup.peak_rss_kb_max'
```

## MCP security hardening (Sprint 21)

Seccomp profiles and capability dropping for high-risk MCP servers (`playwright`, `e2b`). Configurable per server in `mcp-allowlist.toml` — **not** enabled globally by default.

```bash
rmng observe   # global isolation defaults incl. seccomp_profile, drop_capabilities
```

See [security-mcp-usage.md](security-mcp-usage.md) for risk levels and trade-offs.

## Consolidation (Sprint 22)

- [end-to-end-workflow.md](end-to-end-workflow.md) — research → memory → eval → execution → test
- [recommended-agent-setups.md](recommended-agent-setups.md) — agent recipes by task
- `rmng observe --json` — schema v1 with `generated_at`, `cost_rollup`, `resource_rollup`

```bash
rmng observe --json | jq '.schema_version, .generated_at, .resource_rollup.total_mcp_calls'
```

## References

- [docs/llm-configuration.md](../llm-configuration.md)
- [ADR-021](../decisions/ADR-021-cost-governance.md)
