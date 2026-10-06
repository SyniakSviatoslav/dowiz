#!/bin/sh
# S7c — A FORMAT CHANGE SHIPS ITS MIGRATION LAW, OR IT DOES NOT MERGE (W-TASTE2, 2026-10-06).
#
# R-S7 §E6 (docs/research/2026-10-06-s7-sheaf-engine-arxiv-and-experiments.md): bebop never
# rewrites a log when a record gains a field; the DECODER supplies the default (Spivak's Sigma_F
# done at read time). That is only safe while two laws hold for every change -- Delta o Sigma = id
# on old records, and readers agree, decode(old) == decode(encode(decode(old))) -- and CLAUDE.md
# records what skipping them cost (B5 step 1: five gates, two oracles and a harness model). The
# operator (2026-10-06) agreed this gate MAY block a lane's merge.
#
# WHAT IS A FORMAT, as this gate reads the source (comments stripped):
#   * every stock record kind: the field names its `encode` writes, in order
#     (crates/dowiz-hub/src/stock/codec.rs, `{"k":"<kind>",...}`), as `stock.<kind>`;
#   * every DG7 block schema string (crates/dowiz-hub/src/block/schema.rs), as `block.<name>`.
# THE REGISTER (tools/gates/migration-law.registry), one line per VERSION of a format:
#     <format> <version> <fingerprint> <law test path>
# REFUSED, one hit each:
#   1. a format the source has and the register does not;
#   2. a format whose NEWEST registered version does not match the source (changed, no law shipped);
#   3. a law test the source does not hold as `#[test] fn <last segment>` (crates/dowiz-hub/src,
#      workers/api/src) -- a register naming a test nobody wrote proves nothing;
#   4. two versions of one format naming the same law test (each change brings its own);
#   5. a registered format the source no longer has (a stale register is a lying one).
# It does NOT run the tests: cargo does, and run-all runs cargo. It proves they EXIST and are named.
#
# `sh tools/gates/migration-law.sh [ROOT]` -- ROOT defaults to the repo, so `.prove.sh` can run
# this same code on a scratch copy holding a deliberate defect. Exit 1 on any hit.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
REGISTRY="${MIGRATION_LAW_REGISTRY:-$ROOT/tools/gates/migration-law.registry}"

ROOT="$ROOT" REGISTRY="$REGISTRY" python3 - <<'PY'
import os, re, glob, sys
ROOT, REG = os.environ['ROOT'], os.environ['REGISTRY']
strip = lambda s: re.sub(r'//[^\n]*', '', s)
def read(p):
    with open(os.path.join(ROOT, p), encoding='utf-8') as f:
        return strip(f.read())

found = {}
codec = read('crates/dowiz-hub/src/stock/codec.rs')
a, b = codec.find('pub fn encode'), codec.find('pub fn decode')
enc = codec[a:b] if a >= 0 and b > a else ''
for kind, body in re.findall(r'\{\{"k":"(\w+)"(.*?)\}\}"#', enc, re.S):
    found['stock.' + kind] = ','.join(re.findall(r'"(\w+)":', body))
for name, s in re.findall(r'name:\s*"(\w+)",\s*string:\s*"([^"]+)"', read('crates/dowiz-hub/src/block/schema.rs')):
    found['block.' + name] = s
if len([k for k in found if k.startswith('stock.')]) < 10 or len([k for k in found if k.startswith('block.')]) < 4:
    print(f'migration-law: REFUSED -- read only {len(found)} formats; the source moved, update this gate')
    sys.exit(1)

tests = set()
for d in ('crates/dowiz-hub/src', 'workers/api/src'):
    for f in glob.glob(os.path.join(ROOT, d, '**', '*.rs'), recursive=True):
        src = strip(open(f, encoding='utf-8').read())
        tests.update(re.findall(r'#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+(\w+)', src))

reg = {}
hits = []
for n, line in enumerate(open(REG, encoding='utf-8'), 1):
    line = line.split('#', 1)[0].strip()
    if not line:
        continue
    parts = line.split()
    if len(parts) != 4 or not parts[1].isdigit():
        hits.append(f'register line {n}: want "<format> <version> <fingerprint> <law test>", got {line!r}')
        continue
    fmt, ver, fp, law = parts[0], int(parts[1]), parts[2], parts[3]
    reg.setdefault(fmt, []).append((ver, fp, law))
    if law.split('::')[-1] not in tests:
        hits.append(f'{fmt} v{ver}: law test {law} is not a #[test] fn in the source')
for fmt, cur in sorted(found.items()):
    vs = sorted(reg.get(fmt, []))
    if not vs:
        hits.append(f'{fmt}: a format with no migration law registered ({cur})')
        continue
    if vs[-1][1] != cur:
        hits.append(f'{fmt}: changed to {cur!r}; newest registered v{vs[-1][0]} is {vs[-1][1]!r} -- ship the migration law test and register v{vs[-1][0] + 1}')
    laws = [l for _, _, l in vs]
    if len(set(laws)) != len(laws):
        hits.append(f'{fmt}: two versions name the same law test; each change brings its own')
for fmt in sorted(set(reg) - set(found)):
    hits.append(f'{fmt}: registered, but the source has no such format')

for h in hits:
    print('  REFUSED ' + h)
print(f'migration-law: {len(found)} formats ({sum(1 for k in found if k.startswith("stock."))} stock kinds, '
      f'{sum(1 for k in found if k.startswith("block."))} block schemas), {sum(len(v) for v in reg.values())} registered versions, {len(hits)} refusal(s)')
sys.exit(1 if hits else 0)
PY
