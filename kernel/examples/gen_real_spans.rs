// Generates real P83 span-metrics data (metric.jsonl kernel_span rows) by exercising
// the kernel's own instrumented hot paths under a real SpanMetricsObserver — not
// synthetic/hardcoded numbers. Run: `DOWIZ_SPAN_METRICS_DIR=<dir> cargo run --release
// --features telemetry --example gen_real_spans -- <n>` (n defaults to 2000).
use dowiz_kernel::money::Currency;
use dowiz_kernel::vendor::VendorId;
use dowiz_kernel::{place_order, OrderItem};

fn main() {
    let dir = std::env::var_os("DOWIZ_SPAN_METRICS_DIR").map(std::path::PathBuf::from);
    if dir.is_none() {
        eprintln!("gen_real_spans: DOWIZ_SPAN_METRICS_DIR not set — nothing will be written");
    }
    match dowiz_kernel::span_metrics::init(dir) {
        Ok(()) => eprintln!("gen_real_spans: span observer installed"),
        Err(()) => eprintln!("gen_real_spans: an observer was already installed (ok)"),
    }

    let n: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);

    for i in 0..n {
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
        ];
        let _ = place_order(
            format!("ord-real-{i}"),
            Some("cust-real".into()),
            items,
            1_784_500_000_000 + i as i64,
            Some("web".into()),
            None,
        );
    }
    eprintln!("gen_real_spans: exercised place_order x{n} — real spans written if DOWIZ_SPAN_METRICS_DIR was set");
}
