#!/bin/sh
# COVERAGE RATCHET — line coverage of crates/dowiz-hub and workers/api, measured, never claimed.
#
# WHY THIS EXISTS. `scripts/coverage.sh` printed "All passing: YES" as a literal for months: it ran
# nothing it reported. The operator's rule (2026-09-24) is that dowiz is covered by tests, and a rule
# nobody measures is a wish. This gate measures it with the compiler's own instrumentation
# (`-C instrument-coverage`, the same profile format `cargo llvm-cov` reads) and fails when either
# crate FALLS below its baseline in tools/gates/coverage.baseline.
#
#   sh tools/gates/coverage.sh              # measure both, compare, rc 0/1
#   sh tools/gates/coverage.sh worker       # only one crate (hub | worker)
#   On the dev box, through the slot:  bash bebop-lang/tools/slot.sh cov sh tools/gates/coverage.sh
#
# A GATE THAT CANNOT MEASURE NEVER PASSES. Each of these is a named refusal with its own exit code,
# because an empty report, a missing tool or a profile nobody wrote all look like "0 lines missed":
#   rc 2  bad argument / baseline file missing or unreadable
#   rc 3  llvm-profdata or llvm-cov not found, or their LLVM major differs from rustc's
#   rc 4  the tests ran and wrote no .profraw (instrumentation did not happen)
#   rc 5  the report has no row under the crate's src/ or counts 0 lines (measured nothing)
#   rc 6  the crate's tests FAILED (coverage of a red suite is not a number worth comparing)
#   rc 1  a crate fell below its baseline
#
# WHAT IS COUNTED. Lines (llvm-cov's "Lines" column) of every file under <crate>/src, test modules
# included — the same definition the first measurement (main a7fcbd4d: hub 94.8, worker 62.8) used,
# so the baseline and the gate agree on what a percent is. Dependencies are excluded by passing the
# crate's src/ as the report's source filter; the TOTAL row is then the crate alone.
#
# ENV: LLVM_PROFDATA / LLVM_COV override the tools (CI: rustup's llvm-tools). COV_DIR keeps the
# target dir and profiles between runs (an instrumented build is heavy; reuse it). Default: a dir
# under ${TMPDIR:-/tmp} that is removed on exit.
set -u
cd "$(dirname "$0")/../.." || exit 2
ROOT=$(pwd)
BASE=tools/gates/coverage.baseline
[ -r "$BASE" ] || { echo "coverage: REFUSED — baseline $BASE missing"; exit 2; }

PD=${LLVM_PROFDATA:-llvm-profdata}
LC=${LLVM_COV:-llvm-cov}
for t in "$PD" "$LC"; do
  command -v "$t" >/dev/null 2>&1 || { echo "coverage: REFUSED — $t not found (install llvm, or set LLVM_PROFDATA/LLVM_COV)"; exit 3; }
done
want=$(rustc -vV 2>/dev/null | sed -n 's/^LLVM version: \([0-9]*\).*/\1/p')
have=$("$PD" --version 2>/dev/null | sed -n 's/.*LLVM version \([0-9]*\).*/\1/p' | head -1)
if [ -z "$want" ] || [ "$want" != "$have" ]; then
  echo "coverage: REFUSED — rustc is LLVM '$want', $PD is LLVM '$have'; profiles would not merge"
  exit 3
fi

if [ -n "${COV_DIR:-}" ]; then D=$COV_DIR; mkdir -p "$D"; else
  D=$(mktemp -d "${TMPDIR:-/tmp}/dowiz-cov.XXXXXX") || exit 2
  trap 'rm -rf "$D"' EXIT
fi

only=${1:-}
case "$only" in ''|hub|worker) ;; *) echo "coverage: REFUSED — unknown crate '$only' (hub | worker)"; exit 2 ;; esac

# measure <name> <crate dir>  -> prints the percent on stdout, or exits with a refusal code.
measure() {
  name=$1; dir=$2
  prof="$D/prof-$name"; rm -rf "$prof"; mkdir -p "$prof"
  log="$D/$name.test.log"
  ( cd "$ROOT/$dir" && LLVM_PROFILE_FILE="$prof/%p-%m.profraw" RUSTFLAGS="-C instrument-coverage" \
      CARGO_TARGET_DIR="$D/target-$name" cargo test --lib > "$log" 2>&1 )
  rc=$?
  if [ $rc -ne 0 ]; then
    echo "coverage: REFUSED — $name tests failed (rc=$rc); last lines of $log:" >&2
    tail -5 "$log" >&2
    exit 6
  fi
  n=$(ls "$prof" 2>/dev/null | grep -c '\.profraw$')
  [ "$n" -gt 0 ] || { echo "coverage: REFUSED — $name wrote no .profraw under $prof" >&2; exit 4; }
  # The lookup can rebuild, and an instrumented build script writes a profile wherever it runs
  # unless told where: into the crate directory, as default_*.profraw. Kept out of the tree.
  bin=$( cd "$ROOT/$dir" && LLVM_PROFILE_FILE="$D/build-%p-%m.profraw" RUSTFLAGS="-C instrument-coverage" CARGO_TARGET_DIR="$D/target-$name" \
      cargo test --lib --no-run --message-format=json 2>/dev/null \
      | grep -o '"executable":"[^"]*"' | tail -1 | sed 's/"executable":"\(.*\)"/\1/' )
  [ -n "$bin" ] && [ -x "$bin" ] || { echo "coverage: REFUSED — $name test binary not found" >&2; exit 5; }
  "$PD" merge -sparse "$prof"/*.profraw -o "$D/$name.profdata" 2>"$D/$name.merge.err" \
    || { echo "coverage: REFUSED — $name profdata merge failed: $(head -1 "$D/$name.merge.err")" >&2; exit 4; }
  "$LC" report "$bin" -instr-profile="$D/$name.profdata" "$ROOT/$dir/src" > "$D/$name.report" 2>&1
  rows=$(grep -c "^.*\.rs " "$D/$name.report")
  # TOTAL: Regions Missed Cover Functions Missed Executed Lines Missed Cover ...
  lines=$(awk '$1=="TOTAL"{print $8}' "$D/$name.report")
  missed=$(awk '$1=="TOTAL"{print $9}' "$D/$name.report")
  if [ "${rows:-0}" -eq 0 ] || [ -z "$lines" ] || [ "$lines" -eq 0 ]; then
    echo "coverage: REFUSED — $name report counts no lines under $dir/src ($rows rows)" >&2
    exit 5
  fi
  echo "$name.lines=$lines missed=$missed files=$rows profraw=$n" >&2
  awk -v l="$lines" -v m="$missed" 'BEGIN{printf "%.1f", (l-m)*100/l}'
}

base_of() { awk -v k="$1" '$1==k{print $2}' "$BASE"; }

fail=0
line="coverage:"
for pair in "hub crates/dowiz-hub" "worker workers/api"; do
  set -- $pair
  [ -z "$only" ] || [ "$only" = "$1" ] || continue
  b=$(base_of "$1")
  [ -n "$b" ] || { echo "coverage: REFUSED — no baseline for $1 in $BASE"; exit 2; }
  pct=$(measure "$1" "$2") || exit $?
  line="$line $1 $pct% (baseline $b)"
  if awk -v p="$pct" -v b="$b" 'BEGIN{exit !(p+0 < b+0)}'; then
    echo "coverage: $1 FELL to $pct% below its baseline $b%" >&2
    fail=1
  elif awk -v p="$pct" -v b="$b" 'BEGIN{exit !(p+0 > b+0)}'; then
    echo "coverage: $1 rose to $pct% — raise its line in $BASE to lock it in" >&2
  fi
done
echo "$line"
exit $fail
