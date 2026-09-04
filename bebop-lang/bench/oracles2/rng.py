#!/usr/bin/env python3
"""oracles2/rng -- gate `rng`.  PROSE (rng.bp first comment): SplitMix64 and PCG64
RXS-M-XS, exact port of the deleted rng.c.  Constants GOLDEN=0x9E3779B97F4A7C15,
MUL1=0xBF58476D1CE4E5B9, MUL2=0x94D049BB133111EB, PCG_MUL=6364136223846793005;
splitmix: caller does s += GOLDEN, then z=s; z=(z^(z>>30))*MUL1; z=(z^(z>>27))*MUL2;
z^(z>>31).  pcg_step: state*PCG_MUL + inc (the LCG state feeds the next step); the
output permutation is RXS-M-XS 64: w = ((x >> ((x>>59)+5)) ^ x) * 12605985483714917081;
w ^ (w>>43) (the standard PCG64 RXS-M-XS; rng.c is gone, so this half is the public
definition, not the repo's).  NOT IN ANY PROSE: seed, inc, number of draws, fold.
Parameters (env): SEED=42, INC=1442695040888963407|1 (odd), DRAWS=8, FOLD=xor.
Prints the SplitMix64 draws, the PCG draws and the fold (last line, signed i64)."""
import os
M64 = (1 << 64) - 1
def s64(x): x &= M64; return x - (1 << 64) if x >> 63 else x
GOLDEN, MUL1, MUL2 = 0x9E3779B97F4A7C15, 0xBF58476D1CE4E5B9, 0x94D049BB133111EB
PCG_MUL, RXS_MUL = 6364136223846793005, 12605985483714917081
SEED = int(os.environ.get("SEED", "42")); DRAWS = int(os.environ.get("DRAWS", "8"))
INC = int(os.environ.get("INC", str(1442695040888963407 | 1)))
def splitmix(s):
    s = (s + GOLDEN) & M64; z = s
    z = ((z ^ (z >> 30)) * MUL1) & M64
    z = ((z ^ (z >> 27)) * MUL2) & M64
    return s, z ^ (z >> 31)
def pcg_step(st): return (st * PCG_MUL + INC) & M64
def rxs_m_xs(x):
    w = (((x >> ((x >> 59) + 5)) ^ x) * RXS_MUL) & M64
    return w ^ (w >> 43)
s = SEED; sm = []
for _ in range(DRAWS): s, z = splitmix(s); sm.append(z)
# pcg seeded from the splitmix state (a common seeding), then DRAWS outputs
st = s; pc = []
for _ in range(DRAWS): st = pcg_step(st); pc.append(rxs_m_xs(st))
h = 0
for z in sm + pc: h ^= z
print("splitmix", sm); print("pcg", pc)
print(s64(h))
