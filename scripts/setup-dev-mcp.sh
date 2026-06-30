#!/usr/bin/env bash
# Install dev-time MCP config from RMNG-OS example (local machine only)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE="${ROOT}/config/mcp-servers.wsl.example.json"
USER_NAME="${USER:-saini}"
DEST="${HOME}/.config/rmng/mcp-dev.json"

mkdir -p "${HOME}/.config/rmng"
sed "s/REPLACE_USER/${USER_NAME}/g" "${EXAMPLE}" > "${DEST}"

echo "Wrote ${DEST}"
echo "Optional: merge into ~/.cursor/mcp.json for Cursor IDE"
echo ""
echo "Prerequisites:"
echo "  npm/npx  — Node.js"
echo "  uvx      — pip install uv  OR  curl -LsSf https://astral.sh/uv/install.sh | sh"
echo "  gh auth login — for github MCP token"
echo ""
echo "Reference servers: https://github.com/modelcontextprotocol/servers"
