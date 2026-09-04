# pieblock: relative-link block walked at base 0 and base 1000; fold = pie_ok*1e9 + integ_ok*1e8 + sum0*1e3 + cyc
M64 = (1 << 64) - 1
def fnv(cells):
    h = -3750763034362895579 & M64
    for c in cells:
        h = ((h ^ (c & M64)) * 1099511628211) & M64
    return h
block = [1346456865, 3, 100, 2, 200, 2, 500, -4]
def walk(buf, base):
    cur, s = base + 2, 0
    for _ in range(3):
        s += buf[cur]
        cur += buf[cur + 1]
    return s, cur
buf = [0] * 1200
buf[0:8] = block
buf[1000:1008] = block
s0, c0 = walk(buf, 0)
s1, c1 = walk(buf, 1000)
pie_ok = int(s0 == s1)
cyc = int(c0 == 2)
integ_ok = int(fnv(buf[0:8]) == fnv(buf[1000:1008]))
print(pie_ok * 10**9 + integ_ok * 10**8 + s0 * 1000 + cyc)
