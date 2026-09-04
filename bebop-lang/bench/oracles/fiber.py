# fiber: N=4 cooperative fibers, R=3 rounds, cells[i] += cells[(i+1)&3] + 1 in dispatch order; fold = c0*10^6+c1*10^4+c2*10^2+c3 + switches*10^8
c = [1, 1, 1, 1]
switches = 0
for _ in range(3):
    for i in range(4):
        c[i] += c[(i + 1) & 3] + 1
        switches += 1
print(c[0] * 10**6 + c[1] * 10**4 + c[2] * 100 + c[3] + switches * 10**8)
