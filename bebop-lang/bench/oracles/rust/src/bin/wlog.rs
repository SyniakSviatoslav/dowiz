//! B8 oracle for the dowiz integration row: replays wlog.bp's event stream and takes EVERY
//! transition decision from PRODUCTION `dowiz_core::order_machine::assert_transition`.
//!
//! The adjacency masks are DERIVED from assert_transition rather than copied from
//! selfhost/std/wlog.bp, so if bebop's table and dowiz-core's lifecycle rules ever disagree the
//! generated walk itself diverges and every fold below moves. Only the harness -- the LCG, the
//! 1% injection rule, the state numbering -- lives here; the decide is dowiz-core's.
use dowiz_core::order_machine::*;
use OrderStatus::*;

const S: [OrderStatus; 12] = [Pending, Confirmed, Preparing, Ready, InDelivery, Delivered,
    Rejected, Cancelled, Scheduled, PickedUp, Refunding, CompensatedRefund];

fn lcg(s: i64) -> i64 { (s.wrapping_mul(1103515245).wrapping_add(12345)) & 2147483647 }

fn code(e: &TransitionError) -> i64 {
    match e {
        TransitionError::SameStatus(_) => 1,
        TransitionError::ScaffoldDisabled(_, _) => 2,
        TransitionError::Illegal(_, _) => 3,
        TransitionError::Invalid(_) => 4,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: i64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10000);
    let k: i64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);

    // adjacency straight out of dowiz-core
    let mut adj = [0i64; 12];
    for f in 0..12 {
        for t in 0..12 {
            if assert_transition(S[f], S[t]).is_ok() { adj[f] |= 1 << t; }
        }
    }

    let mut lc = 1234i64;
    let (mut state_sum, mut injected, mut revenue) = (0i64, 0i64, 0i64);
    let mut counts = [0i64; 5]; // index 0 = ok, 1..4 = TransitionError codes
    let mut hist = [0i64; 12];
    let mut rev_by_state = [0i64; 12];

    for _oid in 0..n {
        let mut cur: usize = 0;
        let mut sub = 0i64;
        for _ek in 0..k {
            if adj[cur] == 0 { cur = 0; }
            let mask = adj[cur];
            lc = lcg(lc);
            let lv = lc;
            let ill = if lv % 100 == 0 { 1 } else { 0 };
            let selfbit = 1i64 << cur;
            let forb = ((mask ^ 4095) & (selfbit ^ 4095)) & 3839;
            let pool = if ill == 1 { forb } else { mask };
            let np = pool.count_ones() as i64;
            let r = if np == 0 { 0 } else { (lv / 100) % np };
            let nxt = (0..12).filter(|j| (pool >> j) & 1 == 1).nth(r as usize).unwrap_or(0);
            match assert_transition(S[cur], S[nxt]) {
                Ok(()) => { counts[0] += 1; cur = nxt; }
                Err(e) => { counts[code(&e) as usize] += 1; }
            }
            let amt = (lv % 100) + 1;
            revenue += amt;
            sub += amt;
            injected += ill;
        }
        state_sum += cur as i64;
        hist[cur] += 1;
        rev_by_state[cur] += sub;
    }

    println!("adj {}", adj.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(","));
    println!("q1 {}", state_sum);
    println!("injected {}", injected);
    println!("ok {}", counts[0]);
    println!("same {}", counts[1]);
    println!("scaffold {}", counts[2]);
    println!("illegal {}", counts[3]);
    println!("q4_total {}", revenue);
    println!("q2_hist {}", hist.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","));
    println!("q4_by_state {}", rev_by_state.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","));
}
