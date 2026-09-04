# vecinv: C8 ring, f_i=i; div(grad f) row-sum == Laplacian (ident1), circulation has zero div (ident2),
# survives relabel +2 (rot_ok), broken edge F[1][0]=0 leaks div3=1 (caught). fold = ident1*10^6+ident2*10^5+rot_ok*10^4+caught*10^3+div3*10+lf0
N = 8
f = list(range(N))
nxt = lambda i: (i + 1) % N
grad = [[0] * N for _ in range(N)]
circ = [[0] * N for _ in range(N)]
for i in range(N):
    grad[i][nxt(i)] = f[nxt(i)] - f[i]; grad[nxt(i)][i] = f[i] - f[nxt(i)]
    circ[i][nxt(i)] = 1; circ[nxt(i)][i] = -1
lap = lambda i: f[nxt(i)] + f[(i - 1) % N] - 2 * f[i]
ident1 = int(all(sum(grad[i]) == lap(i) for i in range(N)))
ident2 = int(all(sum(circ[i]) == 0 for i in range(N)))
rot_ok = int(all(sum(circ[(i + 2) % N][(j + 2) % N] for j in range(N)) == 0 for i in range(N)))
broken = [row[:] for row in circ]
broken[0][1] = 1; broken[1][0] = 0
div3 = sum(abs(sum(row)) for row in broken)
caught = int(div3 > 0)
print(ident1 * 10**6 + ident2 * 10**5 + rot_ok * 10**4 + caught * 10**3 + div3 * 10 + lap(0))
