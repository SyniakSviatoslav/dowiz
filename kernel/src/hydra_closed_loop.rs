//! hydra_closed_loop — kernel re-export shim for the `no_std` core engine.
//!
//! The pure closed-loop engine lives in `dowiz_core::hydra_closed_loop`; this
//! module re-exports it so existing kernel call sites (`crate::hydra_closed_loop::…`
//! and `dowiz_kernel::hydra_closed_loop::…`) keep working unchanged. The only
//! std-dependent piece is the runtime-probe golden test below, which spawns a
//! kernel binary and therefore stays in the std kernel crate.

pub use dowiz_core::hydra_closed_loop::*;

/// The stdout transcript of `hydra_runtime_probe --verify-golden --cycles <cycles>`:
/// one line per cycle, byte for byte what the binary prints. It lives in the library so
/// the SHA3-256 KAT below hashes the same bytes WITHOUT spawning a binary from a fixed
/// path (`/root/dowiz/kernel/target/debug/...` existed only on the dev box, and
/// `cargo test --lib` never builds the bin, so the KAT failed on every CI runner).
/// `log` receives the two stderr diagnostics (START / END), which the KAT does not hash.
pub fn golden_probe_transcript(cycles: usize, log: &mut dyn FnMut(&str)) -> String {
    use crate::event_log::{MemEventStore, MeshEvent};
    use crate::hydra::{OrganismState, TopoEdge};
    use crate::spectral::DriftClass;
    use core::fmt::Write as _;

    let nodes: usize = 5;
    // The ring 0->1->2->3->4->0, weight 1 (the binary's literal list, unchanged).
    let base: Vec<TopoEdge> =
        (0..nodes).map(|i| TopoEdge { from: i, to: (i + 1) % nodes, weight: 1.0 }).collect();
    let mut cl = HydraClosedLoop::new(MemEventStore::new(), nodes, base.clone(), 1.0, None);
    log(&format!(
        "AUTONOMOUS LOOP START state={:?} rho={:.6}",
        cl.state(),
        cl.baseline_rho()
    ));
    let mut out = String::new();
    for c in 0..cycles {
        let i = c % base.len();
        let delta = [TopoEdge { from: base[i].from, to: base[i].to, weight: 0.3 + (c as f64 % 5.0) }];
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [7u8; 32],
            actor_seq: cl.commit_count(),
            payload: Vec::new(),
        };
        let result = cl.commit_cycle(ev, &delta, false, |_| Ok(()));
        let organism_state = if matches!(cl.state(), OrganismState::Live) { "Live" } else { "Locked" };
        let drift = match result.drift_class {
            DriftClass::Damped => "Damped",
            DriftClass::Resonant => "Resonant",
            DriftClass::Unstable => "Unstable",
        };
        let _ = writeln!(
            out,
            "cycle={} accepted={} drift={} rho={:.6} state={}",
            c, result.accepted, drift, result.rho, organism_state
        );
    }
    log(&format!(
        "AUTONOMOUS LOOP END cycles={} state={:?} rho={:.6}",
        cycles,
        cl.state(),
        cl.baseline_rho()
    ));
    out
}

#[cfg(test)]
mod tests {
    /// Cryptographic golden test: the probe's verification transcript (what
    /// `hydra_runtime_probe --verify-golden --cycles 4` prints, produced by the one
    /// function the binary prints) must be a byte-exact stable sequence. Any change to
    /// probe output formatting or cycle behavior fails this SHA3-256 KAT.
    #[test]
    fn hydra_runtime_probe_golden_sha3_256() {
        let stdout = super::golden_probe_transcript(4, &mut |_| {});
        assert_eq!(stdout.lines().count(), 4, "one line per cycle: {stdout}");

        let expected_hash: [u8; 32] = [
            0xce, 0x55, 0x58, 0xcc, 0x67, 0x25, 0x5d, 0xa6,
            0x48, 0x14, 0x97, 0x43, 0xcb, 0x20, 0x10, 0x6a,
            0xd3, 0x1f, 0xff, 0x3c, 0x0c, 0x0d, 0x93, 0x33,
            0x4f, 0xbe, 0xf2, 0x91, 0xed, 0x87, 0xe3, 0x31,
        ];
        let actual_hash = crate::event_log::sha3_256(stdout.as_bytes());
        assert_eq!(
            actual_hash, expected_hash,
            "probe golden output hash mismatch"
        );
    }
}
