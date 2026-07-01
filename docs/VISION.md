# RMNG-OS Vision

**RMNG-OS** aims to become an **AI Agent-first operating environment** — a Linux foundation where intelligent agents are native citizens, not bolt-on apps. The kernel and system layer provide a stable, high-performance base; the AI layer orchestrates workflows across every domain a user touches.

## North Star

> A personal OS where AI agents understand your full context — dev, creative, business, research, communication — and act on your behalf with kernel-level performance and desktop-level usability.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 4 — AI Agent Orchestration (future)                  │
│  Multi-agent routing, memory, tools, workflow automation      │
├─────────────────────────────────────────────────────────────┤
│  Layer 3 — Integrations (future)                            │
│  Dev, cloud, productivity, creative, data, comms APIs     │
├─────────────────────────────────────────────────────────────┤
│  Layer 2 — RMNG Userspace (future)                          │
│  Services, CLI, desktop shell, agent runtime, config        │
├─────────────────────────────────────────────────────────────┤
│  Layer 1 — Kernel Foundation (current focus) ✅ in progress │
│  WSL2 lab → custom kernel → modules → patches → boot        │
└─────────────────────────────────────────────────────────────┘
```

## Design Principles

1. **Kernel first** — Understand and control the foundation before adding AI complexity.
2. **Out-of-tree discipline** — Keep source clean; builds reproducible; configs versioned.
3. **Biological separation** — LLM = Nervous System (reasoning only). OS = Body + Heart + Brains (execution, memory, policy). See ADR-010.
4. **Agent-native interfaces** — Every integration exposes structured JSON-schema APIs; only the Rust runtime invokes them.
5. **Workflow-unified** — One orchestration layer across dev, ops, creative, and personal tasks.
6. **Local-first, cloud-aware** — Ollama default; external APIs pluggable as reasoning backends only.
7. **Controlled core, documented boundaries** — RMNG runtime and tooling are proprietary; kernel patches are GPLv2; external OSS enters via defined integration tracks (ADR-019).

## Workflow Domains (planned integrations)

| Domain | Examples | Agent role |
|--------|----------|------------|
| **Development** | Kernel, drivers, Git, CI, containers | Code, build, debug, review |
| **Data & Research** | DBs, notebooks, papers, web | Query, summarize, experiment |
| **Creative** | Design, docs, media | Generate, edit, publish |
| **Business & Ops** | Email, calendar, CRM, finance | Schedule, draft, analyze |
| **Infrastructure** | Cloud, monitoring, deploy | Provision, heal, scale |
| **Personal** | Notes, habits, files | Organize, remind, assist |

## Current Phase: Basic OS Foundation

Before any AI layer, we complete:

| Milestone | Status |
|-----------|--------|
| WSL2 dev environment | ✅ |
| Kernel toolchain + ccache | ✅ |
| First full kernel build | ✅ |
| Slim config (`localmodconfig`) | ✅ |
| Dev scripts + GitHub repo | ✅ |
| Incremental rebuild workflow | 🔄 Next |
| Module / patch experiments | Planned |
| Custom RMNG kernel identity | Planned |
| Userspace service scaffold | Planned |

## AI Integration Phases (after foundation)

### Phase A — Agent Runtime Scaffold
- `integrations/` directory with adapter pattern
- Local LLM / API bridge service
- Structured tool registry (shell, git, files, web)

### Phase B — Workflow Connectors
- Per-domain adapters (dev, data, creative, etc.)
- Shared memory / context store
- Event bus for agent triggers

### Phase C — Orchestration
- Multi-agent planner and router
- Permission model (what agents can touch)
- UI shell or CLI for agent interaction

### Phase D — RMNG Identity
- Custom kernel with RMNG branding
- Boot-to-agent experience
- Production deployment target (bare metal / VM / WSL)

## What RMNG-OS Repo Holds Today

This repository spans **Layer 1 (kernel lab)** through **Layer 4 (agent orchestration)** — kernel patches and scripts, plus the proprietary Rust runtime (`agents/`), skills, and integration manifests. Intelligence stays in userspace (nervous system); the kernel provides resources only (Torvalds-style layering).

## Success Metrics

| Stage | Metric |
|-------|--------|
| Foundation | Rebuild kernel in < 5 min with ccache |
| Foundation | Apply patch + rebuild reliably |
| AI scaffold | One agent executes a dev workflow end-to-end |
| Integration | Three domain connectors working |
| Vision | Daily work runs through agent orchestration |