#!/usr/bin/env bash
# Install rmng CLI and rmngd to ~/.cargo/bin
set -euo pipefail
source "${HOME}/.cargo/env" 2>/dev/null || true
cd "${HOME}/dev/projects/RMNG-OS/agents"
cargo build --release
cargo install --path rmng-cli --force
cargo install --path rmngd --force
echo "Installed: $(which rmng) $(which rmngd)"
rmng status
