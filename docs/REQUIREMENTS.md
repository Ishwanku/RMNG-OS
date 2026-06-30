# RMNG-OS Requirements Specification

**Version:** 0.3  
**Status:** Phase 3 complete · Phase 5 in progress  
**Last updated:** 2026-06-30

## 1. Introduction

### 1.1 Purpose

This document defines functional and non-functional requirements for **RMNG-OS** — an AI Agent-first operating environment built on a custom Linux kernel foundation.

### 1.2 Scope

| In scope | Out of scope (v0.1) |
|----------|---------------------|
| WSL2 kernel development lab | Production desktop distribution |
| Build tooling, configs, scripts | Shipping prebuilt ISO images |
| Documentation & requirements | Full multi-user enterprise deployment |
| Agent/integration **scaffold** | Production LLM hosting at scale |

### 1.3 Definitions

| Term | Definition |
|------|------------|
| **KSRC** | Kernel source tree (`~/dev/kernel/linux`) |
| **KBUILD** | Out-of-tree build directory (`~/build/kernel`) |
| **Layer 1** | Kernel foundation (toolchain, build, config) |
| **Layer 2–4** | Userspace, integrations, agent orchestration (future) |
| **Agent** | Autonomous software entity that executes tools on behalf of the user |
| **Integration** | Domain-specific adapter (dev, data, creative, etc.) |

### 1.4 References

- [VISION.md](VISION.md)
- [ARCHITECTURE.md](ARCHITECTURE.md)
- [ROADMAP.md](ROADMAP.md)
- Linux kernel: GPLv2
- RMNG-OS tooling: MIT

---

## 2. Goals & constraints

### 2.1 Business goals

| ID | Goal |
|----|------|
| G-01 | Build a personal OS where AI agents are first-class, not add-ons |
| G-02 | Master kernel-level control before layering AI complexity |
| G-03 | Unify workflows across dev, research, creative, business, and personal domains |
| G-04 | Maintain reproducible, documented, shareable development environment |

### 2.2 Technical constraints

| ID | Constraint |
|----|------------|
| C-01 | Primary dev target: **WSL2 + Ubuntu 24.04** on Windows |
| C-02 | Kernel source **not** vendored in RMNG-OS repo (clone separately) |
| C-03 | Build artifacts **never** committed to git |
| C-04 | Work inside WSL for system changes; Windows files only via documented templates |
| C-05 | Respect GPLv2 for kernel; MIT for RMNG-OS scripts/docs |

### 2.3 Assumptions

| ID | Assumption |
|----|------------|
| A-01 | Developer has 16 GB+ host RAM (12 GB allocated to WSL) |
| A-02 | SSD storage with 50 GB+ free for kernel work |
| A-03 | Internet access for git clone, apt, and future API integrations |
| A-04 | Single primary user (`saini`) on development machine |

---

## 3. Functional requirements — Layer 1 (Kernel foundation)

**Priority key:** P0 = must have · P1 = should have · P2 = nice to have

### 3.1 Environment setup

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-L1-01 | System SHALL provide Ubuntu 24.04 on WSL2 with systemd enabled | P0 | ✅ Done |
| FR-L1-02 | System SHALL install full kernel build toolchain (gcc, make, flex, bison, lib*, dwarves, ccache) | P0 | ✅ Done |
| FR-L1-03 | System SHALL expose WSL tuning via `wsl.conf` and `.wslconfig` examples | P0 | ✅ Done |
| FR-L1-04 | System SHALL use standardized home layout (`~/dev`, `~/build`, `~/scripts`) | P0 | ✅ Done |
| FR-L1-05 | `workspace-setup.sh` SHALL wire symlinks and shell config idempotently | P0 | ✅ Done |

### 3.2 Kernel source & build

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-L1-10 | Developer SHALL clone kernel source separately from torvalds/linux | P0 | ✅ Done |
| FR-L1-11 | Builds SHALL use out-of-tree pattern (`make O=$KBUILD`) | P0 | ✅ Done |
| FR-L1-12 | `kernel-env.sh` SHALL set KSRC, KBUILD, CCACHE_DIR, and compiler wrappers | P0 | ✅ Done |
| FR-L1-13 | System SHALL produce `vmlinux` from source config | P0 | ✅ Done (full config) |
| FR-L1-14 | Repo SHALL ship full and slim `.config` examples | P0 | ✅ Done |
| FR-L1-15 | `slim-config.sh` SHALL generate localmodconfig-based slim config | P1 | ✅ Done |
| FR-L1-16 | `build.sh` SHALL wrap make with consistent defaults (`-j6`) | P0 | ✅ Done |
| FR-L1-17 | `status.sh` SHALL report environment, build, and git health | P0 | ✅ Done |

### 3.3 Performance & iteration

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-L1-20 | ccache SHALL wrap gcc/g++ with 10 GB cache limit | P0 | ✅ Done |
| FR-L1-21 | Incremental rebuild SHALL complete in < 5 min (with warm ccache) | P1 | ✅ 176.39 s (2026-06-30) |
| FR-L1-22 | Slim config build SHALL use significantly less disk than full config | P1 | ✅ 5.4 GB vs 14 GB full |
| FR-L1-23 | Developer SHALL build individual modules via `make M=...` | P1 | ✅ `tun.ko` via `M=drivers/net` |

### 3.4 Kernel customization

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-L1-30 | Config SHALL support `CONFIG_LOCALVERSION="-rmng"` branding | P1 | ✅ Done |
| FR-L1-31 | Developer SHALL apply and track patches under `patches/` | P1 | ✅ Phase 3 |
| FR-L1-32 | Scripted patch-apply + rebuild workflow SHALL exist | P2 | ✅ `rebuild-with-patches.sh` |
| FR-L1-33 | Optional: boot custom kernel in WSL2 | P2 | Planned |

### 3.5 Developer experience

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-L1-40 | VS Code + WSL SHALL open project and kernel source | P0 | ✅ Done |
| FR-L1-41 | GitHub repo SHALL host tooling, docs, configs only | P0 | ✅ Done |
| FR-L1-42 | `gh` SHALL be available for git operations from WSL | P1 | ✅ Installed |
| FR-L1-43 | `gh auth login` SHALL enable passwordless git push | P1 | Pending user action |

---

## 4. Functional requirements — Layer 2 (Userspace)

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-L2-01 | RMNG SHALL provide a CLI entry point (`rmng` command) | P1 | Planned |
| FR-L2-02 | System services SHALL be managed via systemd units | P1 | Planned |
| FR-L2-03 | Central config file SHALL define paths, models, and permissions | P1 | Planned |
| FR-L2-04 | Logging SHALL be structured (JSON) for agent consumption | P2 | Planned |

---

## 5. Functional requirements — Layer 3 (Integrations)

Each integration MUST expose: `name`, `version`, `tools[]` with JSON schema, `auth` requirements.

**Boundary rule (ADR-010):** Integrations are invoked **only** by the local Rust runtime after intent validation. External LLMs never call integrations directly.

| ID | Requirement | Domain | Priority | Status |
|----|-------------|--------|----------|--------|
| FR-L3-01 | Dev integration: git, build, file ops | Development | P0 | Planned |
| FR-L3-02 | Shell command execution with sandbox policy | Development | P0 | Planned |
| FR-L3-03 | File read/write within allowed paths | All | P0 | Planned |
| FR-L3-09 | Integrations SHALL accept only runtime-dispatched calls (not LLM-direct) | All | P0 | Locked |
| FR-L3-10 | Tool inputs/outputs SHALL conform to versioned JSON schemas | All | P0 | Locked |
| FR-L3-04 | Web fetch / search capability | Research | P1 | Planned |
| FR-L3-05 | Database query adapter | Data | P2 | Planned |
| FR-L3-06 | Document generation adapter | Creative | P2 | Planned |
| FR-L3-07 | Calendar / email adapter | Business | P2 | Planned |
| FR-L3-08 | Cloud deploy adapter | Infrastructure | P2 | Planned |

---

## 6. Functional requirements — Layer 4 (AI agents)

### Biological separation model (mandatory — ADR-010)

| Component | Metaphor | Owner | LLM access |
|-----------|----------|-------|------------|
| Reasoning | Nervous System | Pluggable adapter (Ollama default) | Produces JSON intents only |
| Execution | Body | Local Rust runtime | Never LLM-direct |
| State + policy | Heart + Brains | Local Rust processes | Never LLM-direct |

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-L4-01 | Rust runtime SHALL parse LLM JSON intents and dispatch tools | P0 | Locked (ADR-009) |
| FR-L4-02 | Reasoning layer SHALL be hybrid: local-first, external API pluggable | P0 | Locked (ADR-010) |
| FR-L4-03 | Session memory SHALL be owned by local runtime (not LLM provider) | P0 | Locked |
| FR-L4-04 | Persistent memory SHALL be local, user-controlled store | P1 | Locked |
| FR-L4-05 | Permission model SHALL gate all tool execution; LLM cannot bypass | P0 | Locked |
| FR-L4-06 | Multi-agent router SHALL run locally as native Rust processes | P1 | Locked |
| FR-L4-07 | User SHALL approve high-risk actions before runtime executes | P0 | Locked |
| FR-L4-08 | All intent → execution chains SHALL be auditable locally | P1 | Locked |
| FR-L4-09 | LLM SHALL NOT have raw terminal, syscall, or direct integration access | P0 | Locked |
| FR-L4-10 | Replacing reasoning backend SHALL NOT require changes to execution layer | P1 | Locked |
| FR-L4-11 | Primary interface SHALL be CLI (`rmng` command) | P0 | Locked (ADR-011) |

---

## 7. Non-functional requirements

### 7.1 Performance

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-P01 | Full kernel build (slim config, 6 CPUs) | < 60 min cold |
| NFR-P02 | Incremental kernel rebuild (ccache warm) | < 5 min |
| NFR-P03 | Agent tool call latency (local, non-LLM) | < 2 s |
| NFR-P04 | Status script execution | < 3 s |

### 7.2 Reliability

| ID | Requirement |
|----|-------------|
| NFR-R01 | Build scripts SHALL exit non-zero on failure |
| NFR-R02 | `workspace-setup.sh` SHALL be idempotent |
| NFR-R03 | Config backups SHALL be created before destructive config changes |

### 7.3 Security

| ID | Requirement |
|----|-------------|
| NFR-S01 | Secrets SHALL NOT be committed to git |
| NFR-S02 | Agent shell access SHALL use allowlisted commands (future) |
| NFR-S03 | API keys SHALL live in env or secure store, not docs |
| NFR-S04 | User confirmation required for: rm -rf, git push --force, sudo |
| NFR-S05 | LLM layer SHALL NOT receive write access to permission policy or memory stores |
| NFR-S06 | All LLM outputs entering runtime SHALL pass JSON schema validation |
| NFR-S07 | Agent runtime SHALL be implemented in Rust (ADR-009) |

### 7.4 Maintainability

| ID | Requirement |
|----|-------------|
| NFR-M01 | All scripts SHALL include usage comments |
| NFR-M02 | Docs SHALL live in `docs/` with INDEX.md entry |
| NFR-M03 | Requirements SHALL trace to ROADMAP phases |

### 7.5 Portability

| ID | Requirement |
|----|-------------|
| NFR-PO01 | Layer 1 SHALL work on WSL2 Ubuntu 24.04 |
| NFR-PO02 | Scripts SHALL use bash and standard GNU tools |
| NFR-PO03 | Future: Layer 2+ portable to bare-metal / VM |

---

## 8. Acceptance criteria by phase

### Phase 1 — Environment & first build ✅

- [x] `vmlinux` exists (~458 MB)
- [x] RMNG-OS repo on GitHub
- [x] Documented setup reproducible from README

### Phase 2 — Active development workflow ✅

- [x] Slim config generated and saved
- [x] Slim `vmlinux` build completes (440 MB, 5.4 GB build dir)
- [x] ccache incremental rebuild < 5 min (176.39 s)
- [x] Single module build succeeds (`tun.ko`, 1.4 MB)
- [x] Requirements & architecture docs complete
- [x] Validation report: `docs/benchmarks/phase2-validation-20260630.md`

### Phase 3 — RMNG identity ✅

- [x] Custom patch applies cleanly (`0001-rmng-boot-banner.patch`)
- [x] `CONFIG_LOCALVERSION="-rmng"` in build config
- [x] `kernel.release` = `7.1.0-rmng+`
- [x] RMNG banner in `vmlinux` strings
- [x] Patches tracked in `patches/` with apply/rebuild scripts
- [x] Report: `docs/experiments/phase3-validation-20260630.md`

### Phase 5 — AI agent foundation

- [ ] Agent runtime executes one tool end-to-end
- [ ] Dev integration: git status via agent
- [ ] Permission gate blocks unapproved destructive command

### Phase 7 — Full vision

- [ ] User completes daily dev workflow via agent orchestration
- [ ] Three domain integrations operational

---

## 9. Requirements traceability

| Requirement | Roadmap phase | Document |
|-------------|---------------|----------|
| FR-L1-* | Phase 1–2 | setup.md, daily-workflow.md |
| FR-L2-* | Phase 3–5 | ARCHITECTURE.md |
| FR-L3-* | Phase 6 | integrations/README.md |
| FR-L4-* | Phase 5–7 | VISION.md, agents/README.md |

---

## 10. Locked architectural decisions (Phase 5)

| ID | Decision | ADR | Status |
|----|----------|-----|--------|
| Q-01 | Agent runtime language: **Rust** | ADR-009 | **Locked** |
| Q-02 | LLM architecture: **Hybrid local-first; nervous-system / body separation** | ADR-010 | **Locked** |
| Q-03 | Primary interface: **CLI-first** | ADR-011 | **Locked** |
| Q-04 | Bare-metal boot: **Phase 4** | ADR-012 | **Locked** |

### Q-02 enforcement summary

External LLMs function as the **Nervous System** (reasoning/planning only). The local OS owns the **Body, Heart, and Brains** (execution, memory, permissions, sandboxing, multi-agent orchestration). The `integrations/` layer enforces a strict IPC/schema boundary: LLMs emit JSON intents; the Rust runtime alone parses, authorizes, and executes.

## 11. Open questions

| # | Question | Decision needed by |
|---|----------|-------------------|
| Q-05 | Monorepo vs separate repos for agents/integrations? | Phase 5 implementation |

---

## Revision history

| Version | Date | Changes |
|---------|------|---------|
| 0.1 | 2026-06-30 | Initial requirements draft |
| 0.2 | 2026-06-30 | Phase 2 validation complete; Phase 5 decisions locked (ADR-009–012) |
| 0.3 | 2026-06-30 | Phase 3 RMNG kernel identity complete |