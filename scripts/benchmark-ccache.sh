#!/usr/bin/env bash
# Incremental rebuild benchmark after touching init/main.c
set -euo pipefail

SCRIPT="$(readlink -f "$0")"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"
# shellcheck source=/dev/null
source "${HOME}/scripts/kernel-env.sh" 2>/dev/null || source "$ROOT/scripts/kernel-env.sh"

REPORT="${1:-$ROOT/docs/benchmarks/phase2-ccache-$(date +%Y%m%d).txt}"
mkdir -p "$(dirname "$REPORT")"

{
  echo "=== RMNG-OS ccache Incremental Benchmark ==="
  echo "Date: $(date -Iseconds)"
  echo "KSRC: $KSRC"
  echo "KBUILD: $KBUILD"
  echo

  echo "--- Clean source tree markers (OOT-safe) ---"
  make -C "$KSRC" mrproper
  echo

  echo "--- Touch target ---"
  touch "$KSRC/init/main.c"
  ls -l "$KSRC/init/main.c"
  echo

  echo "--- ccache BEFORE ---"
  ccache -s
  echo

  echo "--- Incremental build (make -j${JOBS:-6}) ---"
  START=$(date +%s.%N)
  make -C "$KSRC" O="$KBUILD" -j"${JOBS:-6}"
  END=$(date +%s.%N)
  ELAPSED=$(awk -v s="$START" -v e="$END" 'BEGIN{printf "%.2f", e-s}')
  echo
  echo "Elapsed seconds: $ELAPSED"
  echo

  echo "--- ccache AFTER ---"
  ccache -s
  echo

  echo "--- vmlinux ---"
  ls -lh "$KBUILD/vmlinux"
} | tee "$REPORT"

echo "Report written: $REPORT"