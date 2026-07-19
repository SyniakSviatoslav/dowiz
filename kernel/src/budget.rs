//! budget.rs — P11 §1 ComputeBudget + §4 Modal `JobPort` / `BudgetedJobPort` seam.
//!
//! Zero-dep, pure-`std` (no tokio, no time crate, no NUMA crate). The spend rail
//! reuses the already-built `token_bucket::TokenBucket` *philosophy* only by name;
//! this module owns the **monthly budget accumulator** + the **port seam** so the
//! real Modal adapter (a deferred external port) has a nameable, honest boundary.
//!
//! Governing rule (ARCHITECTURE.md §1/D6): GPU/Modal compute is OFFLINE, behind a
//! port, never in-kernel, never in the request path. The default `OfflineJobPort`
//! therefore returns an honest `Err(JobError::Offline(..))` on every `submit` —
//! never a fake `Ok`, never a fake `JobHandle`. This mirrors `engine::bridge::gpu`'s
//! fail-closed boundary exactly.
//!
//! "Degrade-closed" is the load-bearing word for the budget: when a `submit` would
//! push spend past the `monthly_ceiling`, the port **refuses** (`Err(BudgetExceeded)`)
//! and records NO spend — safe, cost-bounded — rather than degrade-open (proceed and
//! leak cost). The same rule lives in the reusable [`ComputeBudget`] primitive.
//!
//! Atomicity (2026-07-18, contended-bench evidence): the spend accumulator is a
//! LOCK-FREE `AtomicU64` (bit-cast `f64`) CAS loop, not a `Mutex<f64>`. The contended
//! benchmark `kernel/benches/contention.rs::contended_budget` measured the atomic path
//! at ~2× the Mutex single-threaded and ~1.28× at 2–4 threads (tie at 8-way saturation,
//! where both bounce the same hot cache line) — a clean win in the realistic low-
//! contention regime with SIMPLER code (no lock, no poison recovery). Degrade-closed is
//! preserved exactly: the ceiling is re-checked on every CAS retry, so it can never
//! overshoot, and the check-then-debit is now a single atomic op (no check-then-act race).

use std::sync::atomic::{AtomicU64, Ordering};

/// A submitted job's opaque handle (returned by a `JobPort` on success).
///
/// Minimal: today only an id. The real Modal adapter would populate this with a
/// remote run-id; the offline default never constructs one.
#[derive(Clone, Debug, PartialEq)]
pub struct JobHandle {
    pub id: u64,
}

/// Live status of a submitted job, polled via [`JobPort::poll`].
///
/// Kept minimal; the offline default never reaches a non-`Unknown` state because it
/// never mints a handle.
#[derive(Clone, Debug, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
    /// Returned when polling a handle the offline port could never have issued.
    Unknown,
}

/// A unit of offline one-shot compute to submit through a [`JobPort`].
///
/// Minimal by design (P11 §4). `estimate` is the expected monthly-spend units the
/// `BudgetedJobPort` debits before forwarding; the real Modal adapter would carry
/// additional routing/payload fields behind its own feature gate.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub estimate: f64,
}

/// Errors a [`JobPort`] (or its [`BudgetedJobPort`] wrapper) can return.
#[derive(Clone, Debug, PartialEq)]
pub enum JobError {
    /// The monthly budget ceiling would be exceeded; the submit was refused and
    /// no spend was recorded (degrade-closed).
    BudgetExceeded,
    /// The (default) offline adapter was asked to submit, but no Modal adapter is
    /// built — honest `Err`, never a fake `Ok`/`JobHandle`. Carries a reason string.
    Offline(String),
}

/// The Modal compute port (Trait-as-Port). Mirrors the `gpu` boundary: the default
/// build implements only the offline, fail-closed variant; the real adapter is a
/// deferred external port behind a non-default `modal` cargo feature.
pub trait JobPort {
    /// Submit a job. Returns a [`JobHandle`] on success, or a [`JobError`] (budget
    /// refusal or offline) on failure. MUST NOT return a fake success.
    fn submit(&self, job: &Job) -> Result<JobHandle, JobError>;
    /// Poll the status of a previously-submitted handle.
    fn poll(&self, handle: &JobHandle) -> JobStatus;
    /// Tear down any remote resources bound to `handle` (mandatory-teardown watchdog
    /// hook so a scale-to-zero job cannot bill indefinitely if the caller drops it).
    fn teardown(&self, handle: &JobHandle);
}

/// P11 §1 — reusable compute-budget accumulator (lock-free, degrade-closed).
///
/// Tracks a spend accumulator against a fixed `ceiling`. [`ComputeBudget::debit`]
/// refuses (returns `false`, records nothing) when the debit would push spend past
/// the ceiling — the load-bearing "degrade-closed" contract the `BudgetedJobPort`
/// applies per-submit. The accumulator is a lock-free `AtomicU64` (the `f64` spend
/// bit-cast), so `debit` takes `&self` and is safe to share across threads directly —
/// `BudgetedJobPort` holds it inline, no `Mutex` (see the contended-bench note in the
/// module header).
pub struct ComputeBudget {
    /// `f64` spend accumulator, bit-cast into an `AtomicU64` for a lock-free CAS loop.
    spent_bits: AtomicU64,
    ceiling: f64,
}

impl ComputeBudget {
    /// Create an empty accumulator with the given `ceiling`.
    pub fn new(ceiling: f64) -> Self {
        ComputeBudget {
            spent_bits: AtomicU64::new(0.0f64.to_bits()),
            ceiling,
        }
    }

    /// Current spend accumulator.
    pub fn spent(&self) -> f64 {
        f64::from_bits(self.spent_bits.load(Ordering::Relaxed))
    }

    /// Budget ceiling.
    pub fn ceiling(&self) -> f64 {
        self.ceiling
    }

    /// Degrade-closed debit: returns `true` and advances `spent` iff
    /// `spent + amount <= ceiling`. If the debit would exceed the ceiling, returns
    /// `false` and records **no** spend (the caller must refuse, never leak cost).
    ///
    /// Lock-free CAS loop: the ceiling is re-checked on EVERY retry against the
    /// freshly-observed `spent`, so a concurrent debit can never race two grants past
    /// the ceiling (the exact over-grant falsifier `budget_atomic_never_over_grants`
    /// pins). A non-finite or negative `amount` is refused (defense-in-depth: a NaN
    /// makes `spent + NaN > ceiling` false, which would otherwise poison the
    /// accumulator; a negative amount would roll spend backwards).
    pub fn debit(&self, amount: f64) -> bool {
        if !amount.is_finite() || amount < 0.0 {
            return false;
        }
        let mut cur = self.spent_bits.load(Ordering::Relaxed);
        loop {
            let spent = f64::from_bits(cur);
            if spent + amount > self.ceiling {
                return false;
            }
            let next = (spent + amount).to_bits();
            match self.spent_bits.compare_exchange_weak(
                cur,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => cur = actual,
            }
        }
    }
}

/// P11 §4 — wraps any [`JobPort`] with a month-scoped spend ceiling (degrade-closed).
///
/// Before every `submit`, `projected = spent + estimate(job)` is computed. If
/// `projected > monthly_ceiling`, the submit is refused with [`JobError::BudgetExceeded`]
/// and **no** spend is recorded. Otherwise the estimate is debited and the job is
/// forwarded to the inner port. The inner port's own failure (e.g. the offline
/// adapter returning `Err(Offline)`) does not un-debit — the budget was reserved
/// for the attempted submit.
pub struct BudgetedJobPort<P: JobPort> {
    inner: P,
    budget: ComputeBudget,
}

impl<P: JobPort> BudgetedJobPort<P> {
    /// Wrap `inner` with a `monthly_ceiling`-scoped budget.
    pub fn new(inner: P, monthly_ceiling: f64) -> Self {
        BudgetedJobPort {
            inner,
            budget: ComputeBudget::new(monthly_ceiling),
        }
    }

    /// Current spend accumulator (read-only view for telemetry/tests).
    pub fn spent(&self) -> f64 {
        self.budget.spent()
    }
}

impl<P: JobPort> JobPort for BudgetedJobPort<P> {
    fn submit(&self, job: &Job) -> Result<JobHandle, JobError> {
        // Degrade-closed gate as a SINGLE lock-free atomic op: `debit` both checks the
        // ceiling and records the spend, so there is no check-then-act race between the
        // ceiling test and the debit (the old `Mutex`-held two-step). It refuses a NaN/
        // infinite/negative estimate (V1 #5 — a NaN would otherwise poison the
        // accumulator degrade-OPEN; a negative estimate would roll spend backwards) AND
        // any debit that would exceed the ceiling, recording NO spend on refusal.
        if !self.budget.debit(job.estimate) {
            return Err(JobError::BudgetExceeded);
        }
        // Within budget → estimate reserved; forward to the inner port. The inner port's
        // own failure (e.g. offline Err) does not un-debit — the budget was reserved.
        self.inner.submit(job)
    }

    fn poll(&self, handle: &JobHandle) -> JobStatus {
        self.inner.poll(handle)
    }

    fn teardown(&self, handle: &JobHandle) {
        self.inner.teardown(handle)
    }
}

/// P11 §4 — the DEFAULT offline `JobPort`. Always returns
/// `Err(JobError::Offline(..))` on `submit` (never a fake `Ok`/`JobHandle`),
/// mirroring the `gpu` boundary honesty. The real Modal adapter is a deferred
/// external port behind a non-default `modal` feature.
pub struct OfflineJobPort;

impl JobPort for OfflineJobPort {
    fn submit(&self, _job: &Job) -> Result<JobHandle, JobError> {
        Err(JobError::Offline(
            "modal adapter not built — offline".to_string(),
        ))
    }

    fn poll(&self, _handle: &JobHandle) -> JobStatus {
        // No handle was ever minted by the offline port.
        JobStatus::Unknown
    }

    fn teardown(&self, _handle: &JobHandle) {
        // Nothing to tear down: the offline port never bound remote resources.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (a) P11 §1 — degrade-closes: debits advance the accumulator until the
    /// ceiling is reached; the next debit is refused AND spend does not advance.
    #[test]
    fn budget_degrade_closes() {
        let cb = ComputeBudget::new(10.0);
        assert!(cb.debit(3.0)); // spent = 3
        assert!(cb.debit(3.0)); // spent = 6
        assert!(cb.debit(3.0)); // spent = 9 (still <= 10)
        assert_eq!(cb.spent(), 9.0);

        // Next debit of 3.0 would push to 12 > 10 → refused.
        assert!(
            !cb.debit(3.0),
            "debit past the ceiling must be refused (degrade-closed)"
        );
        // Refusal records NO spend: accumulator frozen at 9.0.
        assert_eq!(
            cb.spent(),
            9.0,
            "spent must not advance past the ceiling on refusal"
        );

        // A debit of 0 is always safe (no cost) and does not trip the ceiling.
        assert!(cb.debit(0.0));
        assert_eq!(cb.spent(), 9.0);
    }

    /// (b) P11 §4 — `BudgetedJobPort<OfflineJobPort>` refuses over-ceiling submits
    /// with `Err(BudgetExceeded)` and the accumulator stays frozen on refusal.
    #[test]
    fn budgeted_jobport_refuses_over_ceiling() {
        let port = BudgetedJobPort::new(OfflineJobPort, 10.0);

        // Nine within-budget submits (estimate 1.0 each) accrue spend up to 9.0.
        // The inner offline port returns Err(Offline), but the budget is still
        // debited per the spec — the refusal test only cares about the ceiling.
        for _ in 0..9 {
            let _ = port.submit(&Job { estimate: 1.0 });
        }
        assert_eq!(port.spent(), 9.0);

        // A submit whose estimate would push past the ceiling is refused.
        let before = port.spent();
        let res = port.submit(&Job { estimate: 5.0 });
        assert!(
            matches!(res, Err(JobError::BudgetExceeded)),
            "over-ceiling submit must return BudgetExceeded"
        );
        // Refusal records NO spend: accumulator frozen at the pre-submit value.
        assert_eq!(
            port.spent(),
            before,
            "accumulator must not advance on a refused (over-ceiling) submit"
        );
    }

    /// (c) V1 #5 (ROUND-2 GAP-AUDIT) — a NaN or negative `estimate` is malformed
    /// input and MUST be refused before any debit. Pre-fix, a NaN made
    /// `spent + NaN > ceiling` evaluate false, so it was debited as a poisoned
    /// non-finite spend (degrade-OPEN); a negative estimate rolled spend backwards.
    #[test]
    fn budgeted_jobport_refuses_nan_and_negative_estimate() {
        let port = BudgetedJobPort::new(OfflineJobPort, 10.0);

        // NaN estimate → refused, no spend recorded, accumulator stays finite at 0.
        let res_nan = port.submit(&Job { estimate: f64::NAN });
        assert!(
            matches!(res_nan, Err(JobError::BudgetExceeded)),
            "NaN estimate must be refused (degrade-closed)"
        );
        assert_eq!(port.spent(), 0.0, "NaN submit must not debit");
        assert!(
            port.spent().is_finite(),
            "spend must never become non-finite"
        );

        // +inf estimate → refused, no spend.
        let res_inf = port.submit(&Job {
            estimate: f64::INFINITY,
        });
        assert!(
            matches!(res_inf, Err(JobError::BudgetExceeded)),
            "infinite estimate must be refused"
        );
        assert_eq!(port.spent(), 0.0, "inf submit must not debit");

        // Negative estimate → refused, spend does not roll backwards.
        let res_neg = port.submit(&Job { estimate: -5.0 });
        assert!(
            matches!(res_neg, Err(JobError::BudgetExceeded)),
            "negative estimate must be refused"
        );
        assert_eq!(port.spent(), 0.0, "negative submit must not move spend");
    }

    /// (c) P11 §4 — the default offline adapter returns `Err(Offline)`, never a
    /// fake `Ok`/`JobHandle`.
    #[test]
    fn offline_jobport_returns_err() {
        let port = OfflineJobPort;
        let res = port.submit(&Job { estimate: 1.0 });
        assert!(
            matches!(res, Err(JobError::Offline(_))),
            "offline port must return Err(Offline), never a fake Ok"
        );
    }

    /// (d) ATOMICITY falsifier (2026-07-18): the lock-free CAS `debit` must NEVER let
    /// concurrent threads race two grants past the ceiling. N threads each hammer
    /// `debit(1.0)` against a ceiling of `GRANTS`; EXACTLY `GRANTS` debits may succeed
    /// and final spend must equal `GRANTS` — no over-grant, no lost debit. This is the
    /// concurrency proof the old single-threaded tests could not give.
    #[test]
    fn budget_atomic_never_over_grants() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        const GRANTS: usize = 10_000;
        let budget = Arc::new(ComputeBudget::new(GRANTS as f64));
        let successes = Arc::new(AtomicUsize::new(0));

        std::thread::scope(|s| {
            for _ in 0..8 {
                let budget = Arc::clone(&budget);
                let successes = Arc::clone(&successes);
                s.spawn(move || {
                    // Each thread attempts far more debits than the ceiling allows.
                    for _ in 0..GRANTS {
                        if budget.debit(1.0) {
                            successes.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });

        assert_eq!(
            successes.load(Ordering::Relaxed),
            GRANTS,
            "exactly ceiling-many debits may succeed under concurrency (no over-grant, no lost debit)"
        );
        assert_eq!(
            budget.spent(),
            GRANTS as f64,
            "final spend must equal the ceiling exactly — CAS lost no update and overshot none"
        );
        // One more debit past the exhausted ceiling is refused (degrade-closed holds).
        assert!(!budget.debit(1.0), "exhausted budget refuses further debit");
    }

    /// (e) ComputeBudget::debit itself refuses non-finite / negative amounts (the guard
    /// now lives in the primitive, not only in `BudgetedJobPort::submit`).
    #[test]
    fn compute_budget_debit_refuses_non_finite_and_negative() {
        let cb = ComputeBudget::new(10.0);
        assert!(!cb.debit(f64::NAN), "NaN debit refused");
        assert!(!cb.debit(f64::INFINITY), "inf debit refused");
        assert!(!cb.debit(-1.0), "negative debit refused");
        assert_eq!(cb.spent(), 0.0, "no refused debit moved spend");
        assert!(cb.debit(4.0), "a valid debit still works");
        assert_eq!(cb.spent(), 4.0);
    }
}
