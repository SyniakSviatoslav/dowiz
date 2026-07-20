//! W-4 — matcher + hub_ring consumption with determinism adversarials (DoD-4).
//!
//! Proves HRW assignment is deterministic across calls/permutations, input
//! sensitive (order_id binds the weight), and structurally free of courier
//! scoring (Courier has exactly one field, `pubkey`).

use bebop_delivery_domain::hub_ring::{self, Hub};
use bebop_proto_cap::event_dict::CourierKey;
use bebop_proto_cap::matcher::{self, Courier};

fn courier(byte: u8) -> Courier {
    Courier { pubkey: [byte; 32] }
}

#[test]
fn determinism_across_calls_and_permutations() {
    let order = bebop_proto_cap::matcher::Order {
        id: 4242,
        src: "R".into(),
        dst: "C".into(),
    };
    let cands = vec![courier(1), courier(2), courier(3), courier(4), courier(5)];

    let a = matcher::assign(&order, &cands, cands.len());
    let b = matcher::assign(&order, &cands, cands.len());
    assert_eq!(a, b, "HRW assignment must be identical across calls");

    // Permutation invariance: shuffle must not change the ranking.
    let mut shuffled = cands.clone();
    shuffled.rotate_left(2);
    let c = matcher::assign(&order, &shuffled, shuffled.len());
    assert_eq!(a, c, "HRW is invariant to candidate input order");
}

#[test]
fn input_sensitivity_teeth() {
    // order_id MUST bind the weight: a different id must re-rank for some fixture.
    let cands = vec![courier(10), courier(20), courier(30), courier(40)];
    let o1 = bebop_proto_cap::matcher::Order {
        id: 1,
        src: "R".into(),
        dst: "C".into(),
    };
    let o2 = bebop_proto_cap::matcher::Order {
        id: 2,
        src: "R".into(),
        dst: "C".into(),
    };
    let a = matcher::assign(&o1, &cands, cands.len());
    let b = matcher::assign(&o2, &cands, cands.len());
    assert_ne!(
        a, b,
        "assignment must depend on the order id (weight binds it)"
    );
}

#[test]
fn hub_ring_clamp_and_solo_island() {
    let hubs: Vec<Hub> = (0..7u8).map(|i| Hub::new([i; 32])).collect();
    // R=0 solo-island degenerate case: exactly one owner, no replicas.
    let own = hub_ring::owner_hub(42, &hubs);
    let ownership = hub_ring::assign(42, &hubs, 0);
    assert_eq!(ownership.owner, own);
    assert!(ownership.replicas.is_empty());

    // replica_count > hubs.len()-1 must clamp, never claim a phantom replica.
    let big = hub_ring::assign(42, &hubs, 99);
    assert_eq!(big.replicas.len(), hubs.len() - 1);
}

#[test]
fn no_courier_scoring_red_line() {
    // Structural: Courier has exactly one public field `pubkey`. If a scoring
    // field is ever added, this struct-literal construction fails to compile.
    let _c: Courier = Courier { pubkey: [0u8; 32] };
    let _k: CourierKey = [0u8; 32];
}
