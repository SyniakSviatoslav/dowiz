#!/usr/bin/env bash
# dev_loop.sh (2026-09-06): inner development-loop timings for one compiler binary.
# Usage: bench/dev_loop.sh [compiler.bin (default ./bebop.bin)] [label (default md5)]
# env: CPU (core for pinned steps, default 4), BEBOP_TMP (scratch root), DATE,
#      STEPS (subset of abcdef to run, default all).
# Prints `metric value unit` lines + a markdown table, and appends a section to
# docs/DEV-LOOP.md. Never touches ./bebop.bin or ./bebop.bp.
cd "$(dirname "$0")/.." || exit 1
ulimit -s 65536 2>/dev/null
BIN=$(realpath "${1:-./bebop.bin}"); [ -s "$BIN" ] || { echo "GUARD: $BIN missing or empty"; exit 1; }
MD5=$(md5sum "$BIN" | cut -c1-8); LABEL=${2:-$MD5}
CPU=${CPU:-4}; DATE=${DATE:-$(date +%F)}; STEPS=${STEPS:-abcdef}
S=${BEBOP_TMP:-/tmp/opencode}/devloop; mkdir -p "$S"
SEED=./seed/build/seed
declare -a ROWS

wall() {  # wall <log> <cmd...> -> "<seconds> <rc>" on stdout (read into v and RC)
  local log=$1; shift; local t0 t1 rc
  t0=$(date +%s.%N); "$@" >"$log" 2>&1; rc=$?; t1=$(date +%s.%N)
  awk -v a="$t0" -v b="$t1" -v r="$rc" 'BEGIN{printf "%.2f %d", b-a, r}'
}
step() { [[ $STEPS == *$1* ]]; }
row() { echo "$1 $2 $3${4:+ ($4)}"; ROWS+=("| $1 | $2 | $3 | ${4:-} |"); }

if step a; then
# a. self-compile, one generation, cold (.becache removed so the memo cannot short-circuit)
rm -f "$S"/gen.bin "$S"/gen.bin.becache
read -r v RC < <(wall "$S/a.log" taskset -c "$CPU" $SEED "$BIN" compile bebop.bp "$S/gen.bin")
row selfcompile "$v" s "rc=$RC, cpu$CPU"
fi

if step b; then
# b. one std gate cold compile: sgraph2.bp
rm -f "$S"/sgraph2.bin "$S"/sgraph2.bin.becache
read -r v RC < <(wall "$S/b.log" taskset -c "$CPU" $SEED "$BIN" compile bench/vs_rust/std_tests/sgraph2.bp "$S/sgraph2.bin")
row std_compile_sgraph2 "$v" s "rc=$RC, cpu$CPU"
fi

if step c; then
# c. sequential std_golden (unpinned)
read -r v RC < <(wall "$S/c.log" env BEBOP_TMP="$S/seq" BEBOP_BIN="$BIN" bash bench/vs_rust/std_golden.sh)
row std_golden_seq "$v" s "$(grep '^std_golden:' "$S/c.log" | tail -1), unpinned"
fi

if step d; then
# d. sharded std_golden J=3 (unpinned; std_par pins its shards itself)
read -r v RC < <(wall "$S/d.log" env J=3 BEBOP_TMP="$S/par" BEBOP_BIN="$BIN" bash tools/std_par.sh)
row std_golden_par3 "$v" s "$(grep '^std_golden:' "$S/d.log" | tail -1), unpinned"
fi

if step e; then
# e. fuzz throughput, 30 seeds from 41000, J=1 (repros kept in scratch, not the repo)
read -r v RC < <(wall "$S/e.log" env J=1 REPROS="$S/fz/repros" BEBOP_TMP="$S/fz" BEBOP_BIN="$BIN" taskset -c "$CPU" bash bench/fuzz/fuzz.sh 30 41000)
sum=$(grep '^fuzz:' "$S/e.log" | tail -1); rate=$(sed -n 's/.*rate=\([0-9.]*\)\/s.*/\1/p' <<<"$sum")
row fuzz_rate "${rate:-NA}" seeds/s "wall=${v}s, $(sed 's/^fuzz: //; s/ wall=.*//' <<<"$sum"), cpu$CPU"
fi

if step f; then
# f. construct parity
read -r v RC < <(wall "$S/f.log" env BEBOP_TMP="$S/cp" BEBOP_BIN="$BIN" taskset -c "$CPU" bash bench/vs_rust/construct_parity.sh)
row construct_parity "$v" s "$(grep '^construct parity:' "$S/f.log" | tail -1), cpu$CPU"
fi

DOC=docs/DEV-LOOP.md
[ -f "$DOC" ] || cat >"$DOC" <<'HDR'
# DEV-LOOP — inner development-loop timings

Produced by `bench/dev_loop.sh <bin> <label>`; one section per run, appended.
Caveats: box shared with another agent's compile jobs (cpus 5-6 busy); pinned
steps use `taskset -c <cpu>` on one A78 core; std_golden runs are unpinned
(the sharded one pins its own shards to the A78 cores). Numbers are single
runs, not medians. Logs stay under `$BEBOP_TMP/devloop`.
HDR
{
  echo; echo "## $LABEL — bin $MD5, $DATE, pinned cpu$CPU"; echo
  echo "| metric | $LABEL | unit | note |"; echo "|---|---|---|---|"
  printf '%s\n' "${ROWS[@]}"
} | tee -a "$DOC"
