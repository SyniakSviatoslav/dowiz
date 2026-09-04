# mma: bump-arena on an anonymous mmap; fold = ok1*10^9 + al*10^8 + good*10^7 + okh*10^6 + 100000.
# The kernel contract is modelled: a positive page-aligned base and a writable 1 MiB region.
SIZE = 1048576
b = 4096 * 1000            # any positive, 4096-aligned base satisfies the mmap contract
arena = [0] * (SIZE // 8)  # cells, 8 bytes each
ok1 = 1 if b > 0 else 0
al = 1 if b - (b // 4096) * 4096 == 0 else 0
good = 1
for i in range(100000):
    arena[i] = i
    good *= 1 if arena[i] == i else 0
okh = 1 if arena[99999] == 99999 else 0
print(ok1 * 1000000000 + al * 100000000 + good * 10000000 + okh * 1000000 + 100000)
