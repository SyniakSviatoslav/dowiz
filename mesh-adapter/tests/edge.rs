//! W-1b — the dependency-edge smoke test (DoD-1).
//!
//! Constructs a real `IntakeEdge` (bebop kernel-rlib spine), emits one signed
//! `SignedFrame`, folds it through `DeliveryReceiver::admit_and_fold`, and maps
//! the result back via `to_order_status`. If the whole kernel-rlib closure does
//! NOT link from the dowiz side, this crate does not compile and `cargo test`
//! is RED. GREEN = the edge resolves and the smoke fold succeeds.

use bebop_delivery_domain::intake::{to_order_status, IntakeEdge};
use dowiz_kernel::order_machine::OrderStatus;
use dowiz_kernel::domain::{apply_event, place_order};

#[test]
fn edge_resolves_and_smoke_fold_succeeds() {
    let mut order = place_order(
        "ord-1".into(),
        None,
        vec![dowiz_kernel::domain::OrderItem {
            product_id: "espresso".into(),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: 250,
            vendor_id: dowiz_kernel::vendor::VendorId(0),
            currency: dowiz_kernel::money::Currency::Eur,
        }],
        1_000,
        None,
        None,
    )
    .expect("real kernel order");

    let edge = IntakeEdge::new(0x42);
    let mut recv = edge.receiver();

    // Pending -> Confirmed as a signed frame, folded through the WIRE->LAW->MONEY spine.
    let frame = edge.emit(1, bebop_delivery_domain::DeliveryStatus::Pending, bebop_delivery_domain::DeliveryStatus::Confirmed);
    let status = recv
        .admit_and_fold(&frame, &order, 1_000_000)
        .expect("solo fold");

    // Map back to the kernel enum and apply via the kernel Law.
    let next = to_order_status(status);
    order = apply_event(&order, next).expect("kernel fold");
    assert_eq!(order.status, OrderStatus::Confirmed);
}
