#!/bin/sh
# The committed JavaScript must be what the TypeScript actually produces.
#
# `workers/api/public` is served to the browser as it sits in the tree, so the
# emitted `.js` is committed rather than built at deploy time -- a build that
# has not run yet would serve an empty route. That is the same bargain
# `kernel/src/eqc_gen.rs` makes, and it fails the same way if nobody checks:
# somebody edits the `.js` by hand, the `.ts` beside it still says something
# else, and the next compile silently reverts the fix.
#
# So: compile into a scratch copy and diff. Exit 1 names every file that drifted.
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
PUB="$ROOT/workers/api/public"
TSC="$ROOT/node_modules/typescript/bin/tsc"

[ -x "$(command -v node)" ] || { echo "ts-parity: node not on PATH"; exit 2; }
[ -f "$TSC" ] || { echo "ts-parity: typescript not installed (npm install typescript)"; exit 2; }

count=$(find "$PUB" -name '*.ts' ! -name '*.d.ts' | wc -l)
if [ "$count" -eq 0 ]; then
  echo "ts-parity: no TypeScript sources yet — nothing to check"
  exit 0
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
node "$TSC" -p "$PUB/tsconfig.json" --outDir "$WORK" >/dev/null

drift=0
find "$WORK" -name '*.js' | while read -r built; do
  rel=${built#"$WORK"/}
  committed="$PUB/$rel"
  if [ ! -f "$committed" ]; then
    echo "ts-parity: MISSING  $rel — the source compiles to a file nobody committed"
    drift=1
  elif ! diff -q "$built" "$committed" >/dev/null; then
    echo "ts-parity: DRIFTED  $rel — the committed .js is not what the .ts produces"
    diff -u "$committed" "$built" | head -20
    drift=1
  fi
  [ "$drift" -eq 0 ] || exit 1
done || exit 1

echo "ts-parity: $count TypeScript source(s), every committed .js matches"
