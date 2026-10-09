#!/bin/sh
# CANVAS WIRE BUDGET (Wave CV7; split in CV2P) -- what one canvas surface costs on the wire, and
# that it is allowed to load at all.
#
#   sh tools/gates/canvas-wire.sh [CANVAS_DIR] [HEADERS_FILE] [BASELINE_FILE]
#
# TWO NUMBERS (main's decision 2026-10-07, W-CV2P):
#   (a) THE FIRST FRAME: index.html + board.wasm + every module it reaches by static import, by
#       export-from, or by a dynamic import() of a module that is NOT a lazy row (board.js defers
#       its network stack with import(), but boot() awaits it: that is still the first frame).
#       Budget 61440 B gzip -9 (research 5.3). Never raised.
#   (b) EACH LAZY MODULE: a module listed in tools/gates/canvas-wire.baseline as `lazy <path>=<B>`,
#       reached ONLY by a dynamic import() (board.js onIntent: fetched on a tap). Its cost is its
#       own bytes plus every module it pulls that the first frame does not already carry; its
#       budget is the row (its size at adoption + 25%). The row is a ratchet: it may fall by hand,
#       and only main raises it.
# RED when any of:
#   1. the first frame is over 61440 B, or a lazy module is over its row;
#   2. a lazy row's module is reached by the first frame (a static import of it anywhere the first
#      frame reaches) -- pulled into the first frame;
#   3. board.wasm.src is not the digest of the crate's sources now (crates/dowiz-canvas/build.sh
#      --digest): the module shipped is not the code in the tree (memory
#      bebop-binary-must-match-source);
#   4. the page could not run under the site's CSP: script-src lacks 'self' or 'wasm-unsafe-eval'
#      (WebAssembly.instantiateStreaming needs it), or index.html carries an inline <style>, an
#      inline <script> without src, a style= attribute or an on*= handler -- all blocked by
#      `style-src 'self'` / `script-src 'self'` (memory dowiz-csp-blocked-every-stylesheet);
#   5. index.html's <body> holds anything but one <canvas> (the static half of dom-count).
# A NEW import() with no row is charged to the first frame (conservative: no byte escapes a
# budget); to make it lazy, main adds its row. A lazy edge here is textual: an import() of a row's module. That the import() is not CALLED at
# boot is the runtime half: canvas.mjs dom records every request before the first tap and refuses
# a lazy row's module among them (canvas-dom.prove.sh: "table.js imported at boot").
# The numbers are printed either way; exit 2 when an input is missing (never a pass).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
DIR="${1:-$ROOT/workers/api/public/room/canvas}"
HEADERS="${2:-$ROOT/workers/api/public/_headers}"
BASELINE="${3:-$HERE/canvas-wire.baseline}"
DIGEST=$(sh "$ROOT/crates/dowiz-canvas/build.sh" --digest 2>/dev/null)
[ -n "$DIGEST" ] || { echo "canvas-wire: could not compute the source digest -- NOT a pass"; exit 2; }

DIR="$DIR" PUB="$ROOT/workers/api/public" HEADERS="$HEADERS" DIGEST="$DIGEST" BASELINE="$BASELINE" python3 - <<'PY'
import gzip, os, re, sys
D, PUB, H, DIGEST, BL = (os.environ[k] for k in ('DIR', 'PUB', 'HEADERS', 'DIGEST', 'BASELINE'))
BUDGET = 60 * 1024
def need(p):
    if not os.path.exists(p):
        print(f'canvas-wire: missing {p} -- NOT a pass'); sys.exit(2)
    return open(p, 'rb').read()
html = need(os.path.join(D, 'index.html')).decode()
wasm = need(os.path.join(D, 'board.wasm'))
src = need(os.path.join(D, 'board.wasm.src')).decode().strip()
headers = need(H).decode()
LAZY = {}
for line in need(BL).decode().splitlines():
    m = re.match(r'^lazy\s+(\S+)=(\d+)\s*$', line)
    if m:
        LAZY[m.group(1)] = int(m.group(2))
    elif line.strip() and not line.lstrip().startswith('#'):
        print(f'canvas-wire: unreadable baseline line {line!r} -- NOT a pass'); sys.exit(2)

def local(url, base):
    """A served URL -> (file, path): /room/canvas/* from DIR, the rest from public/."""
    if url.startswith('/'):
        path = url
    else:
        path = os.path.normpath(os.path.join(os.path.dirname(base), url)).replace(os.sep, '/')
    return os.path.join(D, path[len('/room/canvas/'):]) if path.startswith('/room/canvas/') else os.path.join(PUB, path.lstrip('/')), path

STATIC = re.compile(r"""^\s*(?:import|export)\s[^;'"]*?from\s*['"]([^'"]+)['"]|^\s*import\s*['"]([^'"]+)['"]""", re.M)
DYNAMIC = re.compile(r"""\bimport\(\s*['"]([^'"]+)['"]\s*\)""")
bodies = {}
def edges(f, path):
    """(target, lazy) for every import in one module; lazy = an import() of a lazy row's module."""
    if path not in bodies:
        bodies[path] = need(f)
    text = bodies[path].decode('utf-8', 'replace')
    out = [(local(m.group(1) or m.group(2), path), False) for m in STATIC.finditer(text)]
    for m in DYNAMIC.finditer(text):
        t = local(m.group(1), path)
        out.append((t, t[1] in LAZY))
    return out

def closure(roots, skip=frozenset()):
    """Every module reached from roots by eager edges (minus `skip`), in order; and the lazy edges met."""
    seen, order, lazy, todo = set(), [], set(), list(roots)
    while todo:
        f, path = todo.pop()
        if path in seen or path in skip:
            continue
        seen.add(path); order.append(path)
        for t, is_lazy in edges(f, path):
            if is_lazy:
                lazy.add(t)
            else:
                todo.append(t)
    return order, lazy

gz = lambda b: len(gzip.compress(b, 9))
red = []
roots = [local(u, '/room/canvas/index.html') for u in re.findall(r'<script[^>]*\ssrc="([^"]+)"', html)]
first, lazy_edges = closure(roots)
rows = [('/room/canvas/index.html', html.encode()), ('/room/canvas/board.wasm', wasm)] + [(p, bodies[p]) for p in first]
total = 0
print('(a) first frame:')
for p, b in rows:
    total += gz(b)
    print(f'  {gz(b):>6} gz  {len(b):>7} raw  {p}')
print(f'canvas-wire: first frame {total} B gzip over {len(rows)} files (budget {BUDGET}); board.wasm {gz(wasm)} gz / {len(wasm)} raw')
if total > BUDGET:
    red.append(f'the first frame is over budget by {total - BUDGET} B')
for p in sorted(set(first) & set(LAZY)):
    red.append(f'{p} is a lazy module (a baseline row) but the first frame reaches it: pulled into the first frame')

# (b) every module reached only lazily, each with what it adds to the first frame.
print('(b) lazy modules (fetched on a tap):')
done, todo = set(first), sorted(lazy_edges, key=lambda e: e[1])
while todo:
    tf, tp = todo.pop(0)
    if tp in done:
        continue
    done.add(tp)
    mods, more = closure([(tf, tp)], skip=frozenset(first))
    cost = sum(gz(bodies[m]) for m in mods)
    row = LAZY.get(tp)
    pulls = f' (+ {", ".join(mods[1:])})' if len(mods) > 1 else ''
    print(f'  {cost:>6} gz  {tp}{pulls}  budget {row}')
    print(f'canvas-wire: lazy {tp} {cost} B gzip (budget {row})')
    if cost > row:
        red.append(f'lazy {tp} is over its budget by {cost - row} B ({cost} > {row})')
    elif cost * 5 // 4 < row:
        print(f'canvas-wire: the ratchet for {tp} may fall: lower its row to {cost * 5 // 4}')
    todo += sorted((e for e in more if e[1] not in done), key=lambda e: e[1])
for p in sorted(set(LAZY) - done):
    print(f'canvas-wire: note -- baseline row {p} is not loaded at all now; remove the row')
if src != DIGEST:
    red.append(f'board.wasm was built from {src}, the sources are {DIGEST}: run crates/dowiz-canvas/build.sh')
csp = next((l for l in headers.splitlines() if l.strip().startswith('Content-Security-Policy:')), '')
script = re.search(r'script-src([^;]*)', csp)
if not script or "'self'" not in script.group(1) or "'wasm-unsafe-eval'" not in script.group(1):
    red.append("CSP script-src must carry 'self' and 'wasm-unsafe-eval'")
for what, rx in [('inline <style>', r'<style'), ('style= attribute', r'\sstyle='), ('on*= handler', r'\son[a-z]+='),
                 ('inline <script>', r'<script(?![^>]*\ssrc=)[^>]*>')]:
    if re.search(rx, html, re.I):
        red.append(f'index.html has an {what} (blocked by the CSP)')
body = re.search(r'<body[^>]*>(.*)</body>', html, re.S | re.I)
inner = re.sub(r'\s+', '', body.group(1)) if body else '?'
if inner.lower() != '<canvas></canvas>':
    red.append(f'<body> must hold exactly one <canvas>, holds: {inner[:80]}')
for r in red:
    print('canvas-wire: RED --', r)
print('canvas-wire:', 'FAIL' if red else 'PASS')
sys.exit(1 if red else 0)
PY
