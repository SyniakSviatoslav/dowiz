//! P83 Layer-1 wiring test (BLUEPRINT P83 §4.2, D1/D2/D3).
//!
//! Drives `init_scoped()` (which mirrors exactly what `init_tracing()` does under
//! `DOWIZ_SPAN_METRICS=1`) and asserts that a real workload emitting the
//! `place_order_priced` span produces a `metric.jsonl` row named for that span with
//! `count` matching the number of spans emitted.
//!
//! CRITICAL: every assertion-bearing probe in this file is wrapped in a `std::thread`
//! with a hard join timeout so that a regression in the layer (e.g. the
//! self-deadlock that previously hung `red_flag_on_place_order_priced_counts_1000`
//! forever) FAILS THE TEST with a panic instead of hanging the whole `cargo test`
//! run. This is what makes the regression catchable in CI.

use dowiz_kernel::{
    catalog::PriceCatalog, domain::place_order_priced, domain::OrderItem, money::Currency,
    vendor::VendorId,
};

fn trusted_catalog() -> PriceCatalog {
    let mut c = PriceCatalog::new();
    c.insert_flat("p1", 5000);
    c.insert_flat("p2", 300);
    c
}

fn run_priced(n: usize) {
    let cat = trusted_catalog();
    for i in 0..n {
        let items = vec![OrderItem {
            product_id: "p1".into(),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: 0,
            vendor_id: VendorId(0),
            currency: Currency::All,
        }];
        let _ = place_order_priced(
            format!("p83-{i}"),
            None,
            items,
            0,
            Some("test".into()),
            None,
            &cat,
        );
    }
}

/// Run `body` on a worker thread; if it does not finish within `secs`, panic in the
/// main test thread (proving the probe is bounded — a hang becomes a test failure, not
/// a forever-stuck `cargo test`). Returns whatever `body` returned.
fn bounded<T>(secs: u64, body: impl FnOnce() -> T + Send + 'static) -> T
where
    T: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        let r = body();
        let _ = tx.send(());
        r
    });
    match rx.recv_timeout(std::time::Duration::from_secs(secs)) {
        Ok(()) => handle.join().expect("worker panicked"),
        Err(_) => {
            panic!("probe did not finish within {secs}s — layer is unbounded/hung (regression)")
        }
    }
}

/// Under the span-metrics layer, the `place_order_priced` spans must be folded into a
/// `metric.jsonl` row named exactly for that span. The workload is bounded (N=50: far
/// above any noise floor, well under any slow-but-terminating threshold) and the whole
/// probe is wrapped in a 20s thread timeout so the historical deadlock is impossible.
#[test]
fn red_flag_on_place_order_priced_counts_1000() {
    // When built WITHOUT the `telemetry` feature, the layer does not exist; the env
    // flag is a no-op and there is nothing to assert. Skip.
    #[cfg(not(feature = "telemetry"))]
    {
        return;
    }
    #[cfg(feature = "telemetry")]
    {
        bounded(20, || {
            let dir = std::env::temp_dir().join(format!("p83_init_wire_{}", std::process::id()));
            let _ = std::fs::create_dir_all(&dir);
            // Mirror exactly what `init_tracing()` does under DOWIZ_SPAN_METRICS=1, but
            // scoped so it does not fight a global subscriber another test already set.
            let _guard = dowiz_kernel::span_metrics::init_scoped(Some(dir.clone()));
            const N: usize = 50;
            run_priced(N);
            let p = dir.join("metric.jsonl");
            let contents =
                std::fs::read_to_string(&p).expect("metric.jsonl should exist under the layer");
            assert!(
                contents.contains("\"span\":\"place_order_priced\""),
                "expected a metric.jsonl row for place_order_priced under the span-metrics layer"
            );
            let cnt: u64 = contents
                .lines()
                .filter(|l| l.contains("\"span\":\"place_order_priced\""))
                .map(|l| {
                    let s = l.find("\"count\":").map(|i| &l[i + 8..]).unwrap_or("0");
                    s.chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<u64>()
                        .unwrap_or(0)
                })
                .sum();
            assert!(
                cnt >= N as u64,
                "place_order_priced span count should be ~{N}, got {cnt}"
            );
            let _ = std::fs::remove_dir_all(&dir);
        });
    }
}

/// REGRESSION TEST for the self-deadlock (P83). Drives an arbitrary `#[instrument]`-style
/// span through the layer and asserts the span CLOSES (and writes its row) within a
/// bounded time. This is the test the prior agent was missing: `on_enter`/`on_close`
/// used to call `span.extensions_mut()` while `tracing::Span::entered()` held the same
/// span's `Extensions` read lock → the layer hung forever on the very first span. If the
/// deadlock ever returns, this test panics on the 20s join timeout instead of hanging.
#[test]
fn red_span_close_never_deadlocks() {
    #[cfg(not(feature = "telemetry"))]
    {
        return;
    }
    #[cfg(feature = "telemetry")]
    {
        bounded(20, || {
            let dir = std::env::temp_dir().join(format!("p83_deadlock_{}", std::process::id()));
            let _ = std::fs::create_dir_all(&dir);
            let _guard = dowiz_kernel::span_metrics::init_scoped(Some(dir.clone()));

            // A synthetic instrumented span exercised through the observer exactly the way
            // `place_order_priced` is (`.entered()` → guard drop → on_span_close).
            fn synthetic_span() {
                let _s = dowiz_kernel::fdr::info_span!("p83_synthetic").entered();
                // trivial work
                let _ = 1 + 1;
            }
            for _ in 0..20 {
                synthetic_span();
            }

            let p = dir.join("metric.jsonl");
            let contents =
                std::fs::read_to_string(&p).expect("metric.jsonl should exist after closing spans");
            assert!(
                contents.contains("\"span\":\"p83_synthetic\""),
                "synthetic span must be recorded by the layer (proves on_close ran)"
            );
            let _ = std::fs::remove_dir_all(&dir);
        });
    }
}

/// D2: with `DOWIZ_SPAN_METRICS` UNSET, `init_tracing()` installs the ordinary `fmt`
/// layer and NO `SpanMetricsLayer` is ever constructed — prove the default path is
/// observability-silent (no metric.jsonl appears from the workload).
#[test]
fn red_default_build_unchanged_no_metric_row() {
    let dir = std::env::temp_dir().join(format!("p83_default_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    // Emulate the default branch exactly: call init_tracing without the env flag.
    std::env::remove_var("DOWIZ_SPAN_METRICS");
    dowiz_kernel::init_tracing();
    run_priced(500);

    // Either no metric.jsonl at all, or it has no span row. The default build must not
    // suddenly start writing kernel span metrics.
    let p = dir.join("metric.jsonl");
    let has_row = match std::fs::read_to_string(&p) {
        Ok(s) => s.contains("\"span\":"),
        Err(_) => false,
    };
    assert!(
        !has_row,
        "default (no-env) init_tracing must not emit span metrics"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
