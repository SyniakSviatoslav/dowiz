#!/usr/bin/env python3
"""oracles2/stm -- gate `stm` (T34).  PROSE (ROADMAP T34 + stm.bp first comment): the
store = 8 cells (EVEN sector); a transaction's uncommitted writes = Grassmann monomial
ctx = sign*(2*mask+1) over the touched-node generators e_0..e_7 (O0 trigger = ctx!=0,
O1 mask, O2 parity = |mask| mod 2); two slots interleave under an LCG schedule; each
slot keeps `win` = union of contexts committed by the other slot since it began.
Commit: ctx * e_win == 0 (overlap) EXACTLY when the residual sum |store - snapshot|
over the touched nodes is nonzero (monotone writes) -- both detectors counted for
agreement.  No conflict: store += pend, post-residual 0, Stokes: whole-store change ==
sum(pend), ctx contracted by all its generators -> scalar +-1 (parity 0).  Conflict:
abort = ctx*ctx = 0; store hash bit-identical before/after.
Fold = (h % 1e9)*1e6 + commits*1000 + aborts*10 + all_ok, h = mix of every commit
scalar, abort and final store.
NOT IN ANY PROSE: the LCG, the schedule (begin/write/commit probabilities, writes per
txn, write values), the mix, the store hash, the total step count.  Parameters."""
import os
M62 = (1 << 62) - 1
def mix(h, x): return (h * 1000003 + x) & M62
def lcg(s): return (s * 1103515245 + 12345) & 2147483647
STEPS = int(os.environ.get("STEPS", "64")); s = int(os.environ.get("SEED", "99"))
def tok_of(sign, mask): return sign * (2 * mask + 1)
def parts(tok): return (1 if tok > 0 else -1), (abs(tok) - 1) // 2
def mul_gen(tok, r):
    if tok == 0: return 0
    sg, mask = parts(tok)
    if mask >> r & 1: return 0
    return tok_of(sg * (-1) ** bin(mask >> (r + 1)).count("1"), mask | 1 << r)
def mul(a, b):
    if a == 0 or b == 0: return 0
    sb, mb = parts(b); out = a
    for r in range(8):
        if mb >> r & 1: out = mul_gen(out, r)
    return out * sb if out else 0
def contract_all(tok):   # d_{r_k} ... d_{r_1} tok, lowest generator first -> scalar +-1
    sg, mask = parts(tok); out = tok
    for r in range(8):
        if mask >> r & 1:
            s2, m2 = parts(out); inv = bin(m2 & ((1 << r) - 1)).count("1")
            out = tok_of(s2 * (-1) ** inv, m2 & ~(1 << r))
    return out
def store_hash(st):
    h = 0
    for c in st: h = mix(h, c)
    return h
store = [0] * 8
slot = [None, None]            # {ctx, snap, pend, win}
h = 0; commits = aborts = 0; all_ok = 1
for _ in range(STEPS):
    s = lcg(s); k = (s >> 8) % 2; s = lcg(s); act = (s >> 8) % 3
    t = slot[k]
    if t is None:
        slot[k] = {"ctx": 1, "snap": list(store), "pend": [0] * 8, "win": 0}
        continue
    if act < 2:                # write: touch node n with +v (monotone)
        s = lcg(s); n = (s >> 8) % 8; s = lcg(s); v = 1 + (s >> 8) % 9
        nc = mul_gen(t["ctx"], n)
        if nc != 0: t["ctx"] = nc
        t["pend"][n] += v
    else:                      # commit
        sg, mask = parts(t["ctx"])
        residual = sum(abs(store[i] - t["snap"][i]) for i in range(8) if mask >> i & 1)
        nil = mul(t["ctx"], tok_of(1, t["win"])) == 0 if t["win"] else False
        if (residual != 0) != nil: all_ok = 0
        if mask == 0:          # empty transaction: nothing to commit, drop it
            slot[k] = None; continue
        if nil:
            before = store_hash(store); assert mul(t["ctx"], t["ctx"]) == 0
            if store_hash(store) != before: all_ok = 0
            aborts += 1; h = mix(h, 0)
        else:
            tot0 = sum(store)
            for i in range(8): store[i] += t["pend"][i]
            if sum(store) - tot0 != sum(t["pend"]): all_ok = 0
            if sum(abs(store[i] - (t["snap"][i] + t["pend"][i])) for i in range(8) if mask >> i & 1): all_ok = 0
            scal = contract_all(t["ctx"])
            if abs(scal) != 1: all_ok = 0
            commits += 1; h = mix(h, scal)
            o = slot[1 - k]
            if o is not None: o["win"] |= mask
        slot[k] = None
h = mix(h, store_hash(store))
print("store", store, "commits", commits, "aborts", aborts, "all_ok", all_ok, "h", h)
print((h % 1000000000) * 1000000 + commits * 1000 + aborts * 10 + all_ok)
