#!/bin/sh
# THE LEAN GATE'S PROOF: tools/gates/lean.sh is triggered before it is trusted.
#
# Every case runs the REAL gate against a SCRATCH COPY of bebop-lang/formal (its .lake included,
# so only what a case changes is rebuilt) with bench/, .bcas/ and selfhost/ linked read-only.
# The real sources are never edited: their digest is taken first and compared last.
#   1. clean copy                                       -> rc 0, "lean: GREEN"
#   2. census mutant: Bebop.lean gains a theorem proved by `sorry` and a new `axiom`, and the
#      allowlist copy gains an axiom nothing uses        -> rc 1, all three named
#   3. conformance mutant (baseline + results copies only): c84_run's expect-fail line removed,
#      c01_lit listed as expect-fail, floor 999, one row of results.txt flipped
#                                                       -> rc 1, all four named
# Needs `lake`/`lean` on PATH and a built bebop-lang/formal (it runs `lake build` there first).
# Exit 0 when every case answered as wanted; 1 otherwise; 3 no toolchain.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
REAL=$REPO/bebop-lang/formal
export LC_ALL=C.UTF-8
command -v lake >/dev/null 2>&1 || { echo "lean.prove: REFUSED -- no lake on PATH"; exit 3; }
S=$(mktemp -d "${TMPDIR:-/tmp}/lean-prove.XXXXXX")
trap 'rm -rf "$S"' EXIT
fail=0
digest() { (cd "$REPO" && { find bebop-lang/formal -name '*.lean' -not -path '*/.lake/*' | LC_ALL=C sort | xargs cat; \
  cat bebop-lang/formal/results.txt tools/gates/lean.axioms tools/gates/lean.baseline; } | sha256sum | cut -d' ' -f1); }
before=$(digest)
(cd "$REAL" && lake build) > "$S/build.log" 2>&1 || { tail -15 "$S/build.log"; echo "lean.prove: REFUSED -- the real tree does not build"; exit 1; }

copy() { # a fresh scratch tree at $S/r: formal (with .lake), links to the corpus, allow/base copies
  rm -rf "$S/r"; mkdir -p "$S/r/bebop-lang"
  (cd "$REPO/bebop-lang" && tar -cf - formal) | tar -xf - -C "$S/r/bebop-lang"
  for d in bench .bcas selfhost; do ln -s "$REPO/bebop-lang/$d" "$S/r/bebop-lang/$d"; done
  cp "$HERE/lean.axioms" "$S/allow"; cp "$HERE/lean.baseline" "$S/base"
}
gate() { LEAN_FORMAL="$S/r/bebop-lang/formal" LEAN_ALLOW="$S/allow" LEAN_BASE="$S/base" sh "$HERE/lean.sh" > "$S/out" 2>&1; echo $?; }
want() { # want <case> <line pattern>: the gate's output must contain it
  if grep -q -- "$2" "$S/out"; then echo "lean.prove:   $1 names: $(grep -m1 -- "$2" "$S/out" | cut -c1-150)"
  else echo "lean.prove:   $1 MISSING a line matching: $2"; fail=1; fi
}

copy
rc=$(gate); echo "lean.prove: 1 clean copy -> rc=$rc (want 0): $(tail -1 "$S/out")"
[ "$rc" -eq 0 ] || { sed 's/^/  | /' "$S/out" | tail -20; fail=1; }

copy
cat >> "$S/r/bebop-lang/formal/Bebop.lean" <<'LEAN'

-- RED MUTANT (tools/gates/lean.prove.sh, scratch copy only)
theorem leanci_red_mutant : (1 : Nat) = 2 := sorry
axiom leanci_red_axiom : (2 : Nat) = 3
LEAN
echo "leanci_ghost_axiom" >> "$S/allow"
rc=$(LEAN_ONLY=census gate); echo "lean.prove: 2 sorry + new axiom + stale allowlist line -> rc=$rc (want 1)"
[ "$rc" -eq 1 ] || { sed 's/^/  | /' "$S/out" | tail -20; fail=1; }
want 2 "RED sorry.*leanci_red_mutant"
want 2 "RED axiom leanci_red_axiom is not in"
want 2 "lists leanci_ghost_axiom, which no Bebop declaration uses"

copy
sed -i '/^expect-fail positive\/c84_run /d; s/^floor .*/floor 999/' "$S/base"
echo "expect-fail positive/c01_lit RED MUTANT: it passes" >> "$S/base"
sed -i 's/^positive\/c02_arith PASS /positive\/c02_arith FAIL /' "$S/r/bebop-lang/formal/results.txt"
rc=$(gate); echo "lean.prove: 3 floor 999 + unlisted FAIL + listed PASS + stale results.txt -> rc=$rc (want 1)"
[ "$rc" -eq 1 ] || { sed 's/^/  | /' "$S/out" | tail -20; fail=1; }
want 3 "positive/c84_run FAILS and is not a named expect-fail"
want 3 "positive/c01_lit now PASSES"
want 3 "is below the floor 999"
want 3 "results.txt is stale"

after=$(digest)
[ "$before" = "$after" ] || { echo "lean.prove: the REAL sources changed during the proof ($before -> $after)"; fail=1; }
echo "lean.prove: real formal sources, results.txt, lean.axioms, lean.baseline unchanged: $after"
[ $fail -eq 0 ] && { echo "lean.prove: GREEN -- every case answered as wanted"; exit 0; }
echo "lean.prove: RED -- a case above did not answer as wanted"; exit 1
