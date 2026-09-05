#!/usr/bin/env bash
# roadmap_check.sh (D11-L): fails when the roadmap documents drift.
#  (1) ROADMAP.md <= 300 lines; (2) TASKS.md == regenerated from HISTORY.md headers;
#  (3) every T-id named in ROADMAP.md exists in TASKS.md; (4) the std gate count in
#  TG-DONE row 3 matches bench/vs_rust/std_golden.sh.
cd "$(dirname "$0")/.." || exit 1
fail=0
n=$(wc -l < ROADMAP.md); [ "$n" -le 300 ] || { echo "ROADMAP.md is $n lines (> 300)"; fail=1; }
# (2) compare the file with its own regeneration, not with git HEAD (an uncommitted but
# current TASKS.md used to read as drift, 2026-09-06)
prev=$(mktemp); cp TASKS.md "$prev"; python3 tools/split_roadmap.py --from HISTORY.md > /dev/null
cmp -s TASKS.md "$prev" || { echo "TASKS.md drifted from HISTORY.md headers (regenerated it now; commit the result)"; fail=1; }; rm -f "$prev"
python3 - <<'PY' || fail=1
import re
ids=set()
for m in re.finditer(r'^\| (T\d+)(?:-T(\d+))? \|', open('TASKS.md').read(), re.M):
    a=int(m.group(1)[1:]); b=int(m.group(2)) if m.group(2) else a
    ids |= {'T%d' % i for i in range(a, b + 1)}
missing=[t for t in sorted(set(re.findall(r'\bT\d{1,3}\b', open('ROADMAP.md').read())), key=lambda x:int(x[1:])) if t not in ids]
print('\n'.join(t + ' named in ROADMAP.md but not in TASKS.md' for t in missing)); raise SystemExit(1 if missing else 0)
PY
g=$(grep -c '^gate ' bench/vs_rust/std_golden.sh); grep -q "ok=$g" ROADMAP.md || { echo "std gate count $g not in ROADMAP.md TG-DONE row 3"; fail=1; }
[ $fail = 0 ] && echo "roadmap_check: GREEN" || echo "roadmap_check: RED"
exit $fail
