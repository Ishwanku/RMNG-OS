# Memory & Long-term Context — Usage Guide (Sprint 15)

## Prerequisites

```bash
export MEM0_API_KEY="m0-..."          # https://app.mem0.ai/settings/api-keys
export MEM0_DEFAULT_USER_ID="rmng-os"

cp config/mcp-allowlist.toml.example ~/.rmng/mcp-allowlist.toml
./scripts/register-mcp-tool.sh mem0 uvx mem0-mcp-server \
  --tools add_memory,search_memories,get_memory,delete_memory

# Set enabled = true under [servers.mem0]
# Ensure rmngd inherits MEM0_API_KEY (systemd EnvironmentFile or login env)
systemctl --user restart rmngd
```

## Search prior context

```bash
rmng session new
rmng send -f agents/schemas/mcp-mem0-search.intent.json
rmng ask --agent research-curator --session <id> "search memory for RMNG sprint decisions"
```

## Add a distilled fact

```bash
rmng send -f agents/schemas/mcp-mem0-add.intent.json
```

## Session write-back

Mem0 results persist to `shared_context.tool_results` (e.g. `mem0.search_memories`). Follow-up asks include hits in `prompt_context`.

## Agent scopes

| Agent | Mem0 tools | Daily budget |
|-------|------------|--------------|
| research-curator | add, search, get, delete | $1.50 |
| web-researcher | add, search, get, delete | $2.00 |
| repo-keeper | search, get (read-only) | $1.00 |

## Tests

```bash
cd agents && cargo test -p rmng-nervous --test mem0_e2e
```

## Skill

See [skills/memory-management/SKILL.md](../../skills/memory-management/SKILL.md).