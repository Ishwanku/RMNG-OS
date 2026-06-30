#!/usr/bin/env bash
# Slim kernel config to modules/drivers actually in use (localmodconfig)
set -euo pipefail

SCRIPT="$(readlink -f "$0")"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"
# shellcheck source=/dev/null
source "${HOME}/scripts/kernel-env.sh" 2>/dev/null || source "$ROOT/scripts/kernel-env.sh"

if [ ! -f "$KBUILD/.config" ]; then
  echo "ERROR: No .config at $KBUILD/.config" >&2
  exit 1
fi

cp "$KBUILD/.config" "$KBUILD/.config.full-backup"
echo "Backup: $KBUILD/.config.full-backup"

echo "Cleaning source tree markers (out-of-tree safe)..."
make -C "$KSRC" mrproper

echo "Running localmodconfig (accepting defaults for prompts)..."
yes '' | make -C "$KSRC" O="$KBUILD" localmodconfig || true

LINES_BEFORE=$(wc -l < "$KBUILD/.config.full-backup")
LINES_AFTER=$(wc -l < "$KBUILD/.config")
echo "Config lines: $LINES_BEFORE -> $LINES_AFTER"

# Save slim example to repo
DEST="$ROOT/config/wsl-kernel.config.slim.example"
{
  cat <<'EOF'
# RMNG-OS slim kernel config (localmodconfig)
# Generated from running kernel modules. Use: make O=$KBUILD olddefconfig
# Kernel source is GPLv2 — clone torvalds/linux separately.

EOF
  sed 's/CONFIG_LOCALVERSION="-microsoft-standard-WSL2"/CONFIG_LOCALVERSION="-rmng"/' "$KBUILD/.config"
} > "$DEST"
echo "Wrote: $DEST ($(wc -l < "$DEST") lines)"