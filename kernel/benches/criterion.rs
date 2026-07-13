//! Baseline telemetry for the dowiz-kernel hot paths.
//! Run: `cargo bench -p dowiz-kernel` (or `cargo bench` from kernel/).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::{fold_transitions, place_order, OrderItem, OrderStatus};

fn bench_place_order(c: &mut Criterion) {
    c.bench_function("place_order/5_items", |b| {
        b.iter(|| {
            let items = vec![
                OrderItem {
                    product_id: "a".into(),
                    modifier_ids: vec![],
                    quantity: 2,
                    unit_price: 100,
                },
                OrderItem {
                    product_id: "b".into(),
                    modifier_ids: vec![],
                    quantity: 1,
                    unit_price: 250,
                },
                OrderItem {
                    product_id: "c".into(),
                    modifier_ids: vec![],
                    quantity: 3,
                    unit_price: 50,
                },
                OrderItem {
                    product_id: "d".into(),
                    modifier_ids: vec![],
                    quantity: 1,
                    unit_price: 500,
                },
                OrderItem {
                    product_id: "e".into(),
                    modifier_ids: vec![],
                    quantity: 4,
                    unit_price: 75,
                },
            ];
            black_box(place_order("o1".into(), None, items, 0, Some("web".into()), None).unwrap())
        })
    });
}

fn bench_fold_transitions(c: &mut Criterion) {
    // Legal path straight from order_machine's green test (Pending→…→Delivered).
    let path = [
        OrderStatus::Confirmed,
        OrderStatus::Preparing,
        OrderStatus::Ready,
        OrderStatus::InDelivery,
        OrderStatus::Delivered,
    ];
    c.bench_function("fold_transitions/5_hops", |b| {
        b.iter(|| black_box(fold_transitions(OrderStatus::Pending, &path).unwrap()))
    });
}

criterion_group!(benches, bench_place_order, bench_fold_transitions);
criterion_main!(benches);
