# lsys: algae L-system A->AB, B->A from seed A, depth 10 (A=0,B=1); FNV-1 64-bit over cell values, digest as i64; fold = len*10^12 + digest mod 10^12 (C-style, fixed to non-negative)
s = [0]
for _ in range(10):
    s = [c for a in s for c in ([0, 1] if a == 0 else [0])]
h = 14695981039346656037
for v in s:
    h = ((h ^ v) * 1099511628211) & 0xFFFFFFFFFFFFFFFF
h -= (h >> 63) << 64  # re-sign to i64
r = (abs(h) % 10 ** 12) * (1 if h >= 0 else -1)  # C-style truncating %
print(len(s) * 10 ** 12 + (r + 10 ** 12 if r < 0 else r))
