# Sandbox Code Execution Usage (Sprint 17)

Opt-in E2B MCP for verifying scripts and running bounded tests — **never** for host shell access.

## Prerequisites

1. E2B account + API key: https://e2b.dev/docs/api-key
2. `[servers.e2b] enabled = true` in `~/.rmng/mcp-allowlist.toml`
3. `E2B_API_KEY` in rmngd environment (systemd `Environment=` or `~/.config/environment.d/`)

## Smoke test

```bash
rmng send -f agents/schemas/mcp-e2b-run-code.intent.json
tail -1 ~/.rmng/logs/audit.jsonl
```

## Agent scope

| Agent | Tool | Use case |
|-------|------|----------|
| `repo-keeper` | `e2b:run_code` | Verify utility scripts, parse/transform checks |
| `research-curator` | `e2b:run_code` | Validate research snippets, data transforms |

## Workflow: research → memory → evaluation → execution

1. **Research** — `web-researcher` fetches URL or `research-curator` lists GitHub issues
2. **Memory** — `search_memories` recalls prior integration decisions
3. **Evaluation** — `output-validation` + `self-critique` on the synthesis (plan.only)
4. **Execution** — `repo-keeper` or `research-curator` emits `run_code` to verify a minimal repro

Example prompt (repo-keeper):

> After validating the summary, run this in the sandbox: `print(2 + 2)` and confirm output is 4.

Mock LLM (no provider) routes prompts containing `sandbox`, `run code`, or `execute code` to `e2b:run_code`.

## Intent wire format

```json
{
  "schema_version": "2",
  "action": "mcp.proxy",
  "server": "e2b",
  "tool": "run_code",
  "params": {
    "code": "print(sum([1, 2, 3]))"
  }
}
```

## Safety rules (see `skills/code-execution/SKILL.md`)

- Opt-in only — disabled by default
- No secrets in `code` parameter
- Prefer short, bounded scripts (under ~30s effective runtime)
- Treat sandbox output as untrusted
- Do not use for kernel builds or host file mutations

## Tests

```bash
cd agents && cargo test -p rmng-nervous --test sandbox_e2e -- --nocapture
cd agents && cargo test -p rmng-core permission::tests::allows_mcp_e2b -- --nocapture
```

## Cost

E2B bills per sandbox session. Agent daily budgets were bumped slightly for sandbox agents (see agent YAML).
