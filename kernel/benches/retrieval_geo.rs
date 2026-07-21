//! retrieval_geo — P80 (S1 §3.3-C1). Surface the retrieval/geo hot paths that were
//! previously UNBENCHED: BM25 isolation (rank over a small corpus), route projection,
//! and harmonic centrality over a graph.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::geo::progress_along_route;
use dowiz_kernel::harmonic::harmonic_centrality;
use dowiz_kernel::retrieval::bm25::{Bm25, Document};

use dowiz_kernel::retrieval::bm25::tokenize;

fn retrieval_geo(c: &mut Criterion) {
    let mut group = c.benchmark_group("retrieval_geo");

    // ── BM25 isolate: rank a 12-doc corpus (P77 fixture scale) ──
    let corpus: Vec<Document> = (0..12)
        .map(|i| {
            Document::from_text(&format!(
                "doc {i} rust compiler optimization backend allocation register llvm codegen {i}"
            ))
        })
        .collect();
    let bm = Bm25::new(corpus);
    let query = tokenize("rust compiler optimization");
    group.bench_function("bm25_rank_12", |b| b.iter(|| black_box(bm.rank(&query))));

    // ── progress_along_route: project a point onto a 64-segment polyline ──
    let poly: Vec<(f64, f64)> = (0..64)
        .map(|i| {
            let t = i as f64 / 63.0;
            (41.0 + t * 0.1, 20.0 + t * 0.05)
        })
        .collect();
    let pos = (41.05, 20.025);
    group.bench_function("progress_along_route_64", |b| {
        b.iter(|| black_box(progress_along_route(&poly, pos)))
    });

    // ── harmonic_centrality: BFS-based, n=256 star+ring mix ──
    let n = 256usize;
    let mut edges = Vec::new();
    for i in 1..n {
        edges.push((0, i)); // star to hub 0
    }
    for i in 1..(n - 1) {
        edges.push((i, i + 1)); // ring among leaves
    }
    group.bench_function("harmonic_centrality_256", |b| {
        b.iter(|| black_box(harmonic_centrality(n, &edges)))
    });

    group.finish();
}

criterion_group!(benches, retrieval_geo);
criterion_main!(benches);
