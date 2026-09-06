#!/usr/bin/env bash
# T39 compiler fuzzer: gen.py -> $BEBOP_BIN compile -> seed run, compared to
# tools/bpref.py (the semantic oracle). Usage: bench/fuzz/fuzz.sh [N] [START]
# env: J=parallel jobs (default 4), BEBOP_BIN=compiler (default bebop.bin),
# REPROS=repro dir (default bench/fuzz/repros), BEBOP_TMP=scratch root.
# Categories (classifier = bench/fuzz/shrink.py --classify, shared with the
# T77 shrinker): OK DIVERGE COMPILEFAIL CRASH TIMEOUT BPREF-ERROR
# BPREF-DEPTH (oracle call depth > 5000: unbounded recursion = generator
# defect, not a compiler verdict) GENFAIL (generator bug) TRAP-OK (T118: the
# oracle predicted a capacity trap and bebop exited with the same code)
# TRAP-80/81/82 (bebop trapped, the oracle did not predict it: frame heap is
# not modelled by bpref).
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
# item 1 (process-count gate): fuzzd runs a chain in parallel legitimately near the cap, so
# retry every 30 s for up to 10 min (a delayed batch, not a lost one) instead of exit 97 at once.
i=0
while [ -z "${REAP_GATED:-}" ] && ! tools/reap.sh --check "${PROC_CAP:-30}"; do
  i=$((i + 1)); [ "$i" -ge 20 ] && { echo "GUARD: process cap exceeded for 10 min, giving up (item 1)"; exit 97; }
  sleep 30
done
export REAP_GATED=1  # fuzz_batch.py shards skip their own check
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
  ls "$d" | grep -v -E '^(p\.bp|p\.bin|p\.bin\.becache|gerr)$' | sed "s/^/STRAY $s /"  # anything else = a stray file written by a run
  rm -rf "$d"
}
export -f one
t0=$(date +%s.%N)
# J contiguous shards, each ONE python process (bench/fuzz/fuzz_batch.py: gen in-process,
# bpref forked, seed compile/run as subprocesses) -- 3 interpreter starts per seed were
# ~70 % of the wall under proot (0.56/s -> 5/s on one core, 2026-09-06). one() above is
# the reference shape of a seed; fuzz_batch.py prints the same lines.
per=$(( (N + J - 1) / J ))
for ((j = 0; j < J; j++)); do
  st=$((START + j * per)); cnt=$((N - j * per)); [ "$cnt" -gt "$per" ] && cnt=$per; [ "$cnt" -le 0 ] && break
  python3 bench/fuzz/fuzz_batch.py "$st" "$cnt" >"$TMP/results.$j" 2>"$TMP/err.$j" &
done
wait
cat "$TMP"/results.* >"$TMP/results"; cat "$TMP"/err.* >&2
t1=$(date +%s.%N)
awk -v n="$N" -v s="$START" -v t0="$t0" -v t1="$t1" -v bin="$(md5sum "$BEBOP_BIN" | cut -c1-8)" '
  { c[$1]++ }
  END { t = t1 - t0
        printf "fuzz: N=%d START=%d OK=%d DIVERGE=%d COMPILEFAIL=%d CRASH=%d TIMEOUT=%d BPREF-ERROR=%d BPREF-DEPTH=%d BPREF-TIMEOUT=%d GENFAIL=%d STRAY=%d TRAP-OK=%d TRAP-UNPREDICTED=%d TRAP-81=%d TRAP-82=%d wall=%.1fs rate=%.2f/s bin=%s\n",
        n, s, c["OK"], c["DIVERGE"], c["COMPILEFAIL"], c["CRASH"], c["TIMEOUT"], c["BPREF-ERROR"], c["BPREF-DEPTH"], c["BPREF-TIMEOUT"], c["GENFAIL"], c["STRAY"], c["TRAP-OK"], c["TRAP-80"] + c["TRAP-81"] + c["TRAP-82"], c["TRAP-81"], c["TRAP-82"], t, n / (t > 0 ? t : 1), bin }' "$TMP/results"
grep -v -E '^(OK|GENFAIL|BPREF-DEPTH|BPREF-TIMEOUT|TRAP-OK|TRAP-TIMEOUT) ' "$TMP/results" | sort -k2 -n | head -40
# D12-C: TRAP-81 (frame heap) is by design and stays a pass; TRAP-82 (SIGSEGV/SIGBUS) is
# a real bug and is never excluded here, so it alone fails the run.
rc=$([ "$(grep -v -c -E '^(OK|GENFAIL|BPREF-DEPTH|TRAP-81) ' "$TMP/results")" = 0 ]; echo $?)
rm -rf "$TMP"
exit $rc
