# tern: ternary Cl(3) blades (index = basis bitmask); A=e1+e2, B=e2+e3 -> A*B; rotor sandwich R e1 R~ with R=1+e12
# fold = sum (AB[b]+8)*10^b + sum (S[b]+8)*10^(b+8)
def gprod(a, b):
    out = [0] * 8
    for i in range(8):
        for j in range(8):
            if a[i] == 0 or b[j] == 0:
                continue
            swaps = sum(1 for x in range(3) for y in range(3) if x > y and i >> x & 1 and j >> y & 1)
            out[i ^ j] += a[i] * b[j] * (-1) ** swaps
    return out
def mv(**c):  # blade names (s = scalar, e12 = e1^e2 ...) -> mask
    m = [0] * 8
    for k, v in c.items():
        m[sum(1 << (int(ch) - 1) for ch in k[1:])] = v
    return m
A, B = mv(e1=1, e2=1), mv(e2=1, e3=1)
R, Rt, e1 = mv(s=1, e12=1), mv(s=1, e12=-1), mv(e1=1)
ab = gprod(A, B)
s = gprod(gprod(R, e1), Rt)
print(sum((ab[b] + 8) * 10 ** b for b in range(8)) + sum((s[b] + 8) * 10 ** (b + 8) for b in range(8)))
