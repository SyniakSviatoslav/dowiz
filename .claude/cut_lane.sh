#!/bin/bash
# Cut a lane worktree from the CURRENT INDEX (== HEAD when the tree is clean).
# Usage: cut_lane.sh <lane-name>
# Prints the base commit it cut from. Verify it before handing the lane out.
set -euo pipefail
LANE="${1:?usage: cut_lane.sh <lane-name>}"
ROOT=/root/dowiz
DEST="$ROOT/.claude/lanes/$LANE"
rm -rf "$DEST"
mkdir -p "$DEST"
cd "$ROOT"
git ls-files -z bebop-lang | git checkout-index --prefix="$DEST/" -f -z --stdin
# flatten the bebop-lang/ prefix
mv "$DEST/bebop-lang"/* "$DEST"/ 2>/dev/null || true
mv "$DEST/bebop-lang"/.[!.]* "$DEST"/ 2>/dev/null || true
rmdir "$DEST/bebop-lang" 2>/dev/null || true
# rust oracles need crates as a SIBLING of the flattened tree
ln -sfn "$ROOT/crates" "$(dirname "$DEST")/crates"
ln -sfn "$ROOT/crates" "$DEST/crates"
echo "lane=$LANE dest=$DEST base=$(git rev-parse --short HEAD) ($(git log -1 --format=%s))"
