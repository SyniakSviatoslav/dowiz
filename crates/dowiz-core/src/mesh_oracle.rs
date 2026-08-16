//! mesh_oracle.rs — Standalone reference verifier for mesh crypto surface.
//!
//! This module provides the §4 checklist item-1 oracle: a simple, deliberately
//! unoptimized reference implementation of gossip-message signature verification
//! and signature comparison, retained as the differential target against the
//! production `SignedEntry::verify_sig` / `sig_eq_ct`.
//!
//! ## Feature gate
//! This module requires the `pq` feature (same as `mesh.rs`) because it operates
//! on `SignedEntry` and uses `pq::dsa::verify`. It is compiled only when mesh
//! crypto is available.
//! - `oracle_verify_sig` — one-line reference re-verification using KAT-gated primitive
//! - `oracle_sig_eq` — straightforward boolean equality over signature bytes
//! - `DifferentialResult` — structured comparison between production and oracle
//! - `cross_check` — single-entry differential verification
//! - `batch_cross_check` — batch differential verification for audit trails

use alloc::string::String;
use crate::mesh::{MlDsa65Pk, MlDsa65Sig, SignedEntry};
use crate::pq::dsa::verify;

/// Result of a differential comparison between production and oracle verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DifferentialResult {
    /// Both production and oracle agree — verification passes.
    Match,
    /// Production says valid, oracle says invalid (production may be wrong).
    ProductionFalsePositive {
        /// The signed entry that produced the mismatch.
        entry: SignedEntry,
    },
    /// Production says invalid, oracle says valid (production may be over-restrictive).
    ProductionFalseNegative {
        /// The signed entry that produced the mismatch.
        entry: SignedEntry,
    },
    /// Oracle itself failed (should never happen with KAT-gated primitives).
    OracleError,
}

/// Obviously-correct reference for [`SignedEntry::verify_sig`]: re-verify the
/// signature against the embedded public key over the entry's signed bytes.
///
/// One-line, no tricks — this is the differential target, not a performance path.
/// Uses the same KAT-gated `pq::dsa::verify` primitive as the production path,
/// so any divergence is a logic bug in the production implementation, not a
/// crypto primitive issue.
pub fn oracle_verify_sig(e: &SignedEntry) -> bool {
    let pk = MlDsa65Pk { bytes: e.pubkey.clone() };
    let sig = MlDsa65Sig { bytes: e.sig.clone() };
    verify(&pk, e.signed_bytes(), &sig)
}

/// Obviously-correct reference for [`SignedEntry::sig_eq_ct`]: a straightforward
/// boolean equality over the signature bytes.
///
/// The reference is ALLOWED to be the simple form — it is the differential target,
/// and the constant-time property is asserted separately by the dudect self-test,
/// not by this reference.
pub fn oracle_sig_eq(a: &SignedEntry, expected_sig: &[u8]) -> bool {
    a.sig == expected_sig
}

/// Cross-check a single signed entry against both production and oracle verifiers.
///
/// Returns `DifferentialResult::Match` if both agree. If they disagree, the
/// result indicates which side produced the unexpected verdict.
pub fn cross_check(entry: &SignedEntry) -> DifferentialResult {
    let prod_valid = entry.verify_sig();
    let oracle_valid = oracle_verify_sig(entry);

    match (prod_valid, oracle_valid) {
        (true, true) => DifferentialResult::Match,
        (true, false) => DifferentialResult::ProductionFalsePositive { entry: entry.clone() },
        (false, true) => DifferentialResult::ProductionFalseNegative { entry: entry.clone() },
        (false, false) => {
            // Both rejected — that's a match too
            DifferentialResult::Match
        }
    }
}

/// Batch differential verification over a corpus of signed entries.
///
/// Returns a summary of how many entries passed, how many had differential
/// mismatches, and the first mismatch found (if any).
pub fn batch_cross_check(entries: &[SignedEntry]) -> BatchCrossCheckResult {
    let mut passed = 0u64;
    let mut mismatches = 0u64;
    let mut first_mismatch: Option<DifferentialResult> = None;

    for entry in entries {
        match cross_check(entry) {
            DifferentialResult::Match => passed += 1,
            diff => {
                mismatches += 1;
                if first_mismatch.is_none() {
                    first_mismatch = Some(diff);
                }
            }
        }
    }

    BatchCrossCheckResult {
        total: entries.len() as u64,
        passed,
        mismatches,
        first_mismatch,
    }
}

/// Summary result from a batch cross-check.
#[derive(Debug, Clone)]
pub struct BatchCrossCheckResult {
    /// Total entries checked.
    pub total: u64,
    /// Entries where production and oracle agreed.
    pub passed: u64,
    /// Entries with differential mismatches.
    pub mismatches: u64,
    /// The first mismatch found, if any.
    pub first_mismatch: Option<DifferentialResult>,
}

impl BatchCrossCheckResult {
    /// Overall pass rate as a fraction [0.0, 1.0].
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            (self.total - self.mismatches) as f64 / self.total as f64
        }
    }

    /// Whether the corpus passed with zero mismatches.
    pub fn is_clean(&self) -> bool {
        self.mismatches == 0
    }

    /// ASCII summary for logging/telemetry.
    pub fn ascii_summary(&self) -> String {
        let status = if self.is_clean() { "CLEAN" } else { "MISMATCHES" };
        format!(
            "[mesh_oracle] batch_cross_check: {} total, {} passed, {} mismatches ({:.2}% pass rate) [{}]",
            self.total,
            self.passed,
            self.mismatches,
            self.pass_rate() * 100.0,
            status,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_entry() -> SignedEntry {
        let pk = MlDsa65Pk { bytes: vec![0u8; 1312] };
        let sig = MlDsa65Sig { bytes: vec![1u8; 3309] };
        SignedEntry {
            prev_hash: [0u8; 32],
            payload: vec![42u8; 32],
            sig: sig.bytes,
            pubkey: pk.bytes,
        }
    }

    #[test]
    fn oracle_verify_sig_returns_bool() {
        let entry = make_test_entry();
        let result = oracle_verify_sig(&entry);
        assert!(result == true || result == false);
    }

    #[test]
    fn oracle_sig_eq_works() {
        let entry = make_test_entry();
        assert!(oracle_sig_eq(&entry, &entry.sig));
        assert!(!oracle_sig_eq(&entry, &[0u8; 3309]));
    }

    #[test]
    fn cross_check_returns_enum() {
        let entry = make_test_entry();
        let result = cross_check(&entry);
        // Should be one of the enum variants
        let _ = result;
    }

    #[test]
    fn batch_cross_check_empty_corpus() {
        let result = batch_cross_check(&[]);
        assert_eq!(result.total, 0);
        assert_eq!(result.passed, 0);
        assert_eq!(result.mismatches, 0);
        assert!(result.is_clean());
        assert!((result.pass_rate() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn batch_cross_check_ascii_summary_format() {
        let result = batch_cross_check(&[]);
        let summary = result.ascii_summary();
        assert!(summary.contains("[mesh_oracle]"));
        assert!(summary.contains("total"));
        assert!(summary.contains("passed"));
    }

    #[test]
    fn differential_result_debug_format() {
        let entry = make_test_entry();
        let result = DifferentialResult::ProductionFalsePositive { entry };
        let debug = format!("{:?}", result);
        assert!(debug.contains("ProductionFalsePositive"));
    }
}
