#!/usr/bin/env bash
# Idempotent RMNG-OS development environment setup (WSL2 Ubuntu 24.04)
# Aligns with ADR-009/010/014/015 — nervous/body separation preserved.
set -euo pipefail

SCRIPT="$(readlink -f "$0")"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"

echo "============================================"
echo " RMNG-OS dev-environment-setup (idempotent)"
echo " Repo: ${ROOT}"
echo "============================================"

# --- Directory layout ---
echo "[1/6] Directory structure"
mkdir -p \
  "${HOME}/dev/projects" \
  "${HOME}/dev/kernel" \
  "${HOME}/dev/tools" \
  "${HOME}/build/kernel" \
  "${HOME}/build/out" \
  "${HOME}/scripts" \
  "${HOME}/src" \
  "${HOME}/dotfiles" \
  "${HOME}/.config/rmng" \
  "${HOME}/.ccache"

# --- Workspace symlinks + ccache ---
echo "[2/6] Workspace wiring"
"${ROOT}/scripts/workspace-setup.sh"

# --- ~/.rmng runtime home ---
echo "[3/6] RMNG runtime home"
chmod +x "${ROOT}/scripts/setup-rmng-home.sh"
"${ROOT}/scripts/setup-rmng-home.sh"

# --- Shell snippets (idempotent) ---
echo "[4/6] Shell configuration"
append_snippet() {
  local marker="$1"
  local snippet="$2"
  if ! grep -qF "${marker}" "${HOME}/.bashrc" 2>/dev/null; then
    echo "" >> "${HOME}/.bashrc"
    cat "${snippet}" >> "${HOME}/.bashrc"
    echo "  Appended: $(basename "${snippet}")"
  else
    echo "  Skip (already in .bashrc): $(basename "${snippet}")"
  fi
}
append_snippet "RMNG-OS development" "${ROOT}/dotfiles/bashrc.rmng.snippet"

# --- Rust (minimal — only if missing) ---
echo "[5/6] Rust toolchain"
if ! command -v rustc &>/dev/null; then
  echo "  Installing rustup (non-interactive)..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck source=/dev/null
  source "${HOME}/.cargo/env"
else
  echo "  rustc: $(rustc --version)"
fi

# --- Executable bits ---
chmod +x "${ROOT}/scripts/"*.sh 2>/dev/null || true

echo "[6/6] Prerequisite report"
"${ROOT}/scripts/check-dev-prerequisites.sh" || true

echo
echo "============================================"
echo " Setup complete."
echo
echo " Next steps:"
echo "   source ~/.bashrc"
echo "   ./scripts/install-rmng.sh      # build rmng + rmngd"
echo "   ./scripts/setup-dev-mcp.sh     # IDE MCP (dev only)"
echo "   docs/INTEGRATION-STRATEGY.md   # future repo integration"
echo "============================================"