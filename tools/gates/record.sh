#!/bin/sh
# G2 — THE CUSTOMER CARD HOLDS NOTHING A FOLD ALREADY KNOWS.
#
# BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22 §5 G2, §3.1. The card in the venue's
# `people` image is what the venue would write on a paper card by the till: a
# note, tags from a closed list, EU-14 allergens, the usual table, a language, a
# birthday with no year. Order count, spend, last visit, name and phone are
# FOLDS over the order log -- a stored copy is a second number that disagrees
# with the fold, and the one that disagrees is always the copy (`promo.rs`).
# `tier`, `score`, `stamps`, `balance` are ratings or counters and never fields.
#
# TWO COUNTS, one number:
#   * the allow-list itself: `record::FIELDS` must be exactly the blueprint's
#     eight names; every name added or missing is a hit.
#   * every function body that WRITES a card (`put("cust"`, `put(KIND`) --
#     comments stripped -- is searched for a banned key as a string literal.
#
# `sh tools/gates/record.sh [ROOT]` — ROOT defaults to the repo, so
# `record.prove.sh` can run this same code against a scratch copy holding a
# deliberate defect. Exit 1 when the count rises above the baseline (0).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/record.baseline"

found=$(ROOT="$ROOT" python3 - <<'PY'
import os, re, glob

ROOT = os.environ['ROOT']
ALLOW = ['note', 'tags', 'allergens', 'usual_table', 'lang', 'birthday_md',
         'created_at_ms', 'updated_at_ms']
BANNED = ['orders', 'spent', 'last_at', 'lastAt', 'name', 'phone', 'phone_hash',
          'balance', 'stamps', 'tier', 'score', 'rating', 'vip']
WRITE = re.compile(r'\bput\(\s*("cust"|KIND|record::KIND)\s*,')

def strip(src):
    return re.sub(r'//[^\n]*', '', src)

hits = []
rec = os.path.join(ROOT, 'workers/api/src/services/customers/record.rs')
m = re.search(r'pub const FIELDS:[^=]*=\s*\[(.*?)\];', strip(open(rec, encoding='utf-8').read()), re.S)
fields = re.findall(r'"(\w+)"', m.group(1)) if m else []
if not m:
    hits.append('record.rs: no FIELDS allow-list at all')
for f in fields:
    if f not in ALLOW:
        hits.append(f'record.rs::FIELDS holds {f!r}, which is not on the card')
for a in ALLOW:
    if a not in fields:
        hits.append(f'record.rs::FIELDS lost {a!r}')

for f in sorted(glob.glob(os.path.join(ROOT, 'workers/api/src/**/*.rs'), recursive=True)):
    if f.endswith('tests.rs'):
        continue
    src = strip(open(f, encoding='utf-8').read())
    for fm in re.finditer(r'\bfn (\w+)\s*[<(]', src):
        b = src.find('{', fm.end())
        if b < 0:
            continue
        depth, i = 1, b + 1
        while i < len(src) and depth > 0:
            if src[i] == '{': depth += 1
            elif src[i] == '}': depth -= 1
            i += 1
        body = src[b:i]
        if not WRITE.search(body):
            continue
        # `"name"` in a json! literal and `\"name\"` inside a format string.
        for k in re.findall(r'\\?"(\w+)\\?"', body):
            if k in BANNED:
                hits.append(f'{os.path.relpath(f, ROOT)}::{fm.group(1)} writes {k!r} onto a card')

print(len(hits))
for h in hits:
    print('  ' + h)
PY
)
n=$(printf '%s\n' "$found" | head -1)
names=$(printf '%s\n' "$found" | tail -n +2)

if [ ! -f "$BASELINE" ]; then
  printf 'hits=%s\n' "$n" > "$BASELINE"
  echo "record: baseline written at $n"
  exit 0
fi
b=$(awk -F= '/^hits=/{print $2}' "$BASELINE")
if [ "$n" -gt "$b" ]; then
  echo "record: REFUSED — $n field(s) on the customer card that a fold already knows (baseline $b):"
  printf '%s\n' "$names"
  echo "record: derive it from the order log (services::customers::roll) instead of storing it."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "record: $n (baseline $b) — the ratchet has fallen. Lower it to hits=$n in this commit."
  exit 1
fi
echo "record: $n card field(s) that repeat a fold (baseline $b)"
