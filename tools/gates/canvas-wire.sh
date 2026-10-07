#!/bin/sh
# CANVAS WIRE BUDGET (Wave CV7) -- what one canvas surface costs on the wire, and that it is allowed
# to load at all.
#
#   sh tools/gates/canvas-wire.sh [CANVAS_DIR] [HEADERS_FILE]
#
# RED when any of:
#   1. index.html + every module it reaches by static OR dynamic import() (followed transitively,
#      the way the browser fetches them; board.js defers the network stack with import()) + board.wasm is over 60 KB gzip -9 (research §5.3; prototype 12 KB);
#   2. board.wasm.src is not the digest of the crate's sources now (crates/dowiz-canvas/build.sh
#      --digest): the module shipped is not the code in the tree (memory
#      bebop-binary-must-match-source);
#   3. the page could not run under the site's CSP: script-src lacks 'self' or 'wasm-unsafe-eval'
#      (WebAssembly.instantiateStreaming needs it), or index.html carries an inline <style>, an
#      inline <script> without src, a style= attribute or an on*= handler -- all blocked by
#      `style-src 'self'` / `script-src 'self'` (memory dowiz-csp-blocked-every-stylesheet);
#   4. index.html's <body> holds anything but one <canvas> (the static half of dom-count).
# The numbers are printed either way; exit 2 when an input is missing (never a pass).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
DIR="${1:-$ROOT/workers/api/public/room/canvas}"
HEADERS="${2:-$ROOT/workers/api/public/_headers}"
DIGEST=$(sh "$ROOT/crates/dowiz-canvas/build.sh" --digest 2>/dev/null)
[ -n "$DIGEST" ] || { echo "canvas-wire: could not compute the source digest -- NOT a pass"; exit 2; }

DIR="$DIR" PUB="$ROOT/workers/api/public" HEADERS="$HEADERS" DIGEST="$DIGEST" python3 - <<'PY'
import gzip, os, re, sys
D, PUB, H, DIGEST = os.environ['DIR'], os.environ['PUB'], os.environ['HEADERS'], os.environ['DIGEST']
BUDGET = 60 * 1024
def need(p):
    if not os.path.exists(p):
        print(f'canvas-wire: missing {p} -- NOT a pass'); sys.exit(2)
    return open(p, 'rb').read()
html = need(os.path.join(D, 'index.html')).decode()
wasm = need(os.path.join(D, 'board.wasm'))
src = need(os.path.join(D, 'board.wasm.src')).decode().strip()
headers = need(H).decode()

def local(url, base):
    """A served URL -> a file: /room/canvas/* from DIR, the rest from public/."""
    if url.startswith('/'):
        path = url
    else:
        path = os.path.normpath(os.path.join(os.path.dirname(base), url)).replace(os.sep, '/')
    return os.path.join(D, path[len('/room/canvas/'):]) if path.startswith('/room/canvas/') else os.path.join(PUB, path.lstrip('/')), path

IMPORT = re.compile(r'''^\s*(?:import|export)\s[^;'"]*?from\s*['"]([^'"]+)['"]|^\s*import\s*['"]([^'"]+)['"]|\bimport\(\s*['"]([^'"]+)['"]\s*\)''', re.M)
seen, files, todo = set(), [], [m for m in re.findall(r'<script[^>]*\ssrc="([^"]+)"', html)]
todo = [(u, '/room/canvas/index.html') for u in todo]
while todo:
    url, base = todo.pop()
    f, path = local(url, base)
    if path in seen:
        continue
    seen.add(path)
    body = need(f)
    files.append((path, body))
    for m in IMPORT.finditer(body.decode('utf-8', 'replace')):
        todo.append((m.group(1) or m.group(2) or m.group(3), path))

gz = lambda b: len(gzip.compress(b, 9))
rows = [('/room/canvas/index.html', html.encode()), ('/room/canvas/board.wasm', wasm)] + files
total = 0
for p, b in rows:
    total += gz(b)
    print(f'  {gz(b):>6} gz  {len(b):>7} raw  {p}')
print(f'canvas-wire: {total} B gzip over {len(rows)} files (budget {BUDGET}); board.wasm {gz(wasm)} gz / {len(wasm)} raw')
red = []
if total > BUDGET:
    red.append(f'over budget by {total - BUDGET} B')
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
