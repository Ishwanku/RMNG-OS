#!/usr/bin/env bash
# Install rmng CLI, rmngd, systemd user unit, and default config
set -euo pipefail

ROOT="${HOME}/dev/projects/RMNG-OS"
UNIT_SRC="${ROOT}/config/rmngd.service"
UNIT_DST="${HOME}/.config/systemd/user/rmngd.service"
CONFIG_DIR="${HOME}/.rmng"
CONFIG_EXAMPLE="${ROOT}/config/rmng-config.toml.example"

source "${HOME}/.cargo/env" 2>/dev/null || true

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

echo "=== Installing systemd user unit ==="
mkdir -p "${HOME}/.config/systemd/user"
cp "${UNIT_SRC}" "${UNIT_DST}"
systemctl --user daemon-reload
systemctl --user enable rmngd.service
systemctl --user restart rmngd.service

echo "=== Installed ==="
echo "  rmng:  $(command -v rmng)"
echo "  rmngd: $(command -v rmngd)"
echo "  unit:  ${UNIT_DST}"
systemctl --user --no-pager status rmngd.service || true
rmng status
