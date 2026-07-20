#!/usr/bin/env bash
# red-line-classifier.sh — Item 74, the code-plane twin of scope.rs's runtime red-line set,
# and the concrete mechanism item 75's step-zero calls. Maps a changeset → a deterministic
# `touched-red-line: {class, path, source}` verdict per path-prefix zone, exactly like
# scripts/hardening-gate.sh does for HOT-PATHS.tsv.
#
# D11 Q4 hard line: if a touched path falls under a row whose removal_authority=out-of-band-only
# (the CORE-authority hard line — breaker/order/money/decide-core), the classifier REFUSES and
# exits NON-ZERO. This is a category error surfaced at classification time (blueprint §3.3(3)),
# before any verification or human is involved.
#
# Usage:
#   red-line-classifier.sh                      # read changed paths from: git diff vs origin/main
#   red-line-classifier.sh --paths <p1> <p2>…   # classify an explicit list of changed paths
#   red-line-classifier.sh --diff <file>        # classify the paths touched by a saved `git diff`
#   cat diff.txt | red-line-classifier.sh --stdin-diff
#   red-line-classifier.sh --self-row           # assert the registry is in the registry (self-row test)
#   red-line-classifier.sh --no-fail            # print verdicts but always exit 0 (report mode)
#
# Exit codes:
#   0  -> no red-line touched (or --no-fail / --self-row passed)
#   1  -> a CORE-authority (out-of-band-only) red-line is touched  [THE HARD LINE — D11 Q4]
#   2  -> usage / registry-not-found
set -uo pipefail

REGISTRY="${RED_LINE_REGISTRY:-docs/audits/governance/RED-LINE-REGISTRY.tsv}"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
NO_FAIL=0
SELF_ROW_ONLY=0
MODE=""
PATHS=()

while [ $# -gt 0 ]; do
  case "$1" in
    --no-fail) NO_FAIL=1; shift ;;
    --self-row) SELF_ROW_ONLY=1; shift ;;
    --paths) MODE="paths"; shift; while [ $# -gt 0 ] && [[ "$1" != --* ]]; do PATHS+=("$1"); shift; done ;;
    --diff) MODE="diff"; DIFF_FILE="$2"; shift 2 ;;
    --stdin-diff) MODE="stdindiff"; shift ;;
    -h|--help) sed -n '1,20p' "$0"; exit 0 ;;
    *) echo "::error::unknown arg: $1" >&2; exit 2 ;;
  esac
done

[ -f "$ROOT/$REGISTRY" ] || { echo "::error::registry not found: $REGISTRY (searched $ROOT/$REGISTRY)" >&2; exit 2; }

# ── self-row test (blueprint §3.3(4) / §3.4-bypass-c): the registry is in the registry ──
REGISTRY_ROW="$(awk -F'\t' '!/^#/ && NF>=2 && $5!="" && $1=="docs/audits/governance/RED-LINE-REGISTRY.tsv"{print $1}' "$ROOT/$REGISTRY")"
if [ -z "$REGISTRY_ROW" ]; then
  echo "::error::SELF-ROW TEST FAILED — RED-LINE-REGISTRY.tsv's own path_prefix is NOT a row."
  echo "::error::item 73's recursion (the registry must enumerate itself) is broken."
  [ "$SELF_ROW_ONLY" -eq 1 ] && exit 1
  # non-fatal for a normal classify run only if --no-fail; otherwise treat as hard failure
  [ "$NO_FAIL" -eq 0 ] && exit 1
else
  echo "self-row test: PASS — $REGISTRY_ROW is registered (class=governance-self)."
fi
[ "$SELF_ROW_ONLY" -eq 1 ] && exit 0

# ── gather changed paths ──
CHANGED=""
case "$MODE" in
  paths) CHANGED="$(printf '%s\n' "${PATHS[@]}")" ;;
  diff)  CHANGED="$(git diff --name-only "$DIFF_FILE" 2>/dev/null || true)" ;;
  stdindiff) CHANGED="$(git diff --name-only --no-index /dev/null /dev/null 2>/dev/null; cat)" ;;
  *) CHANGED="$(git diff --name-only "origin/main...HEAD" 2>/dev/null || git diff --name-only HEAD 2>/dev/null || true)" ;;
esac

# ── helpers (awk -F tab; skip comments) ──
rows() { awk -F'\t' '!/^#/ && NF>=5 && $1!="" {print}' "$ROOT/$REGISTRY"; }
# longest matching prefix wins (more specific zone overrides a broader one)
starts_with() { case "$1" in "$2"*) return 0;; *) return 1;; esac; }

echo "=== red-line-classifier: ${MODE:-git-diff} ==="
if [ -z "${CHANGED//[$'\n\t ']/}" ]; then
  echo "  (no changed files — nothing classified)"
  [ "$NO_FAIL" -eq 1 ] && exit 0
  exit 0
fi

rc=0
while IFS= read -r f; do
  [ -z "$f" ] && continue
  best_prefix=""; best_class=""; best_source=""; best_removal=""; best_len=-1
  while IFS=$'\t' read -r pfx class removal why source status; do
    [ -z "$pfx" ] && continue
    if starts_with "$f" "$pfx"; then
      local_len=${#pfx}
      if [ "$local_len" -gt "$best_len" ]; then
        best_len="$local_len"; best_prefix="$pfx"; best_class="$class"; best_source="$source"; best_removal="$removal"
      fi
    fi
  done < <(rows)
  if [ -n "$best_prefix" ]; then
    echo "touched-red-line: {class=$best_class, path=$f, source=$best_source, removal_authority=$best_removal}"
    if [ "$best_removal" = "out-of-band-only" ]; then
      echo "::error::CORE-AUTHORITY RED LINE TOUCHED — '$f' is under removal_authority=out-of-band-only ($best_prefix)."
      echo "::error::D11 Q4 hard line: this path is NEVER editable by AI. Changeset REFUSED at classification time."
      rc=1
    fi
  else
    echo "  (no red-line) $f"
  fi
done < <(printf '%s\n' "$CHANGED")

if [ "$rc" -eq 0 ]; then
  echo "=== red-line-classifier: GREEN (no core-authority red line touched) ==="
else
  echo "=== red-line-classifier: RED (core-authority red line touched — D11 Q4 hard line) ==="
fi
[ "$NO_FAIL" -eq 1 ] && exit 0
exit "$rc"
