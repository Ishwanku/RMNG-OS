#!/usr/bin/env bash
# One-time (or repeat) workspace wiring for RMNG-OS development
set -euo pipefail

SCRIPT="$(readlink -f "$0")"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"

echo "=== RMNG-OS workspace setup ==="

# Standard directories
mkdir -p "$HOME/dev/kernel" "$HOME/build/kernel" "$HOME/scripts" "$HOME/dotfiles"

# Symlink kernel-env.sh into ~/scripts
ln -sf "$ROOT/scripts/kernel-env.sh" "$HOME/scripts/kernel-env.sh"
echo "Linked: ~/scripts/kernel-env.sh -> repo"

# Symlink helper scripts
for script in status.sh build.sh; do
  ln -sf "$ROOT/scripts/$script" "$HOME/scripts/rmng-$script"
  chmod +x "$ROOT/scripts/$script"
  echo "Linked: ~/scripts/rmng-$script"
done

chmod +x "$ROOT/scripts/kernel-env.sh" "$ROOT/scripts/make-config-example.sh"

# ccache snippet (idempotent)
SNIPPET="$ROOT/dotfiles/bashrc.ccache.snippet"
if ! grep -q "Kernel build / ccache" "$HOME/.bashrc" 2>/dev/null; then
  echo "" >> "$HOME/.bashrc"
  cat "$SNIPPET" >> "$HOME/.bashrc"
  echo "Appended ccache snippet to ~/.bashrc"
else
  echo "ccache snippet already in ~/.bashrc"
fi

# Kernel source check
if [ ! -d "$HOME/dev/kernel/linux/.git" ]; then
  echo
  echo "NEXT: Clone kernel source:"
  echo "  git clone --depth=1 https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git ~/dev/kernel/linux"
else
  echo "Kernel source: OK ($(du -sh "$HOME/dev/kernel/linux" | cut -f1))"
fi

echo
echo "Setup complete. Run: ~/scripts/rmng-status.sh"