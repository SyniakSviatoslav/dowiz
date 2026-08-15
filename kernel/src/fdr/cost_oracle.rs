//! `fdr/cost_oracle.rs` — std shim over the pure no_std core.
//!
//! The classification ([`CostBucket`]/[`DecisionSurface`]/[`classify`]) and the runtime
//! counters ([`record_group_commit`]/[`snapshot`]/…) live in `dowiz_core::fdr::cost_oracle`
//! and are re-exported here. This shim adds ONLY the std ring round-trip test — the
//! recoverable-from-ring oracle of the parent spec (needs a real `FdrRing`).

pub use dowiz_core::fdr::cost_oracle::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fdr::ring;
    use crate::fdr::schema::{fdr_event_stamp, Kind, StampPolicy};
    use dowiz_core::fdr::Level;

    fn tmp_ring(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "cost_oracle_ring_{}_{}_{}",
            tag,
            std::process::id(),
            crate::typed_metrics::mono_now_ns()
        ));
        let _ = crate::vfs::create_dir_all(&d);
        d
    }

    // Item 67/68 acceptance (parent spec): the runtime counters are recoverable from the FDR ring
    // after the call — a genuine A/B-segment ring round-trip, not a mocked string.
    #[test]
    fn cost_oracle_counters_recoverable_from_fdr_ring() {
        reset_counters();
        // Drive the three operator-gated decisions.
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

        // Emit + recover from a REAL FdrRing under a temp dir.
        let dir = tmp_ring("recover");
        {
            let mut ring = ring::FdrRing::open(dir.clone(), ring::DEFAULT_SEG_CAP).unwrap();
            let ev = fdr_event_stamp(
                0,
                Level::Info,
                Kind::Tuning,
                "cost_oracle".to_string(),
                StampPolicy::Cheap,
                vec![
                    (
                        "group_commit_barriers",
                        snap.group_commit_barriers.to_string(),
                    ),
                    ("group_commit_inserts", snap.group_commit_inserts.to_string()),
                    ("eigensolver_hits", snap.eigensolver_hits.to_string()),
                    (
                        "eigensolver_recomputes",
                        snap.eigensolver_recomputes.to_string(),
                    ),
                    ("crypto_samples", snap.crypto_samples.to_string()),
                    ("crypto_latency_us", snap.crypto_latency_us.to_string()),
                ],
            );
            ring.append(&ev).unwrap();
        }
        let rec = ring::recover(&dir);
        assert_eq!(rec.records.len(), 1, "exactly one cost-oracle record");
        // The recovered payload must carry the SAME counters (round-trip integrity).
        let raw = &rec.records[0].raw;
        for (k, v) in [
            ("group_commit_barriers", "1"),
            ("group_commit_inserts", "64"),
            ("eigensolver_hits", "1"),
            ("eigensolver_recomputes", "1"),
            ("crypto_samples", "1"),
            ("crypto_latency_us", "637"),
        ] {
            let needle = format!("\"{k}\":\"{v}\"");
            assert!(
                raw.contains(&needle),
                "recovered payload must contain {needle}: {raw}"
            );
        }
        let _ = crate::vfs::remove_dir_all(&dir);
        reset_counters();
    }
}
