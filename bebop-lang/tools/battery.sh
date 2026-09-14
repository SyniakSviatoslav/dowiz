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
( bash tools/no_andand.sh > "$T/andand.log" 2>&1 ) & sw  # A26 (re-scoped 2026-09-14): `&&` and `||` are REAL LOGICAL OPERATORS since 0cb2c23 -- value 0/1, no short circuit, precedence `|| < && < cmp`. This comment used to say "&& is a CONSTANT ZERO in this language", which the compiler half made false; the gate was banning a working operator on that premise. It now asserts that c46_andor still pins the DERIVED post-A26 value (101100), which is the regression that would let the pre-A26 mis-parse back in. Cheap: no compile, just a scan
( python3 tools/braille_check.py docs/design/BRAILLE-v1.tsv > "$T/braille.log" 2>&1 ) & sw  # T84: the braille surface table must stay injective, in-block and TOTAL over printable ASCII. covers_source PASSes at ascii_uncovered=0; non-ASCII is --bytes territory by design
( bash bench/traps/run_traps.sh > "$T/traps.log" 2>&1 ) & sw  # every trap in docs/TRAPS.md TRIGGERED rather than read: 16 probes, observed exit code and stderr compared against what the doc promises
( BEBOP_TMP=$T/inv BEBOP_BIN=$BIN BEBOP_SRC=$SRC bash bench/vs_rust/invariants.sh $([ "$FREEZE" = 1 ] && echo --freeze) > "$T/inv.log" 2>&1 ) & sw
python3 tools/census.py "$BIN" | tail -n 1 > "$T/census.txt" 2>&1
python3 tools/check_abi.py "$BIN" > "$T/abi.txt" 2>&1
BEBOP_TMP=$T/diag BEBOP_BIN=$BIN bash bench/vs_rust/diag_check.sh > "$T/diag.log" 2>&1  # T90: line:col diagnostics
python3 tools/check_words.py > "$T/words.log" 2>&1  # item 7: hand-typed em()/st[] literals (L1)
python3 tools/f8_dt.py "$BIN" > "$T/f8_dt.log" 2>&1  # F8: dependent surface types parsing + erasure
python3 tools/trap_census.py > "$T/trap.log" 2>&1  # F1: trap census, derived from the tree (neg/ EXPECT headers + mechanisms); exits 2 NOT MEASURED when the neg/ scan is empty -- it ran in NO battery lane until 2026-09-13
python3 tools/certcheck_census.py > "$T/certs.log" 2>&1  # F6: LRAT certificate soundness; certgen/certcheck had NO automatic caller and the row's four 2026-09-09 negatives left no artifact at all -- bench/cert_pos + bench/cert_neg are that corpus, committed so the measurement is reproducible
wait
red=0
line() { local l; l=$(grep -E "$2" "$T/$1" | tail -n 1); [ -n "$l" ] || { l="MISSING ($1)"; red=1; }; echo "$l" | grep -qE "$3" || red=1; echo "  $l"; }
echo "battery for $BIN ($(md5sum "$BIN" | cut -c1-8)):"
line std.log '^std_golden:' '^std_golden: [1-9][0-9]* pass, 0 fail'  # 2026-09-13 audit: `0 pass, 0 fail` (splitter failure) used to read green; std_par now prints NOT MEASURED there and this expects >= 1 pass
line cp.log '^construct parity:' '^construct parity: pass=[1-9][0-9]* fail=0'
line diag.log '^diag:' '^diag: [1-9][0-9]* pass, 0 fail'
line pd.log '^parity:' '^parity: pass=[1-9][0-9]* fail=0'  # 2026-09-13 audit: an empty kernels dir printed `pass=0 fail=0 skip=1` and matched the old 'fail=0'
line pool.log '^pool_parity:' '^pool_parity: [1-9][0-9]* pass, 0 fail'
line braille.log '^braille_check:' '^braille_check: 7 PASS 0 FAIL$'  # pinned at 7, not [1-9]+: a check that silently disappears must not read green
line traps.log '^trap_verify' '^trap_verify traps=1[0-9] match=1[0-9] mismatch=0 not_triggered=0$'  # a probe that stops triggering must not read green
line oracles.log '^SUMMARY' '^SUMMARY ok=[1-9][0-9]* self-frozen=0 mismatch=0 missing=0'
line bpp.log '^bpref_parity:' '^bpref_parity: agree=[1-9][0-9]* .*disagree=0 error=0'  # A23: agreement between the two implementations; 2026-09-13 audit: an ABSENT tools/bpref.py read `agree=0 unsupported=100 disagree=0 error=0` and matched the old expect
line f7_kcheck.log '^kernel_neg:' ' 0 accepted of 21'  # F7: twin soundness (Python reference) -- NOTE this number is computed by tools/kcheck.py's PYTHON twin, not by tkernel.bin; the kernel's own acceptance is the next line (kernel_neg_bin), which caught two soundness holes on 2026-09-13 while this line stayed green
line f7_kcheck.log '^kernel_neg_bin:' ' 0 accepted of 21'  # F7: kernel binary soundness (must reject all unsound terms)
line f7_kcheck.log '^kernel_pos:' ' 0 rejected of [1-9][0-9]*'  # F7: a kernel that REJECTS EVERYTHING passes kernel_neg and kernel_neg_bin (rejecting all = accepting none); this is the complement that catches it
line f7_kcheck.log '^kernel_parity:' '28/28'  # F7: kernel parity measurement (must be real, not "NOT MEASURED")
line abi.txt 'ABI' '^ABI ok'
line inv.log '^invariants:' 'GREEN'
line andand.log '^no_andand:' '^no_andand: PASS'  # A26: fails if `bench/parity_constructs/c46_andor.bp` stops carrying `// EXPECT 101100`, or is missing. The `&&`/`||` site counts it prints are INFORMATION, not a verdict -- the ban was retired 2026-09-14 because A26 made its premise false. Comment- and string-aware for the counts (a `//` inside a string literal used to hide a real `&&` on the same line -- measured false negative, fixed 2026-09-13). TRIGGERED 2026-09-14: setting the header back to the pre-A26 111100 turns this red with the derivation in the message, restoring 101100 turns it green
line words.log '^words:' '^words: PASS'  # 2026-09-13 audit: in a lane tree (no .git) this was an empty `git diff` against the MAIN repo = PASS measuring nothing; check_words.py now prints NOT MEASURED there unless WORDS_BASE=<base bebop.bp> is set
line std.log '^boxguard:' '.'  # item 9: the timing stage (lcjit) runs last, single-threaded, boxguard status logged next to it. PRESENCE ONLY, by design: this row records that the timing stage ran (it goes MISSING when std_par's timing loop does not); it asserts nothing about the value and cannot go red on one
line f8_dt.log '^f8_dt gate:' '^f8_dt gate: PASS'  # F8: dependent surface types parsing + erasure; 2026-09-13 audit: with bebop.bin ABSENT every probe 'REJECTED' at loader rc=90 and this read PASS -- f8_dt.py now prints NOT MEASURED unless the positives compile and every rejection is the diagnostic exit 110
line trap.log '^trap_unrep:' '^trap_unrep: (1[6-9]|[2-9][0-9]|[1-9][0-9][0-9])/'  # F1: closed/counted census rows, a RATCHET at >=16 closed, not a freeze: pinning `16/35` meant the first trap F2 closes would turn the battery RED, and the denominator moves with the numerator anyway. A `NOT MEASURED` scan has no trap_unrep: line and reads MISSING
line certs.log '^cert_checked:' '^cert_checked: [1-9][0-9]*/[1-9][0-9]*'  # F6: at least one real obligation+certificate pair verified; a NOT MEASURED run has no ratio here and reads MISSING
line certs.log '^checker_neg:' '^checker_neg: 0/[1-9][0-9]*'  # F6: corrupted certificates WRONGLY ACCEPTED, must be 0 of a non-empty corpus -- the kernel_neg convention. The denominator is pinned >=1 so an empty cert_neg/ cannot read 0/0 and pass
echo "  census: $(cat "$T/census.txt")"
grep -h '^FAIL\|MISMATCH\|COMPILEFAIL\|WORD_BUDGET_MISSING\|VALUE_MISMATCH\|NOT MEASURED' "$T"/*.log | head -n 20 | sed 's/^/  /'
[ $red = 0 ] && echo "battery: GREEN" || { echo "battery: RED"; exit 1; }
