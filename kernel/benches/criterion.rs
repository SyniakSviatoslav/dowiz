//! Baseline telemetry for the dowiz-kernel hot paths.
//! Run: `cargo bench -p dowiz-kernel` (or `cargo bench` from kernel/).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::cgraph::CGraph;
use dowiz_kernel::{
    empirical_identify, fold_transitions, place_order, sample_backdoor, OrderItem, OrderStatus,
};

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

/// The analytics-reducer hot path: observational SAMPLES → empirical joint →
/// P(y|do(x)) for the back-door confounded fixture. Splits sampling vs. the
/// identify+reduce pipeline so a regression in either shows up separately.
fn bench_empirical_identify(c: &mut Criterion) {
    let g = CGraph::new(
        vec![vec![1], vec![], vec![1, 0]], // X pa Z; Z root; Y pa Z,X
        vec![vec![], vec![], vec![]],
    )
    .unwrap();
    // Pre-materialize the sample matrix once; bench only the identify+reduce.
    let rows = sample_backdoor(20_000, 0xABCDEF);
    c.bench_function("empirical_identify/20k_samples", |b| {
        b.iter(|| black_box(empirical_identify(&rows, &[2], &[(0, 1)], &g).unwrap()))
    });
    // End-to-end: sampling + identify, the real inference cost.
    c.bench_function("empirical_identify/end_to_end_20k", |b| {
        b.iter(|| {
            let rows = sample_backdoor(20_000, 0xABCDEF);
            black_box(empirical_identify(&rows, &[2], &[(0, 1)], &g).unwrap())
        })
    });
}

criterion_group!(
    benches,
    bench_place_order,
    bench_fold_transitions,
    bench_empirical_identify
);
criterion_main!(benches);
