#!/bin/bash
# R3 compilation metrics — measures build times, sizes, reproducibility.
# Does NOT modify the main Makefile; copies the exact cc command from `make -B -n`.
set -u
NATIVE=/root/dowiz/bebop-lang/native
R3=/root/dowiz/bebop-lang/bench/r3
cd "$NATIVE" || exit 1
export TIMEFORMAT='%3R real %3U user %3S sys'

: > "$R3/raw.log"

# --- capture exact compile command + src list from make (no build) ---
CCLINE=$(make -B -n 2>/dev/null | grep -E '^cc ' | head -1)
SRC=$(echo "$CCLINE" | grep -oE 'src/[^ ]+\.(c|S)' | tr '\n' ' ')
FLAGS_BASE="-std=c11 -Wall -Wextra -Wpedantic -Werror -Wshadow -Wstrict-prototypes -Wmissing-prototypes -Wundef -Wformat=2"
echo "SRC_count=$(echo $SRC | wc -w)" >> "$R3/raw.log"
echo "CCLINE=$CCLINE" >> "$R3/raw.log"

# --- helper: build a variant with timing + 1 retry on transient fs failure ---
build_variant() {
  local opt="$1" out="$2" label="$3"
  local cmd="cc $opt $FLAGS_BASE -o $out $SRC -lm -lpthread"
  for attempt in 1 2; do
    rm -f "$out"
    { time $cmd ; } 2> "$R3/time_${label}.txt"
    if [ -f "$out" ]; then
      echo "${label}: OK attempt=$attempt" >> "$R3/raw.log"
      cat "$R3/time_${label}.txt" >> "$R3/raw.log"
      return 0
    fi
    echo "${label}: attempt=$attempt FAILED, retrying" >> "$R3/raw.log"
    cat "$R3/time_${label}.txt" >> "$R3/raw.log"
    sleep 2
  done
  echo "${label}: FAILED after 2 attempts" >> "$R3/raw.log"
  return 1
}

# --- (b) incremental build: touch one src/*.c, time make (default -O3 -flto) ---
touch src/arena.c
{ time make ; } 2> "$R3/time_incremental.txt"
echo "incremental_exit=$?" >> "$R3/raw.log"
cat "$R3/time_incremental.txt" >> "$R3/raw.log"
echo "--- incremental make log tail ---" >> "$R3/raw.log"

# --- (d) code-size breakdown of default build ---
size build/bebopc > "$R3/size_default.txt" 2>&1

# --- (e) reproducible: first sha256 of current build ---
sha256sum build/bebopc > "$R3/sha_first.txt" 2>&1

# --- (c) optimization-level variants ---
build_variant "-O0"        "$NATIVE/build/bebopc_O0"     "O0"
build_variant "-O2"        "$NATIVE/build/bebopc_O2"     "O2"
build_variant "-O2 -flto"  "$NATIVE/build/bebopc_O2flto" "O2flto"

# sizes of variants
size build/bebopc_O0     > "$R3/size_O0.txt"     2>&1
size build/bebopc_O2     > "$R3/size_O2.txt"     2>&1
size build/bebopc_O2flto > "$R3/size_O2flto.txt" 2>&1

# --- (e) reproducible: rebuild default from clean, second sha256 ---
for attempt in 1 2; do
  make clean >/dev/null 2>&1
  { time make ; } 2> "$R3/time_repro2.txt"
  if [ -f build/bebopc ]; then
    echo "repro2: OK attempt=$attempt" >> "$R3/raw.log"
    cat "$R3/time_repro2.txt" >> "$R3/raw.log"
    break
  fi
  echo "repro2: attempt=$attempt FAILED, retrying" >> "$R3/raw.log"
  sleep 2
done
sha256sum build/bebopc > "$R3/sha_second.txt" 2>&1

echo "ALL_DONE" >> "$R3/raw.log"
