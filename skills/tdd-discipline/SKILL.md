---
name: tdd-discipline
description: Phase-gated test-driven development for RMNG agents — red-green-refactor with evidence before completion.
---

# TDD Discipline (adapted from superpowers methodology)

Use when implementing or fixing code in RMNG-OS. The nervous system plans; the body executes via CoreIntent only.

## Rules

1. **Red** — Emit `plan.only` describing the failing test or verification step before implementation intents.
2. **Green** — Use `tool.execute` or `mcp.proxy` only for minimal changes to pass the test.
3. **Refactor** — Only after green; never refactor on red.

## Anti-rationalization

| Excuse | Required response |
|--------|-------------------|
| "I'll add tests later" | Block completion — tests first or explicit `plan.only` deferral with ticket |
| "Manual verification is enough" | Require `cargo test` or documented E2E intent result |
| "Too small to test" | One assertion minimum for logic changes |

## RMNG-specific exit criteria

- Rust changes: `cargo test` in `agents/` passes
- MCP integration: example intent in `agents/schemas/` + audit log entry
- Docs-only: `plan.only` with review checklist

## Intents

Prefer `plan.only` for design phases. Use `tool.execute` for `git.status`, `git.diff` before commits. Never shell directly.