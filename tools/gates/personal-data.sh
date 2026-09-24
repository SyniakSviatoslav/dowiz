#!/bin/sh
# P1 — EVERY PLACE THAT CAN HOLD A PERSON HAS A ROW.
#
# BLUEPRINT-GDPR-AND-MCP-2026-09-24 §4.1. "Erasure reaches every store", "the
# notice lists every recipient", "nothing else is kept on the device" are
# claims about the WHOLE tree, and a claim about the whole tree is only true
# on the day somebody checked it by hand. This makes it a check that runs on
# every commit, against the table in `workers/api/src/privacy/registry*.rs`:
#
#   1. every `const IMAGE_*` / `*_IMAGE` / `IMAGE` / `K_*` / `KIND` string in
#      `workers/api/src`, and every image name in `platform_store.rs`, must be
#      an `image:` or one of the `kinds:` of a registry row (a row that holds
#      nothing says so with `NotPersonal`);
#   2. every `https://` / `wss://` host in `workers/api/src`, and every host in
#      `public/_headers`' content policy, must be a `Host` row (a processor or
#      `NotARecipient` with its reason);
#   3. every `dw_*` / `dowiz.*` browser-storage key literal under
#      `workers/api/public` must be a `BROWSER` row (or start with a prefix row).
#
# COMMENTS AND TEST CODE ARE STRIPPED FIRST (ROADMAP-2026-09-22 §5): a key in a
# doc comment is not a store, and `https://evil.example` in a test is not a
# recipient. `#[cfg(test)]` blocks are removed by brace matching, and files
# named `tests.rs` or `*.test.mjs` are not read at all.
#
# `sh tools/gates/personal-data.sh [ROOT]` — ROOT defaults to the repo, so
# `personal-data.prove.sh` can run this same code on a scratch copy. Prints
# `personal-data N`; exit 1 when N rises above the baseline.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/personal-data.baseline"

found=$(ROOT="$ROOT" python3 - <<'PY'
import os, re, glob

ROOT = os.environ['ROOT']
SRC = os.path.join(ROOT, 'workers/api/src')
PUB = os.path.join(ROOT, 'workers/api/public')

def strip(src, rust):
    """Comments out, string contents kept: a `//` inside a string is a URL."""
    out, i, n = [], 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith('//', i):
            j = src.find('\n', i); i = n if j < 0 else j; continue
        if src.startswith('/*', i):
            j = src.find('*/', i + 2); i = n if j < 0 else j + 2; continue
        if c == '"' or (not rust and c in "'`"):
            q, j = c, i + 1
            while j < n and src[j] != q:
                j += 2 if src[j] == '\\' else 1
            out.append(src[i:j + 1]); i = j + 1; continue
        if rust and c == "'" and i + 2 < n and (src[i + 2] == "'" or (src[i + 1] == '\\' and i + 3 < n and src[i + 3] == "'")):
            j = src.find("'", i + 1 if src[i + 1] != '\\' else i + 3)
            out.append(src[i:j + 1]); i = j + 1; continue
        out.append(c); i += 1
    return ''.join(out)

def no_tests(s):
    out, i = [], 0
    while True:
        j = s.find('#[cfg(test)]', i)
        if j < 0:
            out.append(s[i:]); break
        out.append(s[i:j])
        k, semi = s.find('{', j), s.find(';', j)
        if k < 0 or (0 <= semi < k):
            i = (semi + 1) if semi >= 0 else len(s); continue
        depth, m = 0, k
        while m < len(s):
            if s[m] == '{': depth += 1
            elif s[m] == '}':
                depth -= 1
                if depth == 0: break
            m += 1
        i = m + 1
    return ''.join(out)

reg_files = [os.path.join(SRC, 'privacy/registry.rs')] + sorted(glob.glob(os.path.join(SRC, 'privacy/registry/*.rs')))
reg = ''.join(strip(open(f, encoding='utf-8').read(), True) for f in reg_files if os.path.exists(f) and not f.endswith('tests.rs'))
images = set(re.findall(r'\bimage:\s*"([^"]+)"', reg))
kinds = set()
for block in re.findall(r'\bkinds:\s*&\[([^\]]*)\]', reg):
    kinds |= set(re.findall(r'"([^"]+)"', block))
hosts = set(re.findall(r'\bHost\s*\{\s*host:\s*"([^"]+)"', reg))
keys = set(re.findall(r'\bkey\(\s*"([^"]+)"', reg))
prefixes = set(re.findall(r'\bprefix\(\s*"([^"]+)"', reg))

hits = []
CONST = re.compile(r'\bconst\s+([A-Z][A-Z0-9_]*)\s*:\s*&(?:\'static\s+)?str\s*=\s*"([^"]*)"')
NAME = re.compile(r'^(IMAGE\w*|\w+_IMAGE|K_\w+|KIND)$')
HOST = re.compile(r'(?:https|wss)://([A-Za-z0-9.*{}_-]+)')
for f in sorted(glob.glob(os.path.join(SRC, '**/*.rs'), recursive=True)):
    rel = os.path.relpath(f, ROOT)
    if f.endswith('tests.rs') or '/privacy/registry' in f:
        continue
    s = no_tests(strip(open(f, encoding='utf-8').read(), True))
    for m in CONST.finditer(s):
        name, val = m.group(1), m.group(2)
        is_image = NAME.match(name) or (f.endswith('/platform_store.rs') and name != 'PLATFORM')
        if is_image and val not in images and val not in kinds:
            hits.append(f'store  "{val}"  ({name} in {rel}) has no registry row')
    for m in HOST.finditer(s):
        if m.group(1) not in hosts:
            hits.append(f'host   {m.group(1)}  ({rel}) is not in registry HOSTS')

hdr = os.path.join(PUB, '_headers')
if os.path.exists(hdr):
    for line in open(hdr, encoding='utf-8'):
        if 'content-security-policy' in line.lower():
            for h in HOST.findall(line):
                if h not in hosts:
                    hits.append(f'host   {h}  (public/_headers CSP) is not in registry HOSTS')

KEY = re.compile(r'[\'"`](dw_[A-Za-z0-9_]*|dowiz\.[A-Za-z0-9_.-]*)')
seen = set()
for f in sorted(glob.glob(os.path.join(PUB, '**/*.js'), recursive=True) + glob.glob(os.path.join(PUB, '**/*.html'), recursive=True)):
    if '.test.' in f:
        continue
    s = strip(open(f, encoding='utf-8').read(), False)
    for k in KEY.findall(s):
        ok = k in keys or k in prefixes or any(k.startswith(p) for p in prefixes)
        if not ok and k not in seen:
            seen.add(k)
            hits.append(f'device "{k}"  ({os.path.relpath(f, ROOT)}) is not in registry BROWSER')

print(len(hits))
for h in hits:
    print('  ' + h)
PY
)
n=$(printf '%s\n' "$found" | head -1)
names=$(printf '%s\n' "$found" | tail -n +2)

if [ ! -f "$BASELINE" ]; then
  printf 'unaccounted=%s\n' "$n" > "$BASELINE"
  echo "personal-data: baseline written at $n"
fi
b=$(awk -F= '/^unaccounted=/{print $2}' "$BASELINE")
echo "personal-data $n"
if [ "$n" -gt "$b" ]; then
  echo "personal-data: REFUSED — $n place(s) that can hold a person have no registry row (baseline $b):"
  printf '%s\n' "$names"
  echo "personal-data: add a row to workers/api/src/privacy/registry (a store, a Host or a BROWSER key)."
  echo "personal-data: the privacy notice and the processor register are read from those rows."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "personal-data: $n (baseline $b) — the ratchet has fallen. Lower it to unaccounted=$n in this commit."
  exit 1
fi
[ -n "$names" ] && printf '%s\n' "$names"
exit 0
