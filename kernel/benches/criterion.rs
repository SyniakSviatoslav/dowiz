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

/// Layer-E hot-path telemetry: batched N-courier Kalman SoA step vs scalar loop.
/// Run: `cargo bench -p dowiz-kernel --bench criterion kalman`. The measured
/// speedup (AVX2 lane over scalar reference) is the §13.2.c DoD number; it is
/// recorded in the bench output, not asserted in `cargo test` (perf in unit CI
/// is flaky — correctness is asserted in simd.rs's parity tests instead).
fn bench_kalman_batch(c: &mut Criterion) {
    use dowiz_kernel::kalman::CourierKalman;
    use dowiz_kernel::simd::{kalman_batch_step, kalman_batch_step_scalar};

    let make = |n: usize| -> (Vec<CourierKalman>, Vec<Option<(f64, f64)>>, f64) {
        let dt = 1.0;
        let mut couriers = Vec::with_capacity(n);
        let mut obs = Vec::with_capacity(n);
        for i in 0..n {
            let c = CourierKalman::new(
                i as f64,
                (i * 7) as f64,
                100.0,
                1e-3,
                1e-3,
                4.0,
            );
            couriers.push(c);
            obs.push(Some(((i as f64) + 0.3, (i as f64 * 7.0) - 0.2)));
        }
        (couriers, obs, dt)
    };

    for &n in &[4usize, 32, 256] {
        let (couriers, obs, dt) = make(n);
        let name = format!("kalman_batch_step/avx2_n{n}");
        c.bench_function(&name, |b| {
            b.iter(|| {
                let mut cs = couriers.clone();
                kalman_batch_step(black_box(&mut cs), dt, black_box(&obs));
            })
        });

        let (couriers, obs, dt) = make(n);
        let name = format!("kalman_batch_step/scalar_n{n}");
        c.bench_function(&name, |b| {
            b.iter(|| {
                let mut cs = couriers.clone();
                kalman_batch_step_scalar(black_box(&mut cs), dt, black_box(&obs));
            })
        });
    }
}

criterion_group!(
    benches,
    bench_place_order,
    bench_fold_transitions,
    bench_kalman_batch
);
criterion_main!(benches);
