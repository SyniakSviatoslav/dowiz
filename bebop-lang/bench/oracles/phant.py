# phant: algae L-system (0->01, 1->0) depth 6 from [0]; connectivity word = OR of bit(k mod 16) over cells==1; one ring round ns = bits whose both ring neighbours are set; re-expand and check identity. fold = ns*10^6 + eq*10^8 + length
def expand():
    s = [0]
    for _ in range(6):
        s = [b for c in s for b in ([0, 1] if c == 0 else [0])]
    return s
def word(cells):
    w = 0
    for k, c in enumerate(cells):
        if c & 1: w |= 1 << (k % 16)
    return w & 0xFFFF
c1 = expand(); cw = word(c1)
ns = sum(1 << i for i in range(16) if (cw >> ((i + 15) % 16)) & 1 and (cw >> ((i + 1) % 16)) & 1)
eq = 1 if word(expand()) == cw else 0
print(ns * 10**6 + eq * 10**8 + len(c1))
