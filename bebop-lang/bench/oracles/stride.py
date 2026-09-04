# stride: (4,4,4) tensor, inner runs padded to 8 cells; fold = align_ok*10^10 + line_ok*10^9 + waste_ok*10^8 + footprint*1000 + n_runs.
d0 = d1 = 4
run = 8
s1, s0 = run, d1 * run
footprint = d0 * s0
n_runs = d0 * d1
logical = d0 * d1 * 4
align_ok = tile_ok = 1
for o0 in range(d0):
    for o1 in range(d1):
        base = o0 * s0 + o1 * s1
        align_ok *= 1 if base % 8 == 0 else 0
        tile_ok *= 1 if base < footprint else 0
line_ok = 1 if run == 8 else 0
waste_ok = 1 if footprint - logical == 64 else 0
print(align_ok * 10000000000 + line_ok * 1000000000 + waste_ok * 100000000 + footprint * 1000 + n_runs)
