#!/usr/bin/env bash
# hardening-gate.sh — CI enforcement for the kernel §4 hardening checklist (roadmap item 6).
#
# Implements the SYNTHESIS §10/P7 correction ("re-execute, never presence-check"): no file is
# evidence. Every verdict comes from a live `cargo test` exit code plus the PARSED live "N passed"
# count, asserted >= the manifest's min_tests. A filter matching zero tests is RED — the anti-forgery
# core (deleting/renaming an oracle test can no longer keep the gate green).
#
# Steps (blueprint §4.2):
#   B. Row check    — a diff touching a hot ZONE with no manifest row = RED
#   C. Re-execute   — for each touched row: cargo test <filter>, assert passed >= min_tests
#   D. Oracle floor — UNCONDITIONALLY re-execute every lib-mode row (a deleted oracle goes RED on
#                     the next run, whatever the diff)
#   E. dudect       — run every dudect-mode row in RELEASE incl. its planted-leak self-test
#   F. Gap ledger   — print each row's ledgered MISSING/KNOWN-RED gap as a ::warning::
#
# Determinism (P6): every cargo call is --locked --offline; Cargo.lock hash asserted unchanged.
#
# Usage: hardening-gate.sh [BASE] [HEAD]   (defaults: origin/main HEAD; three-dot merge-base diff)
#   HARDENING_MANIFEST=<path>   override the manifest (default docs/audits/hardening/HOT-PATHS.tsv)
set -uo pipefail

BASE="${1:-origin/main}"
HEAD="${2:-HEAD}"
MANIFEST="${HARDENING_MANIFEST:-docs/audits/hardening/HOT-PATHS.tsv}"
KERNEL_DIR="kernel"
LOCK="$KERNEL_DIR/Cargo.lock"

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT" || exit 2
[ -f "$MANIFEST" ] || { echo "::error::manifest not found: $MANIFEST"; exit 2; }

rc=0
lock_before="$(sha256sum "$LOCK" | cut -d' ' -f1)"

# ── changed files in the diff range (three-dot = vs merge-base, matches toolchain-bump-gate) ──
CHANGED="$(git diff --name-only "$BASE...$HEAD" 2>/dev/null || git diff --name-only "$BASE" "$HEAD")"
echo "=== hardening-gate: diff $BASE...$HEAD ==="
echo "changed files:"; echo "$CHANGED" | sed 's/^/  /'

# ── manifest accessors (awk -F tab; skip comments/blanks) ──
zones()  { awk -F'\t' '$1=="@ZONE"{print $2}' "$MANIFEST"; }
# data rows: not a comment, not @ZONE, has a path in col1
rows()   { awk -F'\t' '!/^#/ && $1!="@ZONE" && NF>=6 {print}' "$MANIFEST"; }

# starts_with HAYSTACK NEEDLE
starts_with() { case "$1" in "$2"*) return 0;; *) return 1;; esac; }

# run ONE row's cargo filter and assert passed >= min_tests. args: features filter min mode label
run_row() {
  local features="$1" filter="$2" min="$3" mode="$4" label="$5"
  local featflag=() extra=() profile=()
  [ "$features" != "-" ] && featflag=(--features "$features")
  if [ "$mode" = "dudect" ]; then
    profile=(--release); extra=(-- --ignored)
  fi
  local out passed
  out="$(cd "$KERNEL_DIR" && cargo test --locked --offline "${profile[@]}" --lib "${featflag[@]}" "$filter" "${extra[@]}" 2>&1)"
  local cargo_rc=$?
  passed="$(printf '%s\n' "$out" | grep -oE '[0-9]+ passed' | awk '{s+=$1} END{print s+0}')"
  if [ "$cargo_rc" -ne 0 ]; then
    echo "::error::[$label] cargo test '$filter' FAILED (exit $cargo_rc)"
    printf '%s\n' "$out" | grep -E 'error|FAILED|panicked' | head -5 | sed 's/^/    /'
    return 1
  fi
  if [ "$passed" -lt "$min" ]; then
    echo "::error::[$label] filter '$filter' matched $passed passing tests, need >= $min (zero-match/oracle-deleted = RED)"
    return 1
  fi
  echo "  OK  [$label] '$filter' → $passed passed (>= $min)"
  return 0
}

# ── Step B + C: diff-triggered row obligation + re-execution ──
echo "--- step B/C: diff-triggered hot-path obligation ---"
if [ -z "${CHANGED//[$'\n\t ']/}" ]; then
  echo "  (no changed files in range — nothing diff-triggered)"
else
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    in_zone=0
    while IFS= read -r z; do [ -n "$z" ] && starts_with "$f" "$z" && in_zone=1 && break; done < <(zones)
    matched=0
    while IFS=$'\t' read -r path features filter min mode checklist gap; do
      [ -z "$path" ] && continue
      if starts_with "$f" "$path"; then
        matched=1
        run_row "$features" "$filter" "$min" "$mode" "row:$path" || rc=1
      fi
    done < <(rows)
    if [ "$in_zone" -eq 1 ] && [ "$matched" -eq 0 ]; then
      echo "::error::hot-path '$f' is under a designated @ZONE but has NO manifest row — register it in $MANIFEST"
      rc=1
    fi
  done < <(printf '%s\n' "$CHANGED")
fi

# ── Step D: unconditional oracle floor (re-execute every lib-mode row, every run) ──
echo "--- step D: unconditional oracle floor (all lib rows) ---"
while IFS=$'\t' read -r path features filter min mode checklist gap; do
  [ -z "$path" ] && continue
  [ "$mode" = "lib" ] || continue
  run_row "$features" "$filter" "$min" "$mode" "floor:$path" || rc=1
done < <(rows | awk -F'\t' '!seen[$3 FS $2]++')   # dedup by (filter,features): pq::dsa:: appears twice

# ── Step E: dudect gate incl. planted-leak self-test (release) ──
echo "--- step E: dudect gate (release, planted-leak self-test) ---"
while IFS=$'\t' read -r path features filter min mode checklist gap; do
  [ -z "$path" ] && continue
  [ "$mode" = "dudect" ] || continue
  run_row "$features" "$filter" "$min" "$mode" "dudect:$path" || rc=1
done < <(rows)

# ── Step F: gap ledger (visible every run, never silently green) ──
echo "--- step F: ledgered gaps (::warning::) ---"
while IFS=$'\t' read -r path features filter min mode checklist gap; do
  [ -z "$path" ] && continue
  case "$gap" in
    -|"") ;;
    *) echo "::warning::[$path] gap: $gap" ;;
  esac
done < <(rows)

# ── P6 determinism: Cargo.lock must be byte-identical after the run ──
lock_after="$(sha256sum "$LOCK" | cut -d' ' -f1)"
if [ "$lock_before" != "$lock_after" ]; then
  echo "::error::Cargo.lock changed during the gate run (nondeterminism / network) — P6 violation"
  rc=1
fi

if [ "$rc" -eq 0 ]; then
  echo "=== hardening-gate: GREEN ==="
else
  echo "=== hardening-gate: RED ==="
fi
exit "$rc"
