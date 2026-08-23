#!/usr/bin/env bash
# Full Bebop-vs-Rust benchmark run. Produces results/*.txt consumed by aggregate.py
set -u
cd "$(dirname "$0")"
ROOT=../..
BEBOPC=$ROOT/native/build/bebopc
EXECW=$ROOT/native/build/exec_words
R=31
mkdir -p results

echo "== [1/5] compiling .bp kernels (compilewords) =="
for k in k1 k2 k3 k4; do
  T0=$(date +%s%N)
  $BEBOPC compilewords $ROOT/selfhost/expr_compile.bp kernels/$k.bp > /tmp/opencode/$k.full 2>results/$k.compile.err
  RC=$?
  T1=$(date +%s%N)
  grep -v "^OFF" /tmp/opencode/$k.full > /tmp/opencode/$k.w
  grep "^OFF" /tmp/opencode/$k.full > /tmp/opencode/$k.off
  KB=$(( $(wc -c < kernels/$k.bp) + 0 ))
  echo "compile_bp_ns=$((T1-T0)) bytes=$KB" >> results/compile_throughput.txt
  echo "$k compiled rc=$RC in $(( (T1-T0)/1000000 ))ms ($(wc -l < /tmp/opencode/$k.w | head -1)) words"
done

# C + Rust build times (fresh, measured)
echo "== [2/5] building C and Rust references =="
: > results/compile_throughput.txt
for k in k1 k2 k3 k4; do
  T0=$(date +%s%N); $BEBOPC compilewords $ROOT/selfhost/expr_compile.bp kernels/$k.bp > /dev/null 2>&1; T1=$(date +%s%N)
  echo "bebop $k $((T1-T0)) $(wc -c < kernels/$k.bp)" >> results/compile_throughput.txt
done
rm -f c/kernels
T0=$(date +%s%N); gcc -O2 -std=c11 -o c/kernels c/kernels.c; T1=$(date +%s%N)
echo "gcc all-in-one $((T1-T0)) $(wc -c < c/kernels.c)" >> results/compile_throughput.txt
cargo build --release --manifest-path rust/Cargo.toml 2>/dev/null   # warm target dir first
T0=$(date +%s%N); touch rust/src/main.rs; cargo build --release --manifest-path rust/Cargo.toml 2>/dev/null >/dev/null; T1=$(date +%s%N)
echo "rustc all-in-one $((T1-T0)) $(wc -c < rust/src/main.rs)" >> results/compile_throughput.txt

echo "== [3/5] running kernel benchmarks R=$R =="
for k in 1 2 3 4; do
  # bebop (compiled words executed natively)
  $EXECW /tmp/opencode/k$k.w $R /tmp/opencode/k$k.off > results/k${k}.bebop.txt 2>/dev/null
  # C reference
  ./c/kernels $k $R > results/k${k}.c.txt
  # Rust release
  ./rust/target/release/kernels $k $R > results/k${k}.rust.txt
done

echo "== [4/5] startup + RSS + sizes =="
: > results/startup.txt
# trivial workload per stack: bebop trivial kernel words
printf 'fn main() -> i64 { 42 }\n' > /tmp/opencode/ktriv.bp
$BEBOPC compilewords $ROOT/selfhost/expr_compile.bp /tmp/opencode/ktriv.bp > /tmp/opencode/ktriv.full 2>/dev/null
grep -v "^OFF" /tmp/opencode/ktriv.full > /tmp/opencode/ktriv.w
grep "^OFF" /tmp/opencode/ktriv.full > /tmp/opencode/ktriv.off
T0=$(date +%s%N); for i in $(seq 1 50); do $EXECW /tmp/opencode/ktriv.w 1 /tmp/opencode/ktriv.off >/dev/null 2>&1; done; T1=$(date +%s%N)
echo "bebop_exec_words $(( (T1-T0)/50 ))" >> results/startup.txt
T0=$(date +%s%N); for i in $(seq 1 50); do ./c/kernels 1 1 >/dev/null 2>&1; done; T1=$(date +%s%N)
echo "c_binary $(( (T1-T0)/50 ))" >> results/startup.txt
T0=$(date +%s%N); for i in $(seq 1 50); do ./rust/target/release/kernels 1 1 >/dev/null 2>&1; done; T1=$(date +%s%N)
echo "rust_binary $(( (T1-T0)/50 ))" >> results/startup.txt

./rssrun $EXECW /tmp/opencode/k1.w 1 /tmp/opencode/k1.off 2>results/rss_bebop.txt >/dev/null
./rssrun ./c/kernels 1 1 2>results/rss_c.txt >/dev/null
./rssrun ./rust/target/release/kernels 1 1 2>results/rss_rust.txt >/dev/null

{
  stat -c "bebop_wordstream_file %s" /tmp/opencode/k1.w
  stat -c "exec_words_tool %s" $EXECW
  stat -c "c_kernels_bin %s" c/kernels
  stat -c "rust_kernels_bin %s" rust/target/release/kernels
} > results/sizes.txt

echo "== [5/5] done — see results/ =="
