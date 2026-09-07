#!/usr/bin/env bash
# B3 step 4 (docs/blueprints/B3-graphblas-kernels-prejit.md section 5 step 4, design item (b)/
# (e)): builds selfhost/tools/gbpool.bp the pool_parity.sh way (a main-stripped copy of
# bebop.bp concatenated ahead of it -- gb_compile1.bp needs the compiler's own emission
# functions in-process), runs it to PreJIT the 12-kernel pool, then measures the three latency
# tiers (tier0, specialised compile, pool hit) via bench/vs_rust/std_tests/gb_pool.bp (same
# concatenation). Appends a row to bench/vs_rust/RESULT-gbpool.md. Does NOT touch sgraph2.sh.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null || true
T=${BEBOP_TMP:-/tmp/opencode}; mkdir -p "$T"
BB=${BEBOP_BIN:-./bebop.bin}
[ -s "$BB" ] || { echo "GUARD: BEBOP_BIN=$BB missing or empty"; exit 1; }
SEED=./seed/build/seed

trim_bebop() { awk '/^fn main\(/{skip=1} !skip{print} skip&&/^}/{skip=0}' bebop.bp; }

# ---- build the PreJIT pool ----
SCR="$T/gbpool_build.bp"
trim_bebop > "$SCR"
cat selfhost/tools/gbpool.bp >> "$SCR"
t0=$(date +%s%N)
"$SEED" "$BB" compile "$SCR" "$T/gbpool_build.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL gbpool"; exit 1; }
t1=$(date +%s%N)
rm -f "$T/gbpool.gbpool"
V=$(taskset -c 4 "$SEED" "$T/gbpool_build.bin" "$BB" "$T" "$T/gbpool.gbpool" | tail -1)
t2=$(date +%s%N)
build_compile_ms=$(( (t1 - t0) / 1000000 ))
build_run_ms=$(( (t2 - t1) / 1000000 ))
kcount=$(( V / 1000000 )); kbytes=$(( V % 1000000 ))

# ---- latency rows via the gate driver (also prints TIER0_MS/COMPILE_MS/HIT_MS lines) ----
SCR2="$T/gbpool_lat.bp"
trim_bebop > "$SCR2"
cat bench/vs_rust/std_tests/gb_pool.bp >> "$SCR2"
"$SEED" "$BB" compile "$SCR2" "$T/gbpool_lat.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL gb_pool driver"; exit 1; }
OUT=$(taskset -c 4 "$SEED" "$T/gbpool_lat.bin" "$BB" "$T" "$T/gbpool_lat.gbpool")
fold=$(tail -1 <<<"$OUT")
tier0_ms=$(awk '$1=="TIER0_MS"{print $2}' <<<"$OUT")
compile_ms=$(awk '$1=="COMPILE_MS"{print $2}' <<<"$OUT")
hit_ms=$(awk '$1=="HIT_MS"{print $2}' <<<"$OUT")

met() { awk -v v="$1" -v lim="$2" 'BEGIN{print (v+0<=lim)?"MET":"UNMET"}'; }
t0_met=$(met "${tier0_ms:-999}" 1)
tc_met=$(met "${compile_ms:-999}" 50)
th_met=$(met "${hit_ms:-999}" 0.1)
ratio=$(awk -v a="${tier0_ms:-0}" -v b="${compile_ms:-1}" 'BEGIN{if(b<=0)b=0.001; printf "%.2f", a/b}')
r_met=$(met "$ratio" 5)

FOLD_WANT=-4783772994166464769
fold_ok=$([ "$fold" = "$FOLD_WANT" ] && echo equal || echo "MISMATCH($fold)")

out="
## B3 step 4 ($(date -u +%F), $(md5sum "$BB" | cut -c1-8)): PreJIT pool + tier dispatch -- fold $fold_ok

| row | value | target | verdict |
|---|---|---|---|
| PreJIT build: compile gbpool.bp (ms) | $build_compile_ms | - | - |
| PreJIT build: run (generate+compile+append 12 kernels, ms) | $build_run_ms | - | - |
| pool: kernel count | $kcount | 12 | $([ "$kcount" = 12 ] && echo MET || echo UNMET) |
| pool: total bytes | $kbytes | - | - |
| tier 0 (first query, one gb_mxv_generic call, ms) | ${tier0_ms:-NA} | <=1 | $t0_met |
| specialised compile (one kernel, in-process, ms) | ${compile_ms:-NA} | <=50 | $tc_met |
| pool hit (mmap-cached dispatch, ms) | ${hit_ms:-NA} | <=0.1 | $th_met |
| tier0/specialised ratio | $ratio | <=5x | $r_met |
- 1M/5M sgraph2 LCG graph row: not measured (BOX SAFETY time budget -- the 1k-graph rows above
  are the ones the blueprint's section 7 gate names; a 1M-scale gb_mxv specialised kernel is a
  step-4(f)-adjacent item, not gated here).
- fold: pool-hit == tier-0 == the gb_gen golden (bench/oracles/gb_pool.py), rolling-combined the
  same way as gb_gen_tier0/gb_gen_specialised.
"
printf '%s\n' "$out" >> bench/vs_rust/RESULT-gbpool.md
printf '%s\n' "$out"
