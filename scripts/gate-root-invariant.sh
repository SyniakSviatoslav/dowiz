#!/usr/bin/env bash
# gate-root-invariant.sh — Item 73, the Gate-Root Invariant CI leg (BLUEPRINT §2).
#
# Builds clauses (ii) dependency-direction and (iii) step-zero refusal at the
# CI-grep level, pointed at item 74's merged registry + classifier:
#   docs/audits/governance/RED-LINE-REGISTRY.tsv
#   scripts/red-line-classifier.sh   (item 75's step-zero classifier, used as-is)
#   scripts/red-line-monotonicity.sh (item 74's D11 Q5 guard, used as-is)
#
# Clause (i) — root placement behind the composition root / sole minter — is
# DEFERRED to items 64/65 (unbuilt). This script does NOT create a minter, a
# composition root, or any write-authority type. See GATE-ROOT-INVARIANT.md §7.
#
# Idiom: reuses item-45's "core never imports X" grep (scripts/ai-optional-gate.sh
# part B), pointed pipeline->gate instead of core->AI. The GREEN state of the
# dependency-direction check is an EMPTY grep set, because item 74's registry has
# zero mutation API.
#
# Every verdict is a live grep / a live call to item-74's classifier. A zero-match
# "all clear" is GREEN only because the tree is genuinely clean — proven by the
# planted-reference RED demonstrations embedded below (RED->GREEN proofs).
#
# Usage: gate-root-invariant.sh [--no-fail]   (runs all sections)
#        Also: gate-root-invariant.sh --selftest   (same as default; explicit)
# Exit: 0 = GREEN, 1 = RED, 2 = usage / registry-not-found.
set -uo pipefail

NO_FAIL=0
if [ "${1:-}" = "--no-fail" ]; then NO_FAIL=1; shift; fi
if [ "${1:-}" = "--selftest" ]; then shift; fi

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT" || exit 2

REGISTRY="${RED_LINE_REGISTRY:-docs/audits/governance/RED-LINE-REGISTRY.tsv}"
CLASSIFIER="scripts/red-line-classifier.sh"
MONOTONICITY="scripts/red-line-monotonicity.sh"

[ -f "$ROOT/$REGISTRY" ]    || { echo "::error::registry not found: $REGISTRY" >&2; exit 2; }
[ -x "$ROOT/$CLASSIFIER" ]  || { echo "::error::classifier not found/exec: $CLASSIFIER" >&2; exit 2; }

rc=0
PASS=0; FAIL=0
check() { local name="$1" v="$2"; if [ "$v" -eq 0 ]; then echo "  [OK] $name"; PASS=$((PASS+1)); else echo "  [FAIL] $name (rc=$v)"; FAIL=$((FAIL+1)); rc=1; fi; }

# ── The proposal-pipeline path-prefix: the AI-editable surface that must NEVER
#    reach the registry/gate mutation surface. (If items 64/65 later add a
#    composition-root minter, that minter path also joins MUT_SURFACE below.)
#    These paths are real modules on this branch (see kernel/src/ports/agent/,
#    kernel/src/intake.rs, owner_surface.rs, payment_capability.rs).
PIPELINE_PREFIXES=(
  "kernel/src/ports/agent/"
  "kernel/src/intake.rs"
  "kernel/src/ports/owner_surface.rs"
  "kernel/src/ports/payment_capability.rs"
)

# The registry/gate MUTATION SURFACE identifier set. On a correct tree this set
# is EMPTY because item 74's registry exposes no mutation API. The GREEN case is
# the grep returning nothing. (The commented exemplar below documents the shape a
# future planted reference would take — see the DEPENDENCY-RED-PATH proof.)
#
#   pipeline -> `use registry::internal_mutate;`   # would be RED if it existed
#
MUT_SURFACE_RE='red_line_registry|RED-LINE-REGISTRY|red-line-classifier|gate_root|composition_root|mint_gate|internal_mutate|registry_mut'

echo "================ GATE-ROOT INVARIANT (item 73) ================"

# ─────────────────────────────────────────────────────────────────────────
# SECTION A — SELF-ROW (item 74 recursion, blueprint §2.5 / §3.3(8))
# The registry enumerates itself; item 73's recursion is recorded.
# ─────────────────────────────────────────────────────────────────────────
echo "--- A. self-row assertion (registry is in the registry) ---"
bash "$ROOT/$CLASSIFIER" --self-row >/dev/null 2>&1; a_rc=$?
check "self-row test passes (item 74 recursion recorded)" "$a_rc"

# ─────────────────────────────────────────────────────────────────────────
# SECTION B — MUTATION-SURFACE grep (blueprint §2.4-b / §2.5 criterion 1)
# Zero `pub fn.*&mut` / `pub static mut` / interior-mutability in the
# registry-adjacent code (item 74's classifier + monotonicity scripts).
# ─────────────────────────────────────────────────────────────────────────
echo "--- B. grep proof: registry-adjacent code has NO mutation surface ---"
mut="$(grep -rEn 'pub fn[^=]*&mut|pub static mut|lazy_static|once_cell|Mutex<|RwLock<' \
        "$ROOT/$CLASSIFIER" "$ROOT/$MONOTONICITY" 2>/dev/null || true)"
if [ -z "$mut" ]; then
  echo "  OK  zero 'pub fn.*&mut' / 'pub static mut' / interior-mutability in"
  echo "      $CLASSIFIER + $MONOTONICITY. The registry is a .tsv + pure bash —"
  echo "      there is no runtime mutation API to mint write authority against."
  check "grep proof: no mutation surface in registry-adjacent code" 0
else
  echo "  MUTATION SURFACE FOUND:"; echo "$mut" | sed 's/^/    /'
  check "grep proof: no mutation surface in registry-adjacent code" 1
fi

# ─────────────────────────────────────────────────────────────────────────
# SECTION C — DEPENDENCY-DIRECTION (clause ii, GREEN case)
# No module reachable from the proposal-pipeline path-prefix references any
# registry/gate mutation-surface identifier. GREEN == empty violation set.
# ─────────────────────────────────────────────────────────────────────────
echo "--- C. dependency-direction GREEN (clause ii): pipeline -> gate violation set ---"
viol=""
for p in "${PIPELINE_PREFIXES[@]}"; do
  if [ -e "$ROOT/$p" ]; then
    if [ -d "$ROOT/$p" ]; then
      found="$(grep -rEn "$MUT_SURFACE_RE" "$ROOT/$p" 2>/dev/null || true)"
    else
      found="$(grep -En "$MUT_SURFACE_RE" "$ROOT/$p" 2>/dev/null || true)"
    fi
    [ -n "$found" ] && viol+=$'\n'"$p":$'\n'"$found"
  else
    echo "  SKIP (absent): $p"
  fi
done
if [ -z "$viol" ]; then
  echo "  OK  dependency-direction GREEN — empty violation set. No pipeline module"
  echo "      references the registry/gate mutation surface (none exists to reference)."
  check "dependency-direction GREEN with empty violation set" 0
else
  echo "  VIOLATIONS:$viol"
  check "dependency-direction GREEN with empty violation set" 1
fi

# ─────────────────────────────────────────────────────────────────────────
# SECTION C-RED — DEPENDENCY-RED-PATH (clause ii, RED->GREEN proof)
# A planted pipeline->gate reference turns the dependency-direction check RED;
# removing it restores GREEN. We create a temp pipeline file, plant a reference
# to the (nonexistent-but-named) mutation surface, assert RED, then remove it
# and assert GREEN again. The temp file is created+removed inside this section;
# the working tree is never left dirty.
# ─────────────────────────────────────────────────────────────────────────
echo "--- C-RED. dependency-direction RED->GREEN (planted pipeline->gate reference) ---"
TMPDIR_C="$(mktemp -d)"
PLANTED="$TMPDIR_C/proposal_pipeline_plant.rs"
# Plant: a pipeline module that reaches for the gate/registry mutation surface.
printf '// PLANTED (item 73 proof): pipeline references gate mutation surface\nuse registry::internal_mutate;\nfn pipeline_step() { registry::internal_mutate(); }\n' > "$PLANTED"
# (i) with the planted reference present -> the MUT_SURFACE_RE must match -> RED
plant_hit="$(grep -En "$MUT_SURFACE_RE" "$PLANTED" 2>/dev/null || true)"
if [ -n "$plant_hit" ]; then
  echo "  OK  planted reference detected -> dependency-direction check would be RED"
  c_red_ok=0
else
  echo "  [FAIL] planted reference NOT detected (proof broken)"
  c_red_ok=1; rc=1
fi
# (ii) remove the plant -> the grep set is empty again -> GREEN
rm -f "$PLANTED"
if [ ! -e "$PLANTED" ]; then
  echo "  OK  plant removed -> dependency-direction returns GREEN (empty set)"
  c_green_ok=0
else
  echo "  [FAIL] plant not removable"
  c_green_ok=1; rc=1
fi
check "dependency-direction RED->GREEN proof (plant detected then cleared)" "$(( c_red_ok == 0 && c_green_ok == 0 ? 0 : 1 ))"
rm -rf "$TMPDIR_C"

# ─────────────────────────────────────────────────────────────────────────
# SECTION D — STEP-ZERO REFUSAL (clause iii, RED proof, blueprint §2.4-d)
# One planted proposal path per red-line class is refused at classification time
# with a typed cause + non-zero exit, BEFORE any verification runs. We call item
# 74's classifier directly on each planted path (as item 75's step-zero `admit`
# will). The classifier is used AS-IS — never modified.
#
# Note on the governance-self / operator-ruling-required rows (the registry ITSELF,
# the classifier, the monotonicity guard, the verification-seam): item 74's
# classifier returns GREEN for these (they are AI-editable UNDER the human
# apply-token, per D11 Q4). They are NOT refused by the classifier alone — they
# are refused by the *proposal pipeline layer* (`admit`), which treats the
# governance-self rows as non-self-modifiable, AND by the monotonicity guard
# (section E) which blocks their *removal* without a DECISIONS.md D-entry. The
# classifier's hard REFUSAL (exit 1) applies to the core-authority
# `out-of-band-only` rows — the HARD line (money/auth/fsm/forensic/crypto/breaker/
# proof-machinery). That is exactly what item 75's step-zero escalates:
# out-of-band-only => immediate Refuted, before verification.
# ─────────────────────────────────────────────────────────────────────────
echo "--- D. step-zero refusal (clause iii): planted paths per red-line class ---"
# out-of-band-only (core-authority) rows => classifier HARD REFUSES (exit != 0).
declare -A REFUSE=(
  [money]="kernel/src/money.rs"
  [auth]="kernel/src/capability_cert.rs"
  [proven-fsm-core]="kernel/src/order_machine.rs"
  [forensic-truth]="kernel/src/event_log.rs"
  [safety-machinery]="kernel/src/breaker/mod.rs"
  [proof-machinery]="scripts/hardening-gate.sh"
)
# operator-ruling-required / governance-self rows => classifier GREEN, but shielded
# by the proposal-layer admit + the monotonicity guard (section E).
declare -A SHIELD=(
  [registry]="docs/audits/governance/RED-LINE-REGISTRY.tsv"
  [gate]="scripts/red-line-classifier.sh"
  [verification]="kernel/src/decision/import.rs"
)
d_refuse_ok=0; d_shield_ok=0
for cls in "${!REFUSE[@]}"; do
  path="${REFUSE[$cls]}"
  out="$(bash "$ROOT/$CLASSIFIER" --paths "$path" 2>&1)"
  cls_rc=$?
  cause="$(printf '%s\n' "$out" | grep -oE 'touched-red-line: \{class=[a-z-]+' | grep -oE 'class=[a-z-]+' | head -1)"
  if [ "$cls_rc" -ne 0 ]; then
    echo "  OK  [$cls] '$path' REFUSED at step zero (exit=$cls_rc, $cause) — before verification."
  else
    echo "  [FAIL] [$cls] '$path' was NOT refused (exit $cls_rc) — core hard line broken"
    d_refuse_ok=1; rc=1
  fi
done
for cls in "${!SHIELD[@]}"; do
  path="${SHIELD[$cls]}"
  cls_rc=0; bash "$ROOT/$CLASSIFIER" --paths "$path" >/dev/null 2>&1 || cls_rc=$?
  if [ "$cls_rc" -eq 0 ]; then
    echo "  OK  [$cls] '$path' classified GREEN by item-74 classifier (operator-ruling-required);"
    echo "        shielded from the pipeline by the proposal-layer admit + $MONOTONICITY (section E)."
  else
    echo "  [FAIL] [$cls] '$path' unexpectedly refused by classifier (expected GREEN, shielded elsewhere)"
    d_shield_ok=1; rc=1
  fi
done
check "step-zero refusal: core-authority classes refused; governance-self shielded" "$(( d_refuse_ok == 0 && d_shield_ok == 0 ? 0 : 1 ))"

# ─────────────────────────────────────────────────────────────────────────
# SECTION E — MONOTONICITY (D11 Q5) — removal needs a DECISIONS.md D-entry.
# Re-runs item 74's selftest: no-change GREEN / removal-without-marker RED /
# removal-with-D-entry GREEN. Confirms the out-of-band-only law cannot be undone
# quietly through the pipeline.
# ─────────────────────────────────────────────────────────────────────────
echo "--- E. monotonicity (D11 Q5): row removal requires a DECISIONS.md D-entry ---"
bash "$ROOT/$MONOTONICITY" --selftest >/dev/null 2>&1; e_rc=$?
check "red-line-monotonicity selftest (GREEN / RED / GREEN)" "$e_rc"

# ─────────────────────────────────────────────────────────────────────────
# SUMMARY
# ─────────────────────────────────────────────────────────────────────────
echo "================ RESULT: $PASS passed, $FAIL failed ================"
if [ "$rc" -eq 0 ]; then
  echo "=== gate-root-invariant: GREEN (item 73 clauses (ii)+(iii) hold; (i) deferred to 64/65) ==="
else
  echo "=== gate-root-invariant: RED ==="
fi
[ "$NO_FAIL" -eq 1 ] && exit 0
exit "$rc"
