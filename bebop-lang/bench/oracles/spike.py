# spike: activity word = bits {3,5,9,20,33,45} (0x200200100228; the .bp header hex is a typo), dispatch Base(1000)+idx*Stride(8) per set bit LSB-first;
# fold = sum(addr)*10^6 + (n_dispatched==popcnt)*10^3 + last_idx
act = sum(1 << i for i in (3, 5, 9, 20, 33, 45))
idxs = [i for i in range(64) if act >> i & 1]
acc = sum(1000 + i * 8 for i in idxs)
nok = int(len(idxs) == bin(act).count("1"))
print(acc * 10**6 + nok * 1000 + idxs[-1])
