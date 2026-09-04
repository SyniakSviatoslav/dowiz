# lod: algae expansion depth 6 (21 cells, 0->scalar, 1->e12); every cell sandwiched R x R~ with R=1+e12 (Cl(3), blade index = basis bitmask);
# digest = sum (acc[b]+8)*10^b over the accumulated multivector; LOD path recomputes it on the fly from the parent level; fold = digest + eq*10^12
def gprod(a, b):
    out = [0] * 8
    for i in range(8):
        for j in range(8):
            if a[i] and b[j]:
                swaps = sum(1 for x in range(3) for y in range(3) if x > y and i >> x & 1 and j >> y & 1)
                out[i ^ j] += a[i] * b[j] * (-1) ** swaps
    return out
def blade(k):
    m = [0] * 8; m[k] = 1; return m
R, Rt = [1, 0, 0, 1, 0, 0, 0, 0], [1, 0, 0, -1, 0, 0, 0, 0]  # 1+e12, 1-e12
sandwich = {c: gprod(gprod(R, blade(3 if c else 0)), Rt) for c in (0, 1)}
def digest(cells):
    acc = [sum(sandwich[c][b] for c in cells) for b in range(8)]
    return sum((acc[b] + 8) * 10 ** b for b in range(8))
s = [0]
levels = [s]
for _ in range(6):
    s = [c for a in s for c in ([0, 1] if a == 0 else [0])]
    levels.append(s)
d1 = digest(levels[6])
# LOD path: fold while writing the final level from its parent: each parent -> one scalar child, each A parent -> one e12 child
d2 = digest([0] * len(levels[5]) + [1] * levels[5].count(0))
print(d1 + (d1 == d2) * 10 ** 12)
