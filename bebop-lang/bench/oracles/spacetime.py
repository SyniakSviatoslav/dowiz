# spacetime: ring of 8 nodes with x0,x4 pinned; harmonic field crystallized by constant arc increments; checks loop closure,
# zero Laplacian at interior nodes, exact fixpoint under 8 Jacobi averaging steps. fold = close1*10^12 + harm1*10^11 + fix1*10^10 + close2*10^9 + harm2*10^8 + chk1*100 + chk2
def tdiv(a, b): return -(-a // b) if (a < 0) != (b < 0) else a // b
def field(a, b):
    dA, dB = tdiv(b - a, 4), tdiv(a - b, 4)
    x = [a + dA * i for i in range(5)] + [b + dB * i for i in range(1, 4)]
    close = x[7] + dB == x[0]
    harm = all(x[(i - 1) % 8] + x[(i + 1) % 8] - 2 * x[i] == 0 for i in range(8) if i not in (0, 4))
    chk = sum(x[i] * (i + 1) for i in range(8))
    return x, close, harm, chk
x1, close1, harm1, chk1 = field(10, -2)
t = x1[:]
for _ in range(8):
    prev = t[:]
    t = [prev[i] if i in (0, 4) else tdiv(prev[(i - 1) % 8] + prev[(i + 1) % 8], 2) for i in range(8)]
fix1 = t == prev and t[0] == 10 and t[4] == -2
x2, close2, harm2, chk2 = field(20, -4)
print(close1 * 10**12 + harm1 * 10**11 + fix1 * 10**10 + close2 * 10**9 + harm2 * 10**8 + chk1 * 100 + chk2)
