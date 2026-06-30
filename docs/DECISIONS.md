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

## Pending decisions

| ADR | Topic | Options |
|-----|-------|---------|
| ADR-009 | Agent runtime language | Python · Rust · TypeScript |
| ADR-010 | Default LLM backend | Ollama (local) · OpenAI · Anthropic |
| ADR-011 | UI for agents | CLI-first · TUI · Web dashboard |