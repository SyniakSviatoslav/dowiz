//! P11 §7 — CorePinning trait seam (Trait-as-Port).
//!
//! A pluggable CPU-core-affinity port. The kernel core owns the *seam*; the
//! concrete affinity strategy (e.g. a future NUMA-aware pinner) plugs in via
//! this trait without the kernel importing any locality machinery.
//!
//! Today this ships **only the seam** with a zero-cost `NoOpCorePinning`
//! default. No NUMA crate, no new dependency. The DECART decision defers the
//! real crate: on a single-socket host there is no locality to exploit, so a
//! pinner would be a guaranteed no-win (added complexity, no measurable
//! throughput gain). When a multi-socket / NUMA host appears, swap in a real
//! impl behind this same trait — the call sites do not change.

/// Error returned by a failed pin attempt.
///
/// The no-op default never fails; this type exists so a real (NUMA-aware)
/// impl has a typed failure channel to report through.
pub struct PinError(pub String);

/// Pluggable CPU-core-affinity port.
///
/// `pin_current` is called from the kernel's worker-spawn path to bind the
/// *currently executing* thread/worker to a logical core. `topology` reports
/// the set of cores the host exposes so a scheduler can enumerate candidates.
pub trait CorePinning {
    /// Bind the current thread to `core_id`.
    fn pin_current(&self, core_id: usize) -> Result<(), PinError>;

    /// The list of logical core ids available on the host (e.g. `0..n`).
    fn topology(&self) -> Vec<usize>;
}

/// Zero-cost default: does nothing, reports the host's logical core count.
///
/// Honest no-op — documents the DECART decision inline. On a single-socket
/// host there is no memory/local-cache asymmetry to exploit, so binding a
/// thread to a core buys nothing and risks fighting the OS scheduler. We
/// therefore report the topology (so callers can still size pools correctly)
/// but perform no actual affinity operation.
pub struct NoOpCorePinning;

impl CorePinning for NoOpCorePinning {
    fn pin_current(&self, _core_id: usize) -> Result<(), PinError> {
        // DECART-deferred: single-socket host today ⇒ no locality to exploit,
        // expected no-win. Real affinity (NUMA-aware) plugs in here later
        // behind this same trait without touching call sites.
        Ok(())
    }

    fn topology(&self) -> Vec<usize> {
        // Report the host's logical core count so schedulers can size pools;
        // fall back to a single core if the query fails (degraded but safe).
        std::thread::available_parallelism()
            .map(|n| (0..n.get()).collect())
            .unwrap_or_else(|_| vec![0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seam is trivially correct: the honest no-op returns Ok and the
    /// reported topology is always non-empty (at least one core).
    #[test]
    fn noop_core_pinning_is_honest() {
        let pinner = NoOpCorePinning;
        assert!(pinner.pin_current(0).is_ok());
        let topo = pinner.topology();
        assert!(!topo.is_empty(), "topology must be non-empty");
        // Core ids must be contiguous from 0 (the contract we document).
        assert_eq!(topo.first(), Some(&0));
    }
}
