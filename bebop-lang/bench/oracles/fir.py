# fir: 4-tap FIR h={1,1/2,1/4,1/8} in fp 2^32 (fp_mul = sign*floor(|a||b|/2^32)); taps_ok: impulse per lag == tap; bib_ok: all 16 sign patterns |y|<=15/8. fold = taps_ok*10^13 + bib_ok*10^12 + (sum|y|>>16)*10^5 + (max|y|>>16)
ONE = 1 << 32
def fp_mul(a, b):
    p = (abs(a) * abs(b)) >> 32
    return -p if (a < 0) != (b < 0) else p
h = [ONE, ONE // 2, ONE // 4, ONE // 8]
fir4 = lambda x: sum(fp_mul(h[i], x[i]) for i in range(4))
taps_ok = int(all(fir4([ONE if j == k else 0 for j in range(4)]) == h[k] for k in range(4)))
bound = 15 * ONE // 8
ays = [abs(fir4([ONE if (pat >> j) & 1 else -ONE for j in range(4)])) for pat in range(16)]
bib_ok = int(all(ay <= bound for ay in ays))
print(taps_ok * 10**13 + bib_ok * 10**12 + (sum(ays) >> 16) * 10**5 + (max(ays) >> 16))
