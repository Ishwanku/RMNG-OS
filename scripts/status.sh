#!/usr/bin/env bash
# Quick project status — run from anywhere: ~/dev/projects/RMNG-OS/scripts/status.sh
set -euo pipefail

SCRIPT="$(readlink -f "$0")"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"
# shellcheck source=/dev/null
source "${HOME}/scripts/kernel-env.sh" 2>/dev/null || source "$ROOT/scripts/kernel-env.sh"

echo "=== RMNG-OS Status ==="
echo "Date:    $(date)"
echo "Host:    $(uname -n) — $(lsb_release -ds 2>/dev/null || echo unknown)"
echo "Kernel:  $(uname -r)"
echo "CPUs:    $(nproc)"
free -h | awk 'NR==2{printf "RAM:     %s total, %s available\n", $2, $7}'
echo
echo "=== Paths ==="
echo "Project: $ROOT"
echo "KSRC:    $KSRC"
echo "KBUILD:  $KBUILD"
echo
echo "=== Kernel Source ==="
if [ -d "$KSRC/.git" ]; then
  echo "Commit:  $(git -C "$KSRC" describe --always 2>/dev/null || echo unknown)"
  echo "Size:    $(du -sh "$KSRC" 2>/dev/null | cut -f1)"
else
  echo "MISSING — clone: git clone --depth=1 https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git $KSRC"
fi
echo
echo "=== Build ==="
if [ -f "$KBUILD/vmlinux" ]; then
  ls -lh "$KBUILD/vmlinux"
  echo "Build dir: $(du -sh "$KBUILD" 2>/dev/null | cut -f1)"
else
  echo "vmlinux: not built yet"
fi
echo "gcc workers: $(pgrep -c gcc 2>/dev/null || echo 0)"
echo
echo "=== ccache ==="
ccache -s 2>/dev/null | head -6 || echo "ccache not available"
echo
echo "=== Git (RMNG-OS) ==="
git -C "$ROOT" status -sb 2>/dev/null || echo "not a git repo"