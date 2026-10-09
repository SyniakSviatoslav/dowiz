#!/bin/sh
# LEAN GATE -- bebop-lang/formal is checked, not just kept.
#
# WHY THIS EXISTS. Until 2026-10-09 nothing in tools/, bench/ or .github ran Lean: `lake build`
# passed on a clean export (R-LANG, 641 s cold) and `lake exe parityrun` scored 122-126/129, but no
# gate saw either, so formal/ was documentation and formal/results.txt (10-01, 126/129) went stale
# under source changes nobody re-measured. This gate is the `lean` job in .github/workflows/ci.yml.
#
#   sh tools/gates/lean.sh                # build + axiom census + conformance; rc 0/1
#   LEAN_WRITE=1 sh tools/gates/lean.sh   # also refresh bebop-lang/formal/results.txt and raise the floor
#   sh tools/gates/lean.sh --census       # print the axiom census only (how lean.axioms is derived)
#   On the dev box, through the slot:
#     PATH=/root/.elan/toolchains/leanprover--lean4---v4.33.1/bin:$PATH \
#       bash bebop-lang/tools/slot.sh lean sh tools/gates/lean.sh
#
# THREE CHECKS, every red names what it is about:
#   1. `lake build` exits 0, and no declaration "uses 'sorry'".
#   2. AXIOM CENSUS (tools/gates/lean-axioms.lean): for EVERY constant in a Bebop.* module, the
#      axioms `#print axioms` would list. Each must be in tools/gates/lean.axioms, and each line of
#      lean.axioms must still be used. `sorryAx` can never be allowlisted.
#   3. CONFORMANCE: parityrun over bench/parity_constructs (run against a scratch root, so the
#      committed results.txt is never overwritten by a check). Every FAIL must be a named
#      `expect-fail` in tools/gates/lean.baseline, every `expect-fail` must still fail, the pass
#      count must reach `floor`, the emitted lean_sources_sha256 must equal the standard-tools
#      recipe, and the committed results.txt must equal the fresh one (toolchain line aside: it
#      names the host architecture).
#
# EXIT: 0 green; 1 red (each finding on its own `lean: RED` line); 2 a file this gate needs is
# missing; 3 no `lake`/`lean` on PATH; 4 a step measured nothing (empty census, no constructs,
# parityrun refused) -- a gate that cannot measure never passes.
# ENV (the proof uses these; CI sets none): LEAN_FORMAL (default bebop-lang/formal), LEAN_ALLOW,
# LEAN_BASE (default tools/gates/lean.{axioms,baseline}), LEAN_ONLY=census (skip conformance;
# the verdict line then says so).
set -u
cd "$(dirname "$0")/../.." || exit 2
ROOT=$(pwd)
FORMAL=${LEAN_FORMAL:-$ROOT/bebop-lang/formal}
ALLOW=${LEAN_ALLOW:-$ROOT/tools/gates/lean.axioms}
BASE=${LEAN_BASE:-$ROOT/tools/gates/lean.baseline}
CENSUS_SRC=$ROOT/tools/gates/lean-axioms.lean
BL=$(dirname "$FORMAL")
export LC_ALL=C.UTF-8
T=$(mktemp -d "${TMPDIR:-/tmp}/lean-gate.XXXXXX")
trap 'rm -rf "$T"' EXIT
reds=0
red() { # one finding; on GitHub also an annotation (job logs need a token, annotations do not)
  reds=$((reds + 1)); echo "lean: RED $*"
  [ -n "${GITHUB_ACTIONS:-}" ] && printf '::error title=lean gate::%s\n' "$*"
  return 0
}
step() { echo "lean: [$(date +%T)] $*"; }

for f in "$FORMAL/lakefile.lean" "$ALLOW" "$BASE" "$CENSUS_SRC"; do
  [ -r "$f" ] || { echo "lean: REFUSED -- $f missing"; exit 2; }
done
for t in lake lean; do
  command -v $t >/dev/null 2>&1 || { echo "lean: REFUSED -- '$t' not on PATH (elan, pinned by $FORMAL/lean-toolchain)"; exit 3; }
done
step "toolchain $(lean --version)"

# ---- 1. build --------------------------------------------------------------------------------
step "lake build"
(cd "$FORMAL" && lake build) > "$T/build.log" 2>&1; rc=$?
if [ $rc -ne 0 ]; then
  grep -E 'error' "$T/build.log" | head -20 | while IFS= read -r l; do red "lake build: $l"; done
  red "lake build exited $rc"; echo "lean: RED ($reds finding(s))"; exit 1
fi
step "$(grep -v '^[[:space:]]*$' "$T/build.log" | tail -1)"
grep "declaration uses 'sorry'" "$T/build.log" > "$T/sorry"
while IFS= read -r l; do red "sorry: $l"; done < "$T/sorry"
(cd "$FORMAL" && lake build parityrun) > "$T/build-exe.log" 2>&1 || { sed 's/^/  | /' "$T/build-exe.log" | tail -15; red "lake build parityrun failed"; }

# ---- 2. axiom census -------------------------------------------------------------------------
step "axiom census"
(cd "$FORMAL" && lake env lean "$CENSUS_SRC") > "$T/census.log" 2>&1; rc=$?
summary=$(grep '^lean-axioms: ' "$T/census.log" | tail -1)
if [ $rc -ne 0 ] || [ -z "$summary" ]; then
  sed 's/^/  | /' "$T/census.log" | tail -15
  echo "lean: REFUSED -- the axiom census did not run (rc=$rc)"; exit 4
fi
case "$summary" in *" in 0 module(s)"|*" over 0 constant(s)"*) echo "lean: REFUSED -- $summary (scanned nothing)"; exit 4 ;; esac
if [ "${1:-}" = --census ]; then grep '^AXIOM ' "$T/census.log"; echo "$summary"; exit 0; fi
grep -v '^[[:space:]]*#' "$ALLOW" | awk 'NF{print $1}' | LC_ALL=C sort -u > "$T/allow"
grep -qx 'sorryAx' "$T/allow" && red "lean.axioms lists sorryAx -- sorry is never allowlisted"
grep '^AXIOM ' "$T/census.log" > "$T/used"
awk '{print $2}' "$T/used" | LC_ALL=C sort -u > "$T/used.names"
while read -r _ ax n who; do
  grep -qx "$ax" "$T/allow" && continue
  if [ "$ax" = sorryAx ]; then red "sorry: $n declaration(s) rest on sorryAx, e.g. $who"
  else red "axiom $ax is not in tools/gates/lean.axioms -- $n declaration(s) rest on it, e.g. $who"; fi
done < "$T/used"
LC_ALL=C comm -23 "$T/allow" "$T/used.names" > "$T/unused"
while read -r ax; do
  red "tools/gates/lean.axioms lists $ax, which no Bebop declaration uses any more -- delete the line"
done < "$T/unused"
step "$summary"

# ---- 3. conformance --------------------------------------------------------------------------
if [ "${LEAN_ONLY:-}" = census ]; then
  [ $reds -eq 0 ] && { echo "lean: GREEN census only (conformance NOT RUN: LEAN_ONLY=census)"; exit 0; }
  echo "lean: RED ($reds finding(s); conformance NOT RUN)"; exit 1
fi
step "parityrun"
# A scratch root: the .lean sources (what results.txt hashes) and links to the inputs parityrun
# reads, so the run writes ITS results.txt here and never over the committed one.
PR=$T/root; mkdir -p "$PR/formal"
(cd "$FORMAL" && find . -name '*.lean' -not -path './.lake/*' | tar -cf - -T -) | tar -xf - -C "$PR/formal"
for d in bench .bcas selfhost; do [ -e "$BL/$d" ] && ln -s "$BL/$d" "$PR/$d"; done
"$FORMAL/.lake/build/bin/parityrun" "$PR" > "$T/parity.log" 2> "$T/parity.err"; prc=$?
FRESH=$PR/formal/results.txt
if [ $prc -ge 2 ] || [ ! -s "$FRESH" ]; then
  tail -15 "$T/parity.err"; tail -15 "$T/parity.log"
  echo "lean: REFUSED -- parityrun exited $prc and wrote no results (measured nothing)"; exit 4
fi
conf=$(sed -n 's/^lean_conformance //p' "$FRESH"); P=${conf%/*}; N=${conf#*/}
grep -E '^(positive|negative)/[^ ]+ (PASS|FAIL)( |$)' "$FRESH" > "$T/rows"
rows=$(wc -l < "$T/rows"); npass=$(grep -c ' PASS' "$T/rows")
[ "${N:-0}" -gt 0 ] 2>/dev/null || { echo "lean: REFUSED -- results.txt has no lean_conformance count"; exit 4; }
[ "$rows" -eq "$N" ] && [ "$npass" -eq "$P" ] || red "results.txt disagrees with itself: lean_conformance $conf but $npass PASS of $rows rows"
floor=$(awk '$1=="floor"{print $2}' "$BASE")
[ -n "$floor" ] || { echo "lean: REFUSED -- no 'floor' line in $BASE"; exit 2; }
awk '$1=="expect-fail"{print $2}' "$BASE" | LC_ALL=C sort -u > "$T/expect"
awk '$2=="FAIL"{print $1}' "$T/rows" | LC_ALL=C sort > "$T/fails"
LC_ALL=C comm -23 "$T/fails" "$T/expect" > "$T/unexp"
while read -r c; do red "conformance: $c FAILS and is not a named expect-fail: $(grep "^$c " "$T/rows" | cut -d' ' -f3- | cut -c1-160)"; done < "$T/unexp"
while read -r c; do
  if grep -q "^$c PASS" "$T/rows"; then red "conformance: $c now PASSES -- delete its expect-fail line in lean.baseline (and raise the floor)"
  elif ! grep -q "^$c " "$T/rows"; then red "conformance: expect-fail $c names no construct that ran"; fi
done < "$T/expect"
[ "$P" -ge "$floor" ] || red "conformance: $P/$N passes is below the floor $floor"
# The hash half of F4: recomputable without parityrun, with the recipe results.txt prints.
want=$(sed -n 's/^lean_sources_sha256 //p' "$FRESH")
got=$(cd "$PR" && find formal -name '*.lean' | LC_ALL=C sort | xargs sha256sum | sha256sum | cut -d' ' -f1)
[ "$want" = "$got" ] || red "results.txt lean_sources_sha256 $want, but the sha256sum recipe gives $got"
if [ "${LEAN_WRITE:-}" = 1 ]; then
  cp "$FRESH" "$FORMAL/results.txt"; step "wrote $FORMAL/results.txt ($conf)"
  if [ "$P" -gt "$floor" ]; then sed -i "s/^floor $floor\b/floor $P/" "$BASE"; step "floor raised $floor -> $P in $BASE"; fi
else
  grep -v '^toolchain ' "$FORMAL/results.txt" > "$T/committed.cmp" 2>/dev/null
  grep -v '^toolchain ' "$FRESH" > "$T/fresh.cmp"
fi
if [ "${LEAN_WRITE:-}" != 1 ] && ! diff "$T/committed.cmp" "$T/fresh.cmp" > "$T/stale.diff" 2>&1; then
  head -12 "$T/stale.diff" | sed 's/^/  | /'
  red "bebop-lang/formal/results.txt is stale against this tree -- refresh it with LEAN_WRITE=1 sh tools/gates/lean.sh"
fi
[ "$P" -gt "$floor" ] && [ "${LEAN_WRITE:-}" != 1 ] && step "note: $P passes, floor $floor could rise (LEAN_WRITE=1 raises it)"

nexp=$(wc -l < "$T/expect")
if [ $reds -eq 0 ]; then
  echo "lean: GREEN build ok, $(wc -l < "$T/used.names") axioms all allowlisted, conformance $conf (floor $floor, $nexp named expect-fail)"
  exit 0
fi
echo "lean: RED ($reds finding(s)); conformance $conf, floor $floor"
exit 1
