#!/usr/bin/env bash
# miri-gate.sh — CI enforcement for the kernel's Miri UB-interpreter gate (roadmap item 52).
#
# Runs the `mode=miri` rows of the SAME manifest the hardening-gate/kani-gate read
# (docs/audits/hardening/HOT-PATHS.tsv) — one source of truth, one gap ledger — but in a SEPARATE
# CI job, because Miri is a network-installed nightly toolchain (its own rustc + the `miri`
# component) with minutes-scale interpretation, incompatible with hardening-gate's fast
# `--locked --offline` per-PR design and with kani-gate's CBMC workflow (roadmap item 7).
#
# Miri re-executes the REAL unsafe surface (the bump-allocator raw-pointer math in `arena`, the
# scalar/SIMD-fallback paths in `simd`/`householder`) as an interpreter — turning the aspirational
# Miri doc-comments into an enforced, re-run gate. It cannot reach `_rdtsc` / raw `syscall` / inline
# asm (fdr/pmu.rs) or AVX2 intrinsic bodies, which stay covered by items 37/39/7 (per BLUEPRINT-ITEM-52).
#
# Two row kinds (mirrors kani-gate.sh's floor + planted-fault idiom):
#   * normal  (min_tests = N >= 1): `cargo miri test <filter>` MUST exit 0 with parsed "N passed"
#     >= min. A filter matching ZERO tests is RED — deleting/renaming a test can no longer stay green.
#   * planted (min_tests = "UB"):    `cargo miri test <filter>` MUST exit NON-zero AND its output must
#     contain a Miri "Undefined Behavior" report — i.e. the planted-UB self-test is CAUGHT. If Miri
#     exits 0 (UB uncaught) the gate goes RED: the gate detects real UB, not vacuously.
#
# Zero-dep stays mechanically true: the only added source (`kernel/src/miri_selftest.rs`) is behind
# `#[cfg(any(test, miri))]`, compiled out of every normal build; Miri is a CI-only tool, nothing
# enters Cargo.toml/Cargo.lock.
#
# Usage: miri-gate.sh          (runs every mode=miri row)
#   HARDENING_MANIFEST=<path>  override the manifest (default docs/audits/hardening/HOT-PATHS.tsv)
#   MIRI_NIGHTLY=<toolchain>   nightly pin for Miri (default nightly-2025-11-21; recorded in
#                              docs/audits/toolchain/miri-nightly-*.md)
set -uo pipefail

MANIFEST="${HARDENING_MANIFEST:-docs/audits/hardening/HOT-PATHS.tsv}"
KERNEL_DIR="kernel"
MIRI_NIGHTLY="${MIRI_NIGHTLY:-nightly-2025-11-21}"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT" || exit 2
[ -f "$MANIFEST" ] || { echo "::error::manifest not found: $MANIFEST"; exit 2; }

# Miri is a separate (nightly) toolchain. Fail loudly + name the toolchain issue if absent
# (mirrors kani-gate's bootstrap-failure path — a Miri bootstrap failure is a first-class
# reported outcome, not a silent skip; the arena payoff is the whole point).
if ! rustup toolchain list 2>/dev/null | grep -q "$MIRI_NIGHTLY"; then
  echo "::error::Miri nightly '$MIRI_NIGHTLY' not installed — run: rustup toolchain install $MIRI_NIGHTLY && rustup component add --toolchain $MIRI_NIGHTLY miri"
  exit 2
fi
if ! cargo "+$MIRI_NIGHTLY" miri --version >/dev/null 2>&1; then
  echo "::error::cargo miri not available on '$MIRI_NIGHTLY' — run: rustup component add --toolchain $MIRI_NIGHTLY miri"
  exit 2
fi

rc=0

# data rows: not a comment, not @ZONE, has >=6 columns
rows() { awk -F'\t' '!/^#/ && $1!="@ZONE" && NF>=6 {print}' "$MANIFEST"; }

# run ONE miri row. args: features filter min mode label
run_miri_row() {
  local features="$1" filter="$2" min="$3" mode="$4" label="$5"
  local featflag=()
  [ "$features" != "-" ] && featflag=(--features "$features")
  echo "  --- [$label] cargo +$MIRI_NIGHTLY miri test '$filter' ${featflag[*]} ---"
  local out miri_rc
  out="$(cd "$KERNEL_DIR" && cargo "+$MIRI_NIGHTLY" miri test --lib "${featflag[@]}" "$filter" 2>&1)"
  miri_rc=$?
  local passed
  passed="$(printf '%s\n' "$out" | grep -oE '[0-9]+ passed' | awk '{s+=$1} END{print s+0}')"
  passed="${passed:-0}"

  if [ "$min" = "UB" ]; then
    # Planted-fault row: MUST report Undefined Behavior (exit != 0 + UB text).
    if [ "$miri_rc" -ne 0 ] && printf '%s\n' "$out" | grep -qiE 'Undefined Behavior'; then
      echo "  OK  [$label] '$filter' → Miri reported Undefined Behavior (planted-UB CAUGHT, gate honest)"
      return 0
    fi
    echo "::error::[$label] '$filter' planted-UB NOT caught (exit $miri_rc, no 'Undefined Behavior' in output) — Miri is passing vacuously / broken"
    printf '%s\n' "$out" | grep -iE 'Undefined Behavior|error\[|panicked' | head -8 | sed 's/^/    /'
    return 1
  fi

  # Normal row: exit 0 AND passed >= min.
  if [ "$miri_rc" -ne 0 ]; then
    echo "::error::[$label] cargo miri test '$filter' FAILED (exit $miri_rc)"
    printf '%s\n' "$out" | grep -iE 'Undefined Behavior|error\[|FAILED|panicked' | head -8 | sed 's/^/    /'
    return 1
  fi
  if [ "$passed" -lt "$min" ]; then
    echo "::error::[$label] filter '$filter' matched $passed passing tests, need >= $min (zero-match/oracle-deleted = RED)"
    return 1
  fi
  echo "  OK  [$label] '$filter' → $passed passed (>= $min)"
  return 0
}

# ── Step D: unconditional floor — run every mode=miri row (a deleted proof goes RED next run) ──
echo "--- miri-gate: floor (all mode=miri rows, nightly=$MIRI_NIGHTLY) ---"
had_row=0
while IFS=$'\t' read -r path features filter min mode checklist gap; do
  [ -z "$path" ] && continue
  [ "$mode" = "miri" ] || continue
  # min_tests=0 = documented-not-filtered placeholder (e.g. fdr/pmu.rs: Miri cannot
  # interpret _rdtsc/raw-syscall/inline-asm; its coverage stays with items 37/39/7).
  # Visible in the gap ledger below; NOT run as a real filter (would match zero tests).
  [ "$min" = "0" ] && { echo "  ..  [row:$path] mode=miri min=0 → documented-not-filtered (NOT run; $filter)"; continue; }
  had_row=1
  # One-time Miri sysroot setup (idempotent; needed before the first interpretation).
  ( cd "$KERNEL_DIR" && cargo "+$MIRI_NIGHTLY" miri setup >/dev/null 2>&1 ) || true
  run_miri_row "$features" "$filter" "$min" "$mode" "miri:$path" || rc=1
done < <(rows | awk -F'\t' '$5=="miri" && !seen[$3 FS $2]++')  # dedup by (filter,features)
if [ "$had_row" -eq 0 ]; then
  echo "::error::no mode=miri rows in $MANIFEST — the miri-gate has nothing to prove (misconfiguration)"; rc=1
fi

# ── Step F: gap ledger (visible every run, never silently green) ──
echo "--- miri-gate: ledgered gaps (::warning::) ---"
while IFS=$'\t' read -r path features filter min mode checklist gap; do
  [ -z "$path" ] && continue
  [ "$mode" = "miri" ] || continue
  case "$gap" in -|"") ;; *) echo "::warning::[$path] gap: $gap" ;; esac
done < <(rows)

if [ "$rc" -eq 0 ]; then
  echo "=== miri-gate: GREEN ==="
else
  echo "=== miri-gate: RED ==="
fi
exit "$rc"
