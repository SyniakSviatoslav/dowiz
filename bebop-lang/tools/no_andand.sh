#!/usr/bin/env bash
# tools/no_andand.sh -- RE-SCOPED 2026-09-14 by the gate-catch-up lane.
#
# WHAT THIS USED TO BE, AND WHY IT HAD TO CHANGE. From 2026-09-13 this gate BANNED every live
# `&&` in the tree, on a premise stated in its own header: "The && operator in Bebop always
# evaluates to 0 and binds tighter than comparison, causing subtle logic bugs." The first half
# of that was never quite right and the second half stopped being right the same week: A26's
# compiler half (`0cb2c23`) made `&&` and `||` REAL LOGICAL OPERATORS -- value 0/1, both
# operands through `vs_to_bool`, precedence `|| < && < cmp` (bebop.bp:3853), i.e. LOOSER than
# comparison. Measured 2026-09-14: `a > b && b < 9` with a=3 b=5 gives 0, which is what it
# means; under the old grammar it parsed as `a > (b & b) < 9` and gave 1.
#
# So the ban was red-lining a working operator, and `tools/battery.sh` was repeating the false
# premise in its own comments. A gate whose stated reason is false is worse than no gate: it
# spends the reader's trust and it made a whole session's worth of `.bp` edits work around a
# defect that no longer existed.
#
# WHAT IT ASSERTS NOW. The operators are legal, so there is nothing to ban. What IS worth a
# cheap, named guard is the thing that actually broke: the two implementations disagreeing
# about them. `bench/parity_constructs/c46_andor.bp` is the construct that pins the semantics,
# and its header carried 111100 -- the PRE-A26 mis-parse -- while bebop.bin printed 101100 and
# `tools/bpref.py` printed 111100. The oracle agreed with the stale header, so the ordinary
# parity gate could not see the disagreement. This gate now asserts that the pinning construct
# exists and still carries the DERIVED post-A26 value, which is the regression that would let
# the pre-A26 reading back in. Live `&&` / `||` counts are reported as INFORMATION.
#
# Text-only, no compile -- same cost as before, and it keeps the `no_andand:` output prefix so
# `tools/battery.sh`'s existing assertion line needs no change.
#
# Exit 0 = the pin holds. Non-zero = it does not.

cd "$(dirname "$0")/.." || exit 1

PIN=bench/parity_constructs/c46_andor.bp
WANT=101100

if [ ! -f "$PIN" ]; then
  echo "no_andand: FAIL -- $PIN is missing; nothing pins && / || semantics"
  exit 1
fi

got=$(grep -oE '// EXPECT [^ ]+' "$PIN" | head -1 | awk '{print $3}')
if [ "$got" != "$WANT" ]; then
  echo "no_andand: FAIL -- $PIN expects '$got', want $WANT (the A26 value; 111100 is the"
  echo "           pre-A26 mis-parse, in which \`a > b && b < 9\` parsed as \`a > (b & b) < 9\`)"
  exit 1
fi

# Information, not a verdict: how much of the tree uses the operators at all. String literals
# are blanked BEFORE comments are stripped -- a `//` inside a string would otherwise hide a
# real operator on the same line (measured false negative, 2026-09-13, kept from the old gate).
scan() { while IFS= read -r f; do sed 's/"[^"]*"/""/g; s|//.*||g' "$f"; done < <(find . -name '*.bp' -not -path './.git/*') | grep -c "$1"; }
n_and=$(scan '&&')
n_or=$(scan '||')

echo "no_andand: PASS -- c46_andor pins && / || at $WANT (A26: logical, 0/1, no short circuit,"
echo "           precedence || < && < cmp). Live sites, reported not banned: && in $n_and line(s), || in $n_or line(s)."
exit 0
