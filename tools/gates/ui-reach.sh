#!/bin/sh
# UI-REACH — NO BACKEND WITHOUT UI (operator 2026-09-26: "усе що є на бекенді,
# має бути підключено, видно та мати змогу використовуватись на ui також").
#
# THE RULE: every route `workers/api/src/lib.rs` registers is either CALLED from
# a screen under `workers/api/public/**`, or listed in `ui-reach.machine` with
# the reason no screen can call it (a webhook, a printer, a cron probe). A route
# that is neither is an ORPHAN: a capability an owner cannot reach without curl.
#
# HOW A CALL IS FOUND. Every string in the public JS/HTML (quotes and template
# literals) that looks like a path is normalised: `${...}` and a string that
# ends in `/` followed by `+` become one parameter segment. A route matches when
# its pattern (`:param` / `*rest` = one or more segments) equals a path found,
# with the prefixes the helpers add taken off: `/api` (admin `api()`/`post()`,
# `room/net.js`), and `/api/public/locations/:slug` (`kit/data.js call()`,
# `room`). Query strings are ignored.
#
# WHAT IT DOES NOT CHECK, said rather than hidden: the METHOD. A path reached
# by one method counts for every method on it (GET and POST of the same path
# are one row). A path that only a test file names does not count (`*.test.mjs`
# are skipped), and neither does a comment.
#
# BASELINE: the number of orphans. It may only fall; W-WIRE brought it to 0.
set -eu
cd "$(dirname "$0")/../.."
BASELINE=tools/gates/ui-reach.baseline
MACHINE=tools/gates/ui-reach.machine
LIB=${UI_REACH_LIB:-workers/api/src/lib.rs}
PUB=${UI_REACH_PUBLIC:-workers/api/public}

out=$(python3 - "$LIB" "$PUB" "$MACHINE" <<'PY'
import re, sys, os
lib, pub, machine = sys.argv[1:4]
src = open(lib, encoding='utf-8').read()
routes = []
for m in re.finditer(r'\.(get|post|put|delete|patch)(?:_async)?\(\s*"(/[^"]*)"', src):
    routes.append((m.group(1).upper(), m.group(2)))
listed = {}
for ln in open(machine, encoding='utf-8'):
    ln = ln.strip()
    if not ln or ln.startswith('#'):
        continue
    parts = ln.split(None, 2)
    if len(parts) < 3 or not parts[2].strip():
        print('BAD-MACHINE-LINE ' + ln)
        continue
    listed[(parts[0].upper(), parts[1])] = parts[2]

STR = re.compile(r"""'((?:[^'\\\n]|\\.)*)'\s*(\+)?|"((?:[^"\\\n]|\\.)*)"\s*(\+)?|`((?:[^`\\]|\\.)*)`""", re.S)
PATH = re.compile(r'/[A-Za-z0-9_\-./:*]+')
JOIN = re.compile(r"""(['"])(/[^'"\n]*/)\1\s*\+\s*[^'"`;\n]{1,80}?\+\s*(['"])(/[^'"\n]*)\3""")
found = set()
def add(text):
    text = re.sub(r'/\$\{[^}]*\}', '/:x', text)   # a whole segment is a parameter
    text = re.sub(r'\$\{[^}]*\}', '', text)        # anything else is a suffix (a query)
    for p in PATH.findall(text):
        p = p.split('?')[0].rstrip('.')
        if len(p) > 1:
            found.add(p)
for root, _, files in os.walk(pub):
    for f in files:
        if not f.endswith(('.js', '.mjs', '.html')) or f.endswith('.test.mjs') or f == 'dom-shim.mjs':
            continue
        body = open(os.path.join(root, f), encoding='utf-8', errors='replace').read()
        body = re.sub(r'(^|\s)//[^\n]*', r'\1', body)          # line comments (not `https://`, not a regex's `\//`)
        body = re.sub(r'/\*.*?\*/', '', body, flags=re.S)       # block comments
        body = re.sub(r'<!--.*?-->', '', body, flags=re.S)
        for m in JOIN.finditer(body):                          # '/a/' + id + '/b'
            add(m.group(2) + ':x' + m.group(4))
        for m in STR.finditer(body):
            s1, p1, s2, p2, s3 = m.groups()
            s = s1 if s1 is not None else (s2 if s2 is not None else s3)
            plus = p1 or p2
            if s and plus and s.endswith('/'):
                s = s + ':x'
            if s:
                add(s)

def reaches(route, path):
    """Segment by segment: a route's `:p` and a found `:x` match any one
    segment (see below); a route's `*rest` matches one or more; a found path ending in `/`
    reaches a `*rest` route at that point."""
    rs, ps = route.strip('/').split('/'), path.strip('/').split('/')
    # A found parameter stands for a route's LITERAL segment only after two
    # literal ones (`'/staff/till/' + what`); `/owner/${x}` or `/${a}/${b}`
    # would otherwise reach every route of that length.
    if ps[0] == ':x':
        return False
    for i, r in enumerate(rs):
        if i < len(ps) and ps[i] == ':x' and not r.startswith(':') and i < 2:
            return False
        if r.startswith('*'):
            return len(ps) > i or path.endswith('/') and len(ps) == i
        if i >= len(ps):
            return False
        if not (r.startswith(':') or ps[i] == ':x' or ps[i] == r):
            return False
    return len(ps) == len(rs)

def candidates(route):
    c = [route]
    if route.startswith('/api/'):
        c.append(route[4:])
    m = re.match(r'^/api/public/locations/:[a-z]+(/.*)$', route)
    if m:
        c.append(m.group(1))
    return c

orphans = []
reached = 0
for meth, r in routes:
    if (meth, r) in listed:
        continue
    if any(reaches(c, x) for c in candidates(r) for x in found):
        reached += 1
    else:
        orphans.append(f'{meth} {r}')
stale = [f'{m} {r}' for (m, r) in listed if (m, r) not in routes]
for o in orphans:
    print('ORPHAN ' + o)
for s in stale:
    print('STALE-MACHINE ' + s)
print(f'COUNT routes={len(routes)} reached={reached} machine={len(listed) - len(stale)} orphans={len(orphans)} stale={len(stale)}')
PY
)
printf '%s\n' "$out" | grep -v '^COUNT' || true
count=$(printf '%s\n' "$out" | sed -n 's/^COUNT //p')
orphans=$(printf '%s\n' "$count" | sed 's/.*orphans=\([0-9]*\).*/\1/')
stale=$(printf '%s\n' "$count" | sed 's/.*stale=\([0-9]*\).*/\1/')
bad=$(printf '%s\n' "$out" | grep -c '^BAD-MACHINE-LINE' || true)
[ -n "$orphans" ] || { echo "ui-reach: the scan printed no count -- it measured nothing"; exit 2; }
base=$(cat "$BASELINE" 2>/dev/null || echo "")
[ -n "$base" ] || { echo "ui-reach: no baseline at $BASELINE"; exit 2; }
if [ "$stale" -ne 0 ] || [ "$bad" -ne 0 ]; then
  echo "ui-reach: $stale MACHINE-ONLY line(s) name no route and $bad have no reason ($count)"
  exit 1
fi
if [ "$orphans" -gt "$base" ]; then
  echo "ui-reach: $orphans route(s) with no screen and no MACHINE-ONLY reason (baseline $base; $count)"
  exit 1
fi
echo "ui-reach: $orphans orphan route(s) (baseline $base; $count)"
