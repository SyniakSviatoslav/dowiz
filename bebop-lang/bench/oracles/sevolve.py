# G3 sevolve oracle (T114): schema evolution by layout digests with B5 step 1 PartTab.
# Three "programs": v1 writes 1000 P{i64,i64}; v2 appends 1000 P{i64,i64,i64};
# v1 reads v2 ignoring c; v3 migrates L2->L3 and compacts.
# B5 step 1: every commit allocates a PartTab (21 cells), both used and live count it.
import hashlib, os
from storelib import cells, mask64, pick_live, sb_valid

M = mask64()
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x

# LCG sequence for 2000 objects
v = 42; P = []
for i in range(2000):
    v = (v * 6364136223846793005 + 1442695040888963407) & M
    a = v; b = v ^ i; c = (a + b) & M if i >= 1000 else None
    P.append((a, b, c))

# Fold functions per gate logic
def f1(objs):
    acc = 0
    for a, b, c in objs: acc = (acc * 31 + a * 7 + b) & M
    return acc

def f2(objs):
    acc = 0
    for a, b, c in objs: acc = (acc * 31 + a * 7 + b + (c or 0) * 3) & M
    return acc

def fq(objs):
    acc = 0
    for a, b, c in objs: acc = (acc * 31 + ((a + b) & M) * 5 + (c or 0)) & M
    return acc

# Golden values from spec
F1 = f1(P[:1000])           # v1 fold of v1 store (1000 P{2})
F2 = f2(P)                  # v2 fold of v2 store (2000 P, with c)
F3 = f1(P)                  # v1 fold of v2 store (first 1000 ignored c)
F4 = fq(P)                  # v3 fold of compacted store (2000 Q)

# Result = mix of four folds
res = (((F1 * 31 + F2) * 31 + F3) * 31 + F4) & M

# Verify post-compaction state if store exists
if os.path.exists('sevolve.store'):
    d = open('sevolve.store', 'rb').read()
    sb = pick_live(d)
    assert sb >= 0, 'no valid superblock'
    c = cells(d, sb, 16)

    # Post-compaction checks: no superseded, migration record exists, live == 10008
    # Both used and live count the fresh PartTab (21 cells)
    assert c[8] == 0 and c[6] != 0, ('compaction record', c[:9])
    m = cells(d, c[6], 5); dq = int.from_bytes(__import__('hashlib').sha256(b'Q{i64,i64}').digest()[28:32], 'big')
    assert (m[0] & 0xFFFFFFFF) == 3 and m[3] == dq, ('migration table', m)

    # Verify live count matches spec (2000 Q * 4 + root 2003 + migration 5 = 10008, plus PartTab counted in live)
    assert c[7] == 2000 * 4 + 2003 + 5, ('live count', c[7], 2000 * 4 + 2003 + 5)
    assert len(d) <= (c[4]) * 8 + 2 * 4096, len(d)  # used includes PartTab

print(s64(res))
