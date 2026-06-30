# Phase 3 Validation — RMNG Kernel Identity

**Date:** 2026-06-30  
**Status:** ✅ PASSED

---

## Objectives

| Objective | Result |
|-----------|--------|
| Apply RMNG patch series | ✅ `0001-rmng-boot-banner.patch` (clean apply, no fuzz warnings) |
| `CONFIG_LOCALVERSION="-rmng"` | ✅ in `$KBUILD/.config` |
| Patches tracked in `patches/` | ✅ `series` + README |
| Scripted apply + rebuild | ✅ `apply-patches.sh`, `rebuild-with-patches.sh` |
| RMNG banner in binary | ✅ `strings` confirms message |
| Rebuild completes | ✅ 252.20 s (ccache warm, `-j6`) |

---

## Patch applied

**File:** `patches/0001-rmng-boot-banner.patch`

```diff
+pr_info("RMNG-OS: kernel identity active - foundation layer ready\n");
```

**Location:** `init/main.c` after `pr_notice("%s", linux_banner);`

---

## Build metrics

| Metric | Value |
|--------|-------|
| Build elapsed | **252.20 s** (~4 min 12 s) |
| `vmlinux` | 440 MB |
| `bzImage` | 17 MB (build #9) |
| Patch diff | `init/main.c | 1 +` |

---

## Verification commands

```bash
source ~/scripts/kernel-env.sh
grep CONFIG_LOCALVERSION "$KBUILD/.config"
strings "$KBUILD/vmlinux" | grep RMNG-OS
git -C "$KSRC" diff --stat init/main.c
~/dev/projects/RMNG-OS/scripts/rebuild-with-patches.sh
```

### Observed output (2026-06-30)

```
CONFIG_LOCALVERSION="-rmng"
RMNG-OS: kernel identity active - foundation layer ready
 init/main.c | 1 +
 1 file changed, 1 insertion(+)
```

---

## Known note: version string in vmlinux

The `Linux version X.Y.Z-rmng` string embedded in `vmlinux` may show an empty version field on incremental rebuilds until `init/version.o` is fully relinked. The **RMNG banner string** and **`CONFIG_LOCALVERSION="-rmng"`** in `.config` are the authoritative Phase 3 identity markers.

**Relink version if needed:**

```bash
make -C "$KSRC" O="$KBUILD" kernelrelease
make -C "$KSRC" O="$KBUILD" init/version.o
make -C "$KSRC" O="$KBUILD" -j6
```

---

## Artifacts

| File | Purpose |
|------|---------|
| `docs/experiments/phase3-build-20260630.md` | Full build log from `rebuild-with-patches.sh` |
| `patches/0001-rmng-boot-banner.patch` | Boot banner patch |
| `scripts/apply-patches.sh` | Reset source + apply series |
| `scripts/rebuild-with-patches.sh` | End-to-end patch + config + build |

---

## Phase 3 verdict: **COMPLETE**
