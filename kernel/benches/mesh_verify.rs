//! mesh_verify — P80 (S1 §3.3-C1). Mesh signature-chain verification sweep.
//!
//! The `mesh` module is itself `pq`-gated (ML-DSA-65 signing), so this bench is too.
//! It builds a signed `MeshLog` of N entries ONCE (signing cost paid once), then benches
//! ONLY `verify_chain` so the per-entry verification cost is isolated as a function of
//! chain length {1, 8, 64, 256}.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::mesh::{MeshLog, MlDsaSigner, Signer};

fn build_chain(n: usize) -> MeshLog {
    let mut seed = [0u8; 32];
    seed[0] = (n & 0xFF) as u8;
    let signer = MlDsaSigner::from_seed(&seed, [0u8; 32]);
    let mut log = MeshLog::new();
    for i in 0..n {
        let payload = format!("mesh event {i}").into_bytes();
        log.append(&payload, &signer);
    }
    log
}

fn mesh_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_verify");
    for &n in &[1usize, 8, 64, 256] {
        let log = build_chain(n);
        group.bench_function(format!("chain_{n}"), |b| {
            b.iter(|| black_box(log.verify_chain()))
        });
    }
    group.finish();
}

criterion_group!(benches, mesh_verify);
criterion_main!(benches);
