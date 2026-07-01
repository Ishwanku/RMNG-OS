# Browser Research Usage (Sprint 14)

Playwright MCP is opt-in and disabled by default.

## Enable

```bash
cp config/mcp-allowlist.toml.example ~/.rmng/mcp-allowlist.toml
./scripts/register-mcp-tool.sh playwright npx -y @playwright/mcp@latest \
  --tools browser_navigate,browser_snapshot,browser_click
# Set enabled = true under [servers.playwright]
systemctl --user restart rmngd
```

## Navigate

```bash
rmng send -f agents/schemas/mcp-playwright-navigate.intent.json
rmng ask --agent browser-researcher --session <id> "navigate to https://example.com"
```

## Tests

```bash
cd agents && cargo test -p rmng-nervous --test playwright_e2e
```