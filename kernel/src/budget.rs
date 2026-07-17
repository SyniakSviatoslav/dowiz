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

use std::sync::Mutex;

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

/// P11 §1 — reusable compute-budget accumulator (single-owner, degrade-closed).
///
/// Tracks a spend accumulator against a fixed `ceiling`. [`ComputeBudget::debit`]
/// refuses (returns `false`, records nothing) when the debit would push spend past
/// the ceiling — the same load-bearing "degrade-closed" contract the
/// `BudgetedJobPort` applies per-submit. This is the reusable, non-threaded half;
/// `BudgetedJobPort` wraps an instance in a `Mutex` for the threaded port surface.
pub struct ComputeBudget {
    spent: f64,
    ceiling: f64,
}

impl ComputeBudget {
    /// Create an empty accumulator with the given `ceiling`.
    pub fn new(ceiling: f64) -> Self {
        ComputeBudget { spent: 0.0, ceiling }
    }

    /// Current spend accumulator.
    pub fn spent(&self) -> f64 {
        self.spent
    }

    /// Budget ceiling.
    pub fn ceiling(&self) -> f64 {
        self.ceiling
    }

    /// Degrade-closed debit: returns `true` and advances `spent` iff
    /// `spent + amount <= ceiling`. If the debit would exceed the ceiling, returns
    /// `false` and records **no** spend (the caller must refuse, never leak cost).
    pub fn debit(&mut self, amount: f64) -> bool {
        if self.spent + amount > self.ceiling {
            false
        } else {
            self.spent += amount;
            true
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
    budget: Mutex<ComputeBudget>,
}

impl<P: JobPort> BudgetedJobPort<P> {
    /// Wrap `inner` with a `monthly_ceiling`-scoped budget.
    pub fn new(inner: P, monthly_ceiling: f64) -> Self {
        BudgetedJobPort {
            inner,
            budget: Mutex::new(ComputeBudget::new(monthly_ceiling)),
        }
    }

    /// Current spend accumulator (read-only view for telemetry/tests).
    pub fn spent(&self) -> f64 {
        self.budget.lock().unwrap().spent()
    }
}

impl<P: JobPort> JobPort for BudgetedJobPort<P> {
    fn submit(&self, job: &Job) -> Result<JobHandle, JobError> {
        // Degrade-closed gate: refuse BEFORE any spend is recorded when the
        // projected total would exceed the ceiling.
        {
            let mut b = self.budget.lock().unwrap();
            if b.spent() + job.estimate > b.ceiling() {
                return Err(JobError::BudgetExceeded);
            }
            // Within budget → debit, then forward to the inner port.
            b.debit(job.estimate);
        }
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
        let mut cb = ComputeBudget::new(10.0);
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
}
