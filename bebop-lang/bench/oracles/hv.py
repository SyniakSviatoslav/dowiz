# hv: D=1024 HDC core (splitmix64 code, xor bind, majority bundle, bit-rotate permute, hamming, popcount); fold = Horner acc*131 + word over the golden.txt chain, i64 wrap
import os
M = (1 << 64) - 1
GOLDEN, MUL1, MUL2 = 0x9E3779B97F4A7C15, 0xBF58476D1CE4E5B9, 0x94D049BB133111EB

def code(seed):
    out, s = [], seed & M
    for _ in range(16):
        s = (s + GOLDEN) & M
        z = ((s ^ (s >> 30)) * MUL1) & M
        z = ((z ^ (z >> 27)) * MUL2) & M
        out.append(z ^ (z >> 31))
    return out

def bind(a, b): return [x ^ y for x, y in zip(a, b)]

def bundle(vs):
    n = len(vs)
    return [sum(1 << b for b in range(64) if sum((v[w] >> b) & 1 for v in vs) * 2 > n) for w in range(16)]

def permute(v, sh):  # rotate the 1024-bit integer left by sh
    sh %= 1024
    big = sum(x << (64 * k) for k, x in enumerate(v))
    big = ((big << sh) | (big >> (1024 - sh))) & ((1 << 1024) - 1)
    return [(big >> (64 * k)) & M for k in range(16)]

def hamming(a, b): return sum(bin(x ^ y).count("1") for x, y in zip(a, b))
def popcount(v): return sum(bin(x).count("1") for x in v)

# cross-check against the Rust golden (mathematical reference)
gold = {}
for line in open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "vs_rust", "spectral_golden", "golden.txt")):
    if line.startswith("hv ") and ":" in line:
        k, v = line[3:].split(":", 1)
        gold[k.strip()] = [int(x, 16) for x in v.split()]

a, b, c7 = code(42), code(3735928559), code(7)
chain = [code(0), code(1), code(2), a, b, code(1234567890123456789)]
assert chain[3] == gold["code(42)"] and chain[4] == gold["code(3735928559)"]
ab = bind(a, b); assert ab == gold["bind(42,0xDEADBEEF)"]
u = bundle([a, b, c7]); assert u == gold["bundle(42,0xDEADBEEF,7)"]
perms = [permute(a, s) for s in (1, 64, 255, 1023)]
assert perms[0] == gold["perm(42,1)"] and perms[3] == gold["perm(42,1023)"]

acc = 0
def fold(x):
    global acc
    acc = (acc * 131 + x) & M
for v in chain + [ab]:
    for w in v: fold(w)
fold(1 if bind(ab, b) == a else 0)
for w in u: fold(w)
for p in perms:
    for w in p: fold(w)
fold(1 if permute(a, 0) == a else 0)
fold(1 if permute(a, 1024) == a else 0)
fold(hamming(a, b))
fold(popcount(a))
print(acc - (1 << 64) if acc >> 63 else acc)
