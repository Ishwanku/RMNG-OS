# RMNG-OS

A personal **AI Agent-first OS** project — starting with a rock-solid **Linux kernel foundation** on WSL2, then layering intelligent agents and cross-domain workflow integrations.

**Repository:** https://github.com/Ishwanku/RMNG-OS  
**Vision:** [docs/VISION.md](docs/VISION.md)

## Project Status

| Phase | Name | Status |
|-------|------|--------|
| **1** | Environment & First Build | ✅ Complete |
| **2** | Active Development Workflow | ✅ Complete |
| **3** | Customization & RMNG Identity | ✅ Complete |
| **4** | Advanced Kernel + Bare-Metal Boot | Planned (ADR-012) |
| **5** | AI Agent Foundation (Rust, CLI) | 🔄 **Current** |
| **6** | Workflow Integrations | Planned |
| **7** | Agent Orchestration | Planned |

See [docs/ROADMAP.md](docs/ROADMAP.md) for the full plan.

## Documentation

| Document | Description |
|----------|-------------|
| [docs/INDEX.md](docs/INDEX.md) | **Start here** — documentation hub |
| [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) | Functional & non-functional requirements |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Technical architecture & diagrams |
| [docs/VISION.md](docs/VISION.md) | AI-first OS vision |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phased delivery plan |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Architecture decision records |
| [docs/setup.md](docs/setup.md) | WSL install guide |
| [docs/daily-workflow.md](docs/daily-workflow.md) | Daily commands |
| [docs/INTEGRATION-STRATEGY.md](docs/INTEGRATION-STRATEGY.md) | **Integrating OSS repos safely** |

### Phase 1 Achievements

- Ubuntu 24.04 LTS on WSL2 with 12 GB RAM / 6 CPUs
- Full kernel toolchain + ccache
- Kernel source cloned at `~/dev/kernel/linux`
- First out-of-tree build succeeded (`vmlinux` ~458 MB)
- VS Code + WSL integration working

## What This Repo Contains

**Tooling and configuration only** — not the Linux kernel source or build artifacts.

```
RMNG-OS/
├── README.md
├── LICENSE                         # MIT
├── .gitignore
├── scripts/
│   ├── kernel-env.sh               # KSRC, KBUILD, ccache vars
│   ├── build.sh                    # Standardized make wrapper
│   ├── status.sh                   # Environment health check
│   ├── workspace-setup.sh          # One-time workspace wiring
│   ├── make-config-example.sh      # Regenerate config from local build
│   ├── apply-patches.sh            # Apply RMNG patch series
│   └── rebuild-with-patches.sh     # Patch + LOCALVERSION + rebuild
├── config/
│   ├── wsl.conf.example
│   ├── wslconfig.example
│   └── wsl-kernel.config.example   # Sanitized WSL2 baseline .config
├── patches/                        # Kernel patch series (RMNG identity)
├── agents/                         # Rust runtime workspace (Phase 5)
├── integrations/                   # Future workflow adapters (placeholder)
└── docs/
    ├── INDEX.md                    # Documentation hub
    ├── REQUIREMENTS.md             # Requirements specification
    ├── ARCHITECTURE.md             # Technical architecture
    ├── VISION.md                   # AI-first OS vision
    ├── ROADMAP.md                  # Phase plan
    ├── DECISIONS.md                # Architecture decisions
    ├── setup.md                    # Full install guide
    ├── daily-workflow.md           # Common commands
    ├── experiments/                # Phase validation logs
    └── benchmarks/                   # Performance benchmarks
```

## Quick Start (Existing Machine)

If you already completed Phase 1 setup:

```bash
cd ~/dev/projects/RMNG-OS
./scripts/dev-environment-setup.sh   # idempotent: dirs, ~/.rmng, shell, rust check
./scripts/install-rmng.sh              # rmng + rmngd + MCP allowlist
~/scripts/rmng-status.sh               # verify kernel environment
rmng status                            # verify agent runtime
```

**Future integrations:** see [docs/INTEGRATION-STRATEGY.md](docs/INTEGRATION-STRATEGY.md) before adding MCP servers, agent frameworks, or skill packs.

## Fresh Install

See [docs/setup.md](docs/setup.md) for the complete guide.

```bash
git clone https://github.com/Ishwanku/RMNG-OS.git ~/dev/projects/RMNG-OS
cd ~/dev/projects/RMNG-OS
./scripts/workspace-setup.sh
```

Clone kernel source separately:

```bash
git clone --depth=1 https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git ~/dev/kernel/linux
```

## Build the Kernel

```bash
source ~/scripts/kernel-env.sh
~/scripts/rmng-build.sh         # make -j6 out-of-tree
```

Or manually:

```bash
cp config/wsl-kernel.config.example ~/build/kernel/.config
make -C ~/dev/kernel/linux O=~/build/kernel olddefconfig
make -C ~/dev/kernel/linux O=~/build/kernel -j6
```

## Phase 2 — Complete ✅

Validation report: [docs/benchmarks/phase2-validation-20260630.md](docs/benchmarks/phase2-validation-20260630.md)

| Result | Value |
|--------|-------|
| Slim `vmlinux` | 440 MB · 5.4 GB build dir |
| Incremental rebuild | **176.39 s** (ccache warm) |
| Module build | `tun.ko` (1.4 MB) |


## Phase 3 — Complete ✅

Validation: [docs/experiments/phase3-validation-20260630.md](docs/experiments/phase3-validation-20260630.md)

| Result | Value |
|--------|-------|
| RMNG boot banner | Embedded in `vmlinux` |
| `CONFIG_LOCALVERSION` | `-rmng` |
| Patch series | `patches/0001-rmng-boot-banner.patch` |
| Rebuild elapsed | **252.20 s** (ccache warm) |

```bash
./scripts/rebuild-with-patches.sh   # apply patches + rebuild
```


## Phase 5 — In Progress 🔄

Rust agent runtime scaffold in `agents/`:

| Crate | Purpose |
|-------|---------|
| `rmng-core` | Intent parsing, permission gate |
| `rmng-cli` | `rmng` CLI (ADR-011) |
| `rmngd` | System daemon stub |

| Decision | Choice |
|----------|--------|
| Runtime | **Rust** (ADR-009) |
| LLM | **Hybrid local-first** — nervous system / body separation (ADR-010) |
| Interface | **CLI-first** (ADR-011) |
| Bare-metal | **Phase 4** (ADR-012) |

```bash
cd ~/dev/projects/RMNG-OS/agents && cargo build && cargo test
```

## AI-First OS Path

Layer 1 (kernel lab) must be solid before agents ship. Future work lives in:

- `agents/` — agent runtime (placeholder)
- `integrations/` — workflow adapters by domain (placeholder)
- `docs/VISION.md` — full architecture plan

Details: [docs/ROADMAP.md](docs/ROADMAP.md) · [docs/daily-workflow.md](docs/daily-workflow.md)

## WSL Optimization

| File | Install to | Purpose |
|------|------------|---------|
| `config/wsl.conf.example` | `/etc/wsl.conf` | systemd, automount |
| `config/wslconfig.example` | `C:\Users\<you>\.wslconfig` | RAM, CPU, swap |

Apply with `wsl --shutdown` from PowerShell.

## VS Code

```bash
code ~/dev/projects/RMNG-OS     # project repo
code ~/dev/kernel/linux         # kernel source
```

## What Is NOT in This Repo

| Excluded | Location | Size (typical) |
|----------|----------|----------------|
| Kernel source | `~/dev/kernel/linux` | ~2–6 GB |
| Build output | `~/build/kernel` | ~3–14 GB |
| ccache | `~/.ccache` | ~1–2 GB |

## License

MIT License — see [LICENSE](LICENSE).

The Linux kernel source you compile separately is **GPLv2**. This repo contains MIT-licensed tooling only.