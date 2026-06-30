# ADR-015: Intent schema decoupling — poly-intent core envelope

**Date:** 2026-06-30  
**Status:** **Accepted**  
**Related:** ADR-010 (nervous/body separation), ADR-014 (native-first MCP), Phase 6b (`rmng-mcp`)

---

## Context

Phase 5 established `rmngd` as the sole execution authority with a v1 intent envelope (`schema_version: "1"`, `kind` + optional `tool`). Phase 6b will add an MCP client bridge, which introduces a second execution class: proxied external tools that must never be confused with native Rust tools.

The nervous system (BYO-LLM) is adversarial by nature: models hallucinate parameters, invent fields, and blur planning with execution. Without a strict, versioned data contract at the IPC boundary, prompt injection or malformed JSON could reach the Body with ambiguous semantics.

## Decision

Adopt a **v2 poly-intent core envelope** defined by `agents/schemas/core-intent.schema.json` and modeled in Rust as an **internally tagged enum** (`CoreIntent`) with `#[serde(tag = "action")]`.

Three structural modes, mutually exclusive via JSON Schema `oneOf`:

| `action` | Purpose | Execution |
|----------|---------|-----------|
| `tool.execute` | Native RMNG tool via `integrations/` | `rmngd` dispatch |
| `mcp.proxy` | Allowlisted MCP server tool | `rmng-mcp` bridge (Phase 6b) |
| `plan.only` | Abstract reasoning transition | None |

All variants:

- Set `additionalProperties: false` at the root and on `metadata`
- Use flat `parameters` / `mcp_args` objects (no nested object values) to limit structural hallucination
- Support optional `metadata` (`trace_id`, `skill_name`)

v1 `Intent` remains supported for backward compatibility until dispatch and CLI migrate to v2.

## Why internally tagged enum (not untagged structural matching)

### Internally tagged (`"action": "tool.execute"`)

**Chosen.**

1. **Explicit discriminator** — The LLM and validator see one field that selects the mode. No guessing from field presence alone.
2. **Serde alignment** — `#[serde(tag = "action")]` maps 1:1 to JSON Schema `oneOf` + `const` on `action`, reducing drift between schema and Rust.
3. **IPC debuggability** — Audit logs and `rmng send` payloads are human-readable; action is always the first semantic field after parsing.
4. **PermissionGate routing** — The gate can branch on `action` before inspecting nested fields, failing closed on unknown actions at deserialization time.
5. **Forward compatibility** — New actions are additive enum variants with new `const` schema branches; old clients reject unknown actions safely.

### Untagged (infer mode from which fields are present)

**Rejected.**

- Ambiguous payloads (e.g. both `target` and `mcp_server` hallucinated) create undefined behavior
- Harder to audit and harder for LLMs to produce reliably
- Serde untagged matching order becomes implicit protocol logic — fragile and opaque

### Externally tagged (`{"tool.execute": {...}}`)

**Rejected.**

- Verbose on the wire; worse for LLM token efficiency
- Inconsistent with MCP and OpenAI tool-call conventions that use flat action fields

## How this protects the Body

```text
Nervous System (LLM)
        │
        ▼ JSON text
┌───────────────────┐
│ serde deserialize │  deny_unknown_fields → reject extra keys
│ CoreIntent enum   │  tag = action       → reject unknown modes
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│ PermissionGate    │  tool.execute → native allowlist
│                   │  mcp.proxy    → MCP allowlist (6b)
│                   │  plan.only    → allow, no dispatch
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│ rmngd dispatch    │  only approved paths execute
└───────────────────┘
```

1. **Structural rejection before logic** — Unknown fields and actions fail at parse time, not inside tool handlers.
2. **Mode separation at the data layer** — Native tools use `target` + `parameters`; MCP uses `mcp_server` + `mcp_tool` + `mcp_args`. A proxy request cannot masquerade as a native tool without changing `action`.
3. **No shell in schema** — Neither variant includes command strings; execution maps only through registered tool implementations.
4. **Audit correlation** — `metadata.trace_id` ties nervous-system output to Body audit entries without trusting LLM prose in `reasoning`.
5. **Plan-only is non-executable** — `plan.only` carries `reasoning` only; dispatch explicitly no-ops.

## Consequences

- ✅ `core-intent.schema.json` is the single source of truth for v2 validation
- ✅ Rust `CoreIntent` enum prevents unknown-field drift at compile/deserialize time
- ✅ Phase 6b MCP bridge plugs into `mcp.proxy` without conflating native tools
- ⚠️ v1 intents remain until CLI/dispatch migration (planned Phase 6b/c)
- ⚠️ Per-tool parameter tightening will layer via `integrations/*.json` sub-schemas

## References

- Schema: `agents/schemas/core-intent.schema.json`
- Types: `agents/rmng-core/src/intent.rs`
- Legacy schema: `agents/schemas/intent.schema.json` (v1)