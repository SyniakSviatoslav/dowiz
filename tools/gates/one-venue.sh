#!/bin/sh
# F2 — ONE VENUE PER REQUEST.
#
# THE DEFECT. An owner route asked two different questions and used both
# answers: `owner_and_venue` decides which venue the caller is AUTHORISED for
# (`?location_id=` first, then the token's claim), while `Place::of_any` decides
# which Durable Object to ASK (the token's claim first, then the Host). For an
# owner of one venue those always agree. This platform's operator owns two, so
# a request from one venue's console carrying `?location_id=` for the other was
# authorised against one and served -- or WRITTEN -- against the other.
#
# Thirty-seven handlers had that pairing, and the membership check does not
# catch it: the caller really does own the venue they named. What they do not
# own is the answer they get.
#
# THE RULE: the venue you are authorised for is the venue you act on. Where the
# authorisation lands first, build the place from it (`Place::of_authorised`).
# Where the image read is started BESIDE the membership read, `owner_beside`
# refuses a request that names two (`Place::must_be`). What is forbidden is
# having both answers in one body and quietly using the wrong one.
#
# HOW THIS COUNTS. Brace-matched function bodies, not lines: a file-level grep
# would count a handler that legitimately uses one of them beside an unrelated
# handler that uses the other. Python because `sh` cannot match braces, and it
# is already required by the other gates in this tree.
set -eu
cd "$(dirname "$0")/../.."
BASELINE=tools/gates/one-venue.baseline

found=$(python3 - <<'PY'
import re, glob
hits = []
for f in sorted(glob.glob('workers/api/src/**/*.rs', recursive=True)):
    src = open(f, encoding='utf-8').read()
    for m in re.finditer(r'async fn (\w+)\s*\(', src):
        b = src.find('{', m.end())
        if b < 0:
            continue
        depth, i = 1, b + 1
        while i < len(src) and depth > 0:
            if src[i] == '{': depth += 1
            elif src[i] == '}': depth -= 1
            i += 1
        body = src[b:i]
        # `must_be` IS the check: a body that calls it has said out loud which
        # of the two venues wins, and the other is refused rather than served.
        if 'owner_and_venue(' in body and 'Place::of_any(' in body and 'must_be(' not in body:
            hits.append(f + '::' + m.group(1))
print(len(hits))
for h in hits:
    print('  ' + h)
PY
)
n=$(printf '%s\n' "$found" | head -1)
names=$(printf '%s\n' "$found" | tail -n +2)

if [ ! -f "$BASELINE" ]; then
  printf 'handlers=%s\n' "$n" > "$BASELINE"
  echo "one-venue: baseline written at $n"
  exit 0
fi
b=$(awk -F= '/^handlers=/{print $2}' "$BASELINE")

if [ "$n" -gt "$b" ]; then
  echo "one-venue: REFUSED — $n handler(s) decide the venue twice (baseline $b):"
  printf '%s\n' "$names"
  echo "one-venue: authorise first and use Place::of_authorised, or go through owner_beside,"
  echo "one-venue: which refuses a request that names two venues."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "one-venue: $n (baseline $b) — the ratchet has fallen. Lower it to handlers=$n in this commit."
  exit 1
fi
echo "one-venue: $n handler(s) decide the venue twice (baseline $b)"
