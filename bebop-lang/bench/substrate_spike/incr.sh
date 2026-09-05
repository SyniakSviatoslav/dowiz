#!/usr/bin/env bash
# T107 gate: incremental substrate (sweep over a dirty bitset) vs full recompute on
# a 2^16-cell grid DAG, k changed inputs in {1,16,256,4096}, bebop vs its Rust twin,
# folds cross-checked per (k, mode). Pass = the crossover k (first k where the sweep
# stops beating the full recompute) is recorded for both engines. env: BEBOP_BIN,
# BEBOP_TMP, R (runs, default 3). Writes RESULT-incr.md.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; R=${R:-3}
T=${BEBOP_TMP:-/tmp/opencode}/incr; mkdir -p "$T"
[ -s "$BEBOP_BIN" ] || { echo "GUARD: BEBOP_BIN=$BEBOP_BIN missing or empty (L12)"; exit 1; }
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")
./seed/build/seed "$BEBOP_BIN" compile bench/substrate_spike/incr.bp "$T/incr.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL incr"; exit 1; }
rustc -O -o "$T/twin" bench/substrate_spike/incr_twin.rs 2>/dev/null || { echo "RUSTC FAIL"; exit 1; }
med() { printf '%s\n' "$@" | sort -n | awk '{a[NR]=$1} END{print a[int((NR+1)/2)]}'; }
out="# T107 incremental-substrate curve ($(date -u +%F), $(md5sum "$BEBOP_BIN" | cut -c1-8), core $PIN, R=$R medians, us per rep of k changes)

| k | bebop sweep | bebop full | sweep/full | Rust sweep | Rust full | sweep/full | folds |
|---|---|---|---|---|---|---|---|"
xb=""; xr=""; fail=0
for k in 1 16 256 4096; do
  declare -A us; declare -A fold
  for m in s f; do
    bs=(); rs=()
    fold[b$m]=$(taskset -c "$PIN" ./seed/build/seed "$T/incr.bin" "$k" "$m" f | tail -1)
    for i in $(seq "$R"); do
      bs+=($(taskset -c "$PIN" ./seed/build/seed "$T/incr.bin" "$k" "$m" t | tail -1))
      o=$(taskset -c "$PIN" "$T/twin" "$k" "$m"); rs+=($(sed 's/.*us=\([0-9]*\).*/\1/' <<<"$o")); fold[r$m]=$(sed 's/.*fold=\(-*[0-9]*\).*/\1/' <<<"$o")
    done
    us[b$m]=$(med "${bs[@]}"); us[r$m]=$(med "${rs[@]}")
  done
  ok=$([ "${fold[bs]}" = "${fold[bf]}" ] && [ "${fold[bf]}" = "${fold[rs]}" ] && [ "${fold[rs]}" = "${fold[rf]}" ] && echo equal || echo "MISMATCH ${fold[bs]} ${fold[bf]} ${fold[rs]} ${fold[rf]}")
  [ "$ok" = equal ] || fail=1
  rb=$(python3 -c "print(f'{${us[bs]}/max(${us[bf]},1):.2f}x')"); rr=$(python3 -c "print(f'{${us[rs]}/max(${us[rf]},1):.2f}x')")
  [ -z "$xb" ] && [ "${us[bs]}" -ge "${us[bf]}" ] && xb=$k
  [ -z "$xr" ] && [ "${us[rs]}" -ge "${us[rf]}" ] && xr=$k
  out="$out
| $k | ${us[bs]} | ${us[bf]} | $rb | ${us[rs]} | ${us[rf]} | $rr | $ok |"
done
out="$out

- crossover (first k where sweep >= full): bebop ${xb:->4096}, Rust ${xr:->4096}; k/N = $(python3 -c "print(f'{${xb:-8192}/65536:.2%}')") (bebop)
- N = 65536 cells (16 layers x 4096), 64 reps per measurement, same LCG change set in both modes and engines"
echo "$out" > bench/substrate_spike/RESULT-incr.md; echo "$out"
[ "$fail" = 0 ] || { echo "T107 FAIL: fold mismatch"; exit 1; }
