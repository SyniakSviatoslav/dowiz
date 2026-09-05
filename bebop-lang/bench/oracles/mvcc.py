# Oracle for gate `mvcc` (T33): CoW versions + Grassmann reader tokens.
# Monomial form tok = sign*(2*mask+1); 1 = scalar (no readers); 0 = collapsed.
M62 = (1 << 62) - 1
def mix(h, x): return ((h * 1000003) + x) & M62
def sg(n): return 1 - 2 * (n & 1)
def pc(x): return bin(x).count("1")
def gmul(tk, r):                       # e_I * e_r
    m, s = abs(tk) >> 1, (-1 if tk < 0 else 1)
    if tk == 0 or m & (1 << r): return 0          # nilpotent
    return s * sg(pc(m >> (r + 1))) * (2 * (m | (1 << r)) + 1)
def gdel(tk, r):                       # contraction d_r e_I
    m, s = abs(tk) >> 1, (-1 if tk < 0 else 1)
    if not (m & (1 << r)): return 0
    return s * sg(pc(m & ((1 << r) - 1))) * (2 * (m ^ (1 << r)) + 1)

val = [0]*512; prev = [0]*512; tok = [0]*512; kof = [0]*512
head = [0]*4; rver = [-1]*4; rsnap = [0]*4
freed = collapsed = nil_ok = reads_ok = reads = 0; h = 0
for k in range(4):
    val[k] = (k+1)*100; prev[k] = -1; tok[k] = 1; kof[k] = k; head[k] = k
cnt = 4; s = 12345
def collapse(v):
    global freed, collapsed
    tok[v] = 0; val[v] = 0; prev[v] = -2; freed += 3; collapsed += 1
for t in range(64):
    s = (s * 1103515245 + 12345) & 2147483647
    op = (s >> 4) & 3; r = (s >> 8) & 3; k = (s >> 12) & 3
    xa = xr = 0                        # acquire / release token mixed in this step
    if op < 2:                         # update: new record, edge to old
        old = head[k]; nv = cnt; cnt += 1
        val[nv] = (val[old]*3 + t + 1) & 65535; prev[nv] = old; tok[nv] = 1; kof[nv] = k
        head[k] = nv
        if abs(tok[old]) == 1: collapse(old)
    elif op == 2 and rver[r] < 0:      # acquire
        v = head[k]; nt = gmul(tok[v], r); tok[v] = nt
        rver[r] = v; rsnap[r] = val[v]; xa = nt + 64
    elif op == 3 and rver[r] >= 0:     # release
        v = rver[r]
        if gmul(2*(1 << r)+1, r) == 0: nil_ok += 1   # own token squared
        nt = gdel(tok[v], r); tok[v] = nt; rver[r] = -1
        if head[kof[v]] != v and abs(nt) == 1: collapse(v)
        xr = nt + 64
    h = mix(mix(h, xa), xr)
    for rr in range(4):                # every live reader reads its version
        if rver[rr] >= 0:
            reads += 1
            v = rver[rr]
            if val[v] == rsnap[rr] and prev[v] != -2: reads_ok += 1
surv = 1; nfreed = 0
for v in range(cnt):
    sup = head[kof[v]] != v; hastok = abs(tok[v]) > 1; fr = prev[v] == -2
    surv &= (fr == (sup and not hastok)); nfreed += fr
acct = freed == nfreed*3; rok = reads_ok == reads
h = mix(mix(mix(h, reads), nil_ok), cnt)
print((h % 1000000000) * 100000 + collapsed*100 + surv*4 + acct*2 + rok + (7 + 4 * 16) * 10**15)  # T123: absv(-7) + pc4(15)*16
