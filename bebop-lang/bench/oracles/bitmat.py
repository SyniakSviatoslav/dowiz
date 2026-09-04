# bitmat: branch-free first-set-bit index over all 256 8-bit flag words (-1 when empty) vs sequential scan; fold = ok*10^9 + tot*100
ok, tot = 1, 0
for f in range(256):
    idx, nf = 0, 1
    for k in range(8):
        bk = (f >> k) & 1
        idx += k * bk * nf
        nf *= 1 - bk
    idx -= int(f == 0)
    exp = next((k for k in range(8) if f >> k & 1), -1)
    ok &= int(idx == exp)
    tot += idx
print(ok * 10**9 + tot * 100)
