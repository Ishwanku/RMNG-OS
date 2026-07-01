# Recommended Agent Setups

Short recipes for common RMNG-OS workflows. All agents live in `agents/definitions/`.

## Research & curation (L3)

| Agent | MCP / native | Use when |
|-------|----------------|----------|
| `research-curator` | github, fetch, markitdown, mem0 | Issue triage, doc ingest, memory search |
| `web-researcher` | fetch, markitdown, mem0 (add) | URL fetch + remember findings |

```bash
rmng ask -a research-curator -s research-workflow "list open issues for RMNG-OS"
rmng ask -a web-researcher "fetch https://example.com and summarize"
```

## Repository & code (L3)

| Agent | MCP / native | Use when |
|-------|----------------|----------|
| `repo-keeper` | git, github, e2b (opt-in) | Status, diff, sandboxed verification |
| `browser-researcher` | playwright (opt-in) | DOM navigation when fetch is insufficient |

```bash
rmng ask -a repo-keeper "git status and recent commits"
# E2B opt-in:
rmng ask -a repo-keeper "run sandbox code: assert 2+2==4"
```

## Orchestration (L4)

| Agent | Role |
|-------|------|
| `swarm-coordinator` | Delegates to L3 specialists via handoff |

```bash
rmng handoff --session <id> --chain swarm-coordinator,repo-keeper "verify repo clean"
```

## Opt-in high-risk MCP

Enable in `~/.rmng/mcp-allowlist.toml` only when needed:

| Server | Agent | Isolation |
|--------|-------|-----------|
| `playwright` | `browser-researcher` | `seccomp_profile = "playwright"`, `drop_capabilities = true` |
| `e2b` | `repo-keeper`, `research-curator` | `seccomp_profile = "e2b"`, `drop_capabilities = true` |
| `mem0` | `research-curator`, `web-researcher` | `no_new_privs`, cgroup limits |

See [security-mcp-usage.md](security-mcp-usage.md).

## Observability defaults

```bash
rmng observe              # agents, sessions, audit tail, MCP resources
rmng observe --cost       # LLM spend + MCP resource rollups
rmng observe --json | jq '.resource_rollup,.cost_rollup'
rmng audit verify --stats
```

## Session + handoff pattern

```bash
SID=$(rmng session create | awk '{print $NF}')
rmng ask -a swarm-coordinator --session "$SID" "plan research on integration X"
rmng handoff --session "$SID" --from swarm-coordinator --to research-curator "gather issues"
rmng session show "$SID"   # tool_results + handoff_history
```
