# msuper: 4 branch hypervectors (16 cells) -> Gram matrix G (fp 32.32) -> Hotelling power iteration
# topk(k=2, iters=32, i64 fixed-point) -> dominant eigenvector sign pattern {+,+,-,-} and gap
# (ev0-ev1)>>22; readout = argmax <S,H_b> with S = H0+H1-H2-H3.
# fold = collapse_ok*10^12 + read_ok*10^11 + decisive*10^10 + gap*10^5 + win
M = (1 << 64) - 1
def s(x):
    x &= M
    return x if x < (1 << 63) else x - (1 << 64)
def lsr(x, n):  # bebop >> : logical shift on the 64-bit pattern
    return (x & M) >> n
def fp_mul(a, b):
    aa, ab = abs(a), abs(b)
    a1, b1 = aa >> 32, ab >> 32
    a0, b0 = aa - (a1 << 32), ab - (b1 << 32)
    hi = a1 * b1; mid = a1 * b0 + a0 * b1
    ah, bh = a0 >> 16, b0 >> 16
    al, bl = a0 - (ah << 16), b0 - (bh << 16)
    low = ah * bh + ((ah * bl + al * bh + (al * bl >> 16)) >> 16)
    p = s((hi << 32) + mid + low)
    return s(-p) if (a < 0) != (b < 0) else p
def isqrt(v):
    rem = root = 0
    for i in range(31):
        rem = (rem << 2) + (lsr(v, 60 - 2 * i) & 3)
        tst = (root << 2) + 1
        take = 1 if tst <= rem else 0
        root = (root << 1) + take
        rem -= take * tst
    return root
def lcg(r):
    return s(s(r * 6364136223846793005) + 1442695040888963407)
def spmv(rp, ci, vv, n, x):
    out = [0] * n
    for i in range(n):
        for k in range(rp[i], rp[i + 1]):
            out[ci[k]] = s(out[ci[k]] + fp_mul(vv[k], x[i]))
    return out
def normalize(x, n):
    ss = sum((abs(t) >> 8) ** 2 for t in x[:n])
    nrm = isqrt(ss) or 1
    r = (1 << 56) // nrm
    for i in range(n): x[i] = fp_mul(x[i], r)
def deflate(vec, evecs, found, n):
    for om in range(found):
        proj = 0
        for j in range(n): proj = s(proj + fp_mul(evecs[om * n + j], vec[j]))
        for j in range(n): vec[j] = s(vec[j] - fp_mul(proj, evecs[om * n + j]))
def topk(rp, ci, vv, n, k, iters):
    evals, evecs, found = [], [0] * (k * n), 0
    for m in range(k):
        rng = s(-7046029254386353131); x = [0] * n
        for j in range(n):
            rng = lcg(rng); frac = lsr(rng, 31)
            x[j] = s(frac + frac - 4294967296)
        normalize(x, n); deflate(x, evecs, found, n); normalize(x, n)
        for it in range(iters):
            ax = spmv(rp, ci, vv, n, x); deflate(ax, evecs, found, n); normalize(ax, n); x = ax
        tmp = spmv(rp, ci, vv, n, x); deflate(tmp, evecs, found, n)
        lam = 0
        for j in range(n): lam = s(lam + fp_mul(x[j], tmp[j]))
        deflate(x, evecs, found, n); normalize(x, n)
        fpos = next((t for t in x if abs(t) > 65536), 0)
        for j in range(n): evecs[found * n + j] = -x[j] if fpos < 0 else x[j]
        evals.append(lam); found += 1
    # stable sort by descending |lambda| (insertion sort in the gate)
    order = sorted(range(found), key=lambda i: -abs(evals[i]))
    return [evals[i] for i in order], [evecs[i * n + j] for i in order for j in range(n)]
def csr_from_dense(a, n):
    rp, ci, vv = [0] * (n + 1), [], []
    for i in range(n):
        rp[i] = len(ci)
        for j in range(n):
            if a[i * n + j]: ci.append(j); vv.append(a[i * n + j])
    rp[n] = len(ci)
    return rp, ci, vv
one = 1 << 32
h = [[0] * 16 for _ in range(4)]
for i in range(16):
    h0 = 1 if bin(i).count("1") % 2 == 0 else -1
    f01 = -1 if i in (0, 1) else 1
    h1 = h0 * f01
    h2 = -h0 * (-1 if i == 2 else 1)
    h3 = -h1 * (-1 if i == 3 else 1)
    h[0][i], h[1][i], h[2][i], h[3][i] = h0, h1, h2, h3
svec = [h[0][i] + h[1][i] - h[2][i] - h[3][i] for i in range(16)]
g = [sum(h[i][k] * h[j][k] for k in range(16)) * one for i in range(4) for j in range(4)]
rp, ci, vv = csr_from_dense(g, 4)
ev, ec = topk(rp, ci, vv, 4, 2, 32)
gap = lsr(s(ev[0] - ev[1]), 22)
decisive = 1 if gap >= 100 else 0
collapse_ok = 1 if all((1 if ec[b] > 0 else -1) == (1 if b < 2 else -1) for b in range(4)) else 0
scores = [sum(svec[k] * h[b][k] for k in range(16)) for b in range(4)]
win = scores.index(max(scores))
read_ok = 1 if win < 2 else 0
print(collapse_ok * 10**12 + read_ok * 10**11 + decisive * 10**10 + gap * 10**5 + win)
