#!/bin/sh
# W-STRICT (2026-09-29) -- A HANDLER NEVER PARSES ITS BODY WITH `req.json()`.
#
# THE DEFECT THIS GUARDS. workers-rs `Request::json` is `JSON.parse` on the JS
# side followed by `serde_wasm_bindgen`, which reads a struct's DECLARED fields
# off the object and never sees the rest. `#[serde(deny_unknown_fields)]` is
# therefore decoration behind it: measured live on qa-durres, an owner's edit
# with a misspelt field answered `{"ok":true}` and changed nothing, while the
# native unit tests (serde_json) refused the same body and stayed green.
#
# THE RULE. Inside any function that binds a `Request` parameter (`req: Request`,
# `mut req: Request`, `req: &mut Request`, or the route seam's `Call`, which
# handler files import AS `Request`), that parameter's `.json(` is never
# called. Bodies come through `crate::body::parse` / `crate::body::strict`
# (workers/api/src/body.rs): `req.text()` + `serde_json`, the parser the tests
# use, so what a test proves is what the Worker does. The rule covers structs
# that do not deny today too: one gains the attribute later, and nobody
# revisits the call site.
#
# HOW IT COUNTS. Comments and string literals are blanked first, then each
# `fn` body is brace-matched and searched for `<param>.json(` for every
# Request-typed parameter of that function (a `let mut req = req;` rebinding
# keeps the name). A `res.json()` on a Response is not a hit: `res` is not a
# Request parameter.
#
# `sh tools/gates/strict-body.sh [ROOT]` -- ROOT defaults to the repo, so
# `strict-body.prove.sh` runs this same code against a scratch copy holding a
# planted `req.json`. Exit 1 when the count is above the baseline (a ratchet
# down to 0), or below it without the baseline being lowered. Exit 2 when no
# function taking a Request was found: a gate that finds nothing to measure has
# measured nothing.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/strict-body.baseline"

found=$(ROOT="$ROOT" python3 - <<'PY'
import os, re, glob

ROOT = os.environ['ROOT']
PARAM = re.compile(r'(?:\bmut\s+)?(\w+)\s*:\s*(?:&\s*(?:mut\s+)?)?(?:worker::|crate::wire::|wire::)?(?:Request|Call)\b')

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

sites, hits = 0, []
for f in sorted(glob.glob(os.path.join(ROOT, 'workers/api/src/**/*.rs'), recursive=True)):
    if f.endswith('tests.rs') or '/tests/' in f:
        continue
    src = blank(open(f, encoding='utf-8').read())
    for fm in re.finditer(r'\bfn\s+(\w+)\s*(?:<[^{;]*?>)?\s*\(', src):
        # The parameter list: parenthesis-matched from the `(`.
        p0 = fm.end() - 1
        depth, p = 0, p0
        while p < len(src):
            if src[p] == '(': depth += 1
            elif src[p] == ')':
                depth -= 1
                if depth == 0: break
            p += 1
        params = src[p0 + 1:p]
        names = [m.group(1) for m in PARAM.finditer(params)]
        if not names:
            continue
        b = src.find('{', p)
        semi = src.find(';', p)
        if b < 0 or (0 <= semi < b):
            continue  # a trait signature with no body
        e = match_brace(src, b)
        body = src[b:e]
        sites += 1
        for name in names:
            for x in re.finditer(r'\b' + re.escape(name) + r'\s*\.\s*json\s*(?:::<[^>]*>\s*)?\(', body):
                line = src[:b + x.start()].count('\n') + 1
                hits.append(f"{os.path.relpath(f, ROOT)}:{line} {fm.group(1)}: {name}.json() -- use crate::body::parse(&mut {name})")

print(sites)
print(len(hits))
for h in hits:
    print('  ' + h)
PY
)
sites=$(printf '%s\n' "$found" | sed -n 1p)
n=$(printf '%s\n' "$found" | sed -n 2p)
names=$(printf '%s\n' "$found" | tail -n +3)

if [ -z "$sites" ] || [ "$sites" -eq 0 ]; then
  echo "strict-body: found no function taking a Request under $ROOT/workers/api/src -- the parse broke, not the feature"
  exit 2
fi
[ -f "$BASELINE" ] || { echo "strict-body: no baseline at $BASELINE"; exit 2; }
b=$(awk -F= '/^req_json_sites=/{print $2}' "$BASELINE")

if [ "$n" -gt "$b" ]; then
  echo "strict-body: REFUSED -- $n handler(s) parse a body with req.json() (baseline $b, $sites functions take a Request):"
  printf '%s\n' "$names"
  echo "strict-body: req.json() reads only the declared fields, so deny_unknown_fields never fires in production;"
  echo "strict-body: parse with crate::body::parse(&mut req) (same worker::Error) or crate::body::strict (a 400)."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "strict-body: $n (baseline $b) -- the ratchet has fallen. Lower it to req_json_sites=$n in this commit."
  exit 1
fi
echo "strict-body: $n req.json() site(s) across $sites Request-taking function(s) (baseline $b)"
