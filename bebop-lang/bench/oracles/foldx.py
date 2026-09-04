# foldx: constant-fold gate; fold = a*10^6 + b*10^4 + d*100 + cok*10 + eok with i64 wrapping arithmetic.
M = (1 << 64) - 1
def wrap(v):
    v &= M
    return v - (1 << 64) if v >> 63 else v
j = 4
a = wrap(1 + 2 * 3)
b = wrap(2 * (3 + 4))
c = wrap(0 - 6148914691236517206)
d = (1 if j == 4 else 0) * 100
e = wrap(6148914691236517205 + 1)
cneg = wrap(0 - c)
cok = 1 if cneg == 6148914691236517206 else 0
eok = 1 if e == 6148914691236517206 else 0
print(a * 1000000 + b * 10000 + d * 100 + cok * 10 + eok)
