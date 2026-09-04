# qlora: 8 fp32.32 weights -> 4-bit signed quantize (round |w|*8), rank-1 adapter dW=A*B at 2^-8, FNV-1a64 key of packed nibbles
# fold = rt_ok*10^12 + moved*10^11 + invalid*10^10 + (key0&0xffff)*10^5 + (ydelta>>20)
M = (1 << 64) - 1
def tdiv(a, b): return -(-a // b) if (a < 0) != (b < 0) else a // b
one = 1 << 32
w = [tdiv(one, 10), -tdiv(one * 3, 10), tdiv(one, 2), -tdiv(one * 7, 10), tdiv(one * 9, 10), -tdiv(one, 5), tdiv(one * 2, 5), -tdiv(one * 3, 5)]
A = [2, -1, 3, 1, -2, 2, -3, 1]
B = [1, -2, 1, 0, -1, 2, -1, 1]
def quant(wi):
    qa = (abs(wi) * 8 + one // 2) // one
    return -qa if wi < 0 else qa
def pack(ws): return sum((quant(wi) + 8) << (4 * i) for i, wi in enumerate(ws))
rt_ok = all(abs(wi - quant(wi) * one // 8) <= one // 16 for wi in w)
def fp_mul(a, b):
    p = (abs(a) * abs(b)) >> 32
    return -p if (a < 0) != (b < 0) else p
dw = [fp_mul(A[i] * (one // 16), B[i] * (one // 16)) for i in range(8)]
ydelta = sum(dw)
moved = ydelta != 0
wup = [w[i] + dw[i] for i in range(8)]
h0 = (-3750763034362895579) & M
key0 = ((h0 ^ pack(w)) * 1099511628211) & M
key1 = ((h0 ^ pack(wup)) * 1099511628211) & M
invalid = key1 != key0
print(int(rt_ok) * 10**12 + int(moved) * 10**11 + int(invalid) * 10**10 + (key0 & 65535) * 10**5 + ((ydelta & M) >> 20))
