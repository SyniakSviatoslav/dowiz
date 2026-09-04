# rng: PCG64 RXS-M-XS seeded via SplitMix64 (rng_init(42,1), inc=3); fold = Horner acc = acc*31 + out over 8 outputs, i64 wrap
M = (1 << 64) - 1
def s64(x):
    x &= M
    return x - (1 << 64) if x >> 63 else x
def shr(x, n):  # logical shift on 64-bit pattern
    return (x & M) >> n

GOLDEN, MUL1, MUL2 = 0x9E3779B97F4A7C15, 0xBF58476D1CE4E5B9, 0x94D049BB133111EB
PCG_MUL = 6364136223846793005

def splitmix64_next(s):
    z = ((s ^ shr(s, 30)) * MUL1) & M
    z = ((z ^ shr(z, 27)) * MUL2) & M
    return z ^ shr(z, 31)

def pcg_step(st, inc):
    return (st * PCG_MUL + inc) & M

def pcg_output(st):
    rot = shr(st, 59) & 31
    x = shr(st ^ shr(st, 18), 27)
    return (shr(x, rot) | (x << ((-rot) & 63))) & M

def rng_init(seed, stream):
    inc = ((stream << 1) | 1) & M
    st = splitmix64_next(((seed ^ GOLDEN) + GOLDEN) & M)
    return pcg_step((st + inc) & M, inc)

st = rng_init(42, 1)
acc = 0
for _ in range(8):
    st = pcg_step(st, 3)
    acc = (acc * 31 + pcg_output(st)) & M
print(s64(acc))
