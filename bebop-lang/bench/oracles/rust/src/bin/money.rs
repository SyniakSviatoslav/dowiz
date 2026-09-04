//! T66 oracle for gate `money` — calls PRODUCTION `dowiz_core::money` on the
//! case table of selfhost/std/money.bp (same LCG, same hand-picked edges) and
//! prints the same fold. Only the harness (case generation, tag/reason codes,
//! mix) lives here; every monetary result comes from money.rs.
use dowiz_core::money::*;

const M62: i64 = (1 << 62) - 1;
fn lcg(s: i64) -> i64 { (s.wrapping_mul(1103515245).wrapping_add(12345)) & 2147483647 }
fn mix(h: i64, x: i64) -> i64 { h.wrapping_mul(1000003).wrapping_add(x) & M62 }
fn cur(c: i64) -> Currency { [Currency::All, Currency::Eur, Currency::Usd][c as usize] }
/// Production error message -> the gate's reason code (order matters:
/// "denominator" before "negative", "overflow" covers neg/add/sub/mul/tax).
fn reason(msg: &str) -> i64 {
    if msg.contains("cross-currency") { 1 }
    else if msg.contains("denominator") { 3 }
    else if msg.contains("overflow") { 2 }
    else if msg.contains("cannot be negative") { 4 }
    else if msg.contains("rate must be > 0") { 5 }
    else { 9 }
}

struct St { h: i64, oks: i64, errs: i64 }
impl St {
    fn emit(&mut self, r: Result<i64, String>) {
        match r {
            Ok(v) => { self.h = mix(mix(self.h, 1), v); self.oks += 1; }
            Err(m) => { self.h = mix(mix(self.h, 0), reason(&m)); self.errs += 1; }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn case(st: &mut St, op: i64, a: i64, b: i64, ca: i64, cb: i64, rm: i64, m1: i64, m2: i64, nano: i64, s6: i64, flat: i64, mn: i64) {
    let q = (s6 % 20) - 2;
    let flags = s6 >> 8;
    let thr = b.abs();
    let rate = rm as f64 / 1_000_000.0;
    match op {
        1 => st.emit(Money::new(a, cur(ca)).checked_add(Money::new(b, cur(cb))).map(|m| m.minor)),
        2 => st.emit(Money::new(a, cur(ca)).checked_sub(Money::new(b, cur(cb))).map(|m| m.minor)),
        3 => st.emit(Money::new(a, cur(ca)).checked_neg().map(|m| m.minor)),
        4 => st.emit(compute_line_total(a, &[m1, m2], q)),
        5 => st.emit(apply_tax(a, rate, false)),
        6 => st.emit(apply_tax(a, rate, true)),
        7 => st.emit(convert_all_to_eur_cents(a, nano as f64 / 1_000_000_000.0)),
        9 => st.emit(assert_non_negative(a).map(|_| 0)),
        8 => {
            let cfg = OrderTotalConfig {
                fee: FeeConfig {
                    is_pickup: flags & 1 == 1,
                    free_delivery_threshold: if (flags >> 3) & 1 == 1 { Some(thr) } else { None },
                    delivery_fee_flat: if (flags >> 4) & 1 == 1 { Some(flat) } else { None },
                    has_distance_tiers: (flags >> 1) & 1 == 1,
                },
                tax_rate: rate,
                price_includes_tax: (flags >> 2) & 1 == 1,
                min_order_value: if (flags >> 5) & 1 == 1 { Some(mn) } else { None },
            };
            let e = estimate_order_total(a, &cfg);
            let mut h = mix(st.h, 1);
            for v in [
                e.fee_known as i64, e.delivery_fee.unwrap_or(0),
                e.tax_total.is_some() as i64, e.tax_total.unwrap_or(0),
                e.total.is_some() as i64, e.total.unwrap_or(0),
                e.min_not_met as i64,
            ] { h = mix(h, v); }
            st.h = h;
            st.oks += 1;
        }
        _ => unreachable!(),
    }
}

fn main() {
    let mut st = St { h: 0, oks: 0, errs: 0 };
    let (lo, hi) = (i64::MIN, i64::MAX);
    let hand: [[i64; 12]; 28] = [
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
    ];
    for c in hand {
        case(&mut st, c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8], c[9], c[10], c[11]);
    }
    let mut s = 4242;
    let mut d = || { s = lcg(s); s };
    for _ in 0..48 {
        let op = 1 + d() % 8;
        let a = d() % 2000000001 - 1000000000;
        let b = d() % 2000000001 - 1000000000;
        let ca = d() % 3;
        let cb = if d() % 4 == 0 { (ca + 1) % 3 } else { ca };
        let rm = d() % 500001;
        let m1 = d() % 2000001 - 1000000;
        let m2 = d() % 2000001 - 1000000;
        let nano = 1 + d() % 20000000;
        let s6 = d();
        let flat = d() % 100001;
        let mn = d() % 1000001;
        case(&mut st, op, a, b, ca, cb, rm, m1, m2, nano, s6, flat, mn);
    }
    println!("{}", (st.h % 1000000000) * 1000000 + st.oks * 1000 + st.errs);
}
