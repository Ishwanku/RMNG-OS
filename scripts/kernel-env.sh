#!/usr/bin/env bash
# Source this before kernel builds: source ~/scripts/kernel-env.sh
export KSRC="${KSRC:-$HOME/dev/kernel/linux}"
export KBUILD="${KBUILD:-$HOME/build/kernel}"
export CCACHE_DIR="${CCACHE_DIR:-$HOME/.ccache}"
export CC="ccache gcc"
export CXX="ccache g++"
export PATH="/usr/lib/ccache:$PATH"
ccache --max-size=10G 2>/dev/null

echo "Kernel build environment ready:"
echo "  KSRC  = $KSRC   (kernel source)"
echo "  KBUILD= $KBUILD (out-of-tree build dir)"
echo "  CCACHE= $CCACHE_DIR"