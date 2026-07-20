//! fuel.rs — the Wasmtime fuel ↔ `TokenBucket` prepaid-tranche loop (blueprint §2.2).
//!
//! Fuel is NOT 1:1 with budget units: fuel meters CPU instructions, units meter billing.
//! One pinned constant `FUEL_PER_UNIT` converts (kernel `ports::agent::FUEL_PER_UNIT`,
//! a **B4-pending placeholder** — see below). Execution is prepaid, tranche-wise: before
//! each guest slice the host `bucket.try_acquire(TRANCHE_UNITS)`; on grant it loads
//! `TRANCHE_UNITS × FUEL_PER_UNIT` fuel and resumes; on the out-of-fuel trap it attempts
//! the next tranche; if `try_acquire` returns `false`, the instance is TERMINATED with a
//! typed `BudgetExceeded` — refusal, never silent throttling, and NO fuel is ever loaded
//! again after the refusing acquire.
//!
//! The loop is generic over a [`FuelMeter`]. The default build uses the deterministic
//! in-crate [`DeterministicFuelMeter`] that models wasmtime's `set_fuel`/consume/trap
//! semantics exactly (so §4 criterion 4 is a real test of the real control logic). The
//! REAL wasmtime-backed meter compiles behind the `wasmtime-fuel` feature.

use std::sync::atomic::{AtomicUsize, Ordering};

use dowiz_kernel::ports::agent::{FUEL_PER_UNIT, TRANCHE_UNITS};
use dowiz_kernel::token_bucket::TokenBucket;

/// How a single prepaid guest slice ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceOutcome {
    /// The guest finished within the loaded fuel.
    Done,
    /// The guest exhausted the loaded fuel and trapped (needs another tranche).
    OutOfFuel,
    /// The guest trapped for another reason (bad instruction, host-call error).
    Trap(String),
}

/// Typed fuel-loop failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuelError {
    /// The budget bucket was exhausted mid-run; the instance is terminated. Never
    /// resumed at a reduced rate, never queued.
    BudgetExceeded {
        /// Units consumed (prepaid) before the refusing acquire.
        consumed_units: u64,
    },
    /// The guest trapped for a non-fuel reason.
    Trap(String),
}

/// The metered-execution seam. The default reference impl models wasmtime; the real
/// wasmtime `Store` implements this behind the `wasmtime-fuel` feature.
pub trait FuelMeter {
    /// Load `fuel` units of CPU budget into the guest (wasmtime `store.set_fuel`).
    fn set_fuel(&mut self, fuel: u64);
    /// Run the guest until it finishes or traps on out-of-fuel.
    fn run_slice(&mut self) -> SliceOutcome;
}

/// The prepaid-tranche runner. Reused for both the deterministic and wasmtime meters.
pub struct FuelTrancheRunner {
    fuel_per_unit: u64,
    tranche_units: u64,
    set_fuel_calls: AtomicUsize,
}

impl Default for FuelTrancheRunner {
    fn default() -> Self {
        // FUEL_PER_UNIT and TRANCHE_UNITS are the kernel-pinned constants (mirror-pinned
        // kernel↔adapter). FUEL_PER_UNIT is a PLACEHOLDER pending the B4 criterion bench.
        FuelTrancheRunner {
            fuel_per_unit: FUEL_PER_UNIT,
            tranche_units: TRANCHE_UNITS,
            set_fuel_calls: AtomicUsize::new(0),
        }
    }
}

impl FuelTrancheRunner {
    /// Build a runner with explicit constants (tests may shrink the tranche).
    pub fn new(fuel_per_unit: u64, tranche_units: u64) -> Self {
        FuelTrancheRunner {
            fuel_per_unit,
            tranche_units,
            set_fuel_calls: AtomicUsize::new(0),
        }
    }

    /// How many times fuel was loaded (crit 4: must NOT increase after the refusing acquire).
    pub fn set_fuel_calls(&self) -> usize {
        self.set_fuel_calls.load(Ordering::SeqCst)
    }

    /// Run `meter` under `bucket`, prepaid tranche by tranche. `Ok(units)` on guest
    /// completion; `Err(BudgetExceeded)` on bucket exhaustion (terminated, never resumed).
    pub fn run<M: FuelMeter>(&self, meter: &mut M, bucket: &TokenBucket) -> Result<u64, FuelError> {
        let mut consumed_units = 0u64;
        loop {
            // Prepay the next tranche. On refusal: TERMINATE — do not load fuel again.
            if !bucket.try_acquire(self.tranche_units as f64) {
                return Err(FuelError::BudgetExceeded { consumed_units });
            }
            self.set_fuel_calls.fetch_add(1, Ordering::SeqCst);
            meter.set_fuel(self.tranche_units.saturating_mul(self.fuel_per_unit));
            match meter.run_slice() {
                SliceOutcome::Done => return Ok(consumed_units + self.tranche_units),
                SliceOutcome::OutOfFuel => {
                    consumed_units += self.tranche_units;
                    continue;
                }
                SliceOutcome::Trap(e) => return Err(FuelError::Trap(e)),
            }
        }
    }
}

/// A deterministic in-crate `FuelMeter` modelling a guest that needs `total_need` fuel
/// units of CPU. `set_fuel(f)` loads `f`; each `run_slice` consumes `min(loaded,
/// remaining)`; if any need remains it is `OutOfFuel`, else `Done`. `infinite()` models
/// a compute-bomb / spin guest (always `OutOfFuel`) — the crit-4 case.
pub struct DeterministicFuelMeter {
    remaining_need: u128,
    loaded: u64,
}

impl DeterministicFuelMeter {
    /// A guest that needs `total_need` fuel to finish.
    pub fn needs(total_need: u64) -> Self {
        DeterministicFuelMeter {
            remaining_need: total_need as u128,
            loaded: 0,
        }
    }
    /// A guest that never finishes (compute bomb / spin).
    pub fn infinite() -> Self {
        DeterministicFuelMeter {
            remaining_need: u128::MAX,
            loaded: 0,
        }
    }
}

impl FuelMeter for DeterministicFuelMeter {
    fn set_fuel(&mut self, fuel: u64) {
        self.loaded = fuel;
    }
    fn run_slice(&mut self) -> SliceOutcome {
        let spend = (self.loaded as u128).min(self.remaining_need);
        self.remaining_need -= spend;
        self.loaded = 0;
        if self.remaining_need == 0 {
            SliceOutcome::Done
        } else {
            SliceOutcome::OutOfFuel
        }
    }
}

// ── The real wasmtime-backed meter (feature-gated; adapter-side only) ────────────────
#[cfg(feature = "wasmtime-fuel")]
mod wasmtime_meter {
    use super::{FuelMeter, SliceOutcome};
    use wasmtime::{Engine, Instance, Module, Store, TypedFunc};

    /// A real wasmtime `Store`-backed fuel meter. Runs a WASM guest slice; an out-of-fuel
    /// trap becomes `OutOfFuel`, a normal return becomes `Done`. `set_fuel` calls the real
    /// `Store::set_fuel`. This is the production sandbox path (§2.2), compiled only under
    /// the `wasmtime-fuel` feature so the default build stays fast/offline.
    pub struct WasmtimeFuelMeter {
        store: Store<()>,
        entry: TypedFunc<(), ()>,
    }

    impl WasmtimeFuelMeter {
        /// Build from WAT/wasm bytes exporting a nullary `run` function. Fuel consumption
        /// is enabled on the engine (`Config::consume_fuel`). Errors surface as `String`
        /// so the adapter needs no direct `anyhow` dependency.
        pub fn from_wat(wat: &str) -> Result<Self, String> {
            let mut config = wasmtime::Config::new();
            config.consume_fuel(true);
            let engine = Engine::new(&config).map_err(|e| e.to_string())?;
            let module = Module::new(&engine, wat).map_err(|e| e.to_string())?;
            let mut store = Store::new(&engine, ());
            // Start empty; the tranche loop loads fuel before each slice.
            store.set_fuel(0).map_err(|e| e.to_string())?;
            let instance = Instance::new(&mut store, &module, &[]).map_err(|e| e.to_string())?;
            let entry = instance
                .get_typed_func::<(), ()>(&mut store, "run")
                .map_err(|e| e.to_string())?;
            Ok(WasmtimeFuelMeter { store, entry })
        }
    }

    impl FuelMeter for WasmtimeFuelMeter {
        fn set_fuel(&mut self, fuel: u64) {
            let _ = self.store.set_fuel(fuel);
        }
        fn run_slice(&mut self) -> SliceOutcome {
            match self.entry.call(&mut self.store, ()) {
                Ok(()) => SliceOutcome::Done,
                Err(e) => {
                    // A wasmtime out-of-fuel trap is the tranche-boundary signal.
                    if let Some(trap) = e.downcast_ref::<wasmtime::Trap>() {
                        if *trap == wasmtime::Trap::OutOfFuel {
                            return SliceOutcome::OutOfFuel;
                        }
                    }
                    SliceOutcome::Trap(format!("{e}"))
                }
            }
        }
    }
}

#[cfg(feature = "wasmtime-fuel")]
pub use wasmtime_meter::WasmtimeFuelMeter;

#[cfg(test)]
mod tests {
    use super::*;

    // ── §4 criterion 4 — fuel exhaustion = refusal (never resumed, never queued) ──
    #[test]
    fn crit4_compute_bomb_terminates_with_budget_exceeded() {
        // Bucket: 20 units, no refill. Tranche = 8 units, small fuel_per_unit for speed.
        let bucket = TokenBucket::new(20 as f64, 0.0);
        let runner = FuelTrancheRunner::new(1, 8);
        let mut guest = DeterministicFuelMeter::infinite(); // never finishes
        let res = runner.run(&mut guest, &bucket);
        // Terminated with a typed refusal; consumed exactly the 2 grantable tranches (16),
        // then the 3rd acquire (needs 8, only 4 left) refused.
        assert_eq!(res, Err(FuelError::BudgetExceeded { consumed_units: 16 }));
        let calls_at_refusal = runner.set_fuel_calls();
        assert_eq!(
            calls_at_refusal, 2,
            "fuel loaded exactly twice (the 2 grantable tranches)"
        );
        // The bucket is below a tranche now; a second run loads NO fuel at all — proving
        // the instance is never resumed at reduced rate after refusal.
        let runner2 = FuelTrancheRunner::new(1, 8);
        let mut guest2 = DeterministicFuelMeter::infinite();
        let res2 = runner2.run(&mut guest2, &bucket);
        assert_eq!(res2, Err(FuelError::BudgetExceeded { consumed_units: 0 }));
        assert_eq!(
            runner2.set_fuel_calls(),
            0,
            "NO fuel ever loaded once the bucket is exhausted"
        );
    }

    #[test]
    fn finite_guest_completes_within_budget() {
        // Bucket huge; guest needs 10 * fuel_per_unit(=1000) = ... finishes within 2 tranches.
        let bucket = TokenBucket::new(1_000_000 as f64, 0.0);
        let runner = FuelTrancheRunner::new(1000, 8); // 1 tranche = 8*1000 = 8000 fuel
        let mut guest = DeterministicFuelMeter::needs(10_000); // needs 2 tranches (8000 + 2000)
        let units = runner
            .run(&mut guest, &bucket)
            .expect("finite guest finishes");
        assert_eq!(units, 16, "consumed 2 prepaid tranches");
        assert_eq!(runner.set_fuel_calls(), 2);
    }

    // ── the REAL wasmtime path (feature-gated) — same tranche loop, real guest ───
    #[cfg(feature = "wasmtime-fuel")]
    #[test]
    fn wasmtime_compute_bomb_terminates_with_budget_exceeded() {
        // A real WASM guest that spins forever, exported as `run`. Each tranche loads
        // fuel; wasmtime traps OutOfFuel; the loop advances until the bucket is empty.
        let wat = r#"(module (func (export "run") (loop br 0)))"#;
        let mut meter = super::WasmtimeFuelMeter::from_wat(wat).expect("wat compiles");
        let bucket = TokenBucket::new(24 as f64, 0.0); // 3 grantable tranches of 8
        let runner = FuelTrancheRunner::new(1000, 8);
        let res = runner.run(&mut meter, &bucket);
        assert!(
            matches!(res, Err(FuelError::BudgetExceeded { .. })),
            "compute bomb terminated"
        );
        assert_eq!(
            runner.set_fuel_calls(),
            3,
            "fuel loaded exactly the 3 grantable tranches"
        );
    }

    #[test]
    fn fuel_per_unit_is_the_kernel_placeholder() {
        // Mirror-pinned kernel↔adapter; PLACEHOLDER pending B4 (documented, not ledger-grounded).
        assert_eq!(FUEL_PER_UNIT, 100_000);
        let runner = FuelTrancheRunner::default();
        assert_eq!(runner.fuel_per_unit, FUEL_PER_UNIT);
        assert_eq!(runner.tranche_units, TRANCHE_UNITS);
    }
}
