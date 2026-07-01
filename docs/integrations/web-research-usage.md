# Web & Document Research — Usage Guide (Sprint 13)

## Prerequisites

```bash
cp config/mcp-allowlist.toml.example ~/.rmng/mcp-allowlist.toml
# Edit paths (git repo, etc.)

./scripts/register-mcp-tool.sh fetch npx -y @modelcontextprotocol/server-fetch --tools fetch
./scripts/register-mcp-tool.sh markitdown uvx markitdown-mcp --tools convert_to_markdown

systemctl --user restart rmngd
rmng status
```

## Fetch a URL

```bash
rmng session new   # note session id
rmng send -f agents/schemas/mcp-fetch.intent.json

# Or via agent (mock LLM or live):
rmng ask --agent web-researcher --session <id> "fetch https://example.com" 
```

Session write-back stores result under `shared_context.tool_results` as `fetch.fetch`.

## Convert a document

```bash
rmng send -f agents/schemas/mcp-markitdown.intent.json
```

For local files, use `file://` URIs only under approved directories (see `skills/doc-ingestion/SKILL.md`).

## Summarize in session

```bash
rmng ask --agent web-researcher --session <id> "summarize the previous fetch results"
```

The agent reads prior tool output from session context (no re-fetch).

## Observe & audit

```bash
rmng observe --cost
tail ~/.rmng/logs/audit.jsonl | grep mcp.proxy
```

## Agent budget

`web-researcher` has `daily_budget_usd: 2.0` — pair with `[llm_budget]` in config for enforcement.

## Tests

```bash
cd agents && cargo test -p rmng-nervous --test fetch_e2e -- --nocapture
```

Live markitdown loop: `cargo test -p rmng-nervous --test fetch_e2e markitdown_mcp_full_loop`
