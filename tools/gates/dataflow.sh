#!/bin/sh
# R3 — AN EDGE CARRIES A DERIVED NODE, NEVER A SOURCE NODE.
#
# THE DEFECT (docs/research/2026-09-27-dag-architecture.md §4, §7 R3). The Free
# plan kills a Worker at 10 ms of CPU, and the kills cluster where the Worker
# pulls a whole SOURCE image across the hop -- the catalogue (538 KB) through
# `load_catalog(`, the order log through `hubstore::load(` -- and evaluates the
# node itself. The object holds those images in memory and can answer the
# derived value (`/fold/menu`, `/fold/orders`, `/fold/venue`) instead.
#
# THE RULE: every `hubstore::load(` and `load_catalog(` call site outside the
# allowlist is counted, and the count may only fall. The allowlist is the
# writers, the export and the nightly: `hubstore.rs` itself (the door and its
# `with_catalog`), `cloud.rs` and `cloud/night.rs` (the nightly copy; W-LOOP moved
# one venue's night into its own file, run in that venue's own alarm), and any function named
# `with_catalog*`, `seed_*` or `export*`. Tests are not request paths.
#
# COMMENTS ARE STRIPPED FIRST, strings kept (a `//` inside "https://" is not a
# comment): a comment that names a call is its epitaph, not a call, and a gate
# that counted it would measure its own history (ROADMAP-2026-09-22 §5).
#
#   sh tools/gates/dataflow.sh [repo-root]    # the prove script passes a scratch copy
set -eu
ROOT=${1:-$(cd "$(dirname "$0")/../.." && pwd)}
BASELINE="$(cd "$(dirname "$0")" && pwd)/dataflow.baseline"

found=$(ROOT="$ROOT" python3 - <<'PY'
import os, re, glob

TOKEN = re.compile(r'r(#*)"(?:.|\n)*?"\1|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])\'|/\*(?:.|\n)*?\*/|//[^\n]*')
def strip(src):
    def keep(m):
        t = m.group(0)
        if t.startswith('//') or t.startswith('/*'):
            return re.sub(r'[^\n]', ' ', t)
        return t
    return TOKEN.sub(keep, src)

CALL = re.compile(r'(?<![\w:])(?:crate::)?hubstore::load\(|(?<!fn )\bload_catalog\(')
FN = re.compile(r'\bfn\s+(\w+)')
ALLOW_FILES = {'hubstore.rs', 'cloud.rs'}
ALLOW_FN = re.compile(r'^(with_catalog\w*|seed_\w*|export\w*)$')

root = os.environ['ROOT']
src_dir = os.path.join(root, 'workers/api/src')
hits = []
for f in sorted(glob.glob(src_dir + '/**/*.rs', recursive=True)):
    rel = os.path.relpath(f, src_dir)
    base = os.path.basename(f)
    if base in ALLOW_FILES and os.path.dirname(rel) == '' or rel == 'cloud/night.rs' or base == 'tests.rs' or '/tests/' in '/' + rel:
        continue
    src = strip(open(f, encoding='utf-8').read())
    for m in CALL.finditer(src):
        fns = [n.group(1) for n in FN.finditer(src, 0, m.start())]
        owner = fns[-1] if fns else '?'
        if ALLOW_FN.match(owner):
            continue
        line = src.count('\n', 0, m.start()) + 1
        hits.append(f'{rel}:{line} in {owner}')
print(len(hits))
for h in hits:
    print('  ' + h)
PY
)
n=$(printf '%s\n' "$found" | head -1)
names=$(printf '%s\n' "$found" | tail -n +2)

if [ ! -f "$BASELINE" ]; then
  printf 'sites=%s\n' "$n" > "$BASELINE"
  echo "dataflow: baseline written at $n"
  exit 0
fi
b=$(awk -F= '/^sites=/{print $2}' "$BASELINE")

if [ "$n" -gt "$b" ]; then
  printf '%s\n' "$names"
  echo "dataflow: REFUSED — $n source-image loads cross the hop (baseline $b). Ask the object for"
  echo "dataflow: the derived value (/fold/menu, /fold/orders, /fold/venue), or allowlist a writer here."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "dataflow: $n (baseline $b) — the ratchet has fallen. Lower it to sites=$n in this commit."
  exit 1
fi
echo "dataflow: $n source-image loads cross the hop (baseline $b)"
