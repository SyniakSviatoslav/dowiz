//! T66 oracle for gate `ordfsm` — calls PRODUCTION `dowiz_core::order_machine`
//! (assert_transition / fold_transitions / the graph analyses / the golden
//! signature) on the event table of selfhost/std/ordfsm.bp and prints the same
//! fold. Only the harness (state numbering, LCG event table, error codes, mix)
//! lives here; every decide/fold/graph result comes from order_machine.rs.
use dowiz_core::order_machine::*;
use OrderStatus::*;

const M62: i64 = (1 << 62) - 1;
fn lcg(s: i64) -> i64 { (s.wrapping_mul(1103515245).wrapping_add(12345)) & 2147483647 }
fn mix(h: i64, x: i64) -> i64 { h.wrapping_mul(1000003).wrapping_add(x) & M62 }
/// idx_of order (LIFECYCLE_STATES).
const S: [OrderStatus; 12] = [Pending, Confirmed, Preparing, Ready, InDelivery, Delivered,
    Rejected, Cancelled, Scheduled, PickedUp, Refunding, CompensatedRefund];
fn idx(s: OrderStatus) -> i64 { S.iter().position(|&x| x == s).unwrap() as i64 }
/// Loud error codes: forbidden transitions are errors, never no-ops.
fn code(e: &TransitionError) -> i64 {
    match e {
        TransitionError::SameStatus(_) => 1,
        TransitionError::ScaffoldDisabled(_, _) => 2,
        TransitionError::Illegal(_, _) => 3,
        TransitionError::Invalid(_) => 4,
    }
}

const HAND: [i64; 16] = [257176080, 16331280, 1004560, 1030672, 3936, 3952, 4221841936, 3840,
    3968, 3848, 987664, 3845, 4199821840, 3872, 4011, 1026576];

fn main() {
    let (mut h, mut oks, mut errs) = (0i64, 0i64, 0i64);
    // A: exhaustive decide matrix.
    for f in S {
        for t in S {
            match assert_transition(f, t) {
                Ok(()) => { h = mix(h, 0); oks += 1; }
                Err(e) => { h = mix(h, code(&e)); errs += 1; }
            }
        }
    }
    let mut fold = |start: usize, steps: &[OrderStatus]| match fold_transitions(S[start], steps) {
        Ok(fin) => { h = mix(mix(h, 1), idx(fin)); oks += 1; }
        Err((e, reached)) => { h = mix(mix(h, 0), code(&e) * 16 + idx(reached)); errs += 1; }
    };
    // B1: 16 hand-picked event sequences (packed nibbles: start, steps.., 15 sentinel):
    // happy paths (delivery, pickup, direct delivery, refund x2, reject, cancel,
    // late refund), then same-status, scaffold both ways, reopen, from-terminal,
    // refund-after-delivered, skip-confirm, backwards refund, same-status mid-fold.
    for mut v in HAND {
        let start = (v & 15) as usize;
        v >>= 4;
        let mut steps = Vec::new();
        while v & 15 != 15 { steps.push(S[(v & 15) as usize]); v >>= 4; }
        fold(start, &steps);
    }
    // B2: 64 LCG event sequences (seed 1234), first error stops the fold.
    let mut s = 1234;
    let mut d = || { s = lcg(s); s };
    for _ in 0..64 {
        let start = (d() % 12) as usize;
        let len = 1 + d() % 4;
        let mut prev = start;
        let mut steps = Vec::new();
        for _ in 0..len {
            let r = d();
            let nxt = if r % 4 == 0 { ((r >> 4) % 12) as usize } else { (prev + 1 + ((r >> 4) % 2) as usize) % 12 };
            steps.push(S[nxt]);
            prev = nxt;
        }
        fold(start, &steps);
    }
    // C: structural signature of the lifecycle graph.
    let r = fsm_graph_report();
    for v in [r.vertices as i64, r.edges as i64, r.is_acyclic as i64, r.cyclomatic as i64,
              (spectral_radius() * 1_000_000.0) as i64, r.reachable_from_pending as i64,
              r.reachable_states as i64, r.topological_len.map(|n| n as i64).unwrap_or(-1)] {
        h = mix(h, v);
    }
    for st in S { h = mix(h, reachable(st) as i64); }
    if let Some(order) = topological_order() { for st in order { h = mix(h, idx(st)); } }
    h = mix(h, verify_fsm_signature().is_ok() as i64);
    println!("{}", (h % 1000000000) * 1000000 + oks * 1000 + errs);
}
