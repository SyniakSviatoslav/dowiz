//! Ask the kernel. Nothing in this file reads a `.rs` file as text.
//!
//! Every fact below is the value a public `dowiz_core` function RETURNED when
//! this binary called it, which is the whole point of P4: the fifteenth hand
//! copy of `OrderStatus` is prevented by there being something to copy from,
//! and a generator that grepped `order_machine.rs` would be that fifteenth copy
//! wearing a build step.
//!
//! WHAT IS NOT REACHABLE, said here rather than papered over. `LIFECYCLE_STATES`,
//! `allowed_next` and `is_scaffold` are private to the kernel, so this file does
//! not name them; it recovers the same three facts through the public surface
//! (`topological_order`, `assert_transition`, `TransitionError::code`) and then
//! CROSS-CHECKS the result against `FSM_GOLDEN_SIGNATURE`, which the kernel
//! pins by hand and re-derives in its own tests. Two independent routes to the
//! same vertex and edge counts; disagreement is a refusal, never a warning.

use dowiz_core::money::{Currency, MONEY_SCALE_MICRO};
use dowiz_core::order_machine::{
    assert_transition, fsm_graph_report, topological_order, verify_fsm_signature, OrderStatus,
    FSM_GOLDEN_SIGNATURE,
};

/// The emitted vocabulary, as strings, in the kernel's own declaration order.
pub struct Vocabulary {
    pub statuses: Vec<&'static str>,
    pub terminal: Vec<&'static str>,
    pub refused: Vec<&'static str>,
    pub active: Vec<&'static str>,
    pub scaffold: Vec<&'static str>,
    pub next: Vec<(&'static str, Vec<&'static str>)>,
    pub edges: usize,
    pub currencies: Vec<&'static str>,
    pub money_scale_micro: i128,
}

/// The twelve lifecycle states as real `OrderStatus` values.
///
/// `topological_order()` is the only public function that hands out every
/// vertex of the FSM; it returns them in Kahn order, so they are re-sorted by
/// the derived `Ord` (which is discriminant order, i.e. the order they are
/// written in the enum) to give the emitted file a stable, human-readable
/// sequence that does not move when an edge is added.
pub fn lifecycle_states() -> Result<Vec<OrderStatus>, String> {
    verify_fsm_signature()
        .map_err(|d| format!("the kernel's own FSM signature is stale ({d:?}) — fix that first"))?;
    let mut states = topological_order()
        .ok_or_else(|| "topological_order() is None: the lifecycle FSM has a cycle".to_string())?;
    states.sort();
    states.dedup();
    let live = fsm_graph_report().vertices;
    if states.len() != live || states.len() != FSM_GOLDEN_SIGNATURE.vertices {
        return Err(format!(
            "vertex count disagrees: topological_order gave {}, fsm_graph_report {}, golden {}",
            states.len(),
            live,
            FSM_GOLDEN_SIGNATURE.vertices
        ));
    }
    for s in &states {
        if OrderStatus::from_str(s.as_str()) != Some(*s) {
            return Err(format!("{:?} does not round-trip through its wire name", s));
        }
    }
    Ok(states)
}

/// Is `s` the scaffold the FSM refuses to enter or leave?
///
/// `assert_transition` answers `ScaffoldDisabledError` when EITHER end is a
/// scaffold, so a single probe would call every status scaffold (everything has
/// `Scheduled` as a potential `to`). The honest test is "every outward
/// transition is refused for that reason", which only a scaffold satisfies.
fn is_scaffold(s: OrderStatus, all: &[OrderStatus]) -> bool {
    all.iter()
        .filter(|t| **t != s)
        .all(|t| matches!(assert_transition(s, *t), Err(e) if e.code() == "ScaffoldDisabledError"))
}

/// Read the whole vocabulary out of the kernel.
pub fn read_kernel() -> Result<Vocabulary, String> {
    let states = lifecycle_states()?;
    let name = |s: &OrderStatus| s.as_str();
    let pick = |f: &dyn Fn(&OrderStatus) -> bool| -> Vec<&'static str> {
        states.iter().filter(|s| f(s)).map(name).collect()
    };

    // The transition table, recovered by asking `assert_transition` about all
    // n² ordered pairs. `from == to` is a refusal in the kernel (SameStatus), so
    // the diagonal drops out on its own and no special case is needed here.
    let mut next: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    let mut edges = 0usize;
    for from in &states {
        let mut to_list = Vec::new();
        for to in &states {
            if assert_transition(*from, *to).is_ok() {
                to_list.push(name(to));
                edges += 1;
            }
        }
        next.push((name(from), to_list));
    }
    if edges != FSM_GOLDEN_SIGNATURE.edges {
        return Err(format!(
            "edge count disagrees: probing assert_transition found {edges}, golden says {}",
            FSM_GOLDEN_SIGNATURE.edges
        ));
    }

    Ok(Vocabulary {
        statuses: states.iter().map(name).collect(),
        terminal: pick(&|s| s.is_terminal()),
        // The complement of `took_money`, named for what the clients call it.
        refused: pick(&|s| !s.took_money()),
        active: pick(&|s| s.is_active()),
        scaffold: pick(&|s| is_scaffold(*s, &states)),
        next,
        edges,
        currencies: currency_codes()?,
        money_scale_micro: MONEY_SCALE_MICRO,
    })
}

/// How long a currency code this generator is willing to look for.
/// ISO 4217 alphabetic codes are exactly three uppercase letters; the search
/// runs 1..=4 so that a shorter or longer member of `Currency` is FOUND rather
/// than silently dropped from the clients' list.
const MAX_CODE_LEN: usize = 4;

/// The currencies the kernel accepts, discovered by exhaustion.
///
/// `Currency` has no public iterator and no `ALL` array — `from_code` and
/// `code` are the entire public surface — so the set is recovered by offering
/// `from_code` every code in [A-Z]{1,4} (475,254 of them, ~0.1 s) and keeping
/// what it accepts. That is a total read of a total function, not a guess at a
/// list; if a member is added to the enum it appears here without this file
/// being touched. A code that does not round-trip through `code()` is a refusal
/// rather than an entry, because the clients key everything off `code()`.
pub fn currency_codes() -> Result<Vec<&'static str>, String> {
    let mut found: Vec<&'static str> = Vec::new();
    for len in 1..=MAX_CODE_LEN {
        for n in 0..26usize.pow(len as u32) {
            let mut buf = [0u8; MAX_CODE_LEN];
            let mut rest = n;
            for i in 0..len {
                buf[len - 1 - i] = b'A' + (rest % 26) as u8;
                rest /= 26;
            }
            let probe = core::str::from_utf8(&buf[..len]).expect("A-Z is ASCII");
            if let Some(c) = Currency::from_code(probe) {
                if c.code() != probe {
                    return Err(format!(
                        "Currency::from_code({probe:?}) gave {:?}, whose code() is {:?}",
                        c,
                        c.code()
                    ));
                }
                if !found.contains(&c.code()) {
                    found.push(c.code());
                }
            }
        }
    }
    if found.is_empty() {
        return Err("no currency survived the probe — the search space is wrong".to_string());
    }
    found.sort_unstable();
    Ok(found)
}
