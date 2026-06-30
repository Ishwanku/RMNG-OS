# Phase 2 Technical Validation Report

**Date:** 2026-06-30  
**Environment:** WSL2 Ubuntu 24.04 · 6 CPUs · 12 GB RAM · slim config  
**Status:** ✅ PASSED

---

## 1. ccache incremental rebuild benchmark

### Procedure

```bash
source ~/scripts/kernel-env.sh
make -C "$KSRC" mrproper          # fix dirty source tree marker
touch "$KSRC/init/main.c"
make -C "$KSRC" O="$KBUILD" -j6
```

### Results

| Metric | Value |
|--------|-------|
| **Elapsed time** | **176.39 s** (~2 min 56 s) |
| **Exit code** | 0 (success) |
| **NFR-P02 target** | < 5 min (300 s) → **PASS** |
| **vmlinux size** | 440 MB |
| **vmlinux timestamp** | 2026-06-30 10:25 UTC |

### ccache statistics

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Cacheable calls | 13,335 | 13,378 | +43 |
| Hits | 843 (6.32%) | 881 (6.59%) | **+38** |
| Direct hits | 620 | 658 | +38 |
| Misses | 12,492 | 12,497 | +5 |
| Cache size | 2.2 / 10.0 GB | 2.2 / 10.0 GB | stable |

### Key compile steps observed

- `CC init/main.o` — recompiled touched file
- `LD vmlinux` — full kernel relink (expected for `init/main.c` change)
- `BUILD bzImage` — boot image regenerated
- Remaining module `.ko` files rebuilt via modpost chain

### Note

Initial benchmark attempt failed with `source tree is not clean` due to `arch/x86/include/generated` in KSRC. Resolved with `make mrproper` (OOT-safe; does not touch `$KBUILD`).

---

## 2. Single module build

### Procedure

```bash
source ~/scripts/kernel-env.sh
# drivers/char has 0 loadable modules in slim config (all built-in)
# Selected CONFIG_TUN=m from slim config:
make -C "$KSRC" O="$KBUILD" M=drivers/net modules
```

### Results

| Metric | Value |
|--------|-------|
| **Module** | `tun` (`CONFIG_TUN=m`) |
| **Path** | `/home/saini/build/kernel/drivers/net/tun.ko` |
| **Size** | 1,514,760 bytes (~1.4 MB) |
| **Exit code** | 0 (success) |

### Why not `drivers/char`?

Slim config (`localmodconfig`) configures only **19 loadable modules**. All `drivers/char` targets are built-in (`=y`); `modules.order` was empty. `drivers/net` contains `CONFIG_TUN=m`.

---

## 3. Phase 2 acceptance criteria

| Criterion | Status |
|-----------|--------|
| Slim `vmlinux` build | ✅ 440 MB, 5.4 GB build dir |
| ccache incremental < 5 min | ✅ 176.39 s |
| Single module `.ko` produced | ✅ `tun.ko` |
| Requirements & architecture docs | ✅ |

**Phase 2 technical validation: COMPLETE**