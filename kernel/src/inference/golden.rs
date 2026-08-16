//! Item 40 — Per-layer golden-CRC32 self-check (hard-fail to safe state): std shim.
//!
//! The pure no_std half — the integer/checksum math, the golden constants (`GOLDEN_L1`,
//! `GOLDEN_HIDDEN`, `GOLDEN_LOGITS`, `PINNED_VECTORS`), the `Layer` enum, `Weights`,
//! `ChecksumFault`, and the check driver (`self_check_core`) with the production entry
//! points (`self_check_all`, `self_check_all_with_weights`,
//! `self_check_all_with_activation_fault`) — lives in `dowiz_core::inference::golden` and is
//! re-exported below. This shim keeps ONLY the std-dependent parts: `fault` (which writes
//! the typed `Alarm` to a durable file-backed FDR `RingHandle`) and
//! `self_check_all_into_ring` (the test/standalone entry that drives the self-check into an
//! owned ring so the entry is recoverable/verifiable without racing the process-global sink).

pub use dowiz_core::inference::golden::*;

use crate::fdr::RingHandle;
use crate::inference::workspace::H;

/// The hard-fail trap: write the typed FDR `Kind::Alarm` entry recording the faulting
/// layer, then return `Err(ChecksumFault{layer})`. The production path calls
/// `fdr::emit_alarm` (a no-op unless an FDR sink is installed, so a healthy run — which
/// never reaches this fn — is silent). The test/standalone path also writes the SAME typed
/// Alarm record directly to an owned ring so the entry is recoverable/verifiable without
/// racing the process-global `OnceLock` sink.
fn fault(layer: Layer, ring: Option<&mut RingHandle>) -> Result<(), ChecksumFault> {
    crate::fdr::emit_alarm("checksum_fault", &format!("layer={}", layer.as_str()));
    // `RingHandle`/`FdrEvent::stamp` are both native-only (no filesystem, no clock on
    // wasm32 — see `fdr::RingHandle`/`FdrEvent::stamp` docs); `ring` is provably always
    // `None` on wasm32 since nothing there can construct a `RingHandle` value.
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(r) = ring {
        let ev = crate::fdr::schema::fdr_event_stamp(
            0,
            crate::fdr::Level::Error,
            crate::fdr::schema::Kind::Alarm,
            "checksum_fault".to_string(),
            crate::fdr::schema::StampPolicy::Full,
            vec![("layer", layer.as_str().to_string())],
        );
        let _ = r.append(&ev);
        let _ = r.sync();
    }
    #[cfg(target_arch = "wasm32")]
    let _ = ring;
    Err(ChecksumFault { layer })
}

/// Test/standalone variant: run the self-check against an **owned** FDR ring, writing the
/// fault's typed `Alarm` record directly to it (so the entry is recoverable and verifiable
/// without racing the process-global `OnceLock` sink, which may be owned by another test in
/// the same binary). The production path is [`self_check_all`] (global sink); this exists so
/// the planted-fault proofs can assert the FDR entry deterministically.
pub fn self_check_all_into_ring(
    ring: RingHandle,
    wk: &Weights,
    activation_override: Option<&[i8; H]>,
) -> Result<(), ChecksumFault> {
    let mut ring = Some(ring);
    self_check_core(wk, activation_override, |layer| fault(layer, ring.as_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::spec::{B1, B2, SCALE1, SCALE2, W1, W2};

    // ── FDR test harness (native only; wasm has no FDR ring) ────────────────────────
    #[cfg(not(target_arch = "wasm32"))]
    mod fdr {
        use super::*;
        use crate::fdr::ring;

        use core::cell::RefCell;
        /// A unique FDR ring for this test process. The fault's typed Alarm is written
        /// directly to it via [`self_check_all_into_ring`], so the entry is recoverable and
        use core::sync::atomic::{AtomicU64, Ordering};
        thread_local! {
            static TDIR: RefCell<Option<std::path::PathBuf>> = const { RefCell::new(None) };
        }
        fn ring_dir() -> std::path::PathBuf {
            let existing = TDIR.with(|c| c.borrow().clone());
            if let Some(d) = existing {
                d
            } else {
                static GLOBAL: AtomicU64 = AtomicU64::new(0);
                let n = GLOBAL.fetch_add(1, Ordering::SeqCst);
                let d =
                    std::env::temp_dir().join(format!("item40_fdr_{}_{}", std::process::id(), n));
                TDIR.with(|c| c.borrow_mut().replace(d.clone()));
                d
            }
        }
        /// verifiable WITHOUT racing the process-global `OnceLock` sink (which may be owned
        /// by another test in the same binary).
        fn fresh_ring() -> ring::FdrRing {
            let dir = ring_dir();
            let _ = crate::vfs::create_dir_all(&dir);
            ring::FdrRing::open(dir, 1 << 20).expect("open FDR ring")
        }

        fn dir() -> std::path::PathBuf {
            ring_dir()
        }

        fn alarms_with_layer(layer: Layer) -> usize {
            let rec = ring::recover(&dir());
            rec.records
                .iter()
                .filter(|r| {
                    r.kind == "alarm"
                        && r.name == "checksum_fault"
                        && r.raw.contains(&format!("\"layer\":\"{}\"", layer.as_str()))
                })
                .count()
        }

        /// §B5.4 — an uncorrupted run is checksum-SILENT (no false trip, no FDR entry).
        #[test]
        fn healthy_run_is_checksum_silent() {
            let _ = crate::vfs::remove_dir_all(dir());
            let r = fresh_ring();
            let res = self_check_all_into_ring(r, &Weights::spec(), None);
            assert!(res.is_ok(), "healthy run must not trip: {res:?}");
            assert_eq!(
                alarms_with_layer(Layer::L1PreRequant),
                0,
                "healthy run must write NO checksum_fault FDR entry"
            );
            assert_eq!(
                alarms_with_layer(Layer::Hidden),
                0,
                "healthy run must write NO checksum_fault FDR entry"
            );
            assert_eq!(
                alarms_with_layer(Layer::Logits),
                0,
                "healthy run must write NO checksum_fault FDR entry"
            );
        }

        /// §B5.2 / P7 — a planted SINGLE-BIT WEIGHT corruption hard-fails to safe state
        /// (Err at L1) AND writes the typed FDR `checksum_fault` entry. RED→GREEN: deleting
        /// the planted fault (or the check) turns the gate GREEN — the test IS the planted
        /// fault, re-executed by CI on every run (proof B5.5).
        #[test]
        fn planted_weight_fault_hard_fails_and_writes_fdr() {
            let _ = crate::vfs::remove_dir_all(dir());
            let r = fresh_ring();
            // Single-bit corruption of one frozen weight (W1[0]: 2 → 3).
            let mut w1 = W1;
            w1[0] ^= 1;
            let wk = Weights {
                w1: &w1,
                b1: &B1,
                scale1: SCALE1,
                w2: &W2,
                b2: &B2,
                scale2: SCALE2,
            };
            let res = self_check_all_into_ring(r, &wk, None);
            assert!(res.is_err(), "planted weight corruption must hard-fail");
            assert_eq!(
                res.unwrap_err().layer,
                Layer::L1PreRequant,
                "fault must be caught at layer 1"
            );
            assert!(
                alarms_with_layer(Layer::L1PreRequant) >= 1,
                "FDR checksum_fault entry required on weight fault"
            );
        }

        /// §B5.3 / P7 — a planted SINGLE-BIT ACTIVATION corruption (mid-pipeline hidden)
        /// hard-fails (caught at the next golden-checked boundary, `Logits`) AND writes the
        /// typed FDR `checksum_fault` entry. The corruption feeds layer 2's input, so the
        /// first golden-checked layer that diverges is the Logits layer. RED→GREEN: the test
        /// IS the planted fault (proof B5.5).
        #[test]
        fn planted_activation_fault_hard_fails_and_writes_fdr() {
            let _ = crate::vfs::remove_dir_all(dir());
            let r = fresh_ring();
            // Single-bit activation corruption: vec 0's oracle hidden is all-zero; flip
            // bit 0 → 1. This is a corrupted *activation*, not a weight.
            let mut hidden = [0i8; H];
            hidden[0] ^= 1;
            let res = self_check_all_into_ring(r, &Weights::spec(), Some(&hidden));
            assert!(res.is_err(), "planted activation corruption must hard-fail");
            assert_eq!(
                res.unwrap_err().layer,
                Layer::Logits,
                "activation fault must be caught at the Logits boundary"
            );
            assert!(
                alarms_with_layer(Layer::Logits) >= 1,
                "FDR checksum_fault entry required on activation fault"
            );
        }
    }
}
