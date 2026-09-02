#!/usr/bin/env bash
# R6.3 honest bench: 31 in-process clock_ms runs per kernel (Bebop) vs
# the Rust release twins (internal CLOCK_MONOTONIC ns). set -u; no C.
set -u
cd "$(dirname "$0")/../../.."
SEED=./seed/build/seed
R=31
mkdir -p bench/vs_rust/results630
for k in k1t k2t k3t k4t; do
  $SEED bebop.bin compile bench/vs_rust/bench630/$k.bp bench/vs_rust/bench630/$k.bin >/dev/null 2>&1 || { echo "COMPILEFAIL $k"; exit 1; }
  : > bench/vs_rust/results630/$k.bebop.txt
  echo "result=timing" >> bench/vs_rust/results630/$k.bebop.txt
  printf "ms10" >> bench/vs_rust/results630/$k.bebop.txt
  for i in $(seq 1 $R); do
    v=$($SEED bench/vs_rust/bench630/$k.bin 2>/dev/null | tail -1)
    printf " %s" "$v" >> bench/vs_rust/results630/$k.bebop.txt
  done
  echo "" >> bench/vs_rust/results630/$k.bebop.txt
  echo "bebop $k done"
done
KB=bench/vs_rust/rust/target/release/kernels
if [ -x "$KB" ]; then
  for kn in 1 2 3 4; do
    k="k$kn"
    : > bench/vs_rust/results630/$k.rust.txt
    echo "result=timing" >> bench/vs_rust/results630/$k.rust.txt
    printf "ns" >> bench/vs_rust/results630/$k.rust.txt
    for i in $(seq 1 $R); do
      v=$($KB $kn $R 2>/dev/null | grep '^ns' | tail -1 | awk '{print $2}')
      printf " %s" "$v" >> bench/vs_rust/results630/$k.rust.txt
    done
    echo "" >> bench/vs_rust/results630/$k.rust.txt
    echo "rust $k done"
  done
else
  echo "rust binary missing: $KB"
fi
