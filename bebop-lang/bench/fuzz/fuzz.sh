#!/usr/bin/env bash
# T39 compiler fuzzer: gen.py -> $BEBOP_BIN compile -> seed run, compared to
# tools/bpref.py (the semantic oracle). Usage: bench/fuzz/fuzz.sh [N] [START]
# env: J=parallel jobs (default 4), BEBOP_BIN=compiler (default bebop.bin),
# REPROS=repro dir (default bench/fuzz/repros), BEBOP_TMP=scratch root.
# Categories (classifier = bench/fuzz/shrink.py --classify, shared with the
# T77 shrinker): OK DIVERGE COMPILEFAIL CRASH TIMEOUT BPREF-ERROR
# BPREF-DEPTH (oracle call depth > 5000: unbounded recursion = generator
# defect, not a compiler verdict) GENFAIL (generator bug).
# Every compiler-class program is saved to $REPROS/<CAT>-<seed>.bp with an
# expected-vs-got header (L10). A CRASH is re-run 3x before it is believed.
# Every seed/compiled-program run has cwd = the per-seed scratch dir, so a
# stray file written by a misbehaving binary lands there, not in the repo.
ulimit -s 65536 2>/dev/null || true
cd "$(dirname "$0")/../.." || exit 1
N=${1:-200}; START=${2:-1}; J=${J:-4}
export BEBOP_BIN=$(realpath -m "${BEBOP_BIN:-bebop.bin}")
export REPROS=${REPROS:-bench/fuzz/repros}
export TMP=${BEBOP_TMP:-/tmp/opencode/agentB-fuzz}/fuzz.$$
mkdir -p "$TMP" "$REPROS"
[ -s "$BEBOP_BIN" ] || { echo "GUARD: $BEBOP_BIN is missing or empty (L12)"; exit 1; }
# snapshot the compiler: a concurrent promotion (rm+cp of bebop.bin) made
# seed's openat fail mid-run (exit 90 = f_open, classified COMPILEFAIL);
# the whole run now sees ONE artifact, whose md5 is printed in the summary
cp "$BEBOP_BIN" "$TMP/bebop.bin" && export BEBOP_BIN=$TMP/bebop.bin

one() {
  local s=$1 d=$TMP/$1 cat exp got
  mkdir -p "$d"
  python3 bench/fuzz/gen.py --seed "$s" --out "$d/p.bp" 2>"$d/gerr" || { echo "GENFAIL $s"; return; }
  IFS=$'\t' read -r cat exp got < <(python3 bench/fuzz/shrink.py --classify "$d/p.bp")
  case "$cat" in
    OK|BPREF-DEPTH) ;;
    *) { echo "// $cat seed=$s expected=$exp got=$got"; cat "$d/p.bp"; } >"$REPROS/$cat-$s.bp" ;;
  esac
  echo "${cat:-HARNESS-ERROR} $s"
  ls "$d" | grep -v -E '^(p\.bp|p\.bin|gerr)$' | sed "s/^/STRAY $s /"  # anything else = a stray file written by a run
  rm -rf "$d"
}
export -f one
t0=$(date +%s.%N)
seq "$START" $((START + N - 1)) | xargs -P "$J" -I{} bash -c 'one {}' >"$TMP/results"
t1=$(date +%s.%N)
awk -v n="$N" -v s="$START" -v t0="$t0" -v t1="$t1" -v bin="$(md5sum "$BEBOP_BIN" | cut -c1-8)" '
  { c[$1]++ }
  END { t = t1 - t0
        printf "fuzz: N=%d START=%d OK=%d DIVERGE=%d COMPILEFAIL=%d CRASH=%d TIMEOUT=%d BPREF-ERROR=%d BPREF-DEPTH=%d GENFAIL=%d STRAY=%d wall=%.1fs rate=%.2f/s bin=%s\n",
        n, s, c["OK"], c["DIVERGE"], c["COMPILEFAIL"], c["CRASH"], c["TIMEOUT"], c["BPREF-ERROR"], c["BPREF-DEPTH"], c["GENFAIL"], c["STRAY"], t, n / (t > 0 ? t : 1), bin }' "$TMP/results"
grep -v -E '^(OK|GENFAIL|BPREF-DEPTH) ' "$TMP/results" | sort -k2 -n | head -40
rc=$([ "$(grep -v -c -E '^(OK|GENFAIL|BPREF-DEPTH) ' "$TMP/results")" = 0 ]; echo $?)
rm -rf "$TMP"
exit $rc
