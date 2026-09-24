#!/bin/sh
# G6 (AUDIT-BUGS-BLINDSPOTS-TESTS-2026-09-24 §3a G-principal) -- A PRINCIPAL IS
# BOUND TO THE OBJECT IT ACTS ON, NOT ONLY TO THE VENUE.
#
# THE DEFECTS THIS GUARDS. `principal_at` asks one question: "does this token
# belong to this venue". Every customer token of the venue does -- one is
# minted for every coffee, and since `2cfab80b` for every anonymous booking.
#   D18  `social::messages`/`send` took a thread id from the path and asked
#        nothing more: any customer token read and posted in any order's
#        thread, and a courier read every conversation.
#   D13  the room's `pay` took a wallet id from the body and debited it: a
#        waiter could spend any customer's wallet by typing its id.
#   D40  a booking stored the `userId` its body chose.
#
# THE RULE. A handler that authenticates (`principal_at(`, `authenticate(`,
# `authenticate_token(`, `Place::of_any(`, `staff_at(`) and then
#   (a) takes an OBJECT id from the path (`param("id"|"key"|"token"|..)`) must
#       bind the principal to it: call a whole-principal binder (`side_of`,
#       `side_at`, `thread_party`, `owner_at`, `owner_and_venue`, `staff_at`,
#       `courier_at`, `room_admits`), or, in every `Principal::<Role>` match
#       arm, compare the role's own id or capability (`order_id ==`,
#       `courier_id`, `caps.allows(`, the owner's `active_location_id`) or
#       refuse. An arm that answers `true` binds nothing.
#   (b) takes a PERSON id from the body (`body.wallet`, `.user`, `.user_id`)
#       must decide it through `wallet_who(`/`payer(`/`booking_user(`, or be
#       an owner-only route (`Principal::Owner {..}) => {}` then `Ok(_) =>
#       return`).
# `principal-binds.baseline` names the known exceptions (`file::fn`) and may
# only shrink.
#
# Comments and string literals are blanked first (the `idem-done.sh`
# technique); the path parameter's NAME is read from the raw text.
#
# `sh tools/gates/principal-binds.sh [ROOT]` -- ROOT defaults to the repo.
# Exit 1 on an unbound handler not in the baseline, or a baseline entry that
# is bound now (lower the baseline). Exit 2 when it found no authenticated
# handler taking an object id: the parse broke, not the feature.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/principal-binds.baseline"
[ -f "$BASELINE" ] || { echo "principal-binds: no baseline at $BASELINE"; exit 2; }

found=$(ROOT="$ROOT" BASELINE="$BASELINE" python3 - <<'PY'
import os, re, glob

ROOT = os.environ['ROOT']

def blank(src):
    """Comments and string/char literals become spaces; offsets are kept."""
    out, i, n = list(src), 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith('//', i):
            j = src.find('\n', i)
            j = n if j < 0 else j
            for k in range(i, j): out[k] = ' '
            i = j
        elif src.startswith('/*', i):
            j = src.find('*/', i + 2)
            j = n if j < 0 else j + 2
            for k in range(i, j):
                if src[k] != '\n': out[k] = ' '
            i = j
        elif c == 'r' and re.match(r'r#*"', src[i:i + 4]) and (i == 0 or not (src[i-1].isalnum() or src[i-1] == '_')):
            hashes = re.match(r'r(#*)"', src[i:]).group(1)
            end = '"' + hashes
            j = src.find(end, i + 2 + len(hashes))
            j = n if j < 0 else j + len(end)
            for k in range(i, j):
                if src[k] != '\n': out[k] = ' '
            i = j
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            for k in range(i + 1, min(j, n)):
                if src[k] != '\n': out[k] = ' '
            i = j + 1
        elif c == "'" and re.match(r"'(\\.|[^\\'])'", src[i:i + 4]):
            m = re.match(r"'(\\.|[^\\'])'", src[i:i + 4])
            for k in range(i + 1, i + m.end() - 1): out[k] = ' '
            i += m.end()
        else:
            i += 1
    return ''.join(out)

def match_brace(s, b):
    depth, i = 1, b + 1
    while i < len(s) and depth > 0:
        if s[i] == '{': depth += 1
        elif s[i] == '}': depth -= 1
        i += 1
    return i

SOURCE = re.compile(r'principal_at\s*\(|Place::of_any\s*\(|\bauthenticate\s*\(|authenticate_token\s*\(|staff_at\s*\(')
PARAM = re.compile(r'\bparam\(\s*"(id|key|token|order_id|thread_id)"\s*\)')
PERSON = re.compile(r'\b(?:body|b)\.(wallet|user_id|user)\b')
WHOLE = re.compile(r'\b(?:side_of|side_at|thread_party|owner_at|owner_and_venue|staff_at|courier_at|room_admits)\s*\(')
PERSON_BINDER = re.compile(r'\b(?:wallet_who|payer|booking_user)\s*\(')
OWNER_ONLY = re.compile(r'Principal::Owner\s*\{[^}]*\}\s*\)\s*=>\s*\{\s*\}\s*,?\s*Ok\(_\)\s*=>\s*return\b')
ARM = re.compile(r'Principal::(Customer|Courier|Staff|Owner)\b')
BIND = {
    'Customer': re.compile(r'order_id\s*==|==\s*\*?order_id\b'),
    'Courier': re.compile(r'courier_id\s*==|==\s*(?:Some\(\s*)?\*?courier_id\b'),
    'Staff': re.compile(r'caps\.allows\s*\('),
    'Owner': re.compile(r'active_location_id|belongs_to\s*\('),
}
REFUSE = re.compile(r'\breturn\b|\bfalse\b|\bErr\s*\(')

def units(src):
    """Every fn body, and every route closure `.get_async("..", |req, ctx| async move {..})`."""
    for fm in re.finditer(r'\bfn (\w+)\s*[<(]', src):
        b = src.find('{', fm.end())
        if b >= 0:
            yield fm.group(1), b, match_brace(src, b)
    for cm in re.finditer(r'_async\(\s*"\s*"\s*,\s*\|[^|]*\|\s*async\s+move\s*\{', src):
        b = cm.end() - 1
        yield f'route@{src[:b].count(chr(10)) + 1}', b, match_brace(src, b)

def arms_bound(body, via_principal_at):
    """Every `Principal::<Role>` arm compares that role's own id, or refuses."""
    ms = list(ARM.finditer(body))
    if not ms:
        return False
    for k, m in enumerate(ms):
        role = m.group(1)
        end = ms[k + 1].start() if k + 1 < len(ms) else min(len(body), m.end() + 800)
        arm = body[m.start():end]
        if role == 'Owner' and via_principal_at:
            continue  # `principal_at` already holds the owner to this venue
        if BIND[role].search(arm) or REFUSE.search(arm):
            continue
        return False
    return True

scanned, hits = 0, []
for f in sorted(glob.glob(os.path.join(ROOT, 'workers/api/src/**/*.rs'), recursive=True)):
    if f.endswith('tests.rs') or '/tests/' in f:
        continue
    raw = open(f, encoding='utf-8').read()
    src = blank(raw)
    rel = os.path.relpath(f, ROOT)
    for name, b, e in units(src):
        body, rbody = src[b:e], raw[b:e]
        if not SOURCE.search(body):
            continue
        objs = sorted(set(PARAM.findall(rbody)))
        persons = sorted(set(PERSON.findall(body)))
        if not objs and not persons:
            continue
        # A route closure's body also contains its enclosing fn's other
        # closures; a fn unit that merely CONTAINS route closures (the
        # router) is measured through the closures instead.
        if not name.startswith('route@') and re.search(r'_async\(\s*"\s*"\s*,\s*\|', body):
            continue
        scanned += 1
        via_pa = bool(re.search(r'principal_at\s*\(', body))
        if objs and not (WHOLE.search(body) or arms_bound(body, via_pa)):
            hits.append(f'{rel}::{name} path id {",".join(objs)}: no binder')
        if persons and not (PERSON_BINDER.search(body) or OWNER_ONLY.search(body)):
            hits.append(f'{rel}::{name} body {",".join(persons)}: a person id nobody decided')

base = set()
for line in open(os.environ['BASELINE'], encoding='utf-8'):
    line = line.split('#', 1)[0].strip()
    if line and not line.startswith('scanned'):
        base.add(line)
keys = {h.split(' ', 1)[0] for h in hits}
new = [h for h in hits if h.split(' ', 1)[0] not in base]
gone = sorted(base - keys)
print(scanned)
print(len(new))
for h in new: print('  NEW ' + h)
for g in gone: print('  GONE ' + g)
PY
)
scanned=$(printf '%s\n' "$found" | sed -n 1p)
n=$(printf '%s\n' "$found" | sed -n 2p)
rest=$(printf '%s\n' "$found" | tail -n +3)

if [ -z "$scanned" ] || [ "$scanned" -eq 0 ]; then
  echo "principal-binds: found no authenticated handler taking an object or person id under $ROOT/workers/api/src -- the parse broke, not the feature"
  exit 2
fi
if [ "$n" -gt 0 ]; then
  echo "principal-binds: REFUSED -- $n handler(s) authenticate and then act on an id nobody bound ($scanned scanned):"
  printf '%s\n' "$rest" | grep '^  NEW' || true
  echo "principal-binds: bind the principal to the object (order_id == id, courier_id, caps.allows, side_of,"
  echo "principal-binds: thread_party) or decide the person through wallet_who / payer / booking_user (D13, D18, D40)."
  exit 1
fi
if printf '%s\n' "$rest" | grep -q '^  GONE'; then
  echo "principal-binds: a baseline exception is bound now -- delete it from principal-binds.baseline in this commit:"
  printf '%s\n' "$rest" | grep '^  GONE'
  exit 1
fi
echo "principal-binds: 0 unbound handler(s) outside the baseline ($scanned authenticated handler(s) taking an id)"
