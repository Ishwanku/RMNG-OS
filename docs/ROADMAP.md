# RMNG-OS Roadmap

Development phases for the RMNG-OS kernel lab environment.

## Phase 1 — Environment & First Build ✅ COMPLETE

| Task | Status |
|------|--------|
| WSL2 Ubuntu 24.04 setup | ✅ |
| Build toolchain + ccache | ✅ |
| WSL tuning (`wsl.conf`, `.wslconfig`) | ✅ |
| Home directory structure | ✅ |
| Kernel source clone (shallow) | ✅ |
| Out-of-tree first full build | ✅ (`vmlinux` ~458 MB) |
| VS Code + WSL integration | ✅ |
| GitHub repo `RMNG-OS` published | ✅ |

**Outcome:** Reproducible WSL kernel development environment with documented tooling.

---

## Phase 2 — Active Development Workflow ✅ COMPLETE

Goal: Turn the environment into a daily-use kernel lab with faster iteration and slimmer builds.

### 2.1 Workspace Automation

| Task | Priority | Notes |
|------|----------|-------|
| `workspace-setup.sh` — wire symlinks | High | ✅ in repo |
| `status.sh` — one-command health check | High | ✅ in repo |
| `build.sh` — standardized make wrapper | High | ✅ in repo |
| Install `gh` in WSL for git push | Medium | Avoid hung pushes |
| Git credential helper via `gh auth login` | Medium | One-time in WSL |

### 2.2 Config Optimization

| Task | Priority | Notes |
|------|----------|-------|
| Generate slim config with `localmodconfig` | High | ✅ 8821 → 5498 lines, 19 modules |
| Document config diff vs full WSL config | Medium | ✅ `config/wsl-kernel.config.slim.example` |
| `make menuconfig` walkthrough | Medium | Document in `docs/config-guide.md` |

### 2.3 Rebuild Performance

| Task | Priority | Notes |
|------|----------|-------|
| Incremental rebuild benchmark | High | ✅ 176.39 s (see benchmarks/) |
| Single module build | High | ✅ `tun.ko` via `M=drivers/net` |
| Tune `JOBS` / ccache size | Low | Match 6 CPUs, 12 GB RAM |
| `make clean` vs full rebuild docs | Low | Disk management |

### 2.4 Kernel Experimentation

| Task | Priority | Notes |
|------|----------|-------|
| Build a single module (`make M=...`) | High | e.g. `drivers/char` |
| Apply a trivial patch (LOCALVERSION, printk) | High | Learn patch workflow |
| Kernel change journal in `docs/experiments/` | Medium | Log what you tried |
| Requirements & architecture docs | High | ✅ REQUIREMENTS.md, ARCHITECTURE.md, DECISIONS.md |

### 2.5 Repository & Docs

| Task | Priority | Notes |
|------|----------|-------|
| Update README with phase status | High | ✅ this session |
| Add `docs/ROADMAP.md` | High | ✅ this file |
| Add `docs/daily-workflow.md` | Medium | Common commands |
| GitHub topics + repo description polish | Low | |

---

## Phase 3 — Customization & RMNG Identity ✅ COMPLETE

Goal: Make the kernel build distinctly "RMNG" without forking the entire tree.

| Task | Status | Notes |
|------|--------|-------|
| Custom `CONFIG_LOCALVERSION="-rmng"` | ✅ | Set in `$KBUILD/.config` |
| RMNG boot banner patch | ✅ | `patches/0001-rmng-boot-banner.patch` |
| Track patches under `patches/` | ✅ | `series` + README |
| Scripted patch apply + rebuild | ✅ | `apply-patches.sh`, `rebuild-with-patches.sh` |
| Validation report | ✅ | `docs/experiments/phase3-validation-20260630.md` |

**Outcome:** Patched kernel builds with RMNG identity banner in `vmlinux`; rebuild ~252 s with warm ccache.

---

## Phase 4 — Advanced Kernel + Bare-Metal Boot (ADR-012)

| Task | Notes |
|------|-------|
| Boot custom kernel in WSL2 | Requires Microsoft WSL kernel build docs |
| Bare-metal boot timeline | **Locked: Phase 4** — initramfs, hardware drivers |
| eBPF / BTF experiments | Tools already built (pahole, dwarves) |
| GitHub Actions | Lint scripts only — no kernel CI (too heavy) |
| Cross-compile or module-only CI | Lightweight automation |

---

## Phase 5 — AI Agent Foundation ✅ COMPLETE

Goal: Implement Rust runtime with nervous-system / body separation. See [VISION.md](VISION.md), [ADR-009–012](DECISIONS.md).

**Locked:** Rust runtime · Hybrid local-first LLM · CLI-first · JSON intent boundary

| Task | Notes |
|------|-------|
| `agents/` runtime scaffold | 🔄 Rust workspace (`rmng-core`, `rmng-cli`, `rmngd`) |
| `integrations/` adapter layout | 🔄 `integrations/dev/kernel.json` |
| Ollama nervous-system adapter | 🔄 `rmng-nervous` + `rmng ask` |
| External API adapter (pluggable) | OpenAI/Anthropic — intents only |
| `rmng` CLI + `rmngd` daemon | ✅ `run`, `send`, IPC socket |
| Permission gate + audit log | ✅ Gate + `~/.rmng/logs/audit.jsonl` |
| `gh auth login` in WSL | Git push from Ubuntu |

## Phase 6 — Skills, MCP & Integrations (PLANNED)

**Plan:** [PLAN-AGENTS-MCP-SKILLS.md](PLAN-AGENTS-MCP-SKILLS.md) · **ADR:** [ADR-014](DECISIONS.md)

| Sub-phase | Goal |
|-----------|------|
| **6a** | Skills (`skills/`) + dev MCP template + `setup-dev-mcp.sh` |
| **6b** | `rmng-mcp` bridge (rust-sdk) + allowlist |
| **6c** | `rmng ask --skill` nervous-system integration |

## Phase 6 — Workflow Integrations

| Domain | Priority |
|--------|----------|
| Development (git, build, kernel) | High — natural extension of today |
| Data & files | Medium |
| Cloud & infra | Medium |
| Creative & business | Later |

## Phase 7 — Agent Orchestration (IN PROGRESS)

Multi-agent routing, layer model, session store, swarm handoffs.

| Sprint | Status | Deliverables |
|--------|--------|--------------|
| **Sprint 1** | ✅ | IntegrationRegistry, IntentValidator, Audit v2, MCP lifecycle |
| **Sprint 2** | ✅ | Agent definitions, router, `rmng ask --agent`, `rmng observe`, progressive skills |
| **Sprint 3** | ✅ | L1–L4 layer model, session store, layer-aware router, ADR-017 |
| **Sprint 4a** | ✅ | Shared context in prompts, `rmng handoff`, daemon E2E tests, light ingestion |
| **Sprint 4b** | ✅ | Tool result write-back to `shared_context`, multi-hop `--chain` handoffs, `session prune`, collaboration E2E |
| **Sprint 4c** | Planned | Live LLM orchestration, MCP ingestion E2E, automated session TTL |

See [ADR-017](decisions/ADR-017-multi-level-agent-architecture.md).

---

## Immediate Next Actions (Start Here)

```bash
# 1. Wire workspace
cd ~/dev/projects/RMNG-OS
./scripts/workspace-setup.sh

# 2. Check status
~/scripts/rmng-status.sh

# 3. Slim config (done — or re-run)
~/dev/projects/RMNG-OS/scripts/slim-config.sh

# 4. Rebuild with slim config and measure ccache
~/scripts/rmng-build.sh

# 5. Open project in VS Code
code ~/dev/projects/RMNG-OS
```

---

## Success Criteria by Phase

| Phase | Done when |
|-------|-----------|
| **1** | `vmlinux` exists, repo on GitHub | ✅ |
| **2** | Slim config builds, ccache rebuild < 5 min, daily scripts work |
| **3** | Custom patch applies cleanly and rebuilds | ✅ |
| **4** | Optional advanced goal achieved |