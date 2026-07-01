---
name: doc-ingestion
description: >-
  Convert documents to token-efficient Markdown via markitdown MCP.
  Supports http(s), file, and data URIs.
---

# Document Ingestion (Markitdown MCP)

## Tool

- Server: `markitdown`
- Tool: `convert_to_markdown`
- Parameter: `uri` (http:, https:, file:, or data:)

## When to use vs fetch

| Source | Use |
|--------|-----|
| `.pdf`, `.docx`, `.xlsx`, `.pptx` on disk | markitdown with `file://` URI |
| Remote document URL ending in office/pdf extension | markitdown with `https://` URI |
| Plain HTML page | `fetch` (lighter) |

## Security

- Only pass URIs under user-approved paths for `file://` (e.g. `~/Downloads/`, project `docs/`).
- Never exfiltrate `~/.ssh`, `~/.rmng/secrets.env`, or system paths.
- Markitdown runs as subprocess with isolation limits — see allowlist.

## Example intents

```json
{
  "action": "mcp.proxy",
  "server": "markitdown",
  "tool": "convert_to_markdown",
  "params": { "uri": "file:///home/user/Downloads/report.pdf" }
}
```

```json
{
  "action": "mcp.proxy",
  "server": "markitdown",
  "tool": "convert_to_markdown",
  "params": { "uri": "https://example.com/whitepaper.pdf" }
}
```