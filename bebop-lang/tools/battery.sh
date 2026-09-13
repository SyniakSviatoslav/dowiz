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
# --- one-compile guard (operator 2026-09-12) -----------------------------------
# Not inside a slot? Re-exec through it. tools/slot.sh runs SLOTS=1 against ONE global flock
# in /root/.cache/bebop/slots, shared by every lane worktree, so at most one heavy job -- one
# compilation -- exists on the box at any instant. NO_SLOT=1 opts out (main-session triage only).
if [ "${BEBOP_SLOT_HELD:-0}" != 1 ] && [ "${NO_SLOT:-0}" != 1 ]; then
  _slot="$(dirname "$0")/../tools/slot.sh"
  [ -f "$_slot" ] || _slot=/root/dowiz/bebop-lang/tools/slot.sh
  [ -f "$_slot" ] && exec bash "$_slot" "auto:$(basename "$0")" bash "$0" "$@"
fi
[ "${SELF_COPY:-}" ] || cd "$(dirname "$0")/.." || exit 1  # the copy is exec'd with cwd already at repo root; re-deriving it from $0 there would resolve against $T instead
T=${2:?tmp root}; mkdir -p "$T"
[ "${SELF_COPY:-}" ] || { cp "$0" "$T/.battery.sh"; SELF_COPY=1 exec bash "$T/.battery.sh" "$@"; }
BIN=$(realpath -m "${1:?candidate .bin}"); export FREEZE=${FREEZE:-0}; SRC=${SRC:-bebop.bp}
[ -s "$BIN" ] || { echo "GUARD: $BIN missing or empty (L12)"; exit 1; }
# Build F7 kernel binary if missing (gate requires it to measure, not just to check twin)
if [ ! -s "./tkernel.bin" ]; then
  echo "battery: building F7 kernel binary ./tkernel.bin"
  bash tools/build_tkernel.sh || { echo "GUARD: F7 kernel build failed"; exit 1; }
fi
[ "${REAP_GATED:-}" ] || tools/reap.sh --check "${PROC_CAP:-30}" || { echo "GUARD: process cap exceeded (item 1, L19c)"; exit 97; }
export REAP_GATED=1  # one gate per run tree (chain.sh already checked when it drives us)
# SERIAL now DEFAULTS to 1 (2026-09-08, operator "no more Signal 9"): Android's phantom-process
# killer SIGKILLs an app's forked processes past max_phantom_processes=32, and the parallel battery
# peaks at ~40 -- it was the single biggest fork storm on the box. Serial lanes with J=1 peak at ~12.
# It also retires the J=3 false RED the 2026-09-08 journal recorded (the 12 generated
# gb_<op>_<sr>_0_1_0.* files share one BEBOP_TMP, so shards clobbered each other).
# SERIAL=0 restores the old parallel battery -- only on a box where the phantom cap has been lifted.
SERIAL=${SERIAL:-1}
mkdir -p "$T"/{std,cp,pd,pool,bpp}; sw() { [ "$SERIAL" = 1 ] && wait; :; }; [ "$SERIAL" = 1 ] && { J=${J:-1}; export J; }  # EXPORT: run_all.sh is a fresh `bash`, an unexported J left it at xargs -P 4  # lanes one after another, J=1 -- ~12 procs instead of ~40
( J=${J:-3} BEBOP_TMP=$T/std BEBOP_BIN=$BIN bash tools/std_par.sh > "$T/std.log" 2>&1 ) & sw  # sharded std_golden, one shard per A78 core
( BEBOP_TMP=$T/cp BEBOP_BIN=$BIN bash bench/vs_rust/construct_parity.sh > "$T/cp.log" 2>&1;
  BEBOP_TMP=$T/pd BEBOP_BIN=$BIN bash bench/vs_rust/parity_driver.sh > "$T/pd.log" 2>&1 ) & sw
( BEBOP_TMP=$T/pool BEBOP_BIN=$BIN bash bench/vs_rust/pool_parity.sh > "$T/pool.log" 2>&1 ) & sw
( bash bench/oracles/run_all.sh > "$T/oracles.log" 2>&1 ) & sw  # little cores, memoized
( BEBOP_TMP=$T/bpp BEBOP_BIN=$BIN bash bench/vs_rust/bpref_parity.sh > "$T/bpp.log" 2>&1 ) & sw  # A23 differential lane: bpref's evaluator ran in no battery lane until 2026-09-13
( TKERNEL_BIN=./tkernel.bin python3 tools/kcheck.py --corpus bench/kernel_neg > "$T/f7_kcheck.log" 2>&1 ) & sw  # F7: kernel certificate checker vs twin
( BEBOP_TMP=$T/inv BEBOP_BIN=$BIN BEBOP_SRC=$SRC bash bench/vs_rust/invariants.sh $([ "$FREEZE" = 1 ] && echo --freeze) > "$T/inv.log" 2>&1 ) & sw
python3 tools/census.py "$BIN" | tail -n 1 > "$T/census.txt" 2>&1
python3 tools/check_abi.py "$BIN" > "$T/abi.txt" 2>&1
BEBOP_TMP=$T/diag BEBOP_BIN=$BIN bash bench/vs_rust/diag_check.sh > "$T/diag.log" 2>&1  # T90: line:col diagnostics
python3 tools/check_words.py > "$T/words.log" 2>&1  # item 7: hand-typed em()/st[] literals (L1)
python3 tools/f8_dt.py "$BIN" > "$T/f8_dt.log" 2>&1  # F8: dependent surface types parsing + erasure
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
line bpp.log '^bpref_parity:' 'disagree=0 error=0'  # A23: agreement between the two implementations
line f7_kcheck.log '^kernel_neg:' ' 0 accepted of 14'  # F7: twin soundness (Python reference)
line f7_kcheck.log '^kernel_parity:' '18/18'  # F7: kernel parity measurement (must be real, not "NOT MEASURED")
line abi.txt 'ABI' '^ABI ok'
line inv.log '^invariants:' 'GREEN'
line words.log '^words:' 'PASS'
line std.log '^boxguard:' '.'  # item 9: the timing stage (lcjit) runs last, single-threaded, boxguard status logged next to it
line f8_dt.log '^f8_dt gate:' 'PASS'  # F8: dependent surface types parsing + erasure
echo "  census: $(cat "$T/census.txt")"
grep -h '^FAIL\|MISMATCH\|COMPILEFAIL\|WORD_BUDGET_MISSING\|VALUE_MISMATCH' "$T"/*.log | head -n 20 | sed 's/^/  /'
[ $red = 0 ] && echo "battery: GREEN" || { echo "battery: RED"; exit 1; }
