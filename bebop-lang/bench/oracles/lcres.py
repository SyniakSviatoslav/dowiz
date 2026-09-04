# lcres: f0 = 1/(2*pi*sqrt(L*C)) in fp 2^32 (bit-exact fp_mul / 31-step isqrt / restoring fp_div);
# fold = ok1*10^15 + ok2*10^14 + okT*10^13 + (f1>>12)*10^7 + (f2>>12)
M = (1 << 64) - 1
def i64(x):
    x &= M
    return x - (1 << 64) if x >> 63 else x
def shr(x, n):  # logical shift on the 64-bit pattern
    return i64((x & M) >> n)
def fp_mul(a, b):
    na, nb = a < 0, b < 0
    aa, ab = abs(a), abs(b)
    a1, a0 = shr(aa, 32), aa & 0xFFFFFFFF
    b1, b0 = shr(ab, 32), ab & 0xFFFFFFFF
    hi = i64(a1 * b1)
    mid = i64(a1 * b0 + a0 * b1)
    ah, al = a0 >> 16, a0 & 0xFFFF
    bh, bl = b0 >> 16, b0 & 0xFFFF
    low = ah * bh + ((ah * bl + al * bh + ((al * bl) >> 16)) >> 16)
    p = i64(i64(hi << 32) + mid + low)
    return i64(-p) if na != nb else p
def isqrt(s):  # floor(sqrt(s)), 31-step restoring, s < 2^62
    rem = root = 0
    for i in range(31):
        rem = (rem << 2) + (shr(s, 60 - 2 * i) & 3)
        tst = (root << 2) + 1
        take = int(tst <= rem)
        root = (root << 1) + take
        rem -= take * tst
    return root
def fp_div(a, b):  # floor(a*2^32/b), a>=0, b>0
    qint, r = divmod(a, b)
    qf = 0
    for _ in range(32):
        r *= 2
        bit = int(r >= b)
        qf = qf * 2 + bit
        r -= bit * b
    return i64(qint * (1 << 32) + qf)
ONE = 1 << 32
TWO_PI = 26986075410
def f0_of(l, c):
    lc = fp_mul(l, c)
    return fp_div(ONE, fp_mul(TWO_PI, isqrt(i64(lc * ONE))))
if __name__ == "__main__":
    f1 = f0_of(ONE // 16, ONE)
    f2 = f0_of(ONE // 4, ONE // 16)
    t2 = fp_div(ONE, f2)
    ok = lambda v, ref: int(abs(v - ref) < 4294967)
    print(ok(f1, 2734260830) * 10**15 + ok(f2, 5468521660) * 10**14 + ok(t2, 3373259426) * 10**13
          + shr(f1, 12) * 10**7 + shr(f2, 12))
