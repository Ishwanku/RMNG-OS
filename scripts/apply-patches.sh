#!/usr/bin/env bash
# Apply RMNG-OS kernel patch series to KSRC from a clean tree.
set -euo pipefail

SCRIPT="$(readlink -f "$0")"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"
# shellcheck source=/dev/null
source "${HOME}/scripts/kernel-env.sh" 2>/dev/null || source "$ROOT/scripts/kernel-env.sh"

SERIES="$ROOT/patches/series"
PATCH_DIR="$ROOT/patches"

if [ ! -d "$KSRC/.git" ]; then
  echo "ERROR: Kernel source not found at $KSRC" >&2
  exit 1
fi

echo "=== RMNG-OS apply-patches ==="
echo "KSRC: $KSRC"
echo "Series: $SERIES"
echo

echo "--- Reset kernel source to clean state ---"
git -C "$KSRC" checkout -- .
git -C "$KSRC" clean -fd --exclude=.git 2>/dev/null || git -C "$KSRC" clean -fd
make -C "$KSRC" mrproper 2>/dev/null || true
echo

echo "--- Applying patches ---"
cd "$KSRC"
while IFS= read -r patch || [ -n "$patch" ]; do
  [ -z "$patch" ] && continue
  [[ "$patch" =~ ^# ]] && continue
  file="$PATCH_DIR/$patch"
  if [ ! -f "$file" ]; then
    echo "ERROR: missing patch $file" >&2
    exit 1
  fi
  echo "Applying: $patch"
  patch -p1 --forward < "$file"
done < "$SERIES"

echo
echo "Patches applied successfully."
git -C "$KSRC" diff --stat