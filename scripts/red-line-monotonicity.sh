#!/usr/bin/env bash
# red-line-monotonicity.sh — Item 74 monotonicity guard (blueprint §3.3(5), §3.4-bypass-b).
#
# Enforces D11 Q5 (operator ruling, 2026-07-20): removing a row from RED-LINE-REGISTRY.tsv
# is the DANGEROUS direction (the corrigibility-adjacent attack: quietly drop a protected
# path so a later proposal can touch it). Adding a row is always safe (widens protection).
#
# A removed row is permitted ONLY IF the diff/deletion carries a DECISIONS.md D-entry marker
# (e.g. a "D12:" reference) — the operator's recorded ruling. Without it → RED.
#
# Uses the reverse-of-zero-dep-gate `comm` approach: removed = base prefixes NOT in head
# (comm -23). Every removed row must be justified by a D-entry token in the diff text.
#
# Usage:
#   red-line-monotonicity.sh                 # CI mode: diff origin/main...HEAD vs working registry
#   red-line-monotonicity.sh --selftest      # run the 3 planted scenarios (GREEN/RED/GREEN)
#   red-line-monotonicity.sh --base F --head F --diff F   # check an explicit base/head/diff
#
# Exit: 0 = GREEN (no removal, or removal WITH operator D-entry); 1 = RED (removal w/o D-entry);
#       2 = usage / registry-not-found.
set -uo pipefail

REGISTRY="${RED_LINE_REGISTRY:-docs/audits/governance/RED-LINE-REGISTRY.tsv}"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
HAVE_BASE=0; BASE_F=""; HEAD_F=""; DIFF_F=""; SELFTEST=0

while [ $# -gt 0 ]; do
  case "$1" in
    --selftest) SELFTEST=1; shift ;;
    --base) HAVE_BASE=1; BASE_F="$2"; shift 2 ;;
    --head) HEAD_F="$2"; shift 2 ;;
    --diff) DIFF_F="$2"; shift 2 ;;
    *) echo "::error::unknown arg: $1" >&2; exit 2 ;;
  esac
done

# live data-row path_prefixes (skip comments; NF>=5; col1 non-empty)
prefixes() { awk -F'\t' '!/^#/ && NF>=5 && $1!="" {print $1}' "$1" | sort -u; }

echo "=== red-line-monotonicity (D11 Q5) ==="

if [ "$SELFTEST" -eq 1 ]; then
  TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
  BASE="$TMP/base.tsv"; HEAD="$TMP/head.tsv"; D="$TMP/diff.txt"
  cp "$ROOT/$REGISTRY" "$BASE"
  # one concrete live row to remove in the scenarios:
  VICTIM="kernel/src/fdr/json.rs"
  run_scenario() {
    local name="$1" marked="$2"
    cp "$BASE" "$HEAD"
    # remove the victim live row
    grep -vF "$VICTIM"'$' "$BASE" > "$HEAD" 2>/dev/null || cp "$BASE" "$HEAD"
    # the awk-free way: drop the exact live-data line for $VICTIM
    awk -F'\t' -v v="$VICTIM" '!(($1==v) && (!/^#/))' "$BASE" > "$HEAD"
    if [ "$marked" = "marked" ]; then
      # replacement comment carrying the D-entry marker (operator ruling recorded in-file)
      printf '# REMOVED per D12: fdr/json.rs no longer a forensic-truth surface (operator ruling 2026-07-20)\n' >> "$HEAD"
    fi
    diff -u "$BASE" "$HEAD" > "$D" 2>/dev/null || true
    removed="$(comm -23 <(prefixes "$BASE") <(prefixes "$HEAD"))"
    if [ -z "$removed" ]; then
      echo "[$name] GREEN — no row removed."
      return 0
    fi
    if grep -qE 'D[0-9]+:' "$D"; then
      echo "[$name] GREEN — removed row(s): $(echo "$removed" | tr '\n' ' ') carry D-entry marker."
      return 0
    else
      echo "[$name] RED — removed row(s) WITHOUT operator D-entry marker: $(echo "$removed" | tr '\n' ' ')"
      return 1
    fi
  }
  rc=0
  # (a) no change -> GREEN (expected)
  cp "$BASE" "$HEAD"; diff -u "$BASE" "$HEAD" > "$D" 2>/dev/null || true
  if [ -z "$(comm -23 <(prefixes "$BASE") <(prefixes "$HEAD"))" ]; then
    echo "[no-change] GREEN — registry unchanged."; a_ok=1
  else echo "[no-change] RED (UNEXPECTED)"; a_ok=0; rc=1; fi
  # (b) planted removal WITHOUT marker -> RED (expected)
  run_scenario "removal-without-marker" "unmarked"; b_rc=$?
  if [ "$b_rc" -eq 1 ]; then echo "[removal-without-marker] RED (expected)"; b_ok=1
  else echo "[removal-without-marker] GREEN (UNEXPECTED — guard failed)"; b_ok=0; rc=1; fi
  # (c) planted removal WITH D-entry -> GREEN (expected)
  run_scenario "removal-with-D-entry" "marked"; c_rc=$?
  if [ "$c_rc" -eq 0 ]; then echo "[removal-with-D-entry] GREEN (expected)"; c_ok=1
  else echo "[removal-with-D-entry] RED (UNEXPECTED)"; c_ok=0; rc=1; fi
  if [ "$a_ok" -eq 1 ] && [ "$b_ok" -eq 1 ] && [ "$c_ok" -eq 1 ]; then
    echo "=== red-line-monotonicity: SELFTEST GREEN (all 3 scenarios matched expected verdicts) ==="
  else
    echo "=== red-line-monotonicity: SELFTEST RED (a=$a_ok b=$b_ok c=$c_ok) ==="
  fi
  exit "$rc"
fi

# ── CI mode ──
[ -f "$ROOT/$REGISTRY" ] || { echo "::error::registry not found: $REGISTRY" >&2; exit 2; }

if [ "$HAVE_BASE" -eq 1 ]; then
  DIFF_SRC="$DIFF_F"
  removed="$(comm -23 <(prefixes "$BASE_F") <(prefixes "$HEAD_F"))"
else
  # diff vs origin/main (the zero-dep-gate poison-avoidance pattern: never a failing
  # treeish on a missing path)
  if git ls-tree -r --name-only origin/main 2>/dev/null | grep -qxF "$REGISTRY"; then
    BASE_F="$(mktemp)"; HEAD_F="$ROOT/$REGISTRY"; DIFF_F="$(mktemp)"
    git show "origin/main:$REGISTRY" > "$BASE_F"
    git diff "origin/main...HEAD" -- "$REGISTRY" > "$DIFF_F" 2>/dev/null || true
    removed="$(comm -23 <(prefixes "$BASE_F") <(prefixes "$HEAD_F"))"
  else
    echo "GREEN — registry is new on this branch (no baseline to shrink from)."
    exit 0
  fi
fi

if [ -z "$removed" ]; then
  echo "GREEN — no registry row removed in this changeset."
  [ -n "${BASE_F:-}" ] && [[ "$BASE_F" == /tmp/* ]] && rm -f "$BASE_F" "$DIFF_F" 2>/dev/null
  exit 0
fi
if grep -qE 'D[0-9]+:' "$DIFF_F"; then
  echo "GREEN — removed row(s): $(echo "$removed" | tr '\n' ' ') carry DECISIONS.md D-entry marker (D11 Q5 satisfied)."
  [ -n "${BASE_F:-}" ] && [[ "$BASE_F" == /tmp/* ]] && rm -f "$BASE_F" "$DIFF_F" 2>/dev/null
  exit 0
else
  echo "RED — registry row(s) REMOVED without a DECISIONS.md D-entry marker (D11 Q5 violation):"
  echo "$removed" | sed 's/^/    /'
  echo "Fix: record an operator ruling (e.g. 'D12:') and reference it in the removal commit/diff."
  [ -n "${BASE_F:-}" ] && [[ "$BASE_F" == /tmp/* ]] && rm -f "$BASE_F" "$DIFF_F" 2>/dev/null
  exit 1
fi
