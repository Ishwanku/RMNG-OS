#!/usr/bin/env bash
# One-time helper to regenerate config example from local build
set -euo pipefail
SRC="${1:-$HOME/build/kernel/.config}"
DEST="$(dirname "$0")/../config/wsl-kernel.config.example"
{
  cat <<'EOF'
# RMNG-OS example kernel config
# Baseline derived from WSL2 (sanitized). Use: make O=$KBUILD olddefconfig
# Kernel source is GPLv2 — clone torvalds/linux separately.

EOF
  sed 's/CONFIG_LOCALVERSION="-microsoft-standard-WSL2"/CONFIG_LOCALVERSION="-rmng"/' "$SRC"
} > "$DEST"
echo "Wrote $DEST ($(wc -l < "$DEST") lines)"