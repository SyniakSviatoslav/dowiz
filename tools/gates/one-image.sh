#!/bin/sh
# F3 — ONE IMAGE PER TRANSACTION.
#
# THE DEFECT, and it has already been paid for twice. A venue's state is not one
# image: the order log, the catalogue, the stock ledger and the settings each
# have their own root and their own generation. A handler that writes two of
# them is a distributed transaction with no coordinator, and what it does when
# the second write fails is whatever somebody remembered to write by hand.
#
# `storefront::place` reserves stock in one image and appends the order to
# another. When the append failed, the compensation that releases the
# reservation could itself fail -- and then it LOGGED, which is how
# `stock::stranded()` came to exist: a list of holds against orders that never
# happened, shown to an owner with no action attached. The kitchen believes it
# is out of something it has.
#
# `253e1ece` is the same class in its earlier form: a half-applied `db.batch`
# left a booking's status disagreeing with its own history. It was fixed by
# making that aggregate ONE image, which is the event-sourced answer and is the
# answer here too.
#
# THE RULE: one handler body, one image touched. A body that needs two is a
# COMMAND the object should execute in one of its own turns, where the images
# are already in memory and nothing can interleave -- see
# `BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md` P1.
#
# HOW THIS COUNTS. Brace-matched function bodies, not lines. It counts the
# DISTINCT image families a body writes -- `with_catalog`, `with_stock`,
# `with_hub`/`append_for`, `with_settings`, `with_table`/`hubstore::save` -- and
# reports a body that writes more than one. Reads are not counted: reading two
# images to answer a question is a round trip, not a transaction.
#
# Python because `sh` cannot match braces, and it is already required by
# `one-venue.sh` in this directory.
set -eu
cd "$(dirname "$0")/../.."
BASELINE=tools/gates/one-image.baseline

found=$(python3 - <<'PY'
import re, glob

# The WRITE doors, by image family. A body naming two of these families writes
# two images and is what this gate is for.
FAMILIES = {
    'catalogue': ('with_catalog(',),
    'stock':     ('with_stock(',),
    'log':       ('append_for(', 'with_hub('),
    'settings':  ('with_settings(',),
    'table':     ('with_table(', 'with_at('),
}

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
        touched = sorted(k for k, doors in FAMILIES.items() if any(d in body for d in doors))
        if len(touched) > 1:
            hits.append(f + '::' + m.group(1) + '  writes ' + '+'.join(touched))
print(len(hits))
for h in hits:
    print('  ' + h)
PY
)
n=$(printf '%s\n' "$found" | head -1)
names=$(printf '%s\n' "$found" | tail -n +2)

if [ ! -f "$BASELINE" ]; then
  printf 'bodies=%s\n' "$n" > "$BASELINE"
  echo "one-image: baseline written at $n"
  printf '%s\n' "$names"
  exit 0
fi
b=$(awk -F= '/^bodies=/{print $2}' "$BASELINE")

if [ "$n" -gt "$b" ]; then
  echo "one-image: REFUSED — $n handler body/bodies write more than one image (baseline $b):"
  printf '%s\n' "$names"
  echo "one-image: a transaction over two images has no coordinator. Make it a command the"
  echo "one-image: object executes in one turn, or say here why this one cannot be."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "one-image: $n (baseline $b) — the ratchet has fallen. Lower it to bodies=$n in this commit."
  exit 1
fi
echo "one-image: $n handler body/bodies write more than one image (baseline $b)"
printf '%s\n' "$names"
