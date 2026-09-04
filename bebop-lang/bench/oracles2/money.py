#!/usr/bin/env python3
"""oracles2/money -- gate `money`: independent Python port of PRODUCTION
crates/dowiz-core/src/money.rs (checked_add/sub/neg, compute_line_total, apply_tax,
convert_all_to_eur_cents, estimate_order_total, assert_non_negative) driven by the
T66 harness (case table, LCG seed 4242, tag/reason codes, mix) as documented in
bench/oracles/rust/src/bin/money.rs and the money.bp first comment.
Rust i128 arithmetic = Python int; `/` = truncating division (tdiv); f64 rate math
reproduced with the same IEEE double ops; math::round = half away from zero.
Fold = (h % 1e9)*1e6 + oks*1000 + errs."""
import math
M62 = (1 << 62) - 1
I64MAX, I64MIN = (1 << 63) - 1, -(1 << 63)
def lcg(s): return (s * 1103515245 + 12345) & 2147483647
def mix(h, x): return (h * 1000003 + x) & M62
def tdiv(a, b):
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q
def rnd(x):  # dowiz-core math::round: half away from zero
    t = math.trunc(x); frac = abs(x - t)
    return (t + 1 if x > 0 else t - 1) if frac >= 0.5 else t
def fits(v): return I64MIN <= v <= I64MAX
def chk(v): return v if fits(v) else None
# reason codes: 1 cross-currency, 2 overflow, 3 denominator<=0, 4 negative total, 5 rate<=0
def checked_add(a, ca, b, cb):
    if ca != cb: return ("err", 1)
    r = chk(a + b); return ("ok", r) if r is not None else ("err", 2)
def checked_sub(a, ca, b, cb):
    if ca != cb: return ("err", 1)
    r = chk(a - b); return ("ok", r) if r is not None else ("err", 2)
def checked_neg(a):
    r = chk(-a); return ("ok", r) if r is not None else ("err", 2)
def compute_line_total(price, mods, q):
    unit = price
    for m in mods:
        unit = chk(unit + m)
        if unit is None: return ("err", 2)
    r = chk(unit * q); return ("ok", r) if r is not None else ("err", 2)
def apply_tax(sub, rate, incl):
    if sub == 0 or rate == 0.0: return ("ok", 0)
    rm = rnd(rate * 1_000_000.0)
    if incl:
        denom = 1_000_000 + rm
        if denom <= 0: return ("err", 3)
        net = tdiv(sub * 1_000_000 + tdiv(denom, 2), denom)
        tax = sub - net
    else:
        prod = sub * rm
        if not (-(1 << 127) <= prod < (1 << 127)): return ("err", 2)
        tax = tdiv(prod + 500_000, 1_000_000)
    return ("ok", tax) if fits(tax) else ("err", 2)
def convert(amount, rate):
    if rate <= 0.0: return ("err", 5)
    rs = rnd(rate * 1_000_000_000.0)
    r = tdiv(amount * rs * 100 + 500_000_000, 10 ** 9)
    return ("ok", r) if fits(r) else ("err", 2)
def assert_non_negative(a): return ("err", 4) if a < 0 else ("ok", 0)

class St:
    h = 0; oks = 0; errs = 0
    def emit(self, r):
        if r[0] == "ok": self.h = mix(mix(self.h, 1), r[1]); self.oks += 1
        else: self.h = mix(mix(self.h, 0), r[1]); self.errs += 1
def case(st, op, a, b, ca, cb, rm, m1, m2, nano, s6, flat, mn):
    q = (s6 % 20) - 2          # s6 >= 0 in every case -> Rust % == Python %
    flags = s6 >> 8
    thr = abs(b)
    rate = rm / 1_000_000.0
    if op == 1: st.emit(checked_add(a, ca, b, cb))
    elif op == 2: st.emit(checked_sub(a, ca, b, cb))
    elif op == 3: st.emit(checked_neg(a))
    elif op == 4: st.emit(compute_line_total(a, [m1, m2], q))
    elif op == 5: st.emit(apply_tax(a, rate, False))
    elif op == 6: st.emit(apply_tax(a, rate, True))
    elif op == 7: st.emit(convert(a, nano / 1_000_000_000.0))
    elif op == 9: st.emit(assert_non_negative(a))
    elif op == 8:
        is_pickup = flags & 1 == 1
        thr_opt = thr if (flags >> 3) & 1 else None
        flat_opt = flat if (flags >> 4) & 1 else None
        tiers = (flags >> 1) & 1 == 1
        incl = (flags >> 2) & 1 == 1
        mn_opt = mn if (flags >> 5) & 1 else None
        # compute_delivery_fee
        if is_pickup: fee = 0
        elif thr_opt is not None and a >= thr_opt: fee = 0
        elif tiers: fee = None
        elif flat_opt is not None: fee = flat_opt
        else: fee = None
        t = apply_tax(a, rate, incl); tax = t[1] if t[0] == "ok" else None
        min_not_met = (a < mn_opt) if mn_opt is not None else False
        total = None
        if fee is not None and tax is not None:
            s = chk(a + fee)
            if s is not None: total = chk(s + tax)
        h = mix(st.h, 1)
        for v in [int(fee is not None), fee or 0, int(tax is not None), tax or 0,
                  int(total is not None), total or 0, int(min_not_met)]:
            h = mix(h, v)
        st.h = h; st.oks += 1
    else: raise ValueError(op)

st = St()
lo, hi = I64MIN, I64MAX
HAND = [
    [1, hi, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 100, 200, 0, 1, 0, 0, 0, 0, 0, 0, 0],
    [1, hi, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0],
    [1, 1050, 200, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [2, lo, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [2, 1000, 300, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    [2, 1000, 300, 0, 2, 0, 0, 0, 0, 0, 0, 0],
    [3, lo, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [3, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [4, hi, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0],
    [4, hi, 0, 0, 0, 0, 1, 0, 0, 3, 0, 0],
    [4, 100, 0, 0, 0, 0, 10, 20, 0, 5, 0, 0],
    [5, 1000, 0, 0, 0, 200000, 0, 0, 0, 0, 0, 0],
    [5, 1005, 0, 0, 0, 125000, 0, 0, 0, 0, 0, 0],
    [5, -1000, 0, 0, 0, 200000, 0, 0, 0, 0, 0, 0],
    [5, hi, 0, 0, 0, 2000000, 0, 0, 0, 0, 0, 0],
    [5, 12345, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [6, 1200, 0, 0, 0, 200000, 0, 0, 0, 0, 0, 0],
    [6, 1000, 0, 0, 0, -1000000, 0, 0, 0, 0, 0, 0],
    [6, 1000, 0, 0, 0, -1500000, 0, 0, 0, 0, 0, 0],
    [6, 0, 0, 0, 0, -1500000, 0, 0, 0, 0, 0, 0],
    [7, 100000, 0, 0, 0, 0, 0, 0, 7500000, 0, 0, 0],
    [7, -100000, 0, 0, 0, 0, 0, 0, 7500000, 0, 0, 0],
    [7, 100000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [9, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [8, hi, 0, 0, 0, 0, 0, 0, 0, 4096, 1, 0],
    [8, 5000, 10000, 0, 0, 200000, 0, 0, 0, 14336, 500, 6000],
]
for c in HAND: case(st, *c)
s = 4242
def d():
    global s
    s = lcg(s); return s
for _ in range(48):
    op = 1 + d() % 8
    a = d() % 2000000001 - 1000000000
    b = d() % 2000000001 - 1000000000
    ca = d() % 3
    cb = (ca + 1) % 3 if d() % 4 == 0 else ca
    rm = d() % 500001
    m1 = d() % 2000001 - 1000000
    m2 = d() % 2000001 - 1000000
    nano = 1 + d() % 20000000
    s6 = d()
    flat = d() % 100001
    mn = d() % 1000001
    case(st, op, a, b, ca, cb, rm, m1, m2, nano, s6, flat, mn)
print("oks", st.oks, "errs", st.errs, "h", st.h)
print((st.h % 1000000000) * 1000000 + st.oks * 1000 + st.errs)
