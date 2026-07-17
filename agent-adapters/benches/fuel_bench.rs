//! fuel_bench — B4 placeholder-calibration bench for `FUEL_PER_UNIT` (criterion, harness=false).
//!
//! ## What this bench measures
//! The cost of running the real prepaid-tranche loop `FuelTrancheRunner::run` over a
//! `DeterministicFuelMeter::needs(N)` for a range of guest `N` (1, 8, 64, 256 fuel units).
//! It uses `FuelTrancheRunner::new(FUEL_PER_UNIT, 1)` — a 1-unit tranche, i.e. the
//! exact-cost wiring that matches the B1 dispatch.rs path (each tranche loads exactly the
//! fuel for one budget unit). Criterion reports mean time / ops-per-sec per run.
//!
//! ## Why FUEL_PER_UNIT is a PLACEHOLDER (the B4 gap)
//! `FUEL_PER_UNIT` (= 100_000, kernel `ports::agent`) converts billing budget-units into
//! wasmtime CPU-fuel. It is a **B4-pending placeholder**: the REAL value must come from
//! measuring how much wasmtime fuel a *representative guest* consumes per budget-unit, which
//! is only observable behind the `wasmtime-fuel` feature (the real `WasmtimeFuelMeter`).
//! This bench deliberately runs on the deterministic in-crate meter — it establishes the
//! **loop-overhead baseline** (the `set_fuel` call + `run_slice` + bucket `try_acquire`
//! machinery) so a future B4 pass can subtract this fixed overhead and set `FUEL_PER_UNIT`
//! from a *measured* guest-fuel/unit ratio.
//!
//! ## Deterministic, NOT noisy
//! Per AGENTS.md this is a committed-baseline bench, not a host/noisy harness bench: the
//! meter is the deterministic reference impl (no wasmtime, no live daemon), so the numbers
//! are reproducible and gate-able by `benches/bench_track.py`. It does NOT touch the kernel
//! constant, dispatch.rs, or fuel.rs logic — it only exercises the public `run` API.

use agent_adapters::fuel::{DeterministicFuelMeter, FuelTrancheRunner};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::ports::agent::FUEL_PER_UNIT;
use dowiz_kernel::token_bucket::TokenBucket;

/// Guest fuel-unit sizes to sweep (a 1-unit tranche → one run_slice per unit, so N units
/// means N tranches of the loop). Covers tiny (1) up to chunky (256) guest costs.
const NEEDS: &[u64] = &[1, 8, 64, 256];

/// The loop-overhead baseline: run the real prepaid-tranche loop for a guest needing `n`
/// fuel units, on a bucket large enough that it never exhausts (we bench cost, not refusal).
fn bench_fuel_loop_throughput(c: &mut Criterion) {
    // Tranche = 1 unit → exact-cost wiring (matches B1 dispatch.rs). With a 1-unit tranche
    // and FUEL_PER_UNIT fuel loaded per tranche, a guest needing `n` units takes `n` tranches.
    let runner = FuelTrancheRunner::new(FUEL_PER_UNIT, 1);

    let mut group = c.benchmark_group("fuel_loop_throughput");
    for &n in NEEDS {
        // Effectively unbounded bucket so every guest completes within budget (pure cost).
        let bucket = TokenBucket::new(f64::MAX, 0.0);
        group.bench_function(format!("needs_{n}"), |b| {
            b.iter(|| {
                let mut guest = DeterministicFuelMeter::needs(n);
                black_box(runner.run(&mut guest, &bucket)).unwrap();
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_fuel_loop_throughput);
criterion_main!(benches);
