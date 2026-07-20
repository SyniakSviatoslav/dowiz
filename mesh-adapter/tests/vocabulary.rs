//! W-2 — event-vocabulary mapping proof (DoD-3, first half).
//!
//! Round-trips every payload shape, proves wire<->kernel status maps are the
//! identity on the wire side, that kernel-only states fail-closed map to None,
//! and that the WIRE mirror legality table (`assert_status_transition`) stays
//! in lockstep with the kernel Law (`assert_transition`) across all 81 pairs.

use bebop_delivery_domain::intake::{from_order_status, to_order_status};
use bebop_proto_cap::event_dict::{
    ClaimPayload, DeliveryEvent, DeliveryStatus, LedgerPayload, OrderPlacedPayload,
    StatusChangedPayload,
};
use bebop_proto_cap::scope::{Action, Resource, Scope};
use dowiz_kernel::order_machine::assert_transition;
use dowiz_mesh_adapter::vocab_to_wire;

const WIRE_STATES: [DeliveryStatus; 9] = [
    DeliveryStatus::Pending,
    DeliveryStatus::Confirmed,
    DeliveryStatus::Preparing,
    DeliveryStatus::Ready,
    DeliveryStatus::InDelivery,
    DeliveryStatus::Delivered,
    DeliveryStatus::Rejected,
    DeliveryStatus::Cancelled,
    DeliveryStatus::PickedUp,
];

#[test]
fn round_trip_all_payload_shapes() {
    let op = OrderPlacedPayload {
        order_id: 11,
        amount_i64: 500,
        src: "R".into(),
        dst: "C".into(),
    };
    assert_eq!(OrderPlacedPayload::decode(&op.encode()).unwrap(), op);

    let sc = StatusChangedPayload {
        order_id: 11,
        from: DeliveryStatus::Confirmed,
        to: DeliveryStatus::Preparing,
    };
    assert_eq!(StatusChangedPayload::decode(&sc.encode()).unwrap(), sc);

    let cp = ClaimPayload {
        claim_id: 2,
        order_id: 11,
        courier: [0x07; 32],
    };
    assert_eq!(ClaimPayload::decode(&cp.encode()).unwrap(), cp);

    let lp = LedgerPayload {
        order_id: 11,
        amount_i64: 500,
    };
    assert_eq!(LedgerPayload::decode(&lp.encode()).unwrap(), lp);
}

#[test]
fn decode_dispatch_selects_correct_variant() {
    // Six delivery actions -> four payload variants, dispatched by scope.
    let op = OrderPlacedPayload {
        order_id: 1,
        amount_i64: 1,
        src: "R".into(),
        dst: "C".into(),
    };
    assert!(matches!(
        DeliveryEvent::decode(
            Scope::single(Resource::Order, Action::OrderPlaced),
            &op.encode()
        )
        .unwrap(),
        DeliveryEvent::OrderPlaced(_)
    ));

    let sc = StatusChangedPayload {
        order_id: 1,
        from: DeliveryStatus::Pending,
        to: DeliveryStatus::Confirmed,
    };
    assert!(matches!(
        DeliveryEvent::decode(
            Scope::single(Resource::Order, Action::OrderStatusChanged),
            &sc.encode()
        )
        .unwrap(),
        DeliveryEvent::StatusChanged(_)
    ));

    let cp = ClaimPayload {
        claim_id: 1,
        order_id: 1,
        courier: [0u8; 32],
    };
    assert!(matches!(
        DeliveryEvent::decode(
            Scope::single(Resource::Claim, Action::ClaimOffered),
            &cp.encode()
        )
        .unwrap(),
        DeliveryEvent::Claim(_)
    ));

    let lp = LedgerPayload {
        order_id: 1,
        amount_i64: 1,
    };
    assert!(matches!(
        DeliveryEvent::decode(
            Scope::single(Resource::Ledger, Action::SettlementRecorded),
            &lp.encode()
        )
        .unwrap(),
        DeliveryEvent::Settlement(_)
    ));
}

#[test]
fn status_map_is_identity_on_wire_side() {
    for d in WIRE_STATES {
        assert_eq!(
            from_order_status(to_order_status(vocab_to_wire(d))),
            Some(vocab_to_wire(d))
        );
    }
}

#[test]
fn kernel_only_states_map_to_none_fail_closed() {
    use dowiz_kernel::order_machine::OrderStatus;
    assert_eq!(from_order_status(OrderStatus::Scheduled), None);
    assert_eq!(from_order_status(OrderStatus::Refunding), None);
    assert_eq!(from_order_status(OrderStatus::CompensatedRefund), None);
}

#[test]
fn dual_legality_table_parity_sweep() {
    use bebop_proto_cap::event_dict::assert_status_transition;
    // 81 pairs: the wire mirror must agree with the kernel Law everywhere.
    for &from in WIRE_STATES.iter() {
        for &to in WIRE_STATES.iter() {
            let wire_ok = assert_status_transition(from, to).is_ok();
            let kernel_ok = assert_transition(
                to_order_status(vocab_to_wire(from)),
                to_order_status(vocab_to_wire(to)),
            )
            .is_ok();
            assert_eq!(
                wire_ok, kernel_ok,
                "legality table drift at {from:?} -> {to:?}: wire={wire_ok} kernel={kernel_ok}"
            );
        }
    }
}

#[test]
fn truncated_payload_decode_fails() {
    let op = OrderPlacedPayload {
        order_id: 1,
        amount_i64: 1,
        src: "R".into(),
        dst: "C".into(),
    };
    let mut bytes = op.encode();
    bytes.truncate(bytes.len() - 1);
    assert!(OrderPlacedPayload::decode(&bytes).is_err());
}

#[test]
fn mismatched_scope_fails_decode() {
    // Claim scope over LedgerPayload bytes -> fail-closed Err.
    let lp = LedgerPayload {
        order_id: 1,
        amount_i64: 1,
    };
    assert!(DeliveryEvent::decode(
        Scope::single(Resource::Claim, Action::ClaimOffered),
        &lp.encode()
    )
    .is_err());
}
