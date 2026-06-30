#!/usr/bin/env bash
# Out-of-tree kernel build wrapper
# Usage: ./build.sh [make targets...]   (default: -j6)
set -euo pipefail

SCRIPT="$(readlink -f "$0")"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"
# shellcheck source=/dev/null
source "${HOME}/scripts/kernel-env.sh" 2>/dev/null || source "$ROOT/scripts/kernel-env.sh"

JOBS="${JOBS:-6}"
TARGETS=("$@")
if [ ${#TARGETS[@]} -eq 0 ]; then
  TARGETS=("-j${JOBS}")
fi

if [ ! -d "$KSRC" ]; then
  echo "ERROR: Kernel source not found at $KSRC" >&2
  exit 1
fi

mkdir -p "$KBUILD"

if [ ! -f "$KBUILD/.config" ]; then
  echo "No .config found. Copying example config..."
  cp "$ROOT/config/wsl-kernel.config.example" "$KBUILD/.config"
  make -C "$KSRC" O="$KBUILD" olddefconfig
fi

echo "Building: make O=$KBUILD ${TARGETS[*]}"
make -C "$KSRC" O="$KBUILD" "${TARGETS[@]}"