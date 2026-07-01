#!/usr/bin/env bash
# Idempotent ~/.rmng runtime home layout (ADR-010, ADR-014)
set -euo pipefail

RMNG_HOME="${HOME}/.rmng"

echo "=== RMNG runtime home: ${RMNG_HOME} ==="

mkdir -p \
  "${RMNG_HOME}/logs" \
  "${RMNG_HOME}/sessions" \
  "${RMNG_HOME}/allowlists" \
  "${RMNG_HOME}/cache" \
  "${RMNG_HOME}/state"

# Primary config (BYO-LLM) — seed from example if missing
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE="${ROOT}/config/rmng-config.toml.example"
if [[ ! -f "${RMNG_HOME}/config.toml" ]]; then
  cp "${EXAMPLE}" "${RMNG_HOME}/config.toml"
  echo "Created ${RMNG_HOME}/config.toml"
else
  echo "Exists: ${RMNG_HOME}/config.toml"
fi

# MCP production allowlist — seed if missing
if [[ ! -f "${RMNG_HOME}/mcp-allowlist.toml" ]]; then
  "${ROOT}/scripts/setup-mcp-allowlist.sh"
else
  echo "Exists: ${RMNG_HOME}/mcp-allowlist.toml"
fi

# Document subdirs
cat > "${RMNG_HOME}/README.txt" <<'EOF'
RMNG-OS runtime home (~/.rmng)
================================
config.toml           BYO-LLM / nervous-system settings
mcp-allowlist.toml    Production MCP proxy allowlist (rmngd)
rmngd.sock            Unix socket (created by rmngd)
logs/audit.jsonl      Permission + dispatch audit trail
sessions/             Ephemeral agent session artifacts
allowlists/           Snapshots / exports of allowlist changes
cache/                Runtime caches (non-authoritative)
state/                Local runtime state (Phase 7+)

Dev IDE MCP config lives separately at ~/.config/rmng/mcp-dev.json
EOF

chmod 700 "${RMNG_HOME}"
chmod 700 "${RMNG_HOME}/logs" "${RMNG_HOME}/sessions" 2>/dev/null || true

echo "RMNG home ready."