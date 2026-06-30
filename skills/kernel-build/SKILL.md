---
name: kernel-build
description: RMNG-OS kernel lab — patch, configure, rebuild, benchmark. Use for Phase 1–4 kernel work in WSL2.
---

# Kernel Build Skill

## When to use

- Applying patches from `patches/`
- Running `rebuild-with-patches.sh`
- Slim config / ccache benchmarks
- Validating `vmlinux` and RMNG banner

## Constraints

- Work in WSL Ubuntu only
- One kernel build at a time (flock on `rebuild-with-patches.sh`)
- Never commit `~/build/kernel` artifacts

## Tool intents (via rmngd)

| Intent tool | When |
|-------------|------|
| `kernel.status` | Health check before/after build |
| `kernel.apply_patches` | After adding to `patches/series` |
| `kernel.build` | Incremental or clean rebuild |

## Commands

```bash
source ~/scripts/kernel-env.sh
~/scripts/rmng-status.sh
./scripts/rebuild-with-patches.sh
```

## Validation

See `docs/experiments/` and `docs/benchmarks/` for phase gate evidence.
