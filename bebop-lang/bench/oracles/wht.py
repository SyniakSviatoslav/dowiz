# wht: FWHT(e_1) on N=8 -> word = bits of positive cells; fold = word*1000 + (FWHT then FWHT/N round-trips [3,1,4,1,5,9,2,6])
def fwht(x):
    n = len(x); h = 1
    while h < n:
        for i in range(0, n, 2 * h):
            for j in range(i, i + h):
                u, v = x[j], x[j + h]
                x[j], x[j + h] = u + v, u - v
        h *= 2
e = [0] * 8; e[1] = 1
fwht(e)
word = sum(1 << k for k in range(8) if e[k] > 0)
inp = [3, 1, 4, 1, 5, 9, 2, 6]
x = inp[:]
fwht(x); fwht(x)
x = [int(v / 8) for v in x]
print(word * 1000 + (1 if x == inp else 0))
