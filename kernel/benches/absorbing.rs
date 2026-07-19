//! absorbing — P80 (S1 §3.3-C1, R2 §4). Absorbing Markov fundamental-matrix benches.
//!
//! Two corrections vs. the original `criterion.rs::absorbing` entry (R2 doc errors):
//!   (1) TRUE cost is O(n⁴) — `fundamental_matrix` runs n iterations, each doing one
//!       O(n³) `matmul` (Neumann series N = I + Q + Q² + … + Q^{n−1}). The old comment
//!       claimed O(n³); that understated the cost by a factor of n.
//!   (2) The old comment falsely claimed this is "used by agentic decision gating". It is
//!       NOT — the order lifecycle is n=5 fixed (Pending→…→Delivered), zero production
//!       callers. R2 records this as a non-issue; the bench is coverage + tripwire only.
//!
//! Bench group contents:
//!   * `absorbing/cyclic_16`   — the original cyclic-transition bench (relabeled from `_16`)
//!                              which measures the PESSIMAL cyclic path (Q = ring + skip),
//!                              NOT the real DAG lifecycle.
//!   * `absorbing/lifecycle_5` — the REAL order-lifecycle DAG path (lifecycle_qr Q/R), the
//!                              shape that actually ships in the kernel.
//!   * `absorbing/dag_chain`   — sweep over DAG chain lengths {4, 8, 16, 32} (nilpotent Q,
//!                              finite Neumann sum) to keep the cost curve on record.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::absorbing::{expected_steps, fundamental_matrix};

fn absorbing(c: &mut Criterion) {
    let mut group = c.benchmark_group("absorbing");

    // PESSIMAL cyclic path (ring + skip) — relabeled from `fundamental_matrix_16`.
    let n = 16usize;
    let mut q_cyclic = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let j1 = (i + 1) % n;
        let j3 = (i + 3) % n;
        q_cyclic[i][j1] = 0.6;
        q_cyclic[i][j3] = 0.4;
    }
    group.bench_function("cyclic_16", |b| {
        b.iter(|| black_box(fundamental_matrix(&q_cyclic)))
    });

    // REAL order-lifecycle DAG path (Pending→Confirmed→Preparing→Ready→InDelivery).
    let t3 = 1.0 / 3.0;
    let lifecycle_q = vec![
        vec![0.0, t3, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.5, 0.0, 0.5],
        vec![0.0, 0.0, 0.0, 1.0, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 0.5],
        vec![0.0, 0.0, 0.0, 0.0, 0.0],
    ];
    group.bench_function("lifecycle_5", |b| {
        b.iter(|| {
            let nmat = fundamental_matrix(&lifecycle_q).expect("lifecycle DAG is nilpotent");
            black_box(expected_steps(&nmat))
        })
    });

    // DAG-chain sweep (nilpotent: each state → next, terminal at the end).
    for &k in &[4usize, 8, 16, 32] {
        let mut dag = vec![vec![0.0f64; k]; k];
        for i in 0..(k - 1) {
            dag[i][i + 1] = 1.0; // linear chain toward the terminal
        }
        group.bench_function(format!("dag_chain_{k}"), |b| {
            b.iter(|| black_box(fundamental_matrix(&dag)))
        });
    }

    group.finish();
}

criterion_group!(benches, absorbing);
criterion_main!(benches);
