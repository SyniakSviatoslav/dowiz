#!/usr/bin/env python3
"""oracles2/mvcc -- gate `mvcc` (T33).  PROSE (ROADMAP T33 + mvcc.bp first comment):
CoW versions in a bump arena, 3 cells per record (value, prev, liveness token);
restriction map old->new x -> 3x+t+1 mod 2^16 (t = step); liveness = Grassmann
monomial over 4 reader generators, tok = sign*(2*mask+1), tok 1 = no readers, tok 0 =
collapsed; acquire = tok*e_r (0 if e_r already present); release = reader's own token
squared -> 0 and the contraction d_r on the version; a version collapses (3 cells
freed, exact accounting) exactly when superseded AND its liveness is the bare scalar,
checked only at the two events; every read compares the version cell with the
snapshot taken at acquire and checks not-collapsed; 64 LCG-scheduled steps.
Fold = (h % 1e9)*1e5 + collapsed*100 + surv*4 + acct*2 + reads_ok,
h = mixing hash of every token after acquire/release.
NOT IN ANY PROSE: which LCG and how it picks (update|acquire|release|read, reader r),
the mixing hash, the initial value, the Grassmann sign convention for e_r ordering,
what `surv` and `acct` test exactly.  Parameters below; the token algebra is exact."""
import os
M62 = (1 << 62) - 1
def mix(h, x): return (h * 1000003 + x) & M62      # parameter: T66-style mix
def lcg(s): return (s * 1103515245 + 12345) & 2147483647
STEPS = 64; SEED = int(os.environ.get("SEED", "7"))
def tok_of(sign, mask): return sign * (2 * mask + 1)
def parts(tok): return (1 if tok > 0 else -1), (abs(tok) - 1) // 2
def mul_gen(tok, r):     # tok * e_r, generators ordered e_0 < e_1 < e_2 < e_3
    if tok == 0: return 0
    sg, mask = parts(tok)
    if mask >> r & 1: return 0
    inv = bin(mask >> (r + 1)).count("1")   # generators after e_r must hop over it
    return tok_of(sg * (-1) ** inv, mask | 1 << r)
def contract(tok, r):    # d_r tok: remove e_r (0 if absent)
    if tok == 0: return 0
    sg, mask = parts(tok)
    if not mask >> r & 1: return 0
    inv = bin(mask & ((1 << r) - 1)).count("1")
    return tok_of(sg * (-1) ** inv, mask & ~(1 << r))
# records: list of [value, prev, tok, collapsed]; arena accounting in cells
recs = [[1, -1, 1, 0]]; cur = 0; freed = 0; allocated = 3
reader = [None] * 4          # per reader: (version, snapshot value) or None
h = 0; collapsed = 0; reads_ok = 1; s = SEED
def try_collapse(v):
    global freed, collapsed
    r = recs[v]
    if not r[3] and v != cur and r[2] == 1:
        r[3] = 1; freed += 3; collapsed += 1
for t in range(STEPS):
    s = lcg(s); op = (s >> 8) % 4; s = lcg(s); r = (s >> 8) % 4
    if op == 0:                      # update: new version, restriction map edge
        old = cur; recs.append([(3 * recs[old][0] + t + 1) % 65536, old, 1, 0]); allocated += 3
        cur = len(recs) - 1; try_collapse(old)
    elif op == 1 and reader[r] is None:   # acquire on the current version
        nt = mul_gen(recs[cur][2], r)
        if nt != 0: recs[cur][2] = nt; reader[r] = (cur, recs[cur][0])
        h = mix(h, nt)
    elif op == 2 and reader[r] is not None:  # release: own token squared -> 0, contract
        v, _ = reader[r]; reader[r] = None
        own = tok_of(1, 1 << r); assert mul_gen(own, r) == 0
        recs[v][2] = contract(recs[v][2], r); h = mix(h, recs[v][2]); try_collapse(v)
    elif op == 3 and reader[r] is not None:  # read against the snapshot
        v, snap = reader[r]
        if recs[v][0] != snap or recs[v][3]: reads_ok = 0
for v, rr in enumerate(recs): h = mix(h, rr[2])
surv = int(all(not recs[v][3] for rd in reader if rd for v in [rd[0]]))
acct = int(freed == 3 * collapsed and allocated == 3 * len(recs))
print("versions", len(recs), "collapsed", collapsed, "surv", surv, "acct", acct, "reads_ok", reads_ok, "h", h)
print((h % 1000000000) * 100000 + collapsed * 100 + surv * 4 + acct * 2 + reads_ok)
