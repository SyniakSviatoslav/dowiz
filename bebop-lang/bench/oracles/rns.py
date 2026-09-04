# rns: x=12345,y=6789 in RNS over moduli (16383,16381,16379,16375); lane add/mul, Garner decode; fold = ok_add*10^9 + ok_mul*10^8 + (x+y) + (x*y)
M = [16383, 16381, 16379, 16375]
x, y = 12345, 6789
def garner(res):
    v, mult = 0, 1
    for m, r in zip(M, res):
        v += ((r - v) * pow(mult, -1, m) % m) * mult
        mult *= m
    return v
add = garner([(x % m + y % m) % m for m in M])
mul = garner([(x % m) * (y % m) % m for m in M])
print((add == x + y) * 10 ** 9 + (mul == x * y) * 10 ** 8 + add + mul)
