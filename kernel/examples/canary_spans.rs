// canary_spans — periodic synthetic CANARY workload over the REAL P83 Layer-1
// instrumented kernel functions (pre-launch latency observability).
//
// WHAT THIS IS: real kernel calls with valid synthetic inputs, measured by the real
// `SpanMetricsObserver` at real wall-clock granularity — the same thing production
// systems run as a smoke/canary workload before live traffic exists. NOT fabricated
// numbers, NOT mocked calls.
//
// HONESTY DESIGN (canary vs live, never blended): this binary writes to
// `<DOWIZ_SPAN_METRICS_DIR>/canary/metric.jsonl` — a sibling STREAM under the same
// root the `topics latency`/`topics resources` readers resolve. The live stream
// (`<root>/metric.jsonl`, written by real order traffic once it exists) is a
// different file, so canary-sourced samples are structurally incapable of leaking
// into the live bucket. The golden-pinned `metric.jsonl` row format
// (kernel/src/span_metrics/obs.rs `golden_metric_row_exact_bytes`) is byte-identical
// in both streams — the SOURCE is the file, not a row mutation.
//
// LOG GROWTH: rows stream cumulatively (one row per span close), so a scheduled
// canary grows the file forever. Before installing the observer, this binary
// rotates WHOLE FILES: if `canary/metric.jsonl` exceeds DOWIZ_CANARY_ROTATE_BYTES
// (default 4 MiB) it is renamed to `metric.jsonl.1` (replacing the previous `.1`),
// and a fresh stream starts. Whole-file rotation is the only safe policy for the
// reader's cumulative-Δ reconstruction (`tools/telemetry/topics` —
// `reconstruct_durations`): a fresh file always begins with `count==1` rows, which
// the reader already treats as a (re)started stream. Head-truncation inside a file
// is never performed.
//
// All 8 P83 Layer-1 actions are exercised:
//   place_order / place_order_priced / fold_transitions  — native spans (call direct)
//   route / commit_after_decide / decide_settlement /
//   cap_verify_chain                                     — `instrument::` wrappers (telemetry)
//   mldsa_verify                                         — `instrument::` wrapper (pq)
//
// Run: DOWIZ_SPAN_METRICS_DIR=/root/ops/topics cargo run --release \
//        --features "telemetry pq" --example canary_spans -- [n_per_action]
// (n defaults to 50 — enough for stable p50/p99 upper bounds and a real jitter σ
// per run while keeping a scheduled run under ~2s of CPU.)

use dowiz_kernel::catalog::PriceCatalog;
use dowiz_kernel::event_log::{sha3_256, EventLog, MemEventStore, MeshEvent};
use dowiz_kernel::money::Currency;
use dowiz_kernel::order_machine::{fold_transitions, OrderStatus};
use dowiz_kernel::ports::agent::cap::RefSigner;
use dowiz_kernel::ports::agent::SignatureVerifier;
use dowiz_kernel::ports::payment::{
    build_courier_auth, CashAttestation, SettlementEvent, SettlementOutcome, SettlementState,
};
use dowiz_kernel::pq::dsa;
use dowiz_kernel::router::road_graph_from_ways;
use dowiz_kernel::span_metrics::instrument;
use dowiz_kernel::{place_order, OrderItem};

const DEFAULT_N: usize = 50;
const DEFAULT_ROTATE_BYTES: u64 = 4 * 1024 * 1024;

/// Whole-file rotation (never head-truncation — see module comment). Keeps at most
/// one prior generation, so disk usage is bounded at ~2× the cap forever.
fn rotate_if_oversized(dir: &std::path::Path) {
    let cap: u64 = std::env::var("DOWIZ_CANARY_ROTATE_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_ROTATE_BYTES);
    let live = dir.join("metric.jsonl");
    let len = match std::fs::metadata(&live) {
        Ok(m) => m.len(),
        Err(_) => return, // no file yet — nothing to rotate
    };
    if len <= cap {
        return;
    }
    let old = dir.join("metric.jsonl.1");
    match std::fs::rename(&live, &old) {
        Ok(()) => eprintln!(
            "canary_spans: rotated {} ({} bytes > cap {}) -> {}",
            live.display(),
            len,
            cap,
            old.display()
        ),
        Err(e) => eprintln!("canary_spans: rotation failed ({e}) — continuing, file keeps growing"),
    }
}

fn order_items() -> Vec<OrderItem> {
    vec![
        OrderItem {
            product_id: "p1".into(),
            modifier_ids: vec![],
            quantity: 2,
            unit_price: 5000,
            vendor_id: dowiz_kernel::vendor::VendorId(0),
            currency: Currency::All,
        },
        OrderItem {
            product_id: "p2".into(),
            modifier_ids: vec![],
            quantity: 1,
            unit_price: 300,
            vendor_id: dowiz_kernel::vendor::VendorId(0),
            currency: Currency::All,
        },
    ]
}

fn main() {
    // The canary REQUIRES an explicit root — a canary run that silently writes
    // nowhere is worse than a failed run (the cron log must show the failure).
    let root = match std::env::var_os("DOWIZ_SPAN_METRICS_DIR") {
        Some(d) => std::path::PathBuf::from(d),
        None => {
            eprintln!("canary_spans: DOWIZ_SPAN_METRICS_DIR is required (canary stream lands in <root>/canary/metric.jsonl)");
            std::process::exit(2);
        }
    };
    let dir = root.join("canary");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("canary_spans: cannot create {}: {e}", dir.display());
        std::process::exit(2);
    }
    rotate_if_oversized(&dir);

    match dowiz_kernel::span_metrics::init(Some(dir.clone())) {
        Ok(()) => eprintln!(
            "canary_spans: span observer installed -> {}",
            dir.join("metric.jsonl").display()
        ),
        Err(()) => eprintln!("canary_spans: an observer was already installed (ok)"),
    }

    let n: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_N);
    let mut ok = [0usize; 8];

    // 1) place_order — native span (legacy untrusted-price path).
    for i in 0..n {
        if place_order(
            format!("canary-{i}"),
            Some("canary-cust".into()),
            order_items(),
            1_784_500_000_000 + i as i64,
            Some("canary".into()),
            None,
        )
        .is_ok()
        {
            ok[0] += 1;
        }
    }

    // 2) place_order_priced — native span, catalog-authoritative pricing.
    let mut cat = PriceCatalog::new();
    cat.insert_flat("p1", 5000);
    cat.insert_flat("p2", 300);
    for i in 0..n {
        if dowiz_kernel::domain::place_order_priced(
            format!("canary-priced-{i}"),
            None,
            order_items(),
            1_784_500_000_000 + i as i64,
            Some("canary".into()),
            None,
            &cat,
        )
        .is_ok()
        {
            ok[1] += 1;
        }
    }

    // 3) fold_transitions — native span, full legal lifecycle fold.
    let steps = [
        OrderStatus::Confirmed,
        OrderStatus::Preparing,
        OrderStatus::Ready,
        OrderStatus::InDelivery,
        OrderStatus::Delivered,
    ];
    for _ in 0..n {
        if fold_transitions(OrderStatus::Pending, &steps).is_ok() {
            ok[2] += 1;
        }
    }

    // 4) route — telemetry wrapper; 20×20 grid so Dijkstra does real work.
    let side = 20usize;
    let mut nodes = Vec::with_capacity(side * side);
    let mut ways = Vec::new();
    for r in 0..side {
        for c in 0..side {
            nodes.push((41.32 + r as f64 * 1e-3, 19.82 + c as f64 * 1e-3));
            let idx = r * side + c;
            if c + 1 < side {
                ways.push((idx, idx + 1, 100.0 + ((r + c) % 7) as f64));
            }
            if r + 1 < side {
                ways.push((idx, idx + side, 100.0 + ((r * c) % 5) as f64));
            }
        }
    }
    let g = road_graph_from_ways(&nodes, &ways);
    for _ in 0..n {
        if instrument::route(&g, 0, side * side - 1, false, &[]).is_some() {
            ok[3] += 1;
        }
    }

    // 5) commit_after_decide — telemetry wrapper over a real in-memory event log.
    let mut log = EventLog::new(MemEventStore::default());
    for i in 0..n {
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [9u8; 32],
            actor_seq: i as u64,
            payload: format!("canary-commit-{i}").into_bytes(),
        };
        if instrument::commit_after_decide(&mut log, ev, |_e| Ok::<u8, String>(1)).is_ok() {
            ok[4] += 1;
        }
    }

    // 6) decide_settlement — telemetry wrapper; anchored RefSigner courier auth,
    //    place→deliver→attest per order, amount matching the fold-derived total.
    let auth = build_courier_auth(&RefSigner, &[11u8; 32], &[22u8; 32], 1_000);
    let cert_ref = sha3_256(&RefSigner.classical_public(&[22u8; 32]));
    let mut state = SettlementState::new();
    for i in 0..n {
        let oid = format!("canary-settle-{i}");
        state.fold_event(SettlementEvent::OrderPlaced {
            order_id: oid.clone(),
            total_i64: 10_300,
        });
        state.fold_event(SettlementEvent::OrderDelivered {
            order_id: oid.clone(),
        });
        let att = CashAttestation {
            order_id: oid,
            amount_i64: 10_300,
            courier_cert_ref: cert_ref,
            sig: vec![1, 2, 3],
        };
        if matches!(
            instrument::decide_settlement(&state, &att, &auth),
            SettlementOutcome::Recorded { .. }
        ) {
            ok[5] += 1;
        }
    }

    // 7) cap_verify_chain — telemetry wrapper, same anchored chain the settlement used.
    for _ in 0..n {
        if instrument::verify_chain(&RefSigner, auth.roster, auth.chain, auth.cap, auth.now).is_ok()
        {
            ok[6] += 1;
        }
    }

    // 8) mldsa_verify — pq wrapper; real ML-DSA-65 keygen/sign once, verify n×.
    let seed = [7u8; dsa::SEEDBYTES];
    let (pk, sk) = dsa::keygen(&seed);
    let msg = b"dowiz canary span probe";
    let rnd = [3u8; dsa::RNDBYTES];
    let sig = dsa::sign(&sk, msg, &rnd);
    for _ in 0..n {
        if instrument::mldsa_verify(&pk, msg, &sig) {
            ok[7] += 1;
        }
    }

    eprintln!(
        "canary_spans: n={n}/action — ok: place_order={} place_order_priced={} \
         fold_transitions={} route={} commit_after_decide={} decide_settlement={} \
         cap_verify_chain={} mldsa_verify={}",
        ok[0], ok[1], ok[2], ok[3], ok[4], ok[5], ok[6], ok[7]
    );
    // A canary that failed to exercise any action must fail loudly in the cron log.
    if ok.iter().any(|&k| k == 0) {
        eprintln!("canary_spans: ERROR — at least one action recorded zero successes");
        std::process::exit(1);
    }
}
