#!/usr/bin/env bash
# battery.sh (2026-09-06, session-10 speed-up): the whole gate battery against ONE candidate
# compiler, the independent scripts in parallel on the box's 3 A78 cores, one summary block.
# Usage: tools/battery.sh <candidate.bin> <tmp-root> [FREEZE=1] [SRC=<candidate.bp>]
#   std_golden, construct_parity (FREEZE honoured), parity_driver, pool_parity, run_all
#   (oracles, compiler-independent), census/typecheck/check_abi of the candidate.
# invariants.sh runs as its own lane (2026-09-06) against the candidate (BEBOP_BIN/BEBOP_SRC),
# --freeze when FREEZE=1: nothing is left to run after promotion.
# item 8 (self-copy exec): copy into $T before doing any work, so editing this file while a
# run is in progress cannot change that run. item 1 (process-count gate): refuse above ${PROC_CAP:-30} procs (calibrated 2026-09-06, see chain.sh).
[ "${SELF_COPY:-}" ] || cd "$(dirname "$0")/.." || exit 1  # the copy is exec'd with cwd already at repo root; re-deriving it from $0 there would resolve against $T instead
T=${2:?tmp root}; mkdir -p "$T"
[ "${SELF_COPY:-}" ] || { cp "$0" "$T/.battery.sh"; SELF_COPY=1 exec bash "$T/.battery.sh" "$@"; }
BIN=$(realpath -m "${1:?candidate .bin}"); export FREEZE=${FREEZE:-0}; SRC=${SRC:-bebop.bp}
[ -s "$BIN" ] || { echo "GUARD: $BIN missing or empty (L12)"; exit 1; }
[ "${REAP_GATED:-}" ] || tools/reap.sh --check "${PROC_CAP:-30}" || { echo "GUARD: process cap exceeded (item 1, L19c)"; exit 97; }
export REAP_GATED=1  # one gate per run tree (chain.sh already checked when it drives us)
mkdir -p "$T"/{std,cp,pd,pool}
( J=${J:-3} BEBOP_TMP=$T/std BEBOP_BIN=$BIN bash tools/std_par.sh > "$T/std.log" 2>&1 ) &  # sharded std_golden, one shard per A78 core
( BEBOP_TMP=$T/cp BEBOP_BIN=$BIN bash bench/vs_rust/construct_parity.sh > "$T/cp.log" 2>&1;
  BEBOP_TMP=$T/pd BEBOP_BIN=$BIN bash bench/vs_rust/parity_driver.sh > "$T/pd.log" 2>&1 ) &
( BEBOP_TMP=$T/pool BEBOP_BIN=$BIN bash bench/vs_rust/pool_parity.sh > "$T/pool.log" 2>&1 ) &
( bash bench/oracles/run_all.sh > "$T/oracles.log" 2>&1 ) &  # little cores, memoized
( BEBOP_TMP=$T/inv BEBOP_BIN=$BIN BEBOP_SRC=$SRC bash bench/vs_rust/invariants.sh $([ "$FREEZE" = 1 ] && echo --freeze) > "$T/inv.log" 2>&1 ) &
python3 tools/census.py "$BIN" | tail -n 1 > "$T/census.txt" 2>&1
python3 tools/check_abi.py "$BIN" > "$T/abi.txt" 2>&1
BEBOP_TMP=$T/diag BEBOP_BIN=$BIN bash bench/vs_rust/diag_check.sh > "$T/diag.log" 2>&1  # T90: line:col diagnostics
python3 tools/check_words.py > "$T/words.log" 2>&1  # item 7: hand-typed em()/st[] literals (L1)
wait
red=0
line() { local l; l=$(grep -E "$2" "$T/$1" | tail -n 1); [ -n "$l" ] || { l="MISSING ($1)"; red=1; }; echo "$l" | grep -qE "$3" || red=1; echo "  $l"; }
echo "battery for $BIN ($(md5sum "$BIN" | cut -c1-8)):"
line std.log '^std_golden:' ' 0 fail'
line cp.log '^construct parity:' 'fail=0'
line diag.log '^diag:' ' 0 fail'
line pd.log '^parity:' 'fail=0'
line pool.log '^pool_parity:' ' 0 fail'
line oracles.log '^SUMMARY' 'self-frozen=0 mismatch=0 missing=0'
line abi.txt 'ABI' '^ABI ok'
line inv.log '^invariants:' 'GREEN'
line words.log '^words:' 'PASS'
line std.log '^boxguard:' '.'  # item 9: the timing stage (lcjit) runs last, single-threaded, boxguard status logged next to it
echo "  census: $(cat "$T/census.txt")"
grep -h '^FAIL\|MISMATCH\|COMPILEFAIL\|WORD_BUDGET_MISSING\|VALUE_MISMATCH' "$T"/*.log | head -n 20 | sed 's/^/  /'
[ $red = 0 ] && echo "battery: GREEN" || { echo "battery: RED"; exit 1; }
