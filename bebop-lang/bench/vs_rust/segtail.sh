#!/usr/bin/env bash
# B4 step 2'' driver: SEGMENTED TAIL vs today's whole-tail CoW, arms INTERLEAVED in one
# pinned run. Probe only -- its own store, no gate row, sgraph2.bp untouched.
# NOTE: tools/slot.sh exports PIN as a COMMAND PREFIX, so this script never reads $PIN.
# Every run is checked for empty output and fails loudly.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
T=${BEBOP_TMP:-/root/s30/outB3}; mkdir -p "$T"; BB=${BEBOP_BIN:-./bebop.bin}; REPS=${REPS:-3}; RUN="taskset -c 4"
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/segtail.bp "$T/segtail.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL segtail"; exit 1; }
med() { printf '%s\n' "$@" | sort -n | awk '{a[NR]=$1} END{print a[int((NR+1)/2)]}'; }
# g <arm> <want> <capS> <cap>
g() { rm -f segtail.store segtail.store.tmp
      local v; v=$($RUN ./seed/build/seed "$T/segtail.bin" "$1" "$2" "$3" "$4" 100 | tail -1)
      [ -n "$v" ] || { echo "EMPTY OUTPUT arm=$1 want=$2 capS=$3 cap=$4" >&2; exit 2; }
      printf '%s' "$v"; }
echo "== headline: 100 batches x 10^4 entries, cap 100000, arms interleaved x$REPS (ns per entry) =="
W=(); S=()
for r in $(seq 1 "$REPS"); do W+=("$(g w t 256 100000)"); S+=("$(g s t 256 100000)"); done
echo "armW_whole_cow_ns  reps=${W[*]} median=$(med "${W[@]}")"
echo "armS_segmented_ns  reps=${S[*]} median=$(med "${S[@]}")   (capS=256)"
echo "armW_store_bytes $(g w z 256 100000)   armS_store_bytes $(g s z 256 100000)"
echo "== segment-size sweep, cap 100000 (ns per entry) =="
for cs in 64 128 256 512 1024 4096 16384; do echo "  capS=$cs  ns=$(g s t $cs 100000)  store=$(g s z $cs 100000)"; done
echo "== tail-cap sweep: the floor's dependence on the cap (this is what segmentation frees) =="
for c in 100000 200000 500000; do echo "  cap=$c  armW_ns=$(g w t 256 $c)  armS_ns=$(g s t 256 $c)"; done
echo "  cap=1000000  armW_ns=SKIPPED(store>800MB)  armS_ns=$(g s t 256 1000000)"
echo "== folds: final tail (f) and the PINNED batch-1 tail re-read after 100 batches (v) =="
echo "  armW  f=$(g w f 256 100000)  v=$(g w v 256 100000)"
for cs in 64 256 16384; do echo "  armS capS=$cs  f=$(g s f $cs 100000)  v=$(g s v $cs 100000)"; done
echo "== python oracle =="
$RUN python3 - <<'PY'
M=(1<<64)-1
def lcg(x): return (x*M//M*0 + (x*6364136223846793005+1442695040888963407))&M
x=777; s1=0; tail=[]; first=None
for b in range(100):
    if s1 >= 100000: s1=0; tail=[]
    for k in range(10000):
        x=lcg(x); a=(x>>20)%1000000
        x=lcg(x); w=(x>>20)%1000000
        tail.append((a,w))
    s1 += 10000
    if b==0: first=list(tail)
print("  oracle_final_fold", sum((1+a)*(1+w) for a,w in tail))
print("  oracle_pin_fold  ", sum((1+a)*(1+w) for a,w in first))
PY
echo "== done =="
