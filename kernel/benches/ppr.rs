//! ppr — P80 (S1 §3.3-C1, R1 §4). Personalized-PageRank growth sweep at α = 0.15
//! (the kernel's default damping), n ∈ {32, 128, 256}.
//!
//! REVISIT THRESHOLD (written deliberately, R1 §3a): revisit the dense PPR
//! implementation only if a real diffusion graph exceeds ~256 nodes OR
//! `ppr::rank_*` exceeds ~50µs at this α. Until then the dense power-iteration
//! is correct and deterministic (bit-reproducible, fixed K, fixed summation
//! order); approximate methods (Forward-Push / MC / FAST-PPR) are REJECTED
//! (S1 E5) because they break determinism.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::retrieval::ppr::Ppr;

/// Deterministic ring + skip transition matrix (row-stochastic).
fn transition(n: usize) -> Vec<Vec<f64>> {
    let mut w = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let j1 = (i + 1) % n;
        let j7 = (i + 7) % n;
        w[i][j1] = 0.5;
        w[i][j7] = 0.5;
    }
    w
}

fn ppr(c: &mut Criterion) {
    let mut group = c.benchmark_group("ppr");
    for &n in &[32usize, 128, 256] {
        let w = transition(n);
        let ppr = Ppr::new(w);
        group.bench_function(format!("rank_{n}_a0.15_k20"), |b| {
            b.iter(|| black_box(ppr.rank(0, 0.15, 20)))
        });
    }
    group.finish();
}

criterion_group!(benches, ppr);
criterion_main!(benches);
