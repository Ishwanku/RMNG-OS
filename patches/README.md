# RMNG-OS Kernel Patches

Out-of-tree patch series applied to `~/dev/kernel/linux` before build.

**License:** GPLv2 — see [LICENSE.kernel-patches](../LICENSE.kernel-patches) and [ADR-019](../docs/decisions/ADR-019-licensing-and-layering.md).

## Series

| # | Patch | Description |
|---|-------|-------------|
| 1 | `0001-rmng-boot-banner.patch` | RMNG-OS boot banner in `init/main.c` |

## Apply manually

```bash
cd ~/dev/kernel/linux
git checkout -- .          # reset to clean source
patch -p1 < ~/dev/projects/RMNG-OS/patches/0001-rmng-boot-banner.patch
```

## Apply via script

```bash
~/dev/projects/RMNG-OS/scripts/apply-patches.sh
~/dev/projects/RMNG-OS/scripts/rebuild-with-patches.sh
```

## Add a new patch

1. Edit kernel source in `~/dev/kernel/linux`
2. `git diff > ~/dev/projects/RMNG-OS/patches/0002-description.patch`
3. Add filename to `patches/series`
4. Rebuild and verify