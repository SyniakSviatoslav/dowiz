# petri: bit-level Petri net (mark/clear/get on 64-bit cells, tzcnt dispatcher); fold = 3 probe words chained acc*131 + probe
M = (1 << 64) - 1
def mark(x, p):  x[p // 64] |= 1 << (p % 64)
def clear(x, p): x[p // 64] &= M ^ (1 << (p % 64))
def get(x, p):   return (x[p // 64] >> (p % 64)) & 1
def tzcnt(v):
    v &= M
    return (v & -v).bit_length() - 1 if v else -1
def step(x, pre, post, n):
    act = sum(1 << i for i in range(n) if x[0] & pre[i] == pre[i])
    t = tzcnt(act)
    if t >= 0:
        x[0] = (x[0] & (M ^ pre[t])) | post[t]
    return t
x = [0] * 8; pre = [0] * 16; post = [0] * 16
for p in (0, 3, 63, 65): mark(x, p)
acc = get(x, 0) + get(x, 3) * 2 + get(x, 5) * 4 + get(x, 63) * 8 + get(x, 65) * 16
clear(x, 0)
acc = acc * 131 + get(x, 0) - get(x, 3) * 7 + get(x, 65) * 64
x[0] = x[1] = 0; mark(x, 0)
pre[0], post[0], pre[1], post[1] = 1, 2, 1, 4
t0 = step(x, pre, post, 2); t2 = step(x, pre, post, 2)
acc = acc * 131 + (t0 == 0) + 2 * (t2 == -1) + 4 * (get(x, 1) == 1) + 8 * (get(x, 0) == 0)
x[0] = 0
for i in range(4): pre[i], post[i] = 1 << i, 16 << i
for p in range(4): mark(x, p)
seq = tzcnt(15) * 1000 + step(x, pre, post, 4) * 100 + step(x, pre, post, 4) * 10 + step(x, pre, post, 4)
acc = acc * 131 + (seq == 12) + 2 * (get(x, 6) == 1) + 4 * (get(x, 7) == 0)
print(acc)
