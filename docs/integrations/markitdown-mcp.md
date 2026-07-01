# Integration Intake: Markitdown MCP

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/microsoft/markitdown (package: `markitdown-mcp`) |
| **License** | MIT |
| **Date** | 2026-07-01 |
| **Proposed track** | 2 — MCP Proxy |
| **Status** | **Active** |

## Summary

Converts PDF, DOCX, XLSX, HTML, and other formats to Markdown via `convert_to_markdown(uri)`. Reduces token load vs raw binary paste. Complements `fetch` for static HTML pages.

## Evaluation scores (1–5)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Execution plane isolation | 4 | Subprocess via uvx; file:// URI risk |
| Structural determinism | 5 | Single tool, uri parameter |
| Zero-trust security | 3 | file:// path traversal — agent skill restricts paths |
| Architectural fit | 5 | mcp.proxy only |
| **Average** | **4.25** | |

## Threat model

- **file:// URIs** — can read user-accessible files; doc-ingestion skill limits to Downloads/docs
- **https://** — same untrusted content risk as fetch
- Known industry reports on markitdown MCP path validation — treat as high-trust-user-input only

## Implementation

### Track 2
- [x] `config/mcp-allowlist.toml.example` with isolation
- [x] `agents/schemas/mcp-markitdown.intent.json`
- [x] `skills/doc-ingestion/SKILL.md`
- [x] `web-researcher` agent allowlist entry
- [ ] User: `./scripts/register-mcp-tool.sh markitdown uvx markitdown-mcp --tools convert_to_markdown`

## Usage

```bash
rmng send -f agents/schemas/mcp-markitdown.intent.json
rmng ask --agent web-researcher --session <id> "summarize the prior fetch results"
```

## Rollback

`enabled = false` on `[servers.markitdown]`.

## Decision

- [x] Accepted — Track 2 with isolation + skill guardrails