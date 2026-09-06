#!/usr/bin/env bash
# chain.sh (2026-09-06, session-10 speed-up): the three-generation self-hosting chain plus
# the battery, pipelined. gen2 = <bin0> compiling <src>; then IN PARALLEL: gen3 -> gen4
# (the fixpoint test is gen3 == gen4) and the battery against gen2. When the change is
# NOT a codegen change gen2 == gen3 == gen4 and the battery result stands as soon as the
# md5s agree; pass --codegen to run the battery against gen4 instead (gen2 differs then).
# Usage: tools/chain.sh <src.bp> <out-dir> [--codegen] [BIN0=./bebop.bin]
# item 8 (self-copy exec): the run copies itself into $OUT before doing any work, so editing
# this file while a run is in progress cannot change that run.
# item 1 (process-count gate): refuses to start above ${PROC_CAP:-30} procs (tools/reap.sh --check), exit 97.
# Measured 2026-09-06: a chain adds +15 procs (ps -e 26 -> 41 peak, box survived); the gate reads ~+3
# over idle. 30 = idle box without a fuzzd batch; `tools/fuzzd.sh pause` first.
# item 4: --codegen implies FREEZE=1 for the battery it drives (no more forgotten env var).
[ "${SELF_COPY:-}" ] || cd "$(dirname "$0")/.." || exit 1  # the copy is exec'd with cwd already at repo root; re-deriving it from $0 there would resolve against $OUT instead
SRC=${1:?src.bp}; OUT=${2:?out dir}; mkdir -p "$OUT"
[ "${SELF_COPY:-}" ] || { cp "$0" "$OUT/.chain.sh"; SELF_COPY=1 exec bash "$OUT/.chain.sh" "$@"; }
shift 2; CG=0; [ "${1:-}" = --codegen ] && { CG=1; shift; }
BIN0=$(realpath -m "${1:-./bebop.bin}")
[ -s "$BIN0" ] || { echo "GUARD: $BIN0 missing or empty (L12)"; exit 1; }
[ "${REAP_GATED:-}" ] || tools/reap.sh --check "${PROC_CAP:-30}" || { echo "GUARD: process cap exceeded (item 1, L19c)"; exit 97; }
export REAP_GATED=1  # one gate per run tree: battery/fuzz started by this run skip their own check
[ $CG = 1 ] && export FREEZE=1
PIN=${PIN:-taskset -c 4-6}  # the 3 A78 cores; PIN="" to unpin
gen() { $PIN ./seed/build/seed "$1" compile "$SRC" "$2" >/dev/null 2>&1; local rc=$?; [ $rc = 0 ] && [ -s "$2" ] || { echo "gen $2 FAILED rc=$rc"; exit 1; }; }
t0=$(date +%s); gen "$BIN0" "$OUT/gen2.bin"; echo "gen2 $(md5sum < "$OUT/gen2.bin" | cut -c1-8) $(( $(date +%s) - t0 )) s"
( gen "$OUT/gen2.bin" "$OUT/gen3.bin"; echo "gen3 $(md5sum < "$OUT/gen3.bin" | cut -c1-8)"; gen "$OUT/gen3.bin" "$OUT/gen4.bin"; echo "gen4 $(md5sum < "$OUT/gen4.bin" | cut -c1-8)" ) > "$OUT/chain.log" 2>&1 &
if [ $CG = 0 ]; then SRC=$SRC bash tools/battery.sh "$OUT/gen2.bin" "$OUT/bat" > "$OUT/battery.log" 2>&1; fi
wait; cat "$OUT/chain.log"
if [ $CG = 1 ]; then SRC=$SRC bash tools/battery.sh "$OUT/gen4.bin" "$OUT/bat" > "$OUT/battery.log" 2>&1; fi
cat "$OUT/battery.log"
m3=$(md5sum < "$OUT/gen3.bin" 2>/dev/null | cut -c1-8); m4=$(md5sum < "$OUT/gen4.bin" 2>/dev/null | cut -c1-8); m2=$(md5sum < "$OUT/gen2.bin" | cut -c1-8)
if [ -n "$m4" ] && [ "$m3" = "$m4" ]; then
  if [ $CG = 0 ] && [ "$m2" != "$m3" ]; then echo "chain: gen3 == gen4 $m4 but gen2 differs ($m2) -- codegen changed: rerun with --codegen"; exit 1; fi
  echo "chain: fixpoint gen3 == gen4 $m4 ($(( $(date +%s) - t0 )) s total)"; grep -q 'battery: GREEN' "$OUT/battery.log" || exit 1
else echo "chain: NO FIXPOINT (gen3 $m3, gen4 $m4)"; exit 1; fi
# D12-A evals (tools/perf.py): size/constructs/fuzz rows, self-compile + K-kernels interleaved against BIN0,
# docs/PERF.md; exit 1 on an alert. PERF=0 skips (~60 s). E14: the chain's own wall + children cpu-seconds.
if [ "${PERF:-1}" = 1 ]; then
  CAND=$OUT/gen4.bin; [ $CG = 0 ] && CAND=$OUT/gen2.bin
  times > "$OUT/times.txt"; cpu=$(awk 'NR==2{split($1,a,"m"); split($2,b,"m"); print a[1]*60+a[2]+b[1]*60+b[2]}' "$OUT/times.txt" | sed 's/s//g')
  python3 tools/perf.py record --bin "$CAND" chain_wall $(( $(date +%s) - t0 )) s "chain + battery, CG=$CG"
  python3 tools/perf.py record --bin "$CAND" chain_cpu "${cpu:-0}" s "children utime+stime of chain.sh"
  BEBOP_TMP=$OUT python3 tools/perf.py run --bin "$CAND" --base "$BIN0" --n "${PERF_N:-5}" --r "${PERF_R:-11}" > "$OUT/perf.log" 2>&1; prc=$?
  tail -${PERF_TAIL:-3} "$OUT/perf.log"; [ $prc = 0 ] || { echo "perf: ALERT or error (rc=$prc, $OUT/perf.log)"; exit 1; }
fi
