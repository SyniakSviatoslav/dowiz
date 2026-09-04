# tdgstokes oracle: discrete Stokes audit fold from the definition (i64 fp Q32).
ONE = 1 << 32
M = 1000000007
OFF = 1 << 40

def wval(i, j, a, b, c):
    return ((i * a + j * b + c) % 23 - 11) * ONE // 16

wx = {(i, j): wval(i, j, 31, 17, 7) for i in range(9) for j in range(9)}
wy = {(i, j): wval(i, j, 13, 29, 3) for i in range(9) for j in range(9)}

def cell_dw(i, j):
    return wx[i, j] + wy[i + 1, j] - wx[i, j + 1] - wy[i, j]

dw = {(i, j): cell_dw(i, j) for i in range(8) for j in range(8)}
regions = [(0, 0, 8, 8), (1, 2, 5, 7), (3, 3, 4, 4), (0, 5, 8, 6), (2, 0, 6, 8)]

def audit(fold):
    viol = 0
    for x0, y0, x1, y1 in regions:
        b = sum(wx[i, y0] - wx[i, y1] for i in range(x0, x1)) \
          + sum(wy[x1, j] - wy[x0, j] for j in range(y0, y1))
        s = sum(dw[i, j] for i in range(x0, x1) for j in range(y0, y1))
        viol += b != s
        fold = (fold * 131 + (b + OFF) % M) % M
    return (fold * 131 + viol) % M, viol

fold, v1 = audit(0)
wx[3, 4] += 5 * ONE // 16
dw[3, 4] = cell_dw(3, 4)
dw[3, 3] = cell_dw(3, 3)
fold, v2 = audit(fold)
wy[5, 4] += 3 * ONE // 16
wx[2, 5] -= 7 * ONE // 16
fold, v3 = audit(fold)
assert (v1, v2, v3) == (0, 0, 2), (v1, v2, v3)
print(fold + 1)
