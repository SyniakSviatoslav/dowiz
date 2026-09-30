# Oracle for gate `block_money_neg` (DG9, SPEC-DATALOG-AND-CODEC B-3). The twelve cases of
# std_tests/block_money_neg.bp through crates/dowiz-hub/src/block/view.rs Catalogue::unit_gross's rule,
# in exact Python integers: inclusive -> the price; rate = own, or the default for NO_RATE (-1); a rate
# outside 0..=1,000,000 (RatePpm::MAX) -> -18 (bad_rate); tax = (price*rate + 500000) / 1e6 TRUNCATED
# toward zero (dowiz-core eqc_gen apply_tax_exclusive_int, i128); price + tax outside i64 -> -17
# (overflow). Folded acc*31 + (gross or -code), 64-bit wrap.
M64 = (1 << 64) - 1
MAX = (1 << 63) - 1
def trunc(n, d): return n // d if n >= 0 else -((-n) // d)
def gross(p, own, dflt, incl):
    if incl: return p
    r = dflt if own == -1 else own
    if r < 0 or r > 1000000: return -18
    g = p + trunc(p * r + 500000, 1000000)
    return g if -MAX - 1 <= g <= MAX else -17
cases = [(1200, 170000, 0, 0), (999, 60000, 0, 0), (MAX, 200000, 0, 0), (MAX // 2, 1000000, 0, 0),
         (MAX // 2 + 1, 1000000, 0, 0), (800, -1, 200000, 0), (800, 1000001, 0, 0), (800, -5, 0, 0),
         (1500, 200000, 0, 1), (-1500, 200000, 0, 0), (-MAX - 1, 1000000, 0, 0), (-7, 71429, 0, 0)]
acc = 0
for c in cases:
    acc = (acc * 31 + gross(*c)) & M64
print(acc - (1 << 64) if acc >> 63 else acc)
