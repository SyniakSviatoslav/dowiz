//! W-5 — solo-island re-anchored on the dowiz-kernel decider (DoD-5).
//!
//! Starts from a REAL kernel order with integer money (not a status shell),
//! drives the full lifecycle as signed frames through the spine (DOD -> WIRE
//! -> LAW -> MONEY), mirrors each result back into the kernel order, then runs
//! the settlement leg and asserts money conservation. Includes the three
//! re-anchored AC-2 adversarials: forged-skip, replay, expiry.

use bebop_delivery_domain::intake::{to_order_status, FoldError, IntakeEdge};
use dowiz_kernel::domain::{apply_event, place_order, OrderItem};
use dowiz_kernel::order_machine::OrderStatus;

const NOW: u64 = 1_000_000;

fn lifecycle(id: u64) -> Vec<(bebop_delivery_domain::DeliveryStatus, bebop_delivery_domain::DeliveryStatus)> {
    use bebop_delivery_domain::DeliveryStatus::*;
    vec![
        (Pending, Confirmed),
        (Confirmed, Preparing),
        (Preparing, Ready),
        (Ready, InDelivery),
        (InDelivery, Delivered),
    ]
        .into_iter()
        .map(|(from, to)| (from, to))
        .collect()
}

#[test]
fn solo_island_full_flow_from_dowiz_decider_with_money() {
    // 1. A REAL dowiz order with money.
    let items = vec![OrderItem {
        product_id: "espresso".into(),
        modifier_ids: vec![],
        quantity: 2,
        unit_price: 250,
        vendor_id: dowiz_kernel::vendor::VendorId(0),
        currency: dowiz_kernel::money::Currency::Eur,
    }];
    let mut order = place_order("ord-7".into(), None, items, 1_000, None, None).unwrap();
    let subtotal = order.subtotal; // kernel-computed, integer-exact

    // 2. The spine, zero peers: single-hub ring (R=0), one intake edge.
    let edge = IntakeEdge::new(0x42);
    let mut recv = edge.receiver();

    // 3. Drive the FULL lifecycle; fold each frame, mirror back into the order.
    for (from, to) in lifecycle(7) {
        let frame = edge.emit(7, from, to);
        let status = recv.admit_and_fold(&frame, &order, NOW).expect("solo fold");
        order = apply_event(&order, to_order_status(status)).expect("kernel fold");
    }
    assert_eq!(order.status, OrderStatus::Delivered);

    // 4. Settlement leg: conservation probe holds, integer-exact.
    order.post_earn(1, subtotal, dowiz_kernel::money::Currency::Eur).unwrap();
    assert_eq!(order.ledger_balance(), subtotal);
    assert_eq!(order.total, subtotal);
}

#[test]
fn adversarial_forged_skip_rejected_by_law() {
    let mut order = place_order(
        "ord-8".into(),
        None,
        vec![OrderItem {
            product_id: "x".into(),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: 100,
            vendor_id: dowiz_kernel::vendor::VendorId(0),
            currency: dowiz_kernel::money::Currency::Eur,
        }],
        1_000,
        None,
        None,
    )
    .unwrap();

    let edge = IntakeEdge::new(0x42);
    let mut recv = edge.receiver();

    // A VALIDLY signed frame with an ILLEGAL jump (Pending -> Delivered) must
    // be rejected by the LAW gate; the kernel order is untouched.
    let bad = edge.emit(8, bebop_delivery_domain::DeliveryStatus::Pending, bebop_delivery_domain::DeliveryStatus::Delivered);
    let res = recv.admit_and_fold(&bad, &order, NOW);
    assert!(matches!(res, Err(FoldError::Gate(_))), "forged skip must be refused by Law");
    assert_eq!(order.status, OrderStatus::Pending); // untouched
}

#[test]
fn adversarial_replay_rejected() {
    let mut order = place_order(
        "ord-9".into(),
        None,
        vec![OrderItem {
            product_id: "x".into(),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: 100,
            vendor_id: dowiz_kernel::vendor::VendorId(0),
            currency: dowiz_kernel::money::Currency::Eur,
        }],
        1_000,
        None,
        None,
    )
    .unwrap();

    let edge = IntakeEdge::new(0x42);
    let mut recv = edge.receiver();
    let frame = edge.emit(9, bebop_delivery_domain::DeliveryStatus::Pending, bebop_delivery_domain::DeliveryStatus::Confirmed);
    assert!(recv.admit_and_fold(&frame, &order, NOW).is_ok());
    // Second identical frame -> DOD replay set refuses it.
    let again = recv.admit_and_fold(&frame, &order, NOW);
    assert!(again.is_err(), "replay must be rejected");
}

#[test]
fn adversarial_expired_frame_rejected() {
    let mut order = place_order(
        "ord-10".into(),
        None,
        vec![OrderItem {
            product_id: "x".into(),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: 100,
            vendor_id: dowiz_kernel::vendor::VendorId(0),
            currency: dowiz_kernel::money::Currency::Eur,
        }],
        1_000,
        None,
        None,
    )
    .unwrap();

    let edge = IntakeEdge::new(0x42);
    let mut recv = edge.receiver();
    let frame = edge.emit(10, bebop_delivery_domain::DeliveryStatus::Pending, bebop_delivery_domain::DeliveryStatus::Confirmed);
    // now far past the capability expiry (9_999_999_999) -> refused before any kernel touch.
    let res = recv.admit_and_fold(&frame, &order, 20_000_000_000);
    assert!(res.is_err(), "expired frame must be rejected");
}
