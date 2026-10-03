#!/bin/sh
# ci-refs' proof: a gate is triggered before it is trusted.
#   1. clean                                             -> must pass
#   2. a secret the workflows read, unnamed in the docs  -> must refuse
#   3. a run step naming a script that is not there      -> must refuse
#   4. a workflow that does not parse                    -> must refuse
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
R="$SCRATCH/r"
copy() {
  rm -rf "$R"; mkdir -p "$R/docs" "$R/.github"
  for x in "$REPO"/* "$REPO/.claude"; do
    case "$(basename "$x")" in docs) ;; *) ln -s "$x" "$R/$(basename "$x")" ;; esac
  done
  cp -r "$REPO/.github/workflows" "$R/.github/workflows"
  cp "$REPO/docs/operations.md" "$R/docs/operations.md"
}
want() { # want <rc> <label>
  python3 "$HERE/ci-refs.py" "$R" >"$SCRATCH/out" 2>&1; rc=$?
  echo "prove: $2 -> rc=$rc (want $1): $(tail -1 "$SCRATCH/out")"
  [ "$rc" -eq "$1" ] || { cat "$SCRATCH/out"; fail=1; }
}
copy; want 0 clean
copy; sed -i 's/`WALK_LOC`/`WALK_L0C`/' "$R/docs/operations.md"; want 1 "undocumented secret"
copy; sed -i 's#tools/live-checks/health.sh#tools/live-checks/health-gone.sh#' "$R/.github/workflows/health-cron.yml"; want 1 "missing script"
copy; printf 'on: [push]\njobs:\n  x: [\n' > "$R/.github/workflows/zz-broken.yml"; want 1 "unparseable workflow"
[ "$fail" -eq 0 ] && echo "ci-refs.prove: GREEN -- 4 of 4 cases" || echo "ci-refs.prove: RED"
exit "$fail"
