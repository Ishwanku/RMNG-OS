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

## Phase 2 — Active Development Workflow (CURRENT)

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
| Incremental rebuild benchmark | High | Measure ccache speedup |
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

## Phase 3 — Customization & RMNG Identity

Goal: Make the kernel build distinctly "RMNG" without forking the entire tree.

| Task | Notes |
|------|-------|
| Custom `CONFIG_LOCALVERSION="-rmng"` | Already in example config |
| Custom boot logo / printk banner patch | Small out-of-tree patch series |
| Track patches under `patches/` in RMNG-OS repo | Quilt or git format-patch |
| Scripted patch apply + rebuild | `scripts/rebuild-with-patches.sh` |

---

## Phase 4 — Advanced Kernel (Optional)

| Task | Notes |
|------|-------|
| Boot custom kernel in WSL2 | Requires Microsoft WSL kernel build docs |
| eBPF / BTF experiments | Tools already built (pahole, dwarves) |
| GitHub Actions | Lint scripts only — no kernel CI (too heavy) |
| Cross-compile or module-only CI | Lightweight automation |

---

## Phase 5 — AI Agent Foundation (after Phase 2–3)

Goal: Scaffold the agent runtime without disrupting kernel work. See [VISION.md](VISION.md).

| Task | Notes |
|------|-------|
| `agents/` runtime scaffold | ✅ placeholder README |
| `integrations/` adapter layout | ✅ placeholder README |
| Local LLM / API bridge service | Python or Rust service |
| Tool registry (shell, git, files) | Structured JSON schemas |
| `gh auth login` in WSL | Git push from Ubuntu |

## Phase 6 — Workflow Integrations

| Domain | Priority |
|--------|----------|
| Development (git, build, kernel) | High — natural extension of today |
| Data & files | Medium |
| Cloud & infra | Medium |
| Creative & business | Later |

## Phase 7 — Agent Orchestration

Multi-agent routing, shared memory, permissions, UI/CLI shell.

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
| **3** | Custom patch applies cleanly and rebuilds |
| **4** | Optional advanced goal achieved |