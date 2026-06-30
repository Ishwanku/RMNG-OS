#!/usr/bin/env bash
set -euo pipefail
SCRIPT="$(readlink -f "$0")"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"
KBUILD="${KBUILD:-$HOME/build/kernel}"
DEST="$ROOT/config/wsl-kernel.config.slim.example"
{
  cat <<'EOF'
# RMNG-OS slim kernel config (localmodconfig)
# Use: make O=$KBUILD olddefconfig
# Kernel source is GPLv2 — clone torvalds/linux separately.

EOF
  sed 's/CONFIG_LOCALVERSION="-microsoft-standard-WSL2"/CONFIG_LOCALVERSION="-rmng"/' "$KBUILD/.config"
} > "$DEST"
echo "Wrote $DEST ($(wc -l < "$DEST") lines)"