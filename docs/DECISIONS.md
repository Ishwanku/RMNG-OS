# Architecture Decision Records (ADR)

Key decisions for RMNG-OS. Format: context → decision → consequences.

---

## ADR-001: WSL2 as primary development target

**Date:** 2026-06-27  
**Status:** Accepted

### Context
Need a Linux environment on Windows hardware without dual-boot complexity.

### Decision
Use **WSL2 + Ubuntu 24.04** as the primary kernel development platform.

### Consequences
- ✅ Fast iteration, VS Code integration, SSD-backed VHD
- ✅ Access to Windows tools alongside Linux
- ⚠️ Running custom kernels requires extra WSL-specific steps
- ⚠️ Not identical to bare-metal Linux

---

## ADR-002: Out-of-tree kernel builds

**Date:** 2026-06-27  
**Status:** Accepted

### Context
In-tree builds pollute the source tree and complicate git status.

### Decision
All builds use `make O=~/build/kernel` (separate KBUILD directory).

### Consequences
- ✅ Pristine `~/dev/kernel/linux` source
- ✅ Easy to wipe build without touching source
- ⚠️ Must always pass `O=` to make commands

---

## ADR-003: Kernel source not in RMNG-OS repo

**Date:** 2026-06-27  
**Status:** Accepted

### Context
Linux kernel is ~2+ GB, GPLv2, and maintained upstream.

### Decision
RMNG-OS repo contains **tooling only**. Kernel cloned separately.

### Consequences
- ✅ Small, fast-cloning repo
- ✅ No license mixing issues
- ⚠️ Setup requires two clone steps

---

## ADR-004: ccache for compile acceleration

**Date:** 2026-06-27  
**Status:** Accepted

### Context
Kernel rebuilds take 60–90 minutes without caching.

### Decision
Wrap gcc/g++ with ccache; 10 GB cache limit in `~/.bashrc`.

### Consequences
- ✅ Incremental rebuilds dramatically faster
- ⚠️ First build still full duration

---

## ADR-005: Slim config via localmodconfig

**Date:** 2026-06-30  
**Status:** Accepted

### Context
Full WSL config produces ~14 GB builds with hundreds of unused modules.

### Decision
Ship both full and slim configs; use `localmodconfig` for daily builds.

### Consequences
- ✅ ~5,498 line config, ~19 modules
- ✅ Faster builds, less disk
- ⚠️ May miss modules needed for hardware testing

---

## ADR-006: Kernel-first, agents-later

**Date:** 2026-06-30  
**Status:** Accepted

### Context
User vision is AI Agent-first OS across all workflow domains.

### Decision
Complete **Layer 1 kernel foundation** before implementing agent runtime.

### Consequences
- ✅ Stable base for future AI integrations
- ✅ Documented requirements before coding agents
- ⚠️ Agent features delayed until Phase 5

---

## ADR-007: MIT license for RMNG-OS tooling

**Date:** 2026-06-27  
**Status:** Accepted

### Context
Need open license for scripts, docs, and future agent tooling.

### Decision
MIT license for this repository. Kernel remains GPLv2 separately.

### Consequences
- ✅ Permissive reuse of tooling
- ⚠️ Kernel patches may need GPLv2 compliance when distributed

---

## ADR-008: Placeholder scaffold for agents/integrations

**Date:** 2026-06-30  
**Status:** Accepted

### Context
Need to communicate future structure without premature implementation.

### Decision
Add `agents/` and `integrations/` with README placeholders only.

### Consequences
- ✅ Clear repo structure for collaborators and AI assistants
- ✅ Requirements drive implementation later

---

## ADR-009: Agent runtime language — Rust

**Date:** 2026-06-30  
**Status:** **Locked**

### Context
Phase 5 requires a native agent runtime integrated with the OS: tool execution, permission gates, memory stores, IPC boundaries, and multi-agent orchestration. The runtime must coexist with kernel-level workflows and long-running daemons without unpredictable latency.

### Decision
Implement the agent runtime in **Rust**.

### Justification
- **Performance:** Native code with zero-cost abstractions; suitable for hot-path tool dispatch and IPC without interpreter overhead.
- **Memory safety:** Ownership model prevents use-after-free and data races in concurrent agent orchestration without a GC.
- **No GC pauses:** Unlike Python or Node, Rust has no garbage collector — critical for a system daemon that must not stall tool execution or permission checks.
- **Low-level OS integration:** FFI to libc, systemd, eBPF hooks, and future bare-metal components without leaving the systems language domain.
- **Strong typing + schema validation:** `serde` + JSON Schema align with the strict intent boundary (ADR-010).

### Consequences
- ✅ Runtime suitable for `rmngd` system daemon and permission gate
- ✅ Single binary deployment per component
- ⚠️ Steeper initial development curve
- ⚠️ LLM bridge may use thin HTTP client crates; reasoning stays outside runtime control flow

---

## ADR-010: Hybrid LLM architecture — Nervous System / Body separation

**Date:** 2026-06-30  
**Status:** **Locked**

### Context
RMNG-OS is AI Agent-first but must not surrender OS sovereignty to external providers. External LLMs are powerful reasoning engines but must not execute commands, hold authoritative state, or bypass permission policy.

### Decision
Adopt a **hybrid, local-first LLM architecture** with strict biological separation:

| Role | Metaphor | Responsibility | Runs |
|------|----------|----------------|------|
| **Reasoning layer** | Nervous System | Planning, intent generation, language | Local (Ollama default) or external API (pluggable) |
| **Execution layer** | Body | Tool execution, syscalls, shell (gated) | Local Rust runtime only |
| **State & policy** | Heart + Brains | Memory, permissions, sandbox, orchestration | Local Rust processes only |

**Default:** local models via Ollama (or equivalent). External APIs (OpenAI, Anthropic, etc.) are optional **reasoning backends only**.

### IPC / schema boundary (mandatory)

1. LLM outputs **structured intents only** — strict JSON payloads validated against versioned schemas.
2. The **Rust runtime** is the **sole authority** that:
   - Parses intents
   - Verifies permissions
   - Translates approved intents into system calls or allowlisted commands
3. **Prohibited:** LLM direct terminal access, raw shell execution, state mutation, or permission changes.
4. **Swappable reasoning:** Replacing Ollama with an external API changes only the nervous-system adapter; Body/Heart/Brains components are unchanged.

### Justification
- Preserves OS sovereignty and auditability
- Enables offline-first operation
- Prevents prompt-injection from becoming arbitrary code execution
- Cloud models usable for hard reasoning without trusting them with the machine

### Consequences
- ✅ `integrations/` enforces rigid JSON contract at the boundary
- ✅ Permission model is non-bypassable by LLM layer
- ⚠️ Requires schema versioning and intent validation crate in Rust
- ⚠️ External API keys stored in local secure store only

---

## ADR-011: Primary interface — CLI-first

**Date:** 2026-06-30  
**Status:** **Locked**

### Context
RMNG-OS must be operable by developers and agents alike. The interface layer precedes any graphical shell.

### Decision
**CLI-first** interface (`rmng` command) as the primary user and automation surface.

### Justification
- **Unix philosophy:** Compose small tools; pipe agent output to standard utilities.
- **Agent parity:** Background `rmngd` daemon and CLI share the same IPC — agents and humans use identical pathways.
- **Scriptability:** Kernel lab workflows are already terminal-native; CLI extends naturally.
- **Lower complexity:** Defers TUI/web until Layer 2–4 stabilize.

### Consequences
- ✅ Phase 5 delivers `rmng` + `rmngd` before any GUI
- ✅ All integrations expose CLI-invokable tools
- ⚠️ Web dashboard deferred to Phase 7+

---

## ADR-012: Bare-metal boot timeline — Phase 4

**Date:** 2026-06-30  
**Status:** **Locked**

### Context
RMNG-OS currently targets WSL2. Bare-metal boot forces early consideration of real hardware: firmware, drivers, init, and agent daemon startup order.

### Decision
Schedule **bare-metal boot capability in Phase 4** (Advanced Kernel), before full agent orchestration (Phase 7) but after RMNG kernel identity (Phase 3).

### Justification
- Validates kernel configs against real hardware constraints
- Surfaces driver and firmware issues before agent layer depends on them
- Provides non-WSL deployment path for production RMNG-OS

### Consequences
- ✅ Phase 4 explicitly includes bootable kernel + initramfs work
- ⚠️ WSL remains primary dev target through Phase 3
- ⚠️ May require separate hardware test machine or VM with PCI passthrough

---

## Pending decisions

| ADR | Topic | Options |
|-----|-------|---------|
| ADR-013 | Monorepo vs split repos for agents | Monorepo · split |
## ADR-014: Agents, skills, and MCP — native-first, BYO-LLM

**Date:** 2026-06-30  
**Status:** **Accepted**

### Context
Top GitHub OSS ecosystems (MCP, agent skills, orchestration frameworks) offer thousands of tools. RMNG-OS must adopt selectively without violating ADR-010 nervous/body separation or low-overhead philosophy.

### Decision
1. **Native Rust tools** remain the production execution path (`integrations/` + `PermissionGate`).
2. **Skills** ship in-repo under `skills/` (Agent Skills format) — nervous context only.
3. **MCP** is dev-time IDE assist via `~/.config/rmng/mcp-dev.json`; optional gated `rmng-mcp` bridge in Phase 6b via `~/.rmng/mcp-allowlist.toml`.
4. **Agent definitions** in `agents/` are RMNG specialists only.
5. Discovery indexes (awesome-mcp-servers) are referenced, never vendored.

### Consequences
- ✅ Plan: [PLAN-AGENTS-MCP-SKILLS.md](PLAN-AGENTS-MCP-SKILLS.md) · [INTEGRATION-STRATEGY.md](INTEGRATION-STRATEGY.md)
- ⚠️ MCP allowlist required before production bridge

---

## ADR-017: Multi-level agent architecture (L1–L4)

**Status:** Accepted · **Date:** 2026-07-01

Four-layer agent model with downward-only handoffs, session store at ~/.rmng/sessions/, and layer-aware router. Full record: [ADR-017](decisions/ADR-017-multi-level-agent-architecture.md).

## ADR-015: Intent schema decoupling — poly-intent core envelope

**Date:** 2026-06-30  
**Status:** **Accepted**

### Context
Phase 6b requires separating native tool execution from MCP proxy requests at the data layer. v1 intents conflate tool requests under a single `kind` without an MCP path.

### Decision
Adopt v2 **poly-intent** envelope (`agents/schemas/core-intent.schema.json`) with internally tagged `action` field: `tool.execute`, `mcp.proxy`, `plan.only`. Rust models as `CoreIntent` enum with `#[serde(tag = "action", deny_unknown_fields)]`.

### Consequences
- ✅ Full record: [docs/decisions/ADR-015-intent-schema-decoupling.md](decisions/ADR-015-intent-schema-decoupling.md)
- ✅ v1 `Intent` retained for backward compatibility until dispatch migration
- ⚠️ `PermissionGate` extension for `mcp.proxy` in Phase 6b

---

## Pending decisions

| ADR | Topic | Options |
|-----|-------|---------|
| ADR-016 | Monorepo vs split repos for Rust runtime | Monorepo · split |
