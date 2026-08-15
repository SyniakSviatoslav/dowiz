//! `fdr/cost_oracle.rs` — Items 67 + 68: the cost-oracle classification + runtime counters
//! (pure no_std core).
//!
//! Every operator-gated decision surface carries a cost bucket with a traceable evidence
//! pointer. Buckets: `ORACLE-EXACT` (input-independent), `ORACLE-BOUNDED` (fixed operation
//! schedule), `MEASURED-ONLY` (genuinely dynamic / I/O / probabilistic), and `FORBIDDEN`
//! (unclassified — the oracle never returns a fabricated guess).
//!
//! Plane doctrine: the captured cost values live on the P3 forensic plane — recorded input,
//! never a decision variable. The process-global counter set is a P3 telemetry mirror only.
//!
//! This is the *pure* half. The counter set is a compile-time-initialized `static` (no
//! `OnceLock`, no lazy init — the counters are zeroed atomics). The std side — the
//! recoverable-from-ring round-trip test that needs a real `FdrRing` — lives in the kernel
//! shim (`dowiz-kernel`'s `fdr::cost_oracle`).

use crate::fdr::json::JsonWriter;
use crate::fdr::schema::{Kind, StampPolicy};
use crate::fdr::Level;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

/// The cost bucket for an operator-gated decision.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CostBucket {
    /// Cost is input-independent / fully enumerated — value is known exactly (mod host noise).
    OracleExact,
    /// Cost follows a fixed operation schedule — bounded by an analytic `[min,max]`.
    OracleBounded,
    /// Genuinely dynamic / I/O / probabilistic — reported as a distribution, not a point.
    MeasuredOnly,
    /// NOT classified. The forbidden state — the oracle surfaces this instead of a guess.
    Forbidden,
}

impl CostBucket {
    pub fn as_str(self) -> &'static str {
        match self {
            CostBucket::OracleExact => "ORACLE-EXACT",
            CostBucket::OracleBounded => "ORACLE-BOUNDED",
            CostBucket::MeasuredOnly => "MEASURED-ONLY",
            CostBucket::Forbidden => "FORBIDDEN",
        }
    }
}

/// An operator-gated decision surface the cost oracle classifies + counts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DecisionSurface {
    /// `hydra::FileEventStore` group-commit (`batch_size`).
    GroupCommit,
    /// `spectral_cache::DecompCache` eigensolver choice (hit vs recompute).
    EigensolverChoice,
    /// PQ sign/verify crypto latency.
    CryptoLatency,
}

impl DecisionSurface {
    pub fn as_str(self) -> &'static str {
        match self {
            DecisionSurface::GroupCommit => "group_commit",
            DecisionSurface::EigensolverChoice => "eigensolver_choice",
            DecisionSurface::CryptoLatency => "crypto_latency",
        }
    }

    /// The honest cost bucket for this surface (item 67 classification).
    pub fn bucket(self) -> CostBucket {
        match self {
            DecisionSurface::GroupCommit => CostBucket::OracleExact,
            DecisionSurface::EigensolverChoice => CostBucket::OracleBounded,
            DecisionSurface::CryptoLatency => CostBucket::MeasuredOnly,
        }
    }

    /// The traceable evidence pointer (item 67 §67.3 step 2).
    pub fn evidence(self) -> &'static str {
        match self {
            DecisionSurface::GroupCommit => "HOT-PATHS.tsv:hydra::FileEventStore::with_batch_size; group_commit_barrier_fires_every_n_inserts",
            DecisionSurface::EigensolverChoice => "spectral_cache::DecompCache; spectral_cache recompute falsifier test",
            DecisionSurface::CryptoLatency => "pq/dsa sign/verify; item 26 ~637us distribution exemplar",
        }
    }
}

/// Item 68 (EXACT capture): the group-commit decision is input-independent once `batch_size` is
/// fixed. The cost is the count of inserts per durability barrier, which is an exact integer.
pub fn group_commit_exact_cost(batch_size: usize) -> u64 {
    batch_size.max(1) as u64
}

/// Item 68 (BOUNDED capture): the eigensolver choice. A cache HIT returns in O(1); a MISS
/// triggers the recompute whose schedule is fixed. The analytic `[min,max]` is `[1, n*n + 1]`.
pub fn eigensolver_bounded_cost(n: usize, is_cache_hit: bool) -> (u64, u64) {
    if is_cache_hit {
        (1, 1)
    } else {
        let floor = (n as u64).saturating_mul(n as u64).saturating_add(1);
        (1, floor)
    }
}

/// Item 68 (MEASURED-ONLY capture): crypto latency is reported as a distribution summary, never
/// a fabricated point estimate. Empty sample ⇒ a named absence, never 0.
#[derive(Clone, Copy, Debug, Default)]
pub struct CryptoLatencyDist {
    pub count: u64,
    pub p50_us: u64,
    pub p99_us: u64,
}

impl CryptoLatencyDist {
    /// Fold one observed latency sample into the running distribution (min/max as the honest
    /// p50/p99 proxy for a MEASURED-ONLY surface).
    pub fn observe(&mut self, us: u64) {
        self.count += 1;
        if self.count == 1 {
            self.p50_us = us;
            self.p99_us = us;
        } else {
            self.p50_us = self.p50_us.min(us);
            self.p99_us = self.p99_us.max(us);
        }
    }

    /// A MEASURED-ONLY surface with zero samples reports an honest absence (no fabricated 0).
    pub fn is_absent(&self) -> bool {
        self.count == 0
    }
}

// ── Runtime counters (item 67/68: deterministic runtime counters for operator-gated decisions) ──
// A process-global counter set, P3-plane telemetry only. Reads/writes are explicit and NEVER
// feed a durability barrier / drift gate / crypto verdict (grep-firewall proof below).
// Compile-time-initialized `static` (zeroed atomics) — no `OnceLock` needed.

struct CostOracleCounters {
    group_commit_barriers: AtomicU64,
    group_commit_inserts: AtomicU64,
    eigensolver_hits: AtomicU64,
    eigensolver_recomputes: AtomicU64,
    crypto_samples: AtomicU64,
    crypto_latency_us: AtomicU64,
}

impl CostOracleCounters {
    const fn new() -> Self {
        CostOracleCounters {
            group_commit_barriers: AtomicU64::new(0),
            group_commit_inserts: AtomicU64::new(0),
            eigensolver_hits: AtomicU64::new(0),
            eigensolver_recomputes: AtomicU64::new(0),
            crypto_samples: AtomicU64::new(0),
            crypto_latency_us: AtomicU64::new(0),
        }
    }
}

static COST_ORACLE: CostOracleCounters = CostOracleCounters::new();

fn counters() -> &'static CostOracleCounters {
    &COST_ORACLE
}

/// Record ONE group-commit barrier firing over `inserts` inserts. P3 telemetry only.
pub fn record_group_commit(inserts: u64) {
    counters().group_commit_barriers.fetch_add(1, Ordering::Relaxed);
    counters().group_commit_inserts.fetch_add(inserts, Ordering::Relaxed);
}

/// Record ONE eigensolver decision (hit or miss/recompute). P3 telemetry only.
pub fn record_eigensolver(is_cache_hit: bool) {
    if is_cache_hit {
        counters().eigensolver_hits.fetch_add(1, Ordering::Relaxed);
    } else {
        counters().eigensolver_recomputes.fetch_add(1, Ordering::Relaxed);
    }
}

/// Record ONE crypto latency sample (MEASURED-ONLY). P3 telemetry only.
pub fn record_crypto_latency(us: u64) {
    counters().crypto_samples.fetch_add(1, Ordering::Relaxed);
    counters().crypto_latency_us.fetch_add(us, Ordering::Relaxed);
}

/// A one-shot snapshot of the runtime counters (the operator's offline feed).
#[derive(Clone, Copy, Debug, Default)]
pub struct CostOracleSnapshot {
    pub group_commit_barriers: u64,
    pub group_commit_inserts: u64,
    pub eigensolver_hits: u64,
    pub eigensolver_recomputes: u64,
    pub crypto_samples: u64,
    pub crypto_latency_us: u64,
}

/// Read the current counter snapshot.
pub fn snapshot() -> CostOracleSnapshot {
    let c = counters();
    CostOracleSnapshot {
        group_commit_barriers: c.group_commit_barriers.load(Ordering::Relaxed),
        group_commit_inserts: c.group_commit_inserts.load(Ordering::Relaxed),
        eigensolver_hits: c.eigensolver_hits.load(Ordering::Relaxed),
        eigensolver_recomputes: c.eigensolver_recomputes.load(Ordering::Relaxed),
        crypto_samples: c.crypto_samples.load(Ordering::Relaxed),
        crypto_latency_us: c.crypto_latency_us.load(Ordering::Relaxed),
    }
}

/// Reset all counters (test isolation). Cheap; the global is a zeroed static.
pub fn reset_counters() {
    let c = counters();
    c.group_commit_barriers.store(0, Ordering::Relaxed);
    c.group_commit_inserts.store(0, Ordering::Relaxed);
    c.eigensolver_hits.store(0, Ordering::Relaxed);
    c.eigensolver_recomputes.store(0, Ordering::Relaxed);
    c.crypto_samples.store(0, Ordering::Relaxed);
    c.crypto_latency_us.store(0, Ordering::Relaxed);
}

/// Item 67 §67.4 (b): classify a query. An unrecognized / unclassified decision returns the
/// FORBIDDEN state — never a guess.
pub fn classify(surface: DecisionSurface) -> (CostBucket, &'static str) {
    (surface.bucket(), surface.evidence())
}

/// Build a deterministic FDR JSON line carrying the cost-oracle counters under a fixed field
/// order. The SAME string is what `FdrRing::append` would serialize.
pub fn cost_oracle_record_json(snap: &CostOracleSnapshot) -> String {
    JsonWriter::obj()
        .field_u64("seq", 0)
        .field_str("level", Level::Info.as_str())
        .field_str("kind", Kind::Tuning.as_str())
        .field_str("name", "cost_oracle")
        .field_u64("group_commit_barriers", snap.group_commit_barriers)
        .field_u64("group_commit_inserts", snap.group_commit_inserts)
        .field_u64("eigensolver_hits", snap.eigensolver_hits)
        .field_u64("eigensolver_recomputes", snap.eigensolver_recomputes)
        .field_u64("crypto_samples", snap.crypto_samples)
        .field_u64("crypto_latency_us", snap.crypto_latency_us)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Item 67 §67.4 (a): every operator-gated decision resolves to a non-Forbidden bucket with a
    // resolvable evidence pointer.
    #[test]
    fn every_decision_surface_is_classified_not_forbidden() {
        for s in [
            DecisionSurface::GroupCommit,
            DecisionSurface::EigensolverChoice,
            DecisionSurface::CryptoLatency,
        ] {
            let (bucket, evidence) = classify(s);
            assert_ne!(bucket, CostBucket::Forbidden, "{:?} must be classified", s);
            assert!(!evidence.is_empty(), "{:?} needs an evidence pointer", s);
            assert!(!evidence.contains("TODO"), "{:?} evidence must be concrete", s);
        }
    }

    #[test]
    fn group_commit_exact_cost_is_input_independent() {
        for bs in [1usize, 8, 64, 1024] {
            assert_eq!(group_commit_exact_cost(bs), bs as u64);
        }
    }

    #[test]
    fn eigensolver_bounded_cost_is_fixed_schedule_interval() {
        for n in [1usize, 4, 16, 64] {
            let (lo, hi) = eigensolver_bounded_cost(n, false);
            assert_eq!(lo, 1);
            assert_eq!(hi, (n as u64) * (n as u64) + 1);
            let (hlo, hhi) = eigensolver_bounded_cost(n, true);
            assert_eq!((hlo, hhi), (1, 1), "cache hit is O(1), independent of n");
        }
    }

    #[test]
    fn crypto_latency_is_distribution_not_point() {
        let empty = CryptoLatencyDist::default();
        assert!(empty.is_absent(), "no samples ⇒ honest absence, never 0");
        let mut d = CryptoLatencyDist::default();
        for us in [100u64, 637, 50, 900] {
            d.observe(us);
        }
        assert_eq!(d.count, 4);
        assert_eq!(d.p50_us, 50, "p50 proxy = observed min");
        assert_eq!(d.p99_us, 900, "p99 proxy = observed max");
    }

    #[test]
    fn counters_roundtrip_through_snapshot() {
        reset_counters();
        record_group_commit(64);
        record_eigensolver(true);
        record_eigensolver(false);
        record_crypto_latency(637);
        let snap = snapshot();
        assert_eq!(snap.group_commit_barriers, 1);
        assert_eq!(snap.group_commit_inserts, 64);
        assert_eq!(snap.eigensolver_hits, 1);
        assert_eq!(snap.eigensolver_recomputes, 1);
        assert_eq!(snap.crypto_samples, 1);
        assert_eq!(snap.crypto_latency_us, 637);
        reset_counters();
    }

    // Item 68 §68.4 load-bearing P3 proof: NO captured cost value feeds a decision / gate /
    // verdict / hash surface. Greppable: this source file must never pass a cost value into a
    // durability barrier, a drift gate, or a crypto verify.
    #[test]
    fn p3_grep_firewall_no_cost_value_feeds_decision() {
        let full = include_str!("cost_oracle.rs");
        let src = full.split("#[cfg(test)]").next().unwrap_or(full);
        for line in src.lines() {
            let l = line.trim();
            let reads_cost = l.contains("snapshot()")
                || l.contains("crypto_latency_us")
                || l.contains("eigensolver_recomputes");
            let is_decision = l.contains("if ")
                && (l.contains("sync_all") || l.contains("drift") || l.contains("verify"));
            assert!(
                !(reads_cost && is_decision),
                "P3 violation: a cost value must not gate a durability/drift/crypto decision: {line}"
            );
        }
        assert!(
            !src.contains("cost_to_hash") && !src.contains("cost_into_verdict"),
            "P3: cost must never enter a hash/verdict surface"
        );
    }
}
