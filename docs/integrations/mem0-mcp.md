# Integration Intake: Mem0 MCP

| Field | Value |
|-------|-------|
| **Repository** | https://github.com/mem0ai/mem0 |
| **Package** | mem0-mcp-server (uvx) |
| **License** | Apache-2.0 |
| **Date** | 2026-07-02 |
| **Track** | 2 MCP Proxy + 3 Skill |
| **Status** | Active (opt-in) |

## Summary

Long-term semantic memory. RMNG uses stdio subprocess (uvx mem0-mcp-server), not cloud HTTP MCP, because rmng-mcp supports subprocess JSON-RPC only.

## Allowed tools

add_memory, search_memories, get_memory, delete_memory

## Register

export MEM0_API_KEY and MEM0_DEFAULT_USER_ID=rmng-os, then register mem0 server and enable in allowlist.

## Decision

Accepted opt-in Track 2. Cloud HTTP deferred.
