#!/bin/sh
# LEARN — EVERY LESSON POINTS AT A CONTROL THAT EXISTS, AND EVERY CONTROL HAS A LESSON.
#
# docs/design/BLUEPRINT-LEARNING-VIDEOS-WIKI-2026-09-24.md §8.4. A lesson step
# names an anchor (`data-tour="<module>.<control>"`); the in-app tour rings it,
# the recorder clicks it, and this counts it. A renamed control breaks all three
# -- here first, before a tour rings nothing or a video records a blank screen.
#
# RED (counted, and the count is ratcheted in learn.baseline):
#   1. an anchor in an app's markup that no lesson step names
#      (sources: data-tour="…" / `tour: '…'` in the app's non-test files, plus
#      docs/learn/anchors-<app>.txt where the app keeps one)
#   2. a lesson step whose anchor is NOT marked `pending` and is not in the
#      app's source any more (a literal '<id>' / "<id>", or a templated
#      `data-tour="nav.${id}"` / `tour: 'state.' + s` whose suffix is a literal
#      in that same file)
#   3. a module of an app -- the console's TABS and More tiles, the room's
#      views, the courier's screens -- that no lesson `covers`
#   4. lessons.json invalid (a language or a field missing) or stale against its
#      YAML: `node tools/learn/build-lessons.mjs --check`
# PRINTED, NOT RED: `pending` anchors still missing (the design-system lanes add
# them), and `pending` anchors that now exist ("drop pending: in <file>").
#   5. (WARN) a lesson whose video was made from an older YAML: the sha256 the
#      recorder stored (workers/api/public/learn/media/manifest.json `source`,
#      written by tools/learn/publish.sh) is not the YAML's sha256 now. The video
#      then shows steps, captions or anchors the lesson no longer has. Re-render:
#      sh tools/learn/all.sh --out DIR --only <id>. Lessons with no video yet are
#      counted, not listed.
#
# `sh tools/gates/learn.sh [ROOT]` -- the proof runs this same code on a scratch copy.
# Exit 1 when the count rises above the baseline, or falls without the baseline
# being lowered in the same commit (the ratchet only falls).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/learn.baseline"

if node "$ROOT/tools/learn/build-lessons.mjs" --check --root "$ROOT" > "${TMPDIR:-/tmp}/learn-build.$$" 2>&1; then built=0; else built=1; fi
buildlog=$(cat "${TMPDIR:-/tmp}/learn-build.$$"); rm -f "${TMPDIR:-/tmp}/learn-build.$$"

found=$(ROOT="$ROOT" python3 - <<'PY'
import json, os, re, glob

R = os.environ['ROOT']
PUB = os.path.join(R, 'workers/api/public')
DIR = {'owner': 'admin', 'waiter': 'room', 'courier': 'courier', 'guest': 'store'}
lessons = json.load(open(os.path.join(PUB, 'learn/lessons.json'), encoding='utf-8'))['lessons']

def files(app):
    out = {}
    for f in sorted(glob.glob(f'{PUB}/{app}/*.js') + glob.glob(f'{PUB}/{app}/*.html')):
        if '.test.' not in f:
            out[os.path.relpath(f, R)] = open(f, encoding='utf-8').read()
    return out
SRC = {role: files(app) for role, app in DIR.items()}

WRITE = re.compile(r"""(?<!\[)data-tour="([a-z][A-Za-z]*(?:\.[A-Za-z]+)+)"|\btour: ?['"]([a-z][A-Za-z]*(?:\.[A-Za-z]+)+)['"]""")
def tree(role):
    got = {}
    for f, s in SRC[role].items():
        for m in WRITE.finditer(s):
            got.setdefault(m.group(1) or m.group(2), f)
    lst = os.path.join(R, f'docs/learn/anchors-{DIR[role]}.txt')
    if os.path.exists(lst):
        for l in open(lst, encoding='utf-8'):
            if l.strip() and not l.startswith('#'):
                a, at = l.split()[:2]
                got.setdefault(a, at)
    return got

def present(role, a):
    for s in SRC[role].values():
        if f"'{a}'" in s or f'"{a}"' in s:
            return True
    pre, suf = a.rsplit('.', 1)
    for s in SRC[role].values():
        if (re.search(r'data-tour="' + re.escape(pre) + r'\.\$\{', s) or re.search(r"""tour: ?['"]""" + re.escape(pre) + r"""\.['"] ?\+""", s)) and f"'{suf}'" in s:
            return True
    return False

def modules(role):
    s = SRC[role]
    if role == 'owner':
        app, more = s.get('workers/api/public/admin/app.js', ''), s.get('workers/api/public/admin/more.js', '')
        tabs = re.findall(r"^\s*\['([a-zA-Z]+)',\s+'[a-z0-9-]+',\s+'tab", app, re.M)
        g = re.search(r'const GROUPS = \[(.*?)\n\];', more, re.S)
        tiles = re.findall(r"\['([a-zA-Z]+)', '[a-z0-9-]+', open", g.group(1)) if g else []
        return tabs + tiles
    if role == 'waiter':
        return sorted(set(re.findall(r"S\.view === '([a-zA-Z]+)'", s.get('workers/api/public/room/app.js', ''))))
    if role == 'courier':
        return re.findall(r'^export (?:const|function) ([a-z][A-Za-z]*)', s.get('workers/api/public/courier/screens.js', ''), re.M)
    return []

red, lines, info = 0, [], []
seen = {'anchors': 0, 'steps': 0, 'modules': 0}
for role in DIR:
    mine = [l for l in lessons if l['role'] == role]
    named = {st['anchor'] for l in mine for st in l['steps'] if st['anchor']}
    seen['anchors'] += len(tree(role))
    for a, at in sorted(tree(role).items()):
        if a not in named:
            red += 1; lines.append(f'  1 {role}: {a} ({at}) is named by no lesson step')
    for l in mine:
        for st in l['steps']:
            a = st['anchor']
            if not a:
                continue
            seen['steps'] += 1
            here = present(role, a)
            if not st['pending'] and not here:
                red += 1; lines.append(f"  2 {role}: {l['id']} step {st['n']} names {a}, which is not in the markup (renamed? mark it pending: yes)")
            elif st['pending'] and here:
                info.append(f"  resolved: {l['id']} step {st['n']} {a} exists now -- drop `pending` in docs/learn/lessons/{role}/{l['id']}.yaml")
            elif st['pending']:
                info.append(f"  pending: {l['id']} step {st['n']} {a}")
    covered = {c for l in mine for c in l['covers']}
    seen['modules'] += len(modules(role))
    for m in modules(role):
        if m not in covered:
            red += 1; lines.append(f'  3 {role}: module {m} is covered by no lesson (add it to a lesson\'s `covers`)')

import hashlib
mf = os.path.join(PUB, 'learn/media/manifest.json')
media = json.load(open(mf, encoding='utf-8')).get('lessons', {}) if os.path.exists(mf) else {}
stale, novideo = 0, 0
for l in lessons:
    y = os.path.join(R, 'docs/learn/lessons', l['role'], l['id'] + '.yaml')
    m = media.get(l['id'])
    if not m:
        novideo += 1
        continue
    now = hashlib.sha256(open(y, 'rb').read()).hexdigest() if os.path.exists(y) else None
    if m.get('source') != now:
        stale += 1
        info.append(f"  5 WARN stale video: {l['id']} was recorded from {str(m.get('source'))[:12]}, the YAML is {str(now)[:12]} now -- re-render: sh tools/learn/all.sh --out DIR --only {l['id']}")
info.append(f"  videos: {len(lessons) - novideo}/{len(lessons)} lesson(s) have one, {stale} stale")
pend = sum(1 for i in info if i.startswith('  pending'))
print(red, pend, seen['anchors'], seen['steps'], seen['modules'])
for l in lines + info:
    print(l)
PY
)
set -- $(printf '%s\n' "$found" | head -1)
n=$(( $1 + built )); pending=$2; measured="$3 anchors, $4 anchored steps, $5 modules"
names=$(printf '%s\n' "$found" | tail -n +2)
[ "$built" -eq 1 ] && names=$(printf '  4 %s\n%s' "$(printf '%s' "$buildlog" | tail -3)" "$names")

if [ ! -f "$BASELINE" ]; then
  printf 'red=%s\n' "$n" > "$BASELINE"
  echo "learn: baseline written at $n"
  exit 0
fi
b=$(awk -F= '/^red=/{print $2}' "$BASELINE")

if [ "$n" -gt "$b" ]; then
  echo "learn: REFUSED — $n lesson/anchor defect(s) (baseline $b):"
  printf '%s\n' "$names" | grep -v '^  pending:' || true
  echo "learn: a control and its lesson moved apart. Fix the anchor or the lesson YAML, then"
  echo "learn: node tools/learn/build-lessons.mjs (docs/learn/README.md)."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "learn: $n (baseline $b) — the ratchet has fallen. Lower it to red=$n in this commit."
  exit 1
fi
printf '%s\n' "$names" | grep -v '^  pending:' || true
# A gate that read nothing would say 0 too: the counts are the proof it looked.
if [ "$3" -eq 0 ] || [ "$4" -eq 0 ] || [ "$5" -eq 0 ]; then
  echo "learn: REFUSED — measured nothing ($measured): a source moved and the gate is blind"
  exit 1
fi
echo "learn: $n defect(s) (baseline $b), $pending anchor(s) pending — checked $measured"
