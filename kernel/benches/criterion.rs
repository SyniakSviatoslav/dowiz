//! Baseline telemetry for the dowiz-kernel hot paths.
//! Run: `cargo bench -p dowiz-kernel` (or `cargo bench` from kernel/).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::absorbing;
use dowiz_kernel::attention;
use dowiz_kernel::cgraph::CGraph;
use dowiz_kernel::retrieval::ppr::Ppr;
use dowiz_kernel::retrieval::recall::PrimaryRecall;
use dowiz_kernel::spectral_cache::{canonical_content_address, slem_cached, DecompCache};
use dowiz_kernel::money::Currency;
use dowiz_kernel::token_bucket::TokenBucket;
use dowiz_kernel::vendor::VendorId;
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
                OrderItem {
                    product_id: "d".into(),
                    modifier_ids: vec![],
                    quantity: 1,
                    unit_price: 500,
                    vendor_id: VendorId(0),
                    currency: Currency::All,
                },
                OrderItem {
                    product_id: "e".into(),
                    modifier_ids: vec![],
                    quantity: 4,
                    unit_price: 75,
                    vendor_id: VendorId(0),
                    currency: Currency::All,
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

/// The F33 bounded-budget hot path: the Dispatcher calls `try_acquire` once per chat request.
/// This bench isolates the atomic acquire cost (refill + CAS) from any network/harvest work.
fn bench_token_bucket(c: &mut Criterion) {
    c.bench_function("token_bucket/try_acquire_permit", |b| {
        // A typical chat permit is its max_tokens (8). Capacity 64, refill 8/s keeps it satisfied.
        let bucket = TokenBucket::new(64.0, 8.0);
        b.iter(|| black_box(bucket.try_acquire(8.0)))
    });
}

/// T4/A5: content-addressed spectral cache hot path (P-A §6 / §10.4).
///
/// `slem_cached` builds a 10×10 tile once, then calls `slem_cached` TWICE on the
/// SAME `DecompCache` — the second call is a HIT (cached spectrum reused, no
/// `eigenvalues` solve). We time the pair so the amortised hit cost is visible
/// against the one-time solve cost.
fn bench_spectral_cache_slem_cached(c: &mut Criterion) {
    let tile: Vec<Vec<f64>> = (0..10)
        .map(|i| {
            (0..10)
                .map(|j| ((i * 7 + j * 3) % 11) as f64 + 1.0)
                .collect()
        })
        .collect();
    c.bench_function("spectral_cache/slem_cached_10x10_hit", |b| {
        b.iter(|| {
            let mut cache = DecompCache::new();
            let tile = dowiz_kernel::csr::NormalizedTile::from_dense(&tile);
            let first = slem_cached(&mut cache, &tile);
            // second call: same tile ⇒ identical content-address ⇒ HIT, no recompute.
            let second = slem_cached(&mut cache, &tile);
            black_box((first, second))
        })
    });
}

/// T4/A5: canonical content-address cost on a 32×32 dense tile (P-A §6 / §10.4).
///
/// Isolates the hashing/pivot-scaling path from the `eigenvalues` solve, so a
/// regression in the scale-invariant address computation shows up on its own.
fn bench_spectral_cache_canonical_address(c: &mut Criterion) {
    let tile: Vec<Vec<f64>> = (0..32)
        .map(|i| {
            (0..32)
                .map(|j| ((i * 13 + j * 5) % 17) as f64 + 1.0)
                .collect()
        })
        .collect();
    c.bench_function("spectral_cache/canonical_address_32x32", |b| {
        b.iter(|| black_box(canonical_content_address(&tile)))
    });
}

/// W5 (BumpArena): the graph/spectral rebuild-and-rank pass — `from_edges` →
/// `row_normalize` → `personalized_pagerank` — timed heap-vs-arena. The
/// arena serves the transient scratch from one region (O(1) reset between
/// passes); the heap path allocates per pass. The measured delta is the
/// blueprint's §3.3 authority; record it in `BENCH_HISTORY.md`.
fn bench_graph_rebuild_rank(c: &mut Criterion) {
    // n=1024 cycle+skip graph, nnz≈2n — the §3.3 benchmark shape.
    let n = 1024usize;
    let mut edges: Vec<(usize, usize, f64)> = Vec::with_capacity(2 * n);
    for i in 0..n {
        edges.push((i, (i + 1) % n, 1.0));
        edges.push((i, (i + 7) % n, 1.0));
    }
    let seed: Vec<f64> = (0..n).map(|i| ((i % 3) as f64) / 3.0).collect();

    c.bench_function("graph_rebuild_rank/heap", |b| {
        b.iter(|| {
            let g = dowiz_kernel::csr::Csr::from_edges(n, &edges);
            let a = g.row_normalize();
            black_box(a.personalized_pagerank(&seed, 0.15, 30))
        })
    });

    let mut arena = dowiz_kernel::arena::BumpArena::with_capacity(1 << 24);
    c.bench_function("graph_rebuild_rank/arena", |b| {
        b.iter(|| {
            arena.reset(); // O(1) — the whole point: one region, reset between passes.
            let g = dowiz_kernel::csr::Csr::from_edges_in(n, &edges, &arena);
            let a = g.row_normalize_in(&arena);
            black_box(
                a.personalized_pagerank_in(&seed, 0.15, 30, &arena)
                    .expect("arena sized for pass"),
            )
        })
    });
}

/// Blind-spot coverage: personalized PageRank (retrieval M3) is O(k·n^2) in the
/// transition matrix — unbounded graph size makes this the retrieval hot path
/// that was previously UNBENCHED. Bench at a realistic n so regressions surface.
fn bench_ppr(c: &mut Criterion) {
    let n = 32usize;
    // Deterministic row-stochastic transition matrix (ring + skip edges).
    let mut w = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let j1 = (i + 1) % n;
        let j7 = (i + 7) % n;
        w[i][j1] = 0.5;
        w[i][j7] = 0.5;
    }
    let ppr = Ppr::new(w);
    c.bench_function("ppr/rank_32x32_k20", |b| {
        b.iter(|| black_box(ppr.rank(0, 0.85, 20)))
    });
}

/// Blind-spot coverage: absorbing Markov fundamental matrix is O(n^3) — used by
/// agentic decision gating. Was UNBENCHED; regressions here are silent until a
/// large state space is hit. Bench at a modest n to anchor the baseline.
fn bench_absorbing(c: &mut Criterion) {
    let n = 16usize;
    // Q submatrix of transient-transition probabilities (row-stochastic-ish).
    let mut q = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let j1 = (i + 1) % n;
        let j3 = (i + 3) % n;
        q[i][j1] = 0.6;
        q[i][j3] = 0.4;
    }
    c.bench_function("absorbing/fundamental_matrix_16", |b| {
        b.iter(|| black_box(absorbing::fundamental_matrix(&q)))
    });
}

/// Blind-spot coverage: BM25+trigram fusion recall (W18 self-improvement loop).
/// Previously UNBENCHED despite being on the living-knowledge read path.
fn bench_retrieval_recall(c: &mut Criterion) {
    let recall = PrimaryRecall::new();
    c.bench_function("retrieval/recall_at_k_5", |b| {
        b.iter(|| black_box(recall.recall_at_k("pricing model computes subtotal delivery fee", 5)))
    });
}

/// Blind-spot coverage: attention matmul (O(n^2·d) core). UNBENCHED before.
fn bench_attention(c: &mut Criterion) {
    let m = 8usize;
    let d = 8usize;
    let q: Vec<Vec<f64>> = (0..m).map(|i| (0..d).map(|j| ((i + j) as f64) / (d as f64)).collect()).collect();
    let k: Vec<Vec<f64>> = (0..m).map(|i| (0..d).map(|j| ((i * 2 + j) as f64) / (d as f64)).collect()).collect();
    let v: Vec<Vec<f64>> = (0..m).map(|i| (0..d).map(|j| ((i + j * 3) as f64) / (d as f64)).collect()).collect();
    c.bench_function("attention/matmul_8x8", |b| {
        b.iter(|| black_box(attention::attention(&q, &k, &v)))
    });
}

criterion_group!(
    benches,
    bench_place_order,
    bench_fold_transitions,
    bench_empirical_identify,
    bench_token_bucket,
    bench_spectral_cache_slem_cached,
    bench_spectral_cache_canonical_address,
    bench_graph_rebuild_rank,
    bench_ppr,
    bench_absorbing,
    bench_retrieval_recall,
    bench_attention
);
criterion_main!(benches);
