# C3 schain oracle (docs/blueprints/C3-commit-chain-and-heads.md): the commit chain in
# superblock cell 10, the heads table in cell 11 and st_open_at(gen).
# Independent recomputation of the folds from the SPEC -- this file never reads the
# store's own answer back. 1000 generations; generation g's root chain is g nodes
# N{i64 val, ref N prev}, val = LCG from 42; fold(g) = acc*31 + val walked from root_g
# back to root_1 with 64-bit wrap. Result = (h % 1e9) * 100000 + okmask,
# h = mix(mix(mix(0, fold(1)), fold(500)), fold(1000)), mix(h,x) = (h*1000003+x) & 2^62-1,
# okmask = 1023 (all 10 structural checks in bench/vs_rust/std_tests/schain.bp pass).
# Sign is irrelevant to h: a Bebop i64 and the unsigned mod-2^64 value differ by 2^64,
# a multiple of the 2^62 mask.  When schain.store is present after a run its superblock
# is checked against the arena arithmetic derived here, NOT against the program's fold:
#   1000 nodes (2+2 cells) + 2 surviving commits (2+3) + 1 heads table (2+16) = 4028 live,
#   arena_used = 5052, superseded = 0, and cells 10/11 (commit, heads) are both non-zero.
import os, struct
M = (1 << 64) - 1
N = 1000
v = 42
vs = []
for _ in range(N):
    v = (v * 6364136223846793005 + 1442695040888963407) & M
    vs.append(v)
def fold(g):
    acc = 0
    for k in range(g, 0, -1):
        acc = (acc * 31 + vs[k - 1]) & M
    return acc
def mix(h, x):
    return ((h * 1000003) + x) & 4611686018427387903
h = mix(0, fold(1))
h = mix(h, fold(N // 2))
h = mix(h, fold(N))
LIVE = N * 4 + 2 * 5 + 18
if os.path.exists('schain.store'):
    c = struct.unpack('<16q', open('schain.store', 'rb').read(16 * 8))
    assert c[0] == int.from_bytes(b'BEBOPST1', 'little'), c[0]
    assert c[4] == LIVE + 1024 and c[7] == LIVE and c[8] == 0, c[:9]
    assert c[10] != 0 and c[11] != 0, c[9:12]
print((h % 1000000000) * 100000 + 1023)
