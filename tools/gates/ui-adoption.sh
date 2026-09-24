#!/bin/sh
# UI-ADOPTION — HAND-ROLLED UI THAT A /lib/ui COMPONENT REPLACES, COUNTED.
#
# The design system (`workers/api/public/lib/ui/`, docs/design/DESIGN-SYSTEM.md)
# only pays for itself when surfaces stop building their own buttons, fields,
# toasts, empty states and pills by string. Four surfaces each grew their own
# copy of every one of those; the courier app is migrated, the rest follow one
# lane at a time. This gate is the ratchet those lanes turn: per surface and per
# pattern it counts what is still hand-rolled, and the count may only fall.
#
# WHAT IS COUNTED (in every .js and .html of a surface; lib/ui/, vendor code,
# tests and the gallery are not surfaces):
#   button    a <button> tag with no `ui-` class                -> ui.button / ui.iconButton / ui.chip
#   field     an <input>/<textarea>/<select> with no `ui-` class -> ui.field / ui.inputRow
#   esc       a private HTML escaper (`const esc = s => ...`)    -> ui.esc (ONE escaper)
#   toast     class="toast ..." built by hand                    -> ui.createToaster
#   empty     class="empty ..."                                  -> ui.emptyState
#   skeleton  class="skel..."                                    -> ui.skeleton
#   pill      class="tag|chip|pill|status ..."                   -> ui.badge / ui.chip / ui.status
#
# A regex over markup is a proxy, and it says so: it cannot tell a <button>
# built by `ui.button()` from one written by hand except by the `ui-` class,
# which is exactly the thing a migrated surface carries.
#
# `sh tools/gates/ui-adoption.sh [ROOT]` -- ROOT defaults to the repo, so
# `ui-adoption.prove.sh` can run the same code on a scratch copy.
# Exit 1 when any cell rises above its baseline, or falls without the baseline
# being lowered in the same commit.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/ui-adoption.baseline"

counts=$(ROOT="$ROOT" python3 - <<'PY'
import os, re
PUB = os.path.join(os.environ['ROOT'], 'workers/api/public')
# A surface is a top-level folder a person uses; `store` also owns /app.js.
SURFACES = {'admin': ['admin'], 'room': ['room'], 'courier': ['courier'],
            'store': ['store', 'app.js'], 'kit': ['kit'], 'platform': ['platform'], 'lib': ['lib']}
SKIP_DIRS = {'ui', 'vendor', 'map', 'font', 'node_modules'}
NOT_UI = r'(?![^>]*\bclass=["\'][^"\']*\bui-)'
CLS = r'\bclass=["\'](?:[^"\']*\s)?'
PATTERNS = {
    'button':   re.compile(r'<button\b' + NOT_UI),
    'field':    re.compile(r'<(?:input|textarea|select)\b' + NOT_UI),
    'esc':      re.compile(r'\b(?:const|let|var)\s+esc\s*=\s*\(?\s*\w*\s*\)?\s*=>|\bfunction\s+esc\s*\('),
    'toast':    re.compile(CLS + r'toast\b'),
    'empty':    re.compile(CLS + r'empty\b'),
    'skeleton': re.compile(CLS + r'skel(?:\b|-)'),
    'pill':     re.compile(CLS + r'(?:tag|chip|pill|status)\b(?!-)'),
}
def files(entry):
    p = os.path.join(PUB, entry)
    if os.path.isfile(p):
        yield p
        return
    for dp, dns, fs in os.walk(p):
        dns[:] = [d for d in dns if d not in SKIP_DIRS]
        for f in fs:
            if f.endswith(('.js', '.html')) and not f.endswith(('.test.js', '.min.js')):
                yield os.path.join(dp, f)
for s, entries in SURFACES.items():
    n = dict.fromkeys(PATTERNS, 0)
    for e in entries:
        for f in files(e):
            src = open(f, encoding='utf-8', errors='replace').read()
            for k, rx in PATTERNS.items():
                n[k] += len(rx.findall(src))
    for k, v in n.items():
        print(f'{s}.{k}={v}')
PY
)

if [ ! -f "$BASELINE" ]; then
  printf '%s\n' "$counts" > "$BASELINE"
  echo "ui-adoption: baseline written"
  exit 0
fi
rose=''; fell=''; total=0
for line in $counts; do
  k=${line%%=*}; v=${line#*=}; total=$((total + v))
  b=$(awk -F= -v k="$k" '$1==k{print $2}' "$BASELINE")
  [ -z "$b" ] && b=0
  [ "$v" -gt "$b" ] && rose="$rose $k:$b->$v"
  [ "$v" -lt "$b" ] && fell="$fell $k:$b->$v"
done
if [ -n "$rose" ]; then
  echo "ui-adoption: REFUSED — hand-rolled UI grew:$rose"
  echo "ui-adoption: build it with /lib/ui (docs/design/DESIGN-SYSTEM.md, 'Migration recipe')."
  exit 1
fi
if [ -n "$fell" ]; then
  echo "ui-adoption: the ratchet has fallen:$fell — lower tools/gates/ui-adoption.baseline in this commit."
  exit 1
fi
echo "ui-adoption: $total hand-rolled UI pattern(s) left across all surfaces (baseline held)"
