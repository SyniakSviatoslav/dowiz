//! Hard Truth suite — Phase-Zero Step 2 (Determinism Verification).
//!
//! These are PROPERTY tests (proptest), distinct from the example-based unit tests inside each
//! module. They assert universal laws over the UNBOUNDED part of the core — the `Lek(i64)` money
//! algebra, where the input space (2^64) makes exhaustive example tests impossible.
//!
//! Why the state machine is NOT here: `OrderStatus` is a FINITE relation (10×10 = 100 ordered
//! pairs), already asserted exhaustively in `order_status.rs::exhaustive_transition_table`. An
//! exhaustive enumeration of a finite relation IS a proof, not a sample — running proptest over it
//! would be theater. Property testing earns its keep precisely where enumeration cannot reach: the
//! open i64 money space. (This mirrors the Manifesto's own Level-3 note: formal/property methods buy
//! something only on the unbounded parts.)
//!
//! Layer map (PHASE-ZERO.md §4 Hard Truth suite):
//!   - Layer 1 Determinism — `fold(genesis, ops) == fold(genesis, ops)`, always.
//!   - Layer 2 Totality     — no operation ever panics; every path returns a `Result`.
//!   - Layer 3 Corridors    — non-negativity, conservation, and no-silent-wrap invariants.

use domain::{Lek, MoneyError};
use proptest::prelude::*;

/// A money operation — the "command" alphabet of the money state machine that exists TODAY (before
/// the order kernel's `decide`/`fold` is built in a later step). A fold of these over a genesis
/// amount is the smallest honest instance of the Manifesto's `transition(State, Command)` law.
#[derive(Debug, Clone, Copy)]
enum MoneyOp {
    Add(i64),
    Sub(i64),
    MulQty(i64),
}

/// Apply one operation. Pure, total (returns `Result`, never panics), side-effect-free.
fn step(acc: Lek, op: MoneyOp) -> Result<Lek, MoneyError> {
    match op {
        MoneyOp::Add(n) => acc.checked_add(Lek::new(n)?),
        MoneyOp::Sub(n) => acc.checked_sub(Lek::new(n)?),
        MoneyOp::MulQty(q) => acc.checked_mul_qty(q),
    }
}

/// Fold a command sequence into a final state, short-circuiting on the first error — exactly the
/// event-sourcing "replay the log" shape the Immutable Core is built around.
fn fold(genesis: Lek, ops: &[MoneyOp]) -> Result<Lek, MoneyError> {
    ops.iter().try_fold(genesis, |acc, &op| step(acc, op))
}

/// Non-negative amounts (the only representable `Lek`), for genesis + Add/Sub operands.
fn nonneg() -> impl Strategy<Value = i64> {
    0i64..=i64::MAX
}

/// The op alphabet. `MulQty` intentionally spans the FULL i64 range (including negatives and
/// `i64::MIN`) to exercise the negative-quantity guard and the `i64::MIN`-before-`.abs()` corridor.
fn any_op() -> impl Strategy<Value = MoneyOp> {
    prop_oneof![
        nonneg().prop_map(MoneyOp::Add),
        nonneg().prop_map(MoneyOp::Sub),
        any::<i64>().prop_map(MoneyOp::MulQty),
    ]
}

proptest! {
    // ─────────────────────────── Layer 1 — Determinism ───────────────────────────

    /// The roadmap's literal Step-2 goal: the same command sequence over the same genesis yields
    /// the identical result — every time, with no hidden clock/entropy/ordering influence. Because
    /// `fold` also returns rather than panics for EVERY generated sequence, this doubles as the
    /// Layer-2 totality witness.
    #[test]
    fn fold_is_deterministic(
        genesis in nonneg(),
        ops in prop::collection::vec(any_op(), 0..48),
    ) {
        let g = Lek::new(genesis).unwrap();
        let first = fold(g, &ops);
        let second = fold(g, &ops);
        prop_assert_eq!(first, second);
    }

    // ─────────────────────────── Layer 3 — Corridors ───────────────────────────

    /// Non-negativity corridor: no fold can ever land on a negative amount. If the sequence would
    /// dip below zero (or overflow), it is rejected as an `Err`, never a wrapped/negative `Lek`.
    #[test]
    fn fold_ok_result_is_never_negative(
        genesis in nonneg(),
        ops in prop::collection::vec(any_op(), 0..48),
    ) {
        if let Ok(final_amount) = fold(Lek::new(genesis).unwrap(), &ops) {
            prop_assert!(final_amount.minor_units() >= 0);
        }
    }

    /// `checked_add` conservation + no-silent-wrap: the result equals the exact i128 reference sum
    /// when it fits in i64, and is `Overflow` (never a wrapped value) when it does not.
    #[test]
    fn add_matches_reference_or_overflows(a in nonneg(), b in nonneg()) {
        let reference = a as i128 + b as i128;
        match Lek::new(a).unwrap().checked_add(Lek::new(b).unwrap()) {
            Ok(sum) => {
                prop_assert!(reference <= i64::MAX as i128);
                prop_assert_eq!(sum.minor_units() as i128, reference);
            }
            Err(MoneyError::Overflow { .. }) => prop_assert!(reference > i64::MAX as i128),
            Err(e) => prop_assert!(false, "add produced an unexpected error: {e:?}"),
        }
    }

    /// `checked_sub` corridor: succeeds iff the minuend is at least the subtrahend, and then equals
    /// the exact difference; otherwise it is `Negative` — never clamped-to-zero, never wrapped.
    #[test]
    fn sub_succeeds_iff_nonnegative_result(a in nonneg(), b in nonneg()) {
        match Lek::new(a).unwrap().checked_sub(Lek::new(b).unwrap()) {
            Ok(diff) => {
                prop_assert!(a >= b);
                prop_assert_eq!(diff.minor_units(), a - b);
            }
            Err(MoneyError::Negative(_)) => prop_assert!(a < b),
            Err(e) => prop_assert!(false, "sub produced an unexpected error: {e:?}"),
        }
    }

    /// `checked_mul_qty` corridor: any negative quantity (including `i64::MIN`) is rejected by the
    /// early guard BEFORE arithmetic; a non-negative quantity multiplies exactly or overflows —
    /// never wraps, never panics.
    #[test]
    fn mul_qty_guards_negatives_and_matches_reference(a in nonneg(), qty in any::<i64>()) {
        match Lek::new(a).unwrap().checked_mul_qty(qty) {
            Err(MoneyError::NegativeQuantity(q)) => {
                prop_assert_eq!(q, qty);
                prop_assert!(qty < 0);
            }
            Ok(product) => {
                prop_assert!(qty >= 0);
                let reference = a as i128 * qty as i128;
                prop_assert!(reference <= i64::MAX as i128);
                prop_assert_eq!(product.minor_units() as i128, reference);
            }
            Err(MoneyError::Overflow { .. }) => {
                prop_assert!(qty >= 0);
                prop_assert!(a as i128 * qty as i128 > i64::MAX as i128);
            }
            Err(e) => prop_assert!(false, "mul_qty produced an unexpected error: {e:?}"),
        }
    }

    // ─────────────────────────── Layer 2 — Totality (explicit) ───────────────────────────

    /// Construction is total over ALL of i64: every value either builds a non-negative `Lek` or is
    /// rejected as `Negative` — there is no input that panics or yields an invalid amount.
    #[test]
    fn new_is_total_over_all_i64(raw in any::<i64>()) {
        match Lek::new(raw) {
            Ok(l) => prop_assert!(l.minor_units() >= 0 && raw >= 0),
            Err(MoneyError::Negative(n)) => {
                prop_assert_eq!(n, raw);
                prop_assert!(raw < 0);
            }
            Err(e) => prop_assert!(false, "new produced an unexpected error: {e:?}"),
        }
    }
}
