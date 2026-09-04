# cache: DecompCache falsifier - FNV-1a-64 key over 4 fp cells, 8-slot round-robin table; miss1,hit2,miss2,recomputes==1,key-changed,stored-key-ok hashed base 131
M = (1 << 64) - 1
def fnv(cells):
    h = (-3750763034362895579) & M
    for c in cells:
        h = ((h ^ (c & M)) * 1099511628211) & M
    return h
def lookup(keys, key):
    return next((i for i, k in enumerate(keys) if k == key), -1)
S = 8
keys, vals, nxt = [0] * S, [0] * S, 0
def store(key, val):
    global nxt
    slot = nxt % S
    keys[slot], vals[slot] = key, val
    nxt += 1
    return slot
m = [1 << 32, 0, 0, 1 << 32]
k1 = fnv(m)
s1 = lookup(keys, k1)
miss1 = int(s1 == -1)
if miss1: s1 = store(k1, 1 << 32)  # value = rho(I2) = fp(1.0); not on the fold
hit2 = int(lookup(keys, k1) == s1)
m[0] = 1 << 33
k2 = fnv(m)
s3 = lookup(keys, k2)
miss2 = int(s3 == -1)
if miss2: s3 = store(k2, 7)
recomputes = miss2
acc = 0
for b in (miss1, hit2, miss2, int(recomputes == 1), int(k1 != k2), int(keys[s1] == k1)):
    acc = acc * 131 + b
print(acc)
