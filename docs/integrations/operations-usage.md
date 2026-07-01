# Operations & Production Hardening (Sprint 19)

Observability, circuit breakers, budgets, and audit verification for CI/cron.

## Circuit breaker (persistent)

State file: `~/.rmng/circuit-state.json` — survives `rmngd` restarts and syncs across processes via mtime reload.

```bash
rmng observe --cost          # shows open circuits
rmng llm health              # circuits + budget summary
rmng llm health --json       # monitoring-friendly JSON
```

## Cost & budget observability

```bash
rmng observe --cost          # per-agent today + all-time rollups
rmng observe --json          # full JSON: rollups, budgets, circuits, rmngd status
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

## Audit verify (CI/cron)

```bash
rmng audit verify                    # exit 0=valid, 1=tampered, 2=error
rmng audit verify --stats            # + category stats + cost rollup
rmng audit verify --json --stats     # CI JSON with exit_code field
```

## JSON monitoring examples

```bash
rmng observe --json | jq '.budgets.agents'
rmng llm health --json | jq '.circuit_breakers'
rmng audit verify --json --stats | jq '.valid, .stats.spent_today_usd'
```

## References

- [docs/llm-configuration.md](../llm-configuration.md)
- [ADR-021](../decisions/ADR-021-cost-governance.md)
