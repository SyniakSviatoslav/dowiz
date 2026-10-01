#!/usr/bin/env bash
# dl_bench.sh -- docs/PERF.md row `dl_event_ns` (row DG8; SPEC-DATALOG-AND-CODEC A-8: cost per event on the
# 165-dish fixture MUST stay <= 1,000 ns; MEASURED basis R §12.3 "Datalog delta", 82 ns Rust native).
#
# The fixture is selfhost/std/dl_fix.bp set 1: 165 dishes, 120 supplies, 352 recipe lines over 70 dishes,
# unavailable(D) :- recipe(D,S,C), stock(S,Q), qmin(D,M), mul(C,M,CM), lt(Q,CM). One EVENT = one seeded
# stock replacement (keyed on the supply, A-7) applied to the EDB and propagated through the dirty set;
# timed with clock_ms around 10^5 events. The seeded event stream is drawn BEFORE the clock starts (the
# LCG is the harness, not the engine -- R §12.3's 82 ns is likewise one event's delta). The program is gen_dl.bp's
# generated program for set 1 (its joins generated code); each run also reports whether the incremental
# relation equals a from-scratch re-derivation after the last event, and a run that is not equal is a
# failure, never a number. Median of 5 runs; the interpreted engine (no generated code) is printed beside
# it for reference, median of 3.
#
# Prints `dl_event_ns <median> (runs: ...) interpreted <median> (runs: ...) set 1 events 100000` and exits 1
# when the median exceeds 1000, 2 on a missing prerequisite (a NAMED refusal), 3 when a run is not equal.
# env: BEBOP_BIN (candidate, default ./bebop.bin), BEBOP_TMP. Run it through tools/slot.sh.
set -u
cd "$(dirname "$0")/../.." || exit 1
BB=$(realpath -m "${BEBOP_BIN:-./bebop.bin}"); SEED=$(realpath ./seed/build/seed)
T=${BEBOP_TMP:-/tmp/opencode}/dl_bench; rm -rf "$T"; mkdir -p "$T"
[ -s "$BB" ] || { echo "dl_bench: REFUSED -- candidate $BB missing or empty"; exit 2; }
"$SEED" "$BB" compile bench/vs_rust/std_tests/dl_gen.bp "$T/dl_gen.bin" > /dev/null 2> "$T/gen.err" \
  || { echo "dl_bench: REFUSED -- dl_gen.bp does not compile: $(tail -1 "$T/gen.err")"; exit 2; }
timeout 60 "$SEED" "$T/dl_gen.bin" "$T" > "$T/gen.out" 2>&1 || { echo "dl_bench: REFUSED -- gen_dl failed: $(tail -1 "$T/gen.out")"; exit 2; }
"$SEED" "$BB" compile "$T/dl_g1.bp" "$T/dl_g1.bin" > /dev/null 2> "$T/g1.err" \
  || { echo "dl_bench: REFUSED -- the generated dl_g1.bp does not compile: $(tail -1 "$T/g1.err")"; exit 2; }
one() {  # one <args...>: the ns of one run, or a named failure on stdout (exit 3)
  local o; o=$(timeout 300 "$SEED" "$T/dl_g1.bin" "$@" 2>&1); local rc=$?
  local line; line=$(grep -m1 '^dl_event_ns ' <<<"$o")
  [ "$rc" = 0 ] && [ -n "$line" ] || { echo "RUNFAIL(rc=$rc: $(tail -1 <<<"$o"))"; return 3; }
  grep -q ' equal 1$' <<<"$line" || { echo "NOTEQUAL($line)"; return 3; }
  awk '{print $2}' <<<"$line"
}
med() { printf '%s\n' "$@" | sort -n | sed -n "$(( ($# + 1) / 2 ))p"; }
gen=(); for i in 1 2 3 4 5; do v=$(one bench) || { echo "dl_bench: $v"; exit 3; }; gen+=("$v"); done
itp=(); for i in 1 2 3; do v=$(one bench interp) || { echo "dl_bench: interpreted $v"; exit 3; }; itp+=("$v"); done
m=$(med "${gen[@]}"); mi=$(med "${itp[@]}")
echo "dl_event_ns $m (runs: ${gen[*]}) interpreted $mi (runs: ${itp[*]}) set 1 events 100000"
[ "$m" -le 1000 ]
