# Operations & Production Hardening (Sprint 19–28)

Observability, circuit breakers, budgets, MCP resource metrics, audit verification, and production deployment for CI/cron.

## Production setup (recommended)

### Install & validate

```bash
cd ~/dev/projects/RMNG-OS
./scripts/install-rmng.sh          # build, config, allowlist, systemd user unit
rmngd --validate                   # pre-flight: config, dirs, agents, audit
rmng health                        # human summary
rmng health --json                 # monitoring / cron (exit 0 = healthy)
```

`rmngd --validate` checks: `~/.rmng` layout, config parse, writable session/socket dirs, integration manifests, LLM config, MCP allowlist, audit chain, agent registry (`RMNG_PROJECT_ROOT`), and `[auto_continue]` settings. **ERROR** items block systemd start via `ExecStartPre`; **WARN** items log but allow start.

### systemd user unit

Installed to `~/.config/systemd/user/rmngd.service` from `config/rmngd.service`.

| Setting | Purpose |
|---------|---------|
| `RMNG_PROJECT_ROOT` | Agent definitions under `agents/definitions/` |
| `EnvironmentFile=-~/.rmng/secrets.env` | API keys (optional, `-` = ignore if missing) |
| `ExecStartPre=rmngd --validate` | Fail fast on ERROR misconfiguration |
| `RUST_LOG=info` | Journal-friendly structured logs |

```bash
systemctl --user status rmngd
systemctl --user restart rmngd
journalctl --user -u rmngd -f       # handoffs, continuation, budgets, circuits
```

Install generates the unit from `config/rmngd.service.in` with your paths:

```bash
RMNG_PROJECT_ROOT=/your/clone ./scripts/install-rmng.sh
```

If validation fails, install **skips** `systemctl restart` to avoid a restart loop — fix ERROR items, then `rmngd --validate && systemctl --user restart rmngd`.

### Secrets & config

```bash
cp config/rmng-config.toml.example ~/.rmng/config.toml
# Keys in ~/.rmng/secrets.env (never commit):
#   GOOGLE_API_KEY=...
#   XAI_API_KEY=...
./scripts/setup-mcp-allowlist.sh
```

### Cron / health probes

```bash
# Production liveness (daemon + no open circuits + no budget deny):
rmng health --json --strict

# Daemon must be running (readiness-only liveness):
rmng health --require-daemon --json

# Lightweight (no LLM network call):
rmng health --quick --json

# Full probe (LLM + readiness + audit):
rmng health --json

# Deep metrics snapshot:
rmng observe --json
```

Exit codes:

| Command | `0` when | `1` when |
|---------|----------|----------|
| `rmng health` | Readiness OK, audit valid, LLM healthy (if probed) | Readiness ERROR, tampered audit, LLM down |
| `rmng health --require-daemon` | Above + rmngd running | rmngd stopped |
| `rmng health --strict` | Above + rmngd running + no open circuits + budget not DENY | Any strict condition fails |
| `rmng llm health` | Provider reachable, no open circuits | LLM down or circuits open |

JSON schema v2 includes a `failures` array explaining unhealthy results.

## Monitoring commands

| Command | Use when |
|---------|----------|
| `rmng status` | Quick local summary |
| `rmng health` | Production readiness + LLM + audit |
| `rmng health --quick` | Fast cron without LLM ping |
| `rmng llm health` | Provider-only probe + circuits |
| `rmng observe` | Budgets, sessions, audit tail |
| `rmng observe --cost` | Spend rollups + MCP resources |
| `rmng observe --json` | Full schema v1 for dashboards |
| `rmng audit verify` | Tamper detection only |

## Troubleshooting

| Symptom | Check | Fix |
|---------|-------|-----|
| `rmngd` won't start (systemd) | `rmngd --validate` | Fix ERROR rows (config, agents, dirs) |
| `agents: registry empty` | `echo $RMNG_PROJECT_ROOT` | Set in unit or shell to repo root |
| `socket bind failed` | `ls ~/.rmng/rmng.sock` | Stop stale rmngd; remove stale socket |
| LLM `unreachable` | `rmng llm show` | Key in `secrets.env`, provider/model in config |
| `circuit breaker open` | `rmng observe --cost` | Wait cooldown or fix provider; check `circuit-state.json` |
| Budget deny | `rmng observe --json \| jq .budgets` | Raise cap or `enforce = "warn"` |
| Continuation stuck | `rmng observe` awaiting count | Check `auto_continue.timeout_secs`; inspect session orchestration |
| Audit tamper | `rmng audit verify` | Investigate `~/.rmng/audit.log` — do not delete without review |

### Log events (journalctl)

Structured `tracing` output from rmngd includes:

- `check=…` — startup readiness (ok/warn/error)
- `agent handoff` — `nervous.handoff` / `nervous.handoff_return`
- `daemon auto-continue` — lock, steps, timeout, exhausted
- `circuit breaker` — open / half_open / closed
- `budget` — warn or deny before LLM calls

Set `RUST_LOG=rmngd=debug,rmng_nervous=debug` for verbose nervous routing.

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

## Daemon auto-continue (Sprint 26)

rmngd continues multi-hop chains without repeated `rmng ask` when session orchestration state has `awaiting_continuation` or `continuation.enabled`.

```bash
# Explicit continue (blocks until loop finishes)
echo '{"action":"orchestration.continue","session_id":"<sid>"}' | nc -U ~/.rmng/rmng.sock

# After tool dispatch with --session, continuation runs in background automatically
rmng ask --session <sid> --agent swarm-coordinator "run git status"
```

Sprint 27: one continuation loop per session (concurrent triggers are skipped); timeouts finalize session state. Tune in `~/.rmng/config.toml` under `[auto_continue]` — see [orchestration-usage.md](./orchestration-usage.md).
