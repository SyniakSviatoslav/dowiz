#!/bin/sh
# SW-SHELL — EVERY SHELL LISTS EVERY MODULE ITS PAGE STATICALLY IMPORTS.
#
# Audit row G2 (docs/design/AUDIT-BUGS-BLINDSPOTS-TESTS-2026-09-24.md): the
# courier app was BLANK OFFLINE. `/lib/money.js` imports `/lib/vocab.js`, the
# courier's `sw.js` did not list it, and an ES module graph with one missing
# file does not half-load -- the import rejects and `app.js` never runs. The
# service worker existed precisely for the courier who reopens the app
# underground, and it served that courier a white page.
#
# A shell list is written by hand and a module graph grows by hand, in a
# different file, on a different day. Nothing connected the two. This does:
#
#   for every service worker below, the ENTRIES are the `.js` files its list
#   names plus the module scripts and stylesheets of every page it names
#   ('/courier/' -> courier/index.html); the CLOSURE is every static
#   `import ... from '...'` / `export ... from '...'` / `import '...'`
#   reachable from them. A file in the closure that is not in the list is a
#   file the phone will not have offline. That count is the number here.
#
# NOT COUNTED, on purpose: dynamic `import()` (the map, live.js -- they need
# the network to be useful and the shells leave them out deliberately), and
# absolute http(s) URLs. An import that names a file that does not exist is
# reported separately and counted too: it is broken online as well.
#
# `sh tools/gates/sw-shell.sh [ROOT]` -- ROOT defaults to the repo; the prove
# script runs this same code on a scratch copy holding a deliberate defect.
# Exit 1 when the count rises above the baseline, or falls without the baseline
# being lowered in the same commit (the ratchet only falls).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/sw-shell.baseline"

found=$(ROOT="$ROOT" python3 - <<'PY'
import os, re, posixpath

PUB = os.path.join(os.environ['ROOT'], 'workers/api/public')
# The service workers of this product. One that does not exist is skipped (the
# owner console has none today); a NEW one must be added here by hand --
# a `sw.js` on disk that neither SHELLS nor RUNTIME names counts as a gap.
SHELLS = ['/sw.js', '/room/sw.js', '/courier/sw.js', '/admin/sw.js']
# `/kit/sw.js` is not here on purpose: it caches EVERY `/kit/` and `/lib/`
# response at run time (stale-while-revalidate), so a page it served online is
# whole offline whatever its precache list says. These four cache only what
# their list names, and that is the shape this gate exists for.
RUNTIME = ['/kit/sw.js']

IMPORT = re.compile(r"""^\s*(?:import\s+(?:[^'";]*?\s+from\s+)?|export\s+[^'";]*?\s+from\s+)['"]([^'"]+)['"]""", re.M)
SCRIPT = re.compile(r"""<script\b[^>]*\btype=["']module["'][^>]*\bsrc=["']([^"']+)["']""", re.I)
LINK = re.compile(r"""<link\b[^>]*\brel=["']stylesheet["'][^>]*\bhref=["']([^"']+)["']""", re.I)

def read(p):
    f = os.path.join(PUB, p.lstrip('/'))
    if p.endswith('/'):
        f = os.path.join(f, 'index.html')
    try:
        return open(f, encoding='utf-8').read()
    except OSError:
        return None

def resolve(base, spec):
    if re.match(r'^[a-z]+:', spec) or spec.startswith('//'):
        return None
    spec = spec.split('?')[0].split('#')[0]
    return spec if spec.startswith('/') else posixpath.normpath(posixpath.join(posixpath.dirname(base), spec))

def deps(p):
    src = read(p)
    if src is None:
        return None
    if p.endswith('/') or p.endswith('.html'):
        refs = SCRIPT.findall(src) + LINK.findall(src)
    elif p.endswith('.js'):
        refs = IMPORT.findall(re.sub(r'/\*.*?\*/', '', src, flags=re.S))
    else:
        return []
    return [q for q in (resolve(p, r) for r in refs) if q]

total, lines = 0, []
for dp, _, fs in os.walk(PUB):
    for f in fs:
        if f == 'sw.js':
            p = '/' + os.path.relpath(os.path.join(dp, f), PUB)
            if p not in SHELLS and p not in RUNTIME:
                lines.append(f'  {p}: a service worker this gate does not know; add it to SHELLS')
                total += 1
for sw in SHELLS:
    src = read(sw)
    if src is None:
        continue
    # A shell entry is a string literal ALONE ON ITS LINE, which is how every
    # list here is written; `url.pathname.startsWith('/api/')` is not one.
    listed = set(re.findall(r"""^\s*['"](/[^'"\s]*)['"],?\s*(?://.*)?$""", src, re.M))
    # A directory with no index.html on disk is a document the Worker renders
    # (the storefront's '/'): it cannot be walked here, and it is not broken.
    walk = [x for x in listed if x.endswith('.js') or (x.endswith('/') and read(x) is not None)]
    seen, stack, missing = set(), list(walk), []
    while stack:
        p = stack.pop()
        if p in seen:
            continue
        seen.add(p)
        for q in deps(p) or []:
            if q.endswith('/index.html') and q[:-10] in listed:
                continue
            if q not in listed and q not in missing:
                missing.append(q)
            stack.append(q)
    # Named or imported, and not on disk: broken online as well as offline.
    broken = sorted(x for x in (listed | seen) if not x.endswith('/') and read(x) is None)
    missing = [m for m in missing if m not in broken]
    for m in sorted(missing):
        lines.append(f'  {sw}: {m} is imported but not in the shell')
    for b in sorted(broken):
        lines.append(f'  {sw}: {b} is named but does not exist')
    total += len(missing) + len(broken)

print(total)
for l in lines:
    print(l)
PY
)
n=$(printf '%s\n' "$found" | head -1)
names=$(printf '%s\n' "$found" | tail -n +2)

if [ ! -f "$BASELINE" ]; then
  printf 'missing=%s\n' "$n" > "$BASELINE"
  echo "sw-shell: baseline written at $n"
  exit 0
fi
b=$(awk -F= '/^missing=/{print $2}' "$BASELINE")

if [ "$n" -gt "$b" ]; then
  echo "sw-shell: REFUSED — $n module(s) a page imports are missing from its offline shell (baseline $b):"
  printf '%s\n' "$names"
  echo "sw-shell: add each path to that sw.js's SHELL list and bump its SHELL_CACHE name,"
  echo "sw-shell: or the page is blank offline: one missing import rejects the whole graph."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "sw-shell: $n (baseline $b) — the ratchet has fallen. Lower it to missing=$n in this commit."
  exit 1
fi
[ -n "$names" ] && printf '%s\n' "$names"
echo "sw-shell: $n shell gap(s) (baseline $b)"
