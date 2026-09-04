# Exact i64-semantics Haar (in-place, len from n/2 down to 1)
def haar_pow2(x, n):
    ln = n // 2
    while ln >= 1:
        i = 0
        while i < ln:
            a = x[i]; b = x[i + ln]
            x[i] = a + b
            x[i + ln] = a - b
            i += 1
        ln //= 2

def haar_invert(x, n):
    ln = 1
    while ln <= n // 2:
        i = 0
        while i < ln:
            a = x[i]; b = x[i + ln]
            x[i] = (a + b) // 2
            x[i + ln] = (a - b) // 2
            i += 1
        ln *= 2
e = [0,1,0,0,0,0,0,0]
haar_pow2(e, 8)
word = sum(1 << k for k in range(8) if e[k] > 0)
inp = [3,1,4,1,5,9,2,6]
x = inp[:]
haar_pow2(x, 8)
haar_invert(x, 8)
print(word*1000 + (1 if x == inp else 0))
