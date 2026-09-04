# calcbound: mean-value box for f(x)=x^2-x (fp 2^32) at x0=1, d in {-1/8..1/8}; fold = contained*1e9 + sum(|fi|>>16)*1e3 + (f0>>20)
M = (1 << 64) - 1
def w(x): x &= M; return x - (1 << 64) if x >> 63 else x
def div(a, b): q = abs(a) // abs(b); return q if (a < 0) == (b < 0) else -q
def fp_mul(a, b):
    aa, ab = abs(a), abs(b)
    a1, a0 = aa >> 32, aa & 0xFFFFFFFF
    b1, b0 = ab >> 32, ab & 0xFFFFFFFF
    ah, al = a0 >> 16, a0 & 0xFFFF
    bh, bl = b0 >> 16, b0 & 0xFFFF
    low = ah * bh + (((ah * bl + al * bh) + ((al * bl) >> 16)) >> 16)
    p = w((a1 * b1 << 32) + a1 * b0 + a0 * b1 + low)
    return -p if (a < 0) != (b < 0) else p
one = 1 << 32
fmin, fmax, eps, x0 = 3 * one // 4, 5 * one // 4, one // 100, one
f0 = fp_mul(x0, x0) - x0
contained, sq = 1, 0
for m in range(5):
    d = m * (one // 16) - one // 8
    xi = x0 + d
    fi = fp_mul(xi, xi) - xi
    df = fi - f0
    d1, d2 = fp_mul(fmin, d), fp_mul(fmax, d)
    lo, hi = div(d1 + d2 - abs(d1 - d2), 2) - eps, div(d1 + d2 + abs(d1 - d2), 2) + eps
    contained *= int(lo <= df <= hi)
    sq += abs(fi) >> 16
print(contained * 10**9 + sq * 1000 + (f0 >> 20))
