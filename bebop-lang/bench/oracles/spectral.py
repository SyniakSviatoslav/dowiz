# spectral: fixed-point (2^32) power method + Hotelling deflation on B6_bridge CSR, k=3, 32 iters;
# fold = sum |lambda_fp - lambda_golden| over golden.txt "B6_bridge ... topk" vals_fp32 (i64 semantics mirrored).
import os, re
M = (1 << 64) - 1
def wrap(v):
    v &= M
    return v - (1 << 64) if v >> 63 else v
def shr(v, s):            # bebop >> : logical on the 64-bit pattern
    return (v & M) >> s
def div(a, b):            # bebop / : truncating
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q

def fp_mul(a, b):
    aa, ab = abs(a), abs(b)
    a1, b1 = shr(aa, 32), shr(ab, 32)
    a0, b0 = aa - (a1 << 32), ab - (b1 << 32)
    hi = a1 * b1
    mid = a1 * b0 + a0 * b1
    ah, bh = shr(a0, 16), shr(b0, 16)
    al, bl = a0 - (ah << 16), b0 - (bh << 16)
    ll_h, ll_m, ll_l = ah * bh, ah * bl + al * bh, al * bl
    low = ll_h + shr(ll_m + shr(ll_l, 16), 16)
    p = wrap((hi << 32) + mid + low)
    return wrap(-p) if (a < 0) != (b < 0) else p

def isqrt(s):
    rem = root = 0
    for i in range(31):
        sh = 60 - i * 2
        rem = (rem << 2) + (shr(s, sh) & 3)
        tst = (root << 2) + 1
        take = 1 if tst <= rem else 0
        root = (root << 1) + take
        rem -= take * tst
    return root

def lcg_next(r):
    return wrap(wrap(r * 6364136223846793005) + 1442695040888963407)

def spmv(rp, ci, vv, n, x):
    out = [0] * n
    for i in range(n):
        for k in range(rp[i], rp[i + 1]):
            out[ci[k]] = wrap(out[ci[k]] + fp_mul(vv[k], x[i]))
    return out

def normalize(x, n):
    ss = 0
    for i in range(n):
        t = shr(abs(x[i]), 8)
        ss += t * t
    nrm = isqrt(ss) or 1
    r = div(72057594037927936, nrm)
    for i in range(n):
        x[i] = fp_mul(x[i], r)

def deflate(v, n, evecs, found):
    for om in range(found):
        e = evecs[om]
        proj = 0
        for j in range(n):
            proj = wrap(proj + fp_mul(e[j], v[j]))
        for j in range(n):
            v[j] = wrap(v[j] - fp_mul(proj, e[j]))

def topk(rp, ci, vv, n, k, iters):
    evals, evecs = [], []
    for _ in range(k):
        rng = -7046029254386353131
        x = [0] * n
        for j in range(n):
            rng = lcg_next(rng)
            frac = shr(shr(rng, 11), 20)
            x[j] = frac + frac - 4294967296
        normalize(x, n)
        deflate(x, n, evecs, len(evecs))
        normalize(x, n)
        for _ in range(iters):
            ax = spmv(rp, ci, vv, n, x)
            deflate(ax, n, evecs, len(evecs))
            normalize(ax, n)
            x = ax
        tmp = spmv(rp, ci, vv, n, x)
        deflate(tmp, n, evecs, len(evecs))
        lam = 0
        for j in range(n):
            lam = wrap(lam + fp_mul(x[j], tmp[j]))
        deflate(x, n, evecs, len(evecs))
        normalize(x, n)
        fpos = 0
        for j in range(n):
            if fpos == 0 and abs(x[j]) > 65536:
                fpos = x[j]
        if fpos < 0:
            x = [-v for v in x]
        evecs.append(x)
        evals.append(lam)
    # stable insertion sort, descending |lambda|
    for i in range(1, len(evals)):
        key, kr = evals[i], evecs[i]
        j = i - 1
        while j >= 0 and abs(evals[j]) < abs(key):
            evals[j + 1], evecs[j + 1] = evals[j], evecs[j]
            j -= 1
        evals[j + 1], evecs[j + 1] = key, kr
    return evals

golden = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'vs_rust', 'spectral_golden', 'golden.txt')
txt = open(golden).read()
sec = txt[txt.index('== B6_bridge n=6 nnz=14 k=3 iters=32'):]
gvals = [int(t) for t in re.search(r'vals_fp32: ([-\d ]+)', sec).group(1).split()]
csr = txt[txt.index('== csr B6 n=6 nnz=14'):]
rp = [int(t) for t in re.search(r'row_ptr: ([\d ]+)', csr).group(1).split()]
ci = [int(t) for t in re.search(r'col_idx: ([\d ]+)', csr).group(1).split()]
vv = [int(t) for t in re.search(r'val_fp32: ([-\d ]+)', csr).group(1).split()]

ev = topk(rp, ci, vv, 6, 3, 32)
print(sum(abs(a - g) for a, g in zip(ev, gvals)))
