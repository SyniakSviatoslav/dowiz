#!/usr/bin/env bash
# T39 compiler fuzzer: gen.py -> bebop.bin compile -> seed run, compared to
# tools/bpref.py (the semantic oracle). Usage: bench/fuzz/fuzz.sh [N] [START]
# env: J=parallel jobs (default 4), BEBOP_TMP=scratch root.
# Categories: OK DIVERGE COMPILEFAIL CRASH TIMEOUT BPREF-ERROR (GENFAIL = generator bug).
# Every non-OK program is saved to bench/fuzz/repros/<CAT>-<seed>.bp with an
# expected-vs-got header (L10). A CRASH is re-run 3x before it is believed.
ulimit -s 65536 2>/dev/null || true
cd "$(dirname "$0")/../.." || exit 1
N=${1:-200}; START=${2:-1}; J=${J:-4}
export TMP=${BEBOP_TMP:-/tmp/opencode/agentB-fuzz}/fuzz.$$
export REPROS=bench/fuzz/repros
mkdir -p "$TMP" "$REPROS"
[ -s bebop.bin ] || { echo "GUARD: bebop.bin is missing or empty (L12)"; exit 1; }

one() {
  local s=$1 d=$TMP/$1 cat exp got rc n k
  mkdir -p "$d"
  python3 bench/fuzz/gen.py --seed "$s" --out "$d/p.bp" 2>"$d/gerr" || { echo "GENFAIL $s"; return; }
  exp=$(timeout 20 python3 tools/bpref.py "$d/p.bp" 2>"$d/err"); rc=$?
  if [ $rc -ne 0 ]; then
    cat=BPREF-ERROR; got="rc=$rc $(tail -c 160 "$d/err" | tr '\n' ' ')"
  else
    timeout 20 ./seed/build/seed bebop.bin compile "$d/p.bp" "$d/p.bin" >"$d/cout" 2>&1; rc=$?
    if [ $rc -ne 0 ] || [ ! -s "$d/p.bin" ]; then
      cat=COMPILEFAIL; got="compile rc=$rc $(tail -c 80 "$d/cout" | tr '\n' ' ')"
    else
      timeout 5 ./seed/build/seed "$d/p.bin" >"$d/out" 2>/dev/null; rc=$?
      got=$(tail -1 "$d/out")
      if [ $rc -eq 124 ]; then cat=TIMEOUT; got="timeout 5s"
      elif [ $rc -ge 128 ]; then
        n=0; for k in 1 2 3; do timeout 5 ./seed/build/seed "$d/p.bin" >/dev/null 2>&1; [ $? -ge 128 ] && n=$((n+1)); done
        cat=CRASH; got="signal $((rc-128)) ($n/3 reruns crashed)"
      elif [ "$got" = "$exp" ]; then cat=OK
      else cat=DIVERGE; [ $rc -ne 0 ] && got="$got (exit rc=$rc)"
      fi
    fi
  fi
  if [ "$cat" != OK ]; then
    { echo "// $cat seed=$s expected=$exp got=$got"; cat "$d/p.bp"; } >"$REPROS/$cat-$s.bp"
  fi
  echo "$cat $s"
  rm -rf "$d"
}
export -f one
t0=$(date +%s.%N)
seq "$START" $((START + N - 1)) | xargs -P "$J" -I{} bash -c 'one {}' >"$TMP/results"
t1=$(date +%s.%N)
awk -v n="$N" -v t="$(echo "$t1 - $t0" | bc)" '
  { c[$1]++ }
  END { printf "fuzz: N=%d OK=%d DIVERGE=%d COMPILEFAIL=%d CRASH=%d TIMEOUT=%d BPREF-ERROR=%d GENFAIL=%d wall=%.1fs\n",
        n, c["OK"], c["DIVERGE"], c["COMPILEFAIL"], c["CRASH"], c["TIMEOUT"], c["BPREF-ERROR"], c["GENFAIL"], t }' "$TMP/results"
grep -v '^OK' "$TMP/results" | sort -k2 -n | head -40
[ "$(grep -vc '^OK' "$TMP/results")" = 0 ]
