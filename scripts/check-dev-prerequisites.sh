#!/usr/bin/env bash
# Verify minimal toolchain for RMNG-OS (kernel + agents). No installs — report only.
set -euo pipefail

OK=0
WARN=0
FAIL=0

check() {
  local label="$1"
  local cmd="$2"
  if command -v "$cmd" &>/dev/null; then
    echo "  OK   $label ($(command -v "$cmd"))"
    OK=$((OK + 1))
  else
    echo "  MISS $label ($cmd)"
    FAIL=$((FAIL + 1))
  fi
}

warn_if_missing() {
  local label="$1"
  local cmd="$2"
  if command -v "$cmd" &>/dev/null; then
    echo "  OK   $label"
    OK=$((OK + 1))
  else
    echo "  OPT  $label (optional — $cmd)"
    WARN=$((WARN + 1))
  fi
}

echo "=== RMNG-OS prerequisite check ==="

echo "[Kernel build]"
check "gcc" gcc
check "make" make
check "ccache" ccache
check "git" git
check "python3" python3

echo "[Rust agents]"
check "rustc" rustc
check "cargo" cargo
warn_if_missing "rustfmt" rustfmt
warn_if_missing "clippy" cargo-clippy

echo "[MCP / integrations]"
warn_if_missing "node/npx" npx
warn_if_missing "uv/uvx" uvx
warn_if_missing "gh CLI" gh

echo "[Paths]"
for d in ~/dev/projects/RMNG-OS ~/dev/kernel/linux ~/build/kernel ~/.rmng; do
  if [[ -e "$d" ]]; then
    echo "  OK   $d"
    OK=$((OK + 1))
  else
    echo "  MISS $d"
    FAIL=$((FAIL + 1))
  fi
done

echo
echo "Summary: $OK ok, $WARN optional missing, $FAIL required missing"
if [[ $FAIL -gt 0 ]]; then
  echo "Run: ./scripts/dev-environment-setup.sh"
  exit 1
fi