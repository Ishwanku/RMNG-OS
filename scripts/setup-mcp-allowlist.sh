#!/usr/bin/env bash
# Install MCP allowlist for rmngd proxy (local machine only)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE="${ROOT}/config/mcp-allowlist.toml.example"
DEST="${HOME}/.rmng/mcp-allowlist.toml"
REPO="${HOME}/dev/projects/RMNG-OS"
UVX="$(command -v uvx 2>/dev/null || echo "${HOME}/.local/bin/uvx")"
NPX="$(command -v npx 2>/dev/null || echo "npx")"

mkdir -p "${HOME}/.rmng"

# Inject repository path for git MCP server (required for git_log)
cat > "${DEST}" <<EOF
[servers.github]
enabled = true
command = "${NPX}"
args = ["-y", "@github/github-mcp-server"]
allowed_tools = ["get_issue", "create_issue"]

[servers.git]
enabled = true
command = "${UVX}"
args = ["mcp-server-git", "--repository", "${REPO}"]
allowed_tools = ["git.log"]
EOF

echo "Wrote ${DEST}"
echo "Restart rmngd after changes: systemctl --user restart rmngd"