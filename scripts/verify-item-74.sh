#!/usr/bin/env bash
# verify-item-74.sh — Item 74 acceptance harness. Re-executes (P7, never presence-check)
# every required verdict and prints the EXACT output for the commit report.
#
# Requires: rustc (for the standalone registry test; zero cargo deps), comm, bash.
set -uo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT" || exit 2
REG="docs/audits/governance/RED-LINE-REGISTRY.tsv"
T="tests/red-line-registry/test_registry.rs"
BIN="$(mktemp -t rlr-XXXXXX)"
PASS=0; FAIL=0
check() { local name="$1" rc="$2"; if [ "$rc" -eq 0 ]; then echo "  [OK] $name"; PASS=$((PASS+1)); else echo "  [FAIL] $name (rc=$rc)"; FAIL=$((FAIL+1)); fi; }

echo "================ ITEM-74 VERIFY ================"
echo "--- A. classifier: changeset touching money.rs (product-red-line / out-of-band-only) ---"
bash scripts/red-line-classifier.sh --paths kernel/src/money.rs; rc_a=$?
check "classifier money.rs exits non-zero (core hard line)" "$(( rc_a != 0 ? 0 : 1 ))"

echo
echo "--- B. classifier: changeset touching ONLY docs/ (no red line) ---"
bash scripts/red-line-classifier.sh --paths docs/design/foo.md; rc_b=$?
check "classifier docs-only exits zero" "$rc_b"

echo
echo "--- C. monotonicity: no-change (baseline GREEN) ---"
bash scripts/red-line-monotonicity.sh --selftest; rc_c=$?
check "monotonicity selftest (no-change GREEN / removal-no-marker RED / removal-D GREEN)" "$rc_c"

echo
echo "--- D. self-row assertion (registry is in the registry) ---"
bash scripts/red-line-classifier.sh --self-row; rc_d=$?
check "self-row test passes" "$rc_d"

echo
echo "--- E. standalone Rust registry integrity test (self-row + D11 Q4 + no dup + no un-cited) ---"
rustc "$T" -O -o "$BIN" 2>&1 || { echo "rustc compile FAILED"; FAIL=$((FAIL+1)); }
"$BIN" "$REG"; rc_e=$?
check "rustc registry test GREEN" "$rc_e"
rm -f "$BIN"

echo
echo "--- F. grep proof: zero runtime mutation surface in registry-adjacent CODE ---"
mut="$(grep -rEn 'pub fn[^=]*&mut|pub static mut' scripts/red-line-classifier.sh scripts/red-line-monotonicity.sh 2>/dev/null || true)"
if [ -z "$mut" ]; then
  echo "  OK  zero 'pub fn.*&mut' / 'pub static mut' in registry-adjacent code (the two classifier/"
  echo "      monotonicity scripts). The registry is a .tsv + pure bash — there is no runtime mutation API."
  check "grep proof (no mutation surface)" 0
else
  echo "  MUTATION SURFACE FOUND:"; echo "$mut" | sed 's/^/    /'
  check "grep proof (no mutation surface)" 1
fi

echo
echo "================ RESULT: $PASS passed, $FAIL failed ================"
[ "$FAIL" -eq 0 ]
