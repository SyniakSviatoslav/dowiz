//! spectral_math — P80 (S1 §3.3-C1). Surface the kernel's spectral/linear-algebra
//! hot paths that were previously UNBENCHED.
//!
//! * `eigenvalues` sweep straddling the n=32 QR↔Faddeev dispatch boundary: {8, 16, 32,
//!   48} (n ≤ 32 uses householder QR; n > 32 falls back to char-poly + Durand-Kerner).
//! * `matmul_contig` — the single contiguous matrix product (P79 surface).
//! * `kalman` predict / update on a 4-D constant-velocity filter.
//! * `classify_drift` — the DMD |μ|-vs-1 drift classifier.
//! * `laplacian_spmv` — CSR Laplacian matrix-vector product (spectral_laplacian surface).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::csr::{Csr, LaplacianKind};
use dowiz_kernel::kalman::KalmanFilter;
use dowiz_kernel::mat::Mat;
use dowiz_kernel::spectral::{classify_drift, eigenvalues};

/// Build a dense symmetric-ish matrix of size n (deterministic, well-conditioned).
fn dense_matrix(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| ((i * 13 + j * 7 + i * j) % 11) as f64 / 11.0 + if i == j { 1.0 } else { 0.0 })
                .collect()
        })
        .collect()
}

fn spectral_math(c: &mut Criterion) {
    let mut group = c.benchmark_group("spectral_math");

    // ── eigenvalues straddling the n=32 QR↔Faddeev dispatch ──
    for &n in &[8usize, 16, 32, 48] {
        let a = dense_matrix(n);
        group.bench_function(format!("eigenvalues_{n}"), |b| {
            b.iter(|| black_box(eigenvalues(&a)))
        });
    }

    // ── matmul_contig ──
    for &n in &[16usize, 32, 64] {
        let ma = Mat::from_vecvec(&dense_matrix(n));
        let mb = Mat::from_vecvec(&dense_matrix(n));
        group.bench_function(format!("matmul_contig_{n}"), |b| {
            b.iter(|| black_box(dowiz_kernel::mat::matmul_contig(&ma, &mb)))
        });
    }

    // ── kalman predict / update (4-D constant-velocity model) ──
    let p0 = Mat::from_vecvec(&vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0], vec![0.0, 0.0, 0.0, 1.0]]);
    let f = Mat::from_vecvec(&vec![
        vec![1.0, 0.0, 1.0, 0.0], vec![0.0, 1.0, 0.0, 1.0],
        vec![0.0, 0.0, 1.0, 0.0], vec![0.0, 0.0, 0.0, 1.0]]);
    let h = Mat::from_vecvec(&vec![
        vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0]]);
    let q = Mat::from_vecvec(&vec![vec![0.01, 0.0, 0.0, 0.0], vec![0.0, 0.01, 0.0, 0.0],
        vec![0.0, 0.0, 0.01, 0.0], vec![0.0, 0.0, 0.0, 0.01]]);
    let r = Mat::from_vecvec(&vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    let mut kf = KalmanFilter::new(vec![0.0, 0.0, 0.0, 0.0], p0, f, h, q, r);
    group.bench_function("kalman_predict", |b| b.iter(|| kf.predict()));
    let z = [1.0f64, 2.0];
    group.bench_function("kalman_update", |b| {
        b.iter(|| {
            let mut kf = KalmanFilter::new(vec![0.0, 0.0, 0.0, 0.0],
                Mat::from_vecvec(&vec![vec![1.0,0.0,0.0,0.0],vec![0.0,1.0,0.0,0.0],
                    vec![0.0,0.0,1.0,0.0],vec![0.0,0.0,0.0,1.0]]),
                Mat::from_vecvec(&vec![vec![1.0,0.0,1.0,0.0],vec![0.0,1.0,0.0,1.0],
                    vec![0.0,0.0,1.0,0.0],vec![0.0,0.0,0.0,1.0]]),
                Mat::from_vecvec(&vec![vec![1.0,0.0,0.0,0.0],vec![0.0,1.0,0.0,0.0]]),
                Mat::from_vecvec(&vec![vec![0.01,0.0,0.0,0.0],vec![0.0,0.01,0.0,0.0],
                    vec![0.0,0.0,0.01,0.0],vec![0.0,0.0,0.0,0.01]]),
                Mat::from_vecvec(&vec![vec![1.0,0.0],vec![0.0,1.0]]));
            black_box(kf.update(&z))
        })
    });

    // ── classify_drift ──
    let drift_op = dense_matrix(16);
    group.bench_function("classify_drift_16", |b| {
        b.iter(|| black_box(classify_drift(&drift_op)))
    });

    // ── laplacian_spmv (ring graph, n=256) ──
    let n = 256usize;
    let mut edges = Vec::new();
    for i in 0..n {
        edges.push((i, (i + 1) % n, 1.0));
        edges.push(((i + 1) % n, i, 1.0));
    }
    let csr = Csr::from_edges(n, &edges);
    let x = vec![1.0f64; n];
    let mut out = vec![0.0f64; n];
    group.bench_function("laplacian_spmv_256", |b| {
        b.iter(|| {
            let mut o = out.clone();
            csr.laplacian_spmv(&x, &mut o, LaplacianKind::Unnormalized);
            black_box(o)
        })
    });

    group.finish();
}

criterion_group!(benches, spectral_math);
criterion_main!(benches);
