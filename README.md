# RMNG-OS

A personal **AI Agent-first OS** project — starting with a rock-solid **Linux kernel foundation** on WSL2, then layering intelligent agents and cross-domain workflow integrations.

**Repository:** https://github.com/Ishwanku/RMNG-OS  
**Vision:** [docs/VISION.md](docs/VISION.md)

## Project Status

| Phase | Name | Status |
|-------|------|--------|
| **1** | Environment & First Build | ✅ Complete |
| **2** | Active Development Workflow | 🔄 **Current** |
| **3** | Customization & RMNG Identity | Planned |
| **4** | Advanced Kernel (WSL boot, eBPF) | Optional |
| **5** | AI Agent Foundation | Scaffolded |
| **6** | Workflow Integrations | Planned |
| **7** | Agent Orchestration | Planned |

See [docs/ROADMAP.md](docs/ROADMAP.md) for the full plan.

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
│   └── make-config-example.sh      # Regenerate config from local build
├── config/
│   ├── wsl.conf.example
│   ├── wslconfig.example
│   └── wsl-kernel.config.example   # Sanitized WSL2 baseline .config
└── docs/
    ├── setup.md                    # Full install guide
    ├── ROADMAP.md                  # Phase plan
    └── daily-workflow.md           # Common commands
```

## Quick Start (Existing Machine)

If you already completed Phase 1 setup:

```bash
cd ~/dev/projects/RMNG-OS
./scripts/workspace-setup.sh    # wire symlinks
~/scripts/rmng-status.sh        # verify everything
```

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

## Phase 2 — Next Steps

1. ~~**Slim the config**~~ — ✅ `localmodconfig` (8821 → 5498 lines)
2. **Rebuild with slim config** — `~/scripts/rmng-build.sh` (in progress)
3. **Benchmark ccache** — incremental rebuild after a one-line change
4. **Build a single module** — `make M=drivers/char modules`
5. **Apply a test patch** — custom `LOCALVERSION` or printk change
6. **Auth git in WSL** — `gh auth login` for painless `git push`

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