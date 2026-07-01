#!/usr/bin/env bash
# Install rmng CLI, rmngd, systemd user unit, and default config
set -euo pipefail

# Override before running: RMNG_PROJECT_ROOT=/path/to/clone ./scripts/install-rmng.sh
ROOT="${RMNG_PROJECT_ROOT:-${HOME}/dev/projects/RMNG-OS}"
UNIT_TEMPLATE="${ROOT}/config/rmngd.service.in"
UNIT_DST="${HOME}/.config/systemd/user/rmngd.service"
CONFIG_DIR="${HOME}/.rmng"
CONFIG_EXAMPLE="${ROOT}/config/rmng-config.toml.example"

source "${HOME}/.cargo/env" 2>/dev/null || true

if [[ ! -d "${ROOT}/agents" ]]; then
  echo "ERROR: RMNG project root not found: ${ROOT}" >&2
  echo "  Set RMNG_PROJECT_ROOT or clone to ~/dev/projects/RMNG-OS" >&2
  exit 1
fi

echo "=== Building release binaries ==="
cd "${ROOT}/agents"
cargo build --release
cargo install --path rmng-cli --force
cargo install --path rmngd --force

echo "=== Installing BYO-LLM config (if missing) ==="
mkdir -p "${CONFIG_DIR}"
if [[ ! -f "${CONFIG_DIR}/config.toml" ]]; then
  cp "${CONFIG_EXAMPLE}" "${CONFIG_DIR}/config.toml"
  echo "Created ${CONFIG_DIR}/config.toml (llm_provider = none)"
fi

echo "=== Installing MCP allowlist ==="
"${ROOT}/scripts/setup-mcp-allowlist.sh"

echo "=== Validating startup (rmngd --validate) ==="
export RMNG_PROJECT_ROOT="${ROOT}"
VALIDATE_OK=1
if ! rmngd --validate; then
  VALIDATE_OK=0
  echo "" >&2
  echo "ERROR: validation reported ERROR items — fix before enabling rmngd" >&2
  echo "  Docs: ${ROOT}/docs/integrations/operations-usage.md" >&2
  echo "  Hint: export RMNG_PROJECT_ROOT=${ROOT}" >&2
fi

echo "=== Installing systemd user unit ==="
if [[ ! -f "${UNIT_TEMPLATE}" ]]; then
  echo "ERROR: missing unit template: ${UNIT_TEMPLATE}" >&2
  exit 1
fi
mkdir -p "${HOME}/.config/systemd/user"
sed -e "s|@HOME@|${HOME}|g" -e "s|@RMNG_PROJECT_ROOT@|${ROOT}|g" \
  "${UNIT_TEMPLATE}" > "${UNIT_DST}"
echo "  Generated ${UNIT_DST}"
echo "  RMNG_PROJECT_ROOT=${ROOT}"
systemctl --user daemon-reload
systemctl --user enable rmngd.service

if [[ "${VALIDATE_OK}" -eq 1 ]]; then
  systemctl --user restart rmngd.service
else
  echo "" >&2
  echo "SKIP: not restarting rmngd — validation failed (avoids systemd restart loop)" >&2
  echo "  After fixing errors: rmngd --validate && systemctl --user restart rmngd" >&2
fi

echo "=== Installed ==="
echo "  rmng:  $(command -v rmng)"
echo "  rmngd: $(command -v rmngd)"
echo "  unit:  ${UNIT_DST}"
if [[ "${VALIDATE_OK}" -eq 1 ]]; then
  systemctl --user --no-pager status rmngd.service || true
fi
rmng status
echo ""
echo "Monitoring:"
echo "  rmng health --json --strict     # production liveness probe"
echo "  rmng health --require-daemon    # daemon-only check"
echo "  rmng observe --json             # cost, circuits, sessions"
echo "  journalctl --user -u rmngd -f"