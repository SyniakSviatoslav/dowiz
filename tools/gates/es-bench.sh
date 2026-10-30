#!/bin/sh
# ES-BENCH — bebop-store must not get slower (R-EMBED-STORE §4.6, T-ES-50).
#
# WHAT IT MEASURES. Four CPU-only rows over a warm page cache, each as a RATIO of bebop's time
# to a same-process std control doing the same logical work (crates/es-bench/src/gate.rs):
# get (100k random point gets), prefix (1000 x ~100-key scans), verify (bebop's verify-on-open
# crc over the image), build (compacted_bytes_fit). The two are interleaved inside every
# repetition, so a slow second of the machine lands on both. RED when a row's median ratio
# is past its committed baseline by more than the baseline's tol_pct.
#
# WHAT IT DOES NOT. Write rows (unsynced put, durable commit) are report-only, as §4.6 says:
# fsync time on this box is proot's (~273 us per syscall), not the engine's. redb / LMDB /
# SQLite are NOT built here -- the full table is `cargo run --release --features competitors
# -- full all ...` (docs/measurements/ES-BENCH-2026-10-29.md); the gate builds bebop only.
#
# BASELINE PER ARCHITECTURE: crates/es-bench/baseline-<arch>.txt. A ratio cancels the clock, not
# the cache shape, so an x86 runner is not held to an ARM number: with no file for this arch
# the gate REFUSES (exit 2), it does not pass. Record one with
#   (cd crates/es-bench && cargo run --release -- gate-record 15 5 <tol>) > baseline-<arch>.txt
#
# Usage: tools/gates/es-bench.sh [--bin PATH]   (--bin: a prebuilt es-bench, used by the proof)
#        ES_BENCH_CORE=5 pins to one core (the box's A78; the baseline was recorded so).
# Never call it inside slot.sh's own children; on the box run the WHOLE gate under slot.sh.
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
CRATE="$ROOT/crates/es-bench"
ARCH=$(uname -m)
BASE="$CRATE/baseline-$ARCH.txt"
BIN=""
[ "${1:-}" = "--bin" ] && BIN="$2"
if [ -z "$BIN" ]; then
  T="${CARGO_TARGET_DIR:-$CRATE/target}"
  ( cd "$CRATE" && CARGO_TARGET_DIR="$T" cargo build --release --quiet ) || { echo "es-bench: BUILD FAILED"; exit 1; }
  BIN="$T/release/es-bench"
fi
[ -x "$BIN" ] || { echo "es-bench: no binary at $BIN"; exit 1; }
SCR=$(mktemp -d "${TMPDIR:-/tmp}/es-bench-gate.XXXXXX")
# No baseline for this arch: still REFUSED (exit 2), but on CI record one and print it, so the
# runner's own numbers can be committed -- the box is aarch64 and cannot measure an x86 ratio.
if [ ! -f "$BASE" ] && [ -n "${GITHUB_ACTIONS:-}" ]; then
  # The record goes OUTSIDE $SCR: gate-record deletes its whole ES_BENCH_DIR when it ends.
  REC=$(mktemp "${TMPDIR:-/tmp}/es-bench-rec.XXXXXX")
  ES_BENCH_DIR="$SCR" "$BIN" gate-record "${ES_BENCH_REPS:-15}" 5 15 > "$REC"
  rrc=$?
  echo "es-bench: recorded on this runner, rc=$rrc (commit as crates/es-bench/baseline-$ARCH.txt):"
  grep -v '^# get:\|^# prefix:\|^# verify:\|^# build:' "$REC" || echo "es-bench: the record is EMPTY"
  rm -rf "$SCR" "$REC"
  echo "es-bench: REFUSED -- no baseline at $BASE; rc=2 (arch $ARCH)"
  exit 2
fi
PIN=""
[ -n "${ES_BENCH_CORE:-}" ] && command -v taskset >/dev/null && PIN="taskset -c $ES_BENCH_CORE"
ES_BENCH_DIR="$SCR" $PIN "$BIN" gate "$BASE" "${ES_BENCH_REPS:-15}"
rc=$?
rm -rf "$SCR"
echo "es-bench: rc=$rc (arch $ARCH, baseline $BASE)"
exit $rc
