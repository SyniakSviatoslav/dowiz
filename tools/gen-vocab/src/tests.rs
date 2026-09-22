//! What these tests are for: proving the generator READ the kernel.
//!
//! A generator with the twelve names typed into it would pass a test that only
//! checked the output contains twelve names, so every assertion below is
//! written against a `dowiz_core` call rather than against a literal — except
//! the two that are deliberately literal, which record the defect P4 exists to
//! close (`COMPENSATED_REFUND` absent from the hand copies) and would therefore
//! be meaningless if they were derived.

use crate::read::{currency_codes, lifecycle_states, read_kernel};
use crate::{emit, generate, ARTEFACT};
use dowiz_core::money::{Currency, MONEY_SCALE_MICRO};
use dowiz_core::order_machine::{assert_transition, OrderStatus, FSM_GOLDEN_SIGNATURE};

fn vocab() -> crate::read::Vocabulary {
    read_kernel().expect("the kernel refused to be read")
}

#[test]
fn every_status_comes_from_the_kernel() {
    let v = vocab();
    assert_eq!(v.statuses.len(), FSM_GOLDEN_SIGNATURE.vertices);
    // Not "the list is twelve long" but "each entry is a status the kernel
    // parses back to the value whose `as_str` produced it".
    for name in &v.statuses {
        let parsed = OrderStatus::from_str(name).expect("emitted a name the kernel cannot parse");
        assert_eq!(parsed.as_str(), *name);
    }
    let mut sorted = v.statuses.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), v.statuses.len(), "a status was emitted twice");
}

#[test]
fn the_sets_are_the_kernel_predicates() {
    let v = vocab();
    for s in lifecycle_states().unwrap() {
        let n = s.as_str();
        assert_eq!(v.terminal.contains(&n), s.is_terminal(), "TERMINAL {n}");
        assert_eq!(v.refused.contains(&n), !s.took_money(), "REFUSED {n}");
        assert_eq!(v.active.contains(&n), s.is_active(), "ACTIVE {n}");
    }
}

/// THE DEFECT, pinned. `['REJECTED', 'CANCELLED']` is what the owner console,
/// the storefront tracker, the sea and the kit each held by hand. It is a
/// STRICT subset of what the kernel says, and the missing member is the one a
/// refund produces. This assertion is literal on purpose: it is the record of
/// what was wrong, and deriving it would delete the record.
#[test]
fn the_hand_copies_were_short_by_the_refund() {
    let v = vocab();
    assert!(v.refused.contains(&"REJECTED"));
    assert!(v.refused.contains(&"CANCELLED"));
    assert!(
        v.refused.contains(&"COMPENSATED_REFUND"),
        "the generated REFUSED set no longer names the state the hand copies missed"
    );
    assert!(!OrderStatus::CompensatedRefund.took_money());
}

#[test]
fn the_transition_table_is_the_transition_table() {
    let v = vocab();
    let states = lifecycle_states().unwrap();
    let mut seen = 0usize;
    for (from, tos) in &v.next {
        let f = OrderStatus::from_str(from).unwrap();
        for to in &states {
            let allowed = assert_transition(f, *to).is_ok();
            assert_eq!(
                tos.contains(&to.as_str()),
                allowed,
                "NEXT[{from}] disagrees with assert_transition about {}",
                to.as_str()
            );
        }
        seen += tos.len();
    }
    assert_eq!(seen, FSM_GOLDEN_SIGNATURE.edges);
    assert_eq!(v.edges, FSM_GOLDEN_SIGNATURE.edges);
}

#[test]
fn the_scaffold_is_found_not_assumed() {
    let v = vocab();
    // Whatever the scaffold is, the kernel refuses every transition out of it.
    for name in &v.scaffold {
        let s = OrderStatus::from_str(name).unwrap();
        for t in lifecycle_states().unwrap() {
            if t != s {
                assert!(
                    assert_transition(s, t).is_err(),
                    "{name} -> {t:?} was allowed"
                );
            }
        }
    }
    // ...and something outside it is not refused for the scaffold reason.
    assert!(!v.scaffold.contains(&"PENDING"));
    assert!(assert_transition(OrderStatus::Pending, OrderStatus::Confirmed).is_ok());
}

#[test]
fn currencies_are_exhaustive_and_round_trip() {
    let codes = currency_codes().unwrap();
    for c in &codes {
        assert_eq!(Currency::from_code(c).unwrap().code(), *c);
    }
    // A code the kernel does NOT know must not be in the list, which is what
    // makes the probe a read rather than a wish.
    assert!(Currency::from_code("XXX").is_none());
    assert!(!codes.contains(&"XXX"));
    assert_eq!(vocab().money_scale_micro, MONEY_SCALE_MICRO);
}

#[test]
fn the_output_is_byte_deterministic() {
    assert_eq!(generate().unwrap(), generate().unwrap());
}

#[test]
fn the_module_says_it_is_generated_and_names_its_sources() {
    let js = generate().unwrap();
    assert!(js.starts_with("// GENERATED FILE — DO NOT EDIT."));
    assert!(js.contains("crates/dowiz-core/src/order_machine.rs"));
    assert!(js.contains("crates/dowiz-core/src/money.rs"));
    assert!(js.contains("tools/gen-vocab"));
    for name in &vocab().statuses {
        assert!(
            js.contains(&format!("'{name}'")),
            "{name} is not in the module"
        );
    }
}

/// An empty list must emit a legal `new Set()`, not `new Set([\n])`. Reached
/// the moment a kernel gains a predicate nothing satisfies yet — which is
/// exactly how `COMPENSATED_REFUND` entered the FSM.
#[test]
fn an_empty_set_is_still_javascript() {
    let v = crate::read::Vocabulary {
        statuses: vec!["A"],
        terminal: vec![],
        refused: vec![],
        active: vec![],
        scaffold: vec![],
        next: vec![("A", vec![])],
        edges: 0,
        currencies: vec!["ALL"],
        money_scale_micro: 1,
    };
    let js = emit::render(&v);
    assert!(js.contains("export const TERMINAL = new Set();"));
    assert!(js.contains("  A: [],"));
    assert!(!js.contains("new Set([\n]"));
}

/// The gate, as a test: the committed artefact IS what this generator emits.
/// `tools/gates/vocab.sh` says the same thing to CI; this says it to whoever
/// runs `cargo test` in this directory after touching the kernel.
#[test]
fn the_committed_file_is_not_stale() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(ARTEFACT);
    let on_disk = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert_eq!(
        on_disk,
        generate().unwrap(),
        "{ARTEFACT} is stale — regenerate it (see tools/gates/vocab.sh)"
    );
}
