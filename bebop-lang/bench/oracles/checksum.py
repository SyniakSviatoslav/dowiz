# checksum: polynomial base-31 fold h = h*31 + x over [97,98,99] starting at 0 (i64, no overflow here).
M = (1 << 64) - 1
def wrap(v):
    v &= M
    return v - (1 << 64) if v >> 63 else v
acc = 0
for x in [97, 98, 99]:
    acc = wrap(acc * 31 + x)
print(acc)
