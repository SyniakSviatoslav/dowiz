# Oracle for gate `stm` (T34): Z2 transactions -- odd-sector Grassmann write
# contexts, commit = nilpotency conflict test vs sheaf residual, abort = ctx^2 = 0.
# Monomial form ctx = sign*(2*mask+1); 0 = no context.
M62 = (1 << 62) - 1
def mix(h, x): return ((h * 1000003) + x) & M62
def sg(n): return 1 - 2 * (n & 1)
def pc(x): return bin(x).count("1")
def gmul(tk, r):                       # e_I * e_r
    m, s = abs(tk) >> 1, (-1 if tk < 0 else 1)
    if tk == 0 or m & (1 << r): return 0
    return s * sg(pc(m >> (r + 1))) * (2 * (m | (1 << r)) + 1)
def gdel(tk, r):                       # contraction d_r e_I
    m, s = abs(tk) >> 1, (-1 if tk < 0 else 1)
    if not (m & (1 << r)): return 0
    return s * sg(pc(m & ((1 << r) - 1))) * (2 * (m ^ (1 << r)) + 1)
def gprod(ta, tb):                     # e_I * e_J, generators of J ascending
    mb, sb = abs(tb) >> 1, (-1 if tb < 0 else 1)
    acc = ta
    for j in range(8):
        if (mb >> j) & 1: acc = gmul(acc, j)
    return acc * sb
def gcontract(tk):
    m = abs(tk) >> 1; acc = tk
    for j in range(8):
        if (m >> j) & 1: acc = gdel(acc, j)
    return acc
def shash(store):
    h = 7
    for v in store: h = mix(h, v)
    return h

store = [(i + 1) * 10 for i in range(8)]
snap = [[0]*8, [0]*8]; pend = [[0]*8, [0]*8]
ctx = [0, 0]; win = [0, 0]; preg = [0, 0]
commits = aborts = agree = nil_ok = post_ok = stokes_ok = par_ok = ident = attempts = 0
h = 0; s = 777
for t in range(64):
    s = (s * 1103515245 + 12345) & 2147483647
    xm = 0                                         # value mixed into h this step
    x = s & 1; act = (s >> 1) & 1
    if act == 0:                                   # begin
        if ctx[x] != 0: h = mix(h, 0); continue
        msk = ((s >> 8) & 255) | (1 << ((s >> 16) & 7))
        tk = 1
        for i in range(7, -1, -1):                 # generators multiplied 7..0
            snap[x][i] = store[i]
            pend[x][i] = (((s >> (i * 2)) & 3) + 1) if (msk >> i) & 1 else 0
            if (msk >> i) & 1: tk = gmul(tk, i)
        ctx[x] = tk; win[x] = 0; preg[x] = pc(msk) & 1
    else:                                          # commit attempt
        tk = ctx[x]
        if tk == 0: h = mix(h, 0); continue
        attempts += 1
        msk = abs(tk) >> 1
        conflict = gprod(tk, 2 * win[x] + 1) == 0  # nilpotency = overlap
        res = sum(abs(store[i] - snap[x][i]) for i in range(8) if (msk >> i) & 1)
        agree += (conflict == (res != 0))          # sheaf residual detector agrees
        hb = shash(store)
        if conflict:                               # abort: ctx^2 = 0
            aborts += 1
            nil_ok += (gprod(tk, tk) == 0)
            rm = 0
        else:                                      # commit: writes land
            commits += 1
            before = sum(store)
            for i in range(8): store[i] += pend[x][i]
            post = sum(abs(store[i] - (snap[x][i] + pend[x][i])) for i in range(8) if (msk >> i) & 1)
            post_ok += (post == 0)
            stokes_ok += (sum(store) - before == sum(pend[x]))   # T21: interior change == boundary flux
            win[1 - x] |= msk
            rm = gcontract(tk)                     # -> even scalar +-1
        grade = pc(abs(rm) >> 1) & 1
        preg[x] = grade; par_ok += (grade == 0)
        ctx[x] = 0
        ident += (conflict and shash(store) == hb)
        xm = (rm + 2) + (5 if conflict else 0)
    h = mix(h, xm)
ok = int(agree == attempts and nil_ok == aborts and post_ok == commits
         and stokes_ok == commits and par_ok == attempts and ident == aborts)
h = mix(h, shash(store))
import sys
if len(sys.argv) > 1: print(dict(commits=commits, aborts=aborts, agree=agree, attempts=attempts, nil_ok=nil_ok, post_ok=post_ok, stokes_ok=stokes_ok, par_ok=par_ok, ident=ident), file=sys.stderr)
print((h % 1000000000) * 1000000 + commits * 1000 + aborts * 10 + ok)
