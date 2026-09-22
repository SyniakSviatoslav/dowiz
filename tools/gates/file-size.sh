#!/bin/sh
# F1 — THE FILE-SIZE RATCHET, over the code that serves a venue.
#
# Blueprint 2 §5 wants 300 lines hard and 150 as the target. `extra.rs` is 3,853
# lines, so a hard limit today would be a gate that is red on the first commit
# and therefore switched off by the second. The mechanism that works on a number
# nobody can reach in one step is the one `tools/gates/no-sql.sh` and
# `bebop-lang/tools/arch_check.py` already use: record the worst value and
# refuse to let it get worse. It may only fall, by hand, in the commit that
# earns the reduction.
#
# TWO NUMBERS, because one of them can hide the other. `over` is how many files
# are past the 300-line line: it falls when a file is split. `worst` is the
# longest file: it falls when THE file is split. A gate with only `over` would
# let `extra.rs` grow to ten thousand lines as long as somebody else split two
# files; a gate with only `worst` would let every other file drift to 299.
#
# SCOPE is the Worker and the two crates a venue's request actually executes.
# `dowiz-core` is deliberately out: it carries research code (`academia_*`,
# 5,192 lines) that no request touches, and a gate dominated by the code it is
# not defending is a gate nobody reads.
set -eu
cd "$(dirname "$0")/../.."
BASELINE=tools/gates/file-size.baseline
HARD=300

# TESTS DO NOT COUNT, which blueprint 2 §5 says in one line and which matters
# more than it sounds: a gate that counts `tests.rs` refuses the commit that
# adds the tests a split was done FOR, and the way out of that is always to
# write fewer tests. Free to adopt today -- the longest `tests.rs` in scope is
# 200 lines -- and it stays free forever, which is the point.
sizes=$(find workers/api/src crates/dowiz-hub/src crates/bebop-store/src -name '*.rs' \
  ! -name 'tests.rs' -exec wc -l {} + | grep -v ' total$')
over=$(printf '%s\n' "$sizes" | awk -v h="$HARD" '$1>h' | wc -l | tr -d ' ')
worst=$(printf '%s\n' "$sizes" | awk '{print $1}' | sort -rn | head -1)
worst_file=$(printf '%s\n' "$sizes" | sort -rn | head -1 | awk '{print $2}')

if [ ! -f "$BASELINE" ]; then
  printf 'over=%s\nworst=%s\n' "$over" "$worst" > "$BASELINE"
  echo "file-size: baseline written at over=$over worst=$worst"
  exit 0
fi
b_over=$(awk -F= '/^over=/{print $2}' "$BASELINE")
b_worst=$(awk -F= '/^worst=/{print $2}' "$BASELINE")

echo "file-size: $over file(s) over $HARD lines (baseline $b_over); worst $worst in $worst_file (baseline $b_worst)"

rc=0
if [ "$over" -gt "$b_over" ]; then
  echo "file-size: REFUSED — a file crossed $HARD lines. Split it, or earn the number."
  rc=1
fi
if [ "$worst" -gt "$b_worst" ]; then
  echo "file-size: REFUSED — $worst_file grew to $worst lines (baseline $b_worst)."
  rc=1
fi
if [ "$over" -lt "$b_over" ] || [ "$worst" -lt "$b_worst" ]; then
  echo "file-size: the ratchet has fallen. Lower $BASELINE to over=$over worst=$worst in this commit."
fi
exit $rc
