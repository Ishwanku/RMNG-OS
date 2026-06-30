---
name: kernel-config
description: RMNG-OS kernel configuration — menuconfig, localmodconfig, slim vs full configs, and config export scripts. Use when tuning .config or choosing build profiles.
---

# Kernel Config Skill

## When to use

- Editing kernel options interactively (`menuconfig`)
- Generating or refreshing a slim config (`localmodconfig`)
- Choosing between full WSL config and slim daily-build config
- Exporting config examples to the RMNG-OS repo
- Setting or verifying `CONFIG_LOCALVERSION="-rmng"`

## Full config vs slim config

| Profile | When to use | Trade-offs |
|---------|-------------|------------|
| **Full** (`config/wsl-kernel.config.example`) | First build, hardware bring-up, module experiments outside current WSL drivers | ~8k+ lines, more modules, slower builds, larger `~/build/kernel` |
| **Slim** (`config/wsl-kernel.config.slim.example`) | Daily iteration, ccache benchmarks, patch rebuilds | ~5.5k lines, ~19 modules; may omit drivers needed for bare-metal or new hardware |

**Default for daily work:** slim. Re-expand to full only when you need drivers or subsystems not present in the running WSL kernel.

## Constraints

- Work in WSL Ubuntu only
- Kernel source stays at `~/dev/kernel/linux` (not in RMNG-OS repo)
- Out-of-tree build: always use `O=$KBUILD` (`~/build/kernel`)
- Never commit `~/build/kernel/.config` or build artifacts — export via repo scripts only
- After `slim-config.sh`, verify `CONFIG_LOCALVERSION="-rmng"` in the exported example

## Workflows

### Interactive edit

```bash
source ~/scripts/kernel-env.sh
make -C "$KSRC" O="$KBUILD" menuconfig
~/dev/projects/RMNG-OS/scripts/make-config-example.sh   # sync changes to repo example
```

### Generate slim config

```bash
cd ~/dev/projects/RMNG-OS
./scripts/slim-config.sh
```

This backs up to `$KBUILD/.config.full-backup`, runs `localmodconfig`, and writes `config/wsl-kernel.config.slim.example`.

### Restore from repo example

```bash
source ~/scripts/kernel-env.sh
cp ~/dev/projects/RMNG-OS/config/wsl-kernel.config.slim.example "$KBUILD/.config"
make -C "$KSRC" O="$KBUILD" olddefconfig
```

## Tool intents (via rmngd)

| Intent tool | When |
|-------------|------|
| `kernel.status` | Confirm `KBUILD/.config` exists and build dir health before/after config changes |
| `kernel.build` | Rebuild after config change (`target`: `all`, `modules`, or `bzImage`) |

Config editing itself is done via the commands above — not via MCP or direct shell from the LLM layer.

## Validation

- `grep CONFIG_LOCALVERSION "$KBUILD/.config"` shows `"-rmng"`
- `wc -l "$KBUILD/.config"` — slim should be materially smaller than full backup
- Incremental rebuild completes; see `docs/benchmarks/` for timing expectations
- After export, `git diff config/` in RMNG-OS shows only intentional example updates