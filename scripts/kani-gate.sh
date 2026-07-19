#!/usr/bin/env bash
# kani-gate.sh — CI enforcement for the kernel's Kani bounded-model-checker proofs (roadmap item 7).
#
# Runs the `mode=kani` rows of the SAME manifest the hardening-gate reads
# (docs/audits/hardening/HOT-PATHS.tsv) — one source of truth, one gap ledger — but in a SEPARATE
# CI job, because Kani is a network-installed toolchain (its own rustc + CBMC) with minutes-scale
# proof times, incompatible with hardening-gate's fast `--locked --offline` per-PR design.
#
# Anti-forgery core (mirrors hardening-gate §10-P7): every verdict is a live `cargo kani` run plus
# the PARSED "N successfully verified harnesses" count, asserted >= the manifest's min. A kani filter
# matching ZERO harnesses is RED — deleting/renaming a proof can no longer stay green.
#
# The PLANTED-FAULT self-test row (kernel/src/kani_selftest.rs) runs every invocation: a deliberate
# i32 overflow annotated #[kani::should_panic] that verifies SUCCESSFUL *only because the fault is
# caught*, proving the gate detects real bugs and is not passing vacuously.
#
# Zero-dep stays mechanically true: all harnesses live behind #[cfg(kani)]; nothing is added to
# Cargo.toml/Cargo.lock (the `kani` API crate is injected by `cargo kani` only under cfg(kani)).
#
# Usage: kani-gate.sh          (runs the floor: every mode=kani row with min>0, + placeholders)
#   HARDENING_MANIFEST=<path>  override the manifest (default docs/audits/hardening/HOT-PATHS.tsv)
#   KANI_EXTRA_ARGS="..."      extra flags passed to every cargo kani call (e.g. tighter --harness)
set -uo pipefail

MANIFEST="${HARDENING_MANIFEST:-docs/audits/hardening/HOT-PATHS.tsv}"
KERNEL_DIR="kernel"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT" || exit 2
[ -f "$MANIFEST" ] || { echo "::error::manifest not found: $MANIFEST"; exit 2; }
command -v cargo-kani >/dev/null 2>&1 || command -v kani >/dev/null 2>&1 || {
  echo "::error::cargo-kani not installed — run: cargo install kani-verifier --locked && cargo kani setup"; exit 2; }

rc=0

# data rows: not a comment, not @ZONE, has 7 columns
rows() { awk -F'\t' '!/^#/ && $1!="@ZONE" && NF>=6 {print}' "$MANIFEST"; }

# run ONE kani row: cargo kani --harness <filter> [--features F]; assert verified >= min.
# args: features filter min label
run_kani_row() {
  local features="$1" filter="$2" min="$3" label="$4"
  local featflag=()
  [ "$features" != "-" ] && featflag=(--features "$features")
  echo "  --- [$label] cargo kani --harness '$filter' ${featflag[*]} ---"
  local out verified failures
  out="$(cd "$KERNEL_DIR" && cargo kani "${featflag[@]}" --harness "$filter" ${KANI_EXTRA_ARGS:-} 2>&1)"
  local kani_rc=$?
  # Kani summary: "Complete - N successfully verified harnesses, M failures, T total."
  verified="$(printf '%s\n' "$out" | grep -oiE '[0-9]+ successfully verified harnesses' | grep -oE '^[0-9]+' | tail -1)"
  failures="$(printf '%s\n' "$out" | grep -oiE '[0-9]+ (failures|failed)' | grep -oE '^[0-9]+' | tail -1)"
  verified="${verified:-0}"; failures="${failures:-0}"
  if [ "$min" -eq 0 ]; then
    # Placeholder obligation (e.g. a future item's harness not on this branch yet): visible, no floor.
    echo "::warning::[$label] placeholder kani row (min=0): $verified verified, 0 required (obligation inherited by owning item)"
    return 0
  fi
  if [ "$kani_rc" -ne 0 ] || [ "$failures" -ne 0 ]; then
    echo "::error::[$label] cargo kani '$filter' had $failures failure(s) (exit $kani_rc)"
    printf '%s\n' "$out" | grep -iE 'FAILED|Failed Checks|VERIFICATION:- FAILED|error(\[|:)' | head -8 | sed 's/^/    /'
    return 1
  fi
  if [ "$verified" -lt "$min" ]; then
    echo "::error::[$label] filter '$filter' verified $verified harnesses, need >= $min (zero-match/proof-deleted = RED)"
    return 1
  fi
  echo "  OK  [$label] '$filter' → $verified verified (>= $min)"
  return 0
}

# ── Step D: unconditional floor — run every mode=kani row (a deleted proof goes RED next run) ──
echo "--- kani-gate: floor (all mode=kani rows) ---"
had_row=0
while IFS=$'\t' read -r path features filter min mode checklist gap; do
  [ -z "$path" ] && continue
  [ "$mode" = "kani" ] || continue
  had_row=1
  run_kani_row "$features" "$filter" "$min" "kani:$path" || rc=1
done < <(rows | awk -F'\t' '$5=="kani" && !seen[$3 FS $2]++')  # dedup by (filter,features)
if [ "$had_row" -eq 0 ]; then
  echo "::error::no mode=kani rows in $MANIFEST — the kani-gate has nothing to prove (misconfiguration)"; rc=1
fi

# ── Step F: gap ledger (visible every run, never silently green) ──
echo "--- kani-gate: ledgered gaps (::warning::) ---"
while IFS=$'\t' read -r path features filter min mode checklist gap; do
  [ -z "$path" ] && continue
  [ "$mode" = "kani" ] || continue
  case "$gap" in -|"") ;; *) echo "::warning::[$path] gap: $gap" ;; esac
done < <(rows)

if [ "$rc" -eq 0 ]; then
  echo "=== kani-gate: GREEN ==="
else
  echo "=== kani-gate: RED ==="
fi
exit "$rc"
