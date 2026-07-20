// Real Instant-measured p50/p99/p999 wall-clock for the order-placement hot path
// (`place_order`, the decide/fold core). `cargo run --release --example bench_p99` from
// kernel/. Complements kernel/benches/criterion.rs (Criterion's own statistical estimate)
// with an explicit percentile readout — the number this repo's benchmark-reporting asks for.
use dowiz_kernel::money::Currency;
use dowiz_kernel::vendor::VendorId;
use dowiz_kernel::{place_order, OrderItem};
use std::time::Instant;

fn main() {
    let n = 50_000usize;
    let mut durations_ns: Vec<u64> = Vec::with_capacity(n);
    for _ in 0..n {
        let items = vec![
            OrderItem {
                product_id: "a".into(),
                modifier_ids: vec![],
                quantity: 2,
                unit_price: 100,
                vendor_id: VendorId(0),
                currency: Currency::All,
            },
            OrderItem {
                product_id: "b".into(),
                modifier_ids: vec![],
                quantity: 1,
                unit_price: 250,
                vendor_id: VendorId(0),
                currency: Currency::All,
            },
            OrderItem {
                product_id: "c".into(),
                modifier_ids: vec![],
                quantity: 3,
                unit_price: 50,
                vendor_id: VendorId(0),
                currency: Currency::All,
            },
        ];
        let start = Instant::now();
        let _ = std::hint::black_box(place_order(
            "ord-bench".into(),
            Some("cust-bench".into()),
            std::hint::black_box(items),
            1_784_500_000_000,
            Some("web".into()),
            None,
        ));
        durations_ns.push(start.elapsed().as_nanos() as u64);
    }
    durations_ns.sort_unstable();
    let pct = |p: f64| -> u64 {
        let idx = ((p / 100.0) * (durations_ns.len() as f64 - 1.0)).round() as usize;
        durations_ns[idx]
    };
    let sum: u64 = durations_ns.iter().sum();
    println!("dowiz-kernel place_order/3_items — n={n} real Instant-measured wall-clock:");
    println!("  mean  = {:.1} ns", sum as f64 / n as f64);
    println!("  p50   = {} ns", pct(50.0));
    println!("  p90   = {} ns", pct(90.0));
    println!("  p99   = {} ns", pct(99.0));
    println!("  p999  = {} ns", pct(99.9));
    println!("  max   = {} ns", durations_ns[durations_ns.len() - 1]);
}
