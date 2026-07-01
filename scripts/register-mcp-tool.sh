#!/usr/bin/env bash
# Register or update an MCP server in ~/.rmng/mcp-allowlist.toml
# Usage:
#   ./scripts/register-mcp-tool.sh <server> <command> [args...] --tools tool1,tool2
# Example:
#   ./scripts/register-mcp-tool.sh git uvx mcp-server-git --repository ~/dev/projects/RMNG-OS --tools git.log
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ALLOWLIST="${HOME}/.rmng/mcp-allowlist.toml"
EDITOR_PY="${ROOT}/scripts/lib/allowlist-edit.py"

usage() {
  cat <<'EOF'
Usage: register-mcp-tool.sh <server_name> <command> [args ...] --tools <t1,t2,...>

Registers a server for rmngd MCP proxy (production path only).
IDE dev MCP belongs in ~/.config/rmng/mcp-dev.json via setup-dev-mcp.sh.

Options:
  --tools <csv>     Required. Comma-separated tool IDs (e.g. git.log,get_issue)
  --disable         Mark server enabled = false
  --dry-run         Print planned entry without writing

After registration:
  systemctl --user restart rmngd
  rmng status

See docs/INTEGRATION-STRATEGY.md (Track 2: MCP Proxy Plane).
EOF
}

if [[ $# -lt 3 ]]; then
  usage
  exit 1
fi

SERVER="$1"
COMMAND="$2"
shift 2

TOOLS_CSV=""
DISABLE=""
DRY_RUN=""

ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --tools)
      TOOLS_CSV="$2"
      shift 2
      ;;
    --disable)
      DISABLE="--disable"
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      ARGS+=("$1")
      shift
      ;;
  esac
done

if [[ -z "${TOOLS_CSV}" ]]; then
  echo "ERROR: --tools is required"
  usage
  exit 1
fi

IFS=',' read -ra TOOLS <<< "${TOOLS_CSV}"

if [[ -n "${DRY_RUN}" ]]; then
  echo "Would register servers.${SERVER}:"
  echo "  command: ${COMMAND}"
  echo "  args:    ${ARGS[*]}"
  echo "  tools:   ${TOOLS[*]}"
  exit 0
fi

mkdir -p "${HOME}/.rmng/allowlists"
if [[ -f "${ALLOWLIST}" ]]; then
  cp "${ALLOWLIST}" "${HOME}/.rmng/allowlists/mcp-allowlist.$(date +%Y%m%d-%H%M%S).bak"
fi

python3 "${EDITOR_PY}" \
  --file "${ALLOWLIST}" \
  --server "${SERVER}" \
  --command "${COMMAND}" \
  --args "${ARGS[@]}" \
  --tools "${TOOLS[@]}" \
  ${DISABLE}

echo "Wrote ${ALLOWLIST}"
echo "Restart rmngd: systemctl --user restart rmngd"
echo "Document in:   docs/integrations/<server>.md (use TEMPLATE.md)"