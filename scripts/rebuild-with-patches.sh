#!/usr/bin/env bash
# Apply RMNG patches, set RMNG LOCALVERSION, and rebuild kernel.
set -euo pipefail

SCRIPT="$(readlink -f "$0")"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"
LOCK_FILE="${KBUILD_LOCK:-/tmp/rmng-kernel-build.lock}"
# shellcheck source=/dev/null
source "${HOME}/scripts/kernel-env.sh" 2>/dev/null || source "$ROOT/scripts/kernel-env.sh"

exec 9>"$LOCK_FILE"
if ! flock -n 9; then
  echo "ERROR: Another kernel build is running (lock: $LOCK_FILE)" >&2
  echo "Wait for it to finish, or remove the lock only if no make process is active." >&2
  exit 1
fi

JOBS="${JOBS:-6}"
REPORT="$ROOT/docs/experiments/phase3-build-$(date +%Y%m%d).md"
mkdir -p "$ROOT/docs/experiments"

{
  echo "# Phase 3 RMNG Kernel Build"
  echo "Date: $(date -Iseconds)"
  echo

  echo "## Step 1: Apply patches"
  "$ROOT/scripts/apply-patches.sh"
  echo

  echo "## Step 2: Configure LOCALVERSION"
  mkdir -p "$KBUILD"
  if [ ! -f "$KBUILD/.config" ]; then
    cp "$ROOT/config/wsl-kernel.config.slim.example" "$KBUILD/.config"
    sed -i '/^# RMNG-OS/d;/^# Baseline/d;/^# Use:/d;/^# Kernel source/d;/^$/d' "$KBUILD/.config" 2>/dev/null || true
  fi
  sed -i 's/CONFIG_LOCALVERSION="-microsoft-standard-WSL2"/CONFIG_LOCALVERSION="-rmng"/' "$KBUILD/.config"
  sed -i 's/CONFIG_LOCALVERSION="-rmng-os"/CONFIG_LOCALVERSION="-rmng"/' "$KBUILD/.config"
  if ! grep -q 'CONFIG_LOCALVERSION="-rmng"' "$KBUILD/.config"; then
    echo 'CONFIG_LOCALVERSION="-rmng"' >> "$KBUILD/.config"
  fi
  make -C "$KSRC" O="$KBUILD" olddefconfig
  echo "LOCALVERSION: $(grep CONFIG_LOCALVERSION= "$KBUILD/.config" | grep -v AUTO)"
  make -C "$KSRC" O="$KBUILD" kernelrelease 2>/dev/null || \
    make -C "$KSRC" O="$KBUILD" include/config/kernel.release 2>/dev/null || true
  echo "kernel.release: $(cat "$KBUILD/include/config/kernel.release" 2>/dev/null || echo pending)"
  echo

  echo "## Step 3: Build"
  START=$(date +%s.%N)
  make -C "$KSRC" O="$KBUILD" -j"$JOBS"
  END=$(date +%s.%N)
  ELAPSED=$(awk -v s="$START" -v e="$END" 'BEGIN{printf "%.2f", e-s}')
  echo "Elapsed: ${ELAPSED}s"
  echo

  echo "## Step 4: Verify"
  ls -lh "$KBUILD/vmlinux"
  strings "$KBUILD/vmlinux" | grep -E 'RMNG-OS|Linux version' | head -5
  echo "kernel.release file: $(cat "$KBUILD/include/config/kernel.release" 2>/dev/null || echo n/a)"
  echo "patched source: $(git -C "$KSRC" diff --stat init/main.c 2>/dev/null || echo n/a)"
} | tee "$REPORT"

echo "Report: $REPORT"