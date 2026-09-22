#!/bin/sh
# F37 — A CONTROL A THUMB CAN HIT.
#
# THE TOKEN EXISTS AND IS IGNORED. `lib/tokens.css:96` declares `--tap: 44px`,
# which is the size every platform guideline and every usability study puts a
# touch target at. Controls across the served surfaces are 36, 38 and 40 — and
# `e2e/kit-regression/mobile.mjs`, the audit that would have caught it, is in
# NO workflow. A token nothing honours and an audit nothing runs is the shape
# this repo keeps finding: `worker_errors`, which never received a row;
# `StockLedger::stranded()`, which nothing called; `no-courier-scoring`, a CI
# job that did not exist.
#
# WHO IT IS FOR, because 4px sounds like pedantry and is not: the person using
# this is a waiter holding plates in one hand, or a courier on a scooter in the
# rain. A miss is not an annoyance, it is a wrong order sent to a kitchen.
#
# WHAT IT COUNTS: a `min-height` under the token on a rule whose selector names
# an interactive class. It is a ratchet that may only fall, not a refusal of
# the tree as it stands -- there are real ones today and they are listed.
#
# WHAT IT CANNOT SEE, said plainly: a control sized by padding and line-height
# rather than `min-height`, one sized in a different unit, and one whose class
# is not in the list below. It is a CSS scan, not a rendered measurement --
# `mobile.mjs` is that, and belongs in CI beside this.
set -eu
cd "$(dirname "$0")/../.."
BASELINE_FILE=tools/gates/tap-size.baseline
TAP=44

hits() {
  for f in $(find workers/api/public -name '*.css' | sort); do
    sed 's,/\*.*\*/,,' "$f" \
      | grep -nE '^\.(act|btn|tab|seg-b|cta|chip|pill|vstate|nav|row|card)[^{]*\{[^}]*min-height: *[0-9]+px' \
      | while IFS= read -r line; do
          px=$(printf '%s' "$line" | sed -n 's/.*min-height: *\([0-9]*\)px.*/\1/p')
          [ -n "$px" ] || continue
          [ "$px" -lt "$TAP" ] && printf '%s:%s  (%spx < %spx)\n' "$f" "${line%%:*}" "$px" "$TAP"
        done
  done || true
}

n=$(hits | grep -c . || true)
echo "tap-size: $n interactive control(s) below the --tap token of ${TAP}px"
if [ ! -f "$BASELINE_FILE" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "tap-size: baseline recorded at $n"
  exit 0
fi
baseline=$(cat "$BASELINE_FILE")
if [ "$n" -gt "$baseline" ]; then
  echo "tap-size: FAILED — $((n - baseline)) more than the baseline of $baseline."
  echo "tap-size: the person tapping this is holding plates. A miss is a wrong order."
  hits | sed 's|^|  |'
  exit 1
fi
if [ "$n" -lt "$baseline" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "tap-size: ratchet lowered $baseline -> $n. Commit the baseline with the change."
fi
[ "$n" -gt 0 ] && hits | sed 's|^|  |'
exit 0
