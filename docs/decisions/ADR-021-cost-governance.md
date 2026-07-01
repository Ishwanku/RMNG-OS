# ADR-021: Cost Governance & Operational Control

**Date:** 2026-07-01  
**Status:** **Accepted**  
**Related:** ADR-020 (audit/isolation), Sprint 9 (telemetry), Sprint 11

---

## Context

Sprint 10 delivered tamper-evident audit logs and MCP isolation. Sprint 11 closes the loop from visibility to governance: editable pricing, cost rollups, persistent circuit breakers, and opt-in budget enforcement.

---

## Decision

### 1. Editable catalog pricing

- `input_cost_per_m` / `output_cost_per_m` on `[[providers.*.models]]` in `~/.rmng/llm-catalog.toml`
- `enrich_usage_cost()` prefers catalog → heuristics
- `rmng llm models --pricing` for inspection

### 2. Cost rollups (`rmng observe --cost`)

- `AuditLog::read_all()` + `rollup_llm_costs()` in `rmng-core`
- Aggregates: total, daily, weekly (ISO week), by session, by agent
- `--json` for external dashboards

### 3. Persistent circuit breaker

- State file: `~/.rmng/circuit-state.json` (version 1)
- Survives process restarts; `reload_from_disk()` before observe/health
- Exposed in `rmng observe`, `rmng llm health`, `--cost --json`

### 4. Budget enforcement (opt-in)

```toml
[llm_budget]
daily_usd = 5.0
warn_threshold = 0.8
deny_threshold = 1.0
enforce = "warn"  # off | warn | deny
```

- Spend computed from today's `AuditCategory::llm` entries
- `warn`: log `nervous.budget_warn`, allow calls
- `deny`: log `nervous.budget_deny`, block new LLM calls in connector

### 5. `rmng audit verify`

- Standalone integrity check with exit code 1 on tamper
- `--stats` adds cost rollup; `--json` for CI/cron

---

## Consequences

**Positive:** Operators can cap spend, audit costs forensically, and survive daemon restarts without losing circuit state.

**Negative:** Budget uses estimated costs (not provider billing APIs); per-profile budgets deferred to Sprint 12.

---

## References

- `agents/rmng-core/src/budget.rs` · `cost_rollup.rs`
- `agents/rmng-nervous/src/providers/circuit_breaker.rs` · `cost.rs`
- `agents/rmng-cli/src/observe.rs` · `audit_cmd.rs`