# mutlsys: 2-symbol L-system (0 -> [0, rule[1]], 1 -> [rule[0]]) expanded 6 levels; FNV-1a64 of the string is fitness;
# 4 mutations flip rule[gen%2], kept iff fitness (signed) improves. fold = accepted*10^11 + (final digest mod 10^11, non-negative)
M = (1 << 64) - 1
def s64(v): v &= M; return v - (1 << 64) if v >> 63 else v
def fnv(cells):
    h = 14695981039346656037
    for v in cells: h = ((h ^ (v & M)) * 1099511628211) & M
    return s64(h)
def expand(rule):
    s = [0]
    for _ in range(6):
        s = [c for v in s for c in ([0, rule[1]] if v == 0 else [rule[0]])]
    return s
rule = [0, 1]; accepted = 0
for gen in range(4):
    trial = rule[:]; trial[gen % 2] ^= 1
    if fnv(expand(trial)) > fnv(expand(rule)):
        rule = trial; accepted += 1
dig = fnv(expand(rule))
r = dig % 10**11 if dig >= 0 else -((-dig) % 10**11)
print((r + 10**11 if r < 0 else r) + accepted * 10**11)
