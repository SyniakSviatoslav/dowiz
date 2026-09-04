# sinc: fp 2^32 Taylor sinc(z)=1-z2/3!+z4/5!-z6/7!+z8/9!-z10/11!, z=pi*x; fold = (sinc(1/2)>>12)*1e7 + (sinc(1/4)>>12)*1e4 + (|sinc(1)|>>16)*10 + ok
M = (1 << 64) - 1
def w(x): x &= M; return x - (1 << 64) if x >> 63 else x
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
pi = 13493037705
import math
def sinc_fp(x):
    z = fp_mul(pi, x)
    z2 = fp_mul(z, z)
    zp, acc, sign = one, one, -1
    for k in (3, 5, 7, 9, 11):
        zp = fp_mul(zp, z2)
        acc += sign * fp_mul(zp, one // math.factorial(k))
        sign = -sign
    return acc
vq, vh, v1 = sinc_fp(one // 4), sinc_fp(one // 2), sinc_fp(one)
e1 = abs(v1)
print((vh >> 12) * 10**7 + (vq >> 12) * 10**4 + (e1 >> 16) * 10 + int(e1 < 4294967))
