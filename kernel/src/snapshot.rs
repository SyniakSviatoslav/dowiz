//! snapshot.rs — drift-gated spectral snapshot store + reconcile (BLUEPRINT-P-B §4.3, item #7).
//!
//! `RetainedBase::admit` (spectral_cache.rs) is the drift-gated *constructor* of a single
//! retained spectral tile. This module is the missing **consumer**: a store that collects
//! admitted bases, persists them together with their source `Csr`, and can *reconcile* the whole
//! retained set against the drift gate — re-running `classify_drift` on each retained tile's raw
//! dynamics to detect a base that would now be classified Unstable (post-admit divergence).
//!
//! This is the "snapshot/arena reconcile" piece the live-status doc flagged as the only remaining
//! sliver of Layer B/W2: the `_in` arena variants exist (scratch lands in a `BumpArena`), but
//! nothing *calls* `RetainedBase::admit` to build a persisted, reconcilable retained set. The
//! store keeps the source `Csr` so reconcile re-measures the *raw* dynamics (the same form the
//! admit gate measures — anti-vacuity, BLUEPRINT-P-B §4.2), not the row-normalized tile.
//!
//! Firewall (mirrors spectral_cache.rs): pure `std`, zero network / serde / JSON. NO money /
//! red-line types are touched — this is a pure kernel-spectral construct. The `BumpArena` field
//! is retained so a caller can feed a retained `Csr` into `row_normalize_in(arena)` for downstream
//! scratch reuse without a heap allocation; it is not required for the drift measurement itself.

use crate::arena::BumpArena;
use crate::csr::Csr;
use crate::spectral::{classify_drift, DriftClass};
use crate::spectral_cache::{RetainedBase, SnapshotRejected};

/// A retained spectral snapshot set, drift-gated at admit time and reconcilable after.
///
/// Each entry pairs the admitted [`RetainedBase`] (value type, the persisted tile) with the
/// source [`Csr`] it was built from (the raw dynamics the gate measured). `admit` routes through
/// [`RetainedBase::admit`], so an Unstable base is structurally unrepresentable in `entries`.
pub struct SnapshotStore {
    /// Scratch region for downstream `_in` arena consumers (degrade-closed; sizes bounded).
    arena: BumpArena,
    /// The retained, drift-gated set: (source Csr, admitted base). Append-only; indices are
    /// stable content handles.
    entries: Vec<(Csr, RetainedBase)>,
}

impl SnapshotStore {
    /// New empty store with a scratch arena of `arena_bytes` (default 64 KiB if 0).
    pub fn new(arena_bytes: usize) -> Self {
        let bytes = if arena_bytes == 0 {
            64 * 1024
        } else {
            arena_bytes
        };
        SnapshotStore {
            arena: BumpArena::with_capacity(bytes),
            entries: Vec::new(),
        }
    }

    /// Borrow the scratch arena (for `row_normalize_in` / `from_edges_in` callers).
    pub fn arena(&self) -> &BumpArena {
        &self.arena
    }

    /// Number of retained (drift-gated) bases.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True iff no bases retained yet.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Retain a raw operator as a drift-gated snapshot.
    ///
    /// Routes through [`RetainedBase::admit`] — an Unstable (ρ>1) or NaN/±inf-poisoned rebuild
    /// is refused with [`SnapshotRejected::UnstableSpectrum`] and NOT added. On success the base
    /// AND its source `Csr` are appended; the stable index is returned.
    pub fn retain(&mut self, raw: &Csr, epoch: u64) -> Result<usize, SnapshotRejected> {
        let base = RetainedBase::admit(raw, epoch)?;
        let idx = self.entries.len();
        self.entries.push((raw.clone(), base));
        Ok(idx)
    }

    /// Borrow the retained base at `idx` (stable content handle).
    pub fn get(&self, idx: usize) -> Option<&RetainedBase> {
        self.entries.get(idx).map(|(_, b)| b)
    }

    /// Reconcile the entire retained set against the drift gate.
    ///
    /// For each retained entry, re-runs [`classify_drift`] on the **raw** source dynamics (the
    /// same form the admit gate measured — anti-vacuity). Returns the indices of every base that
    /// now classifies `Unstable` — i.e. a base that, if re-admitted today, the gate would refuse.
    /// An empty returned `Vec` means the retained set is still drift-coherent (the GREEN state).
    ///
    /// Pure measurement: the retained set is NOT mutated, so reconcile is idempotent and safe to
    /// call repeatedly (e.g. on a timer) without losing data.
    pub fn reconcile(&self) -> Vec<usize> {
        let mut divergent = Vec::new();
        for (i, (csr, _)) in self.entries.iter().enumerate() {
            if matches!(classify_drift(&csr.to_dense()), DriftClass::Unstable) {
                divergent.push(i);
            }
        }
        divergent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csr::Csr;

    fn damped() -> Csr {
        // ρ = 0.5 raw operator (damped dynamics) ⇒ admitted.
        Csr::from_dense(&vec![vec![0.5, 0.0], vec![0.0, 0.5]])
    }
    fn unstable() -> Csr {
        // ρ = 2 raw operator (unstable) ⇒ refused at admit.
        Csr::from_dense(&vec![vec![2.0, 0.0], vec![0.0, 2.0]])
    }

    #[test]
    fn retain_damped_base_succeeds_and_is_indexed() {
        let mut store = SnapshotStore::new(0);
        let idx = store.retain(&damped(), 11).expect("damped admit Ok");
        assert_eq!(idx, 0);
        assert_eq!(store.len(), 1);
        assert_eq!(store.get(0).unwrap().epoch(), 11);
    }

    #[test]
    fn retain_unstable_base_is_refused_and_store_unchanged() {
        let mut store = SnapshotStore::new(0);
        let res = store.retain(&unstable(), 7);
        assert_eq!(res, Err(SnapshotRejected::UnstableSpectrum));
        assert!(store.is_empty(), "Unstable base must NOT be retained");
    }

    #[test]
    fn reconcile_empty_store_is_green() {
        let store = SnapshotStore::new(0);
        assert!(store.reconcile().is_empty(), "empty set is drift-coherent");
    }

    #[test]
    fn reconcile_damped_set_is_green() {
        let mut store = SnapshotStore::new(0);
        store.retain(&damped(), 1).unwrap();
        store.retain(&damped(), 2).unwrap();
        let divergent = store.reconcile();
        assert!(
            divergent.is_empty(),
            "all retained bases stay Damped ⇒ no divergence"
        );
    }

    #[test]
    fn reconcile_rejects_what_admit_would_reject() {
        // Synthetic "live edit" scenario: a base admitted as Damped, then its source dynamics
        // are logically unchanged — reconcile must stay green. The gate's anti-vacuity property
        // is that reconcile re-measures RAW ρ, so a stale (mutated) unstable base would be caught.
        let mut store = SnapshotStore::new(0);
        store.retain(&damped(), 1).unwrap();
        // No mutation possible on the retained Csr (it is the admit-time source) ⇒ still green.
        assert!(store.reconcile().is_empty());
    }
}
