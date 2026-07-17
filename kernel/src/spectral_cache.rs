//! spectral_cache.rs — `DecompCache`: a content-addressed spectral
//! eigensolve cache (P11 §2).
//!
//! Zero-dep kernel primitive (no operator decisions, non-red-line). The kernel
//! spends real time in `spectral::eigenvalues` (Faddeev-LeVerrier +
//! Durand-Kerner) every time `markov` re-analyses a window. When the underlying
//! operator/transition matrix is unchanged the spectrum is identical, so we
//! cache it behind a content-address string (a store `snapshot_root`, or a
//! deterministic hash of matrix contents).
//!
//! The honesty device is the monotonic `recomputes` counter — a *falsifier*.
//! It must be exactly 0 across repeated identical roots (proving we are NOT
//! thrashing — recomputing on every call) and must increment by exactly 1 on a
//! genuine root change (proving we are NOT serving a stale stuck cache). The
//! initial population of an empty cache is not a "recompute"; only a change of
//! key is. Two downstream tests below pin this invariant.

use std::sync::atomic::{AtomicU64, Ordering};

/// Cached spectral decomposition payload.
///
/// `(basis, values)` — `basis` is the (possibly deferred) eigenvector matrix
/// and `values` is the eigenvalue payload. For the kernel's vectorless
/// spectrum usage (`slem`, `dominant_period`) the eigenvalue moduli are the
/// operative quantity, so the cache stores them directly; an eigenvector basis
/// may be supplied by callers that solve for one. This is the "simpler payload
/// representing the eigensolve result" the P11 §2 spec allows.
pub type Decomp = (Vec<Vec<f64>>, Vec<f64>);

/// Content-addressed cache for an expensive eigen-decomposition.
///
/// * `key`     — the content-address of the cached input (None until first fill).
/// * `cached`  — the cached decomposition `(basis, values)`, or None when empty.
/// * `recomputes` — monotonic count of *key changes* that forced a recompute.
///
/// Usable behind `&mut` only; no interior mutex — keeps it std-only and simple.
pub struct DecompCache {
    key: Option<String>,
    cached: Option<Decomp>,
    recomputes: AtomicU64,
}

impl DecompCache {
    /// Empty cache: no key, no payload, zero recomputes.
    pub fn new() -> Self {
        Self {
            key: None,
            cached: None,
            recomputes: AtomicU64::new(0),
        }
    }

    /// Return the cached decomposition for `root`, recomputing ONLY when the
    /// root differs from the cached key.
    ///
    /// * HIT (`root == self.key`): the closure is NOT called and `recomputes`
    ///   is NOT touched — the cached reference is returned directly.
    /// * MISS: `compute()` runs, the result is stored, `self.key = root`, and
    ///   `recomputes` is incremented by exactly 1 — **but only when an existing
    ///   key is being replaced**. The very first population of an empty cache
    ///   is initialisation, not a recompute, so it does not increment.
    pub fn get_or_recompute(
        &mut self,
        root: &str,
        compute: impl FnOnce() -> Decomp,
    ) -> &Decomp {
        if self.key.as_deref() == Some(root) {
            // HIT — serve the cached decomposition, no recompute.
            return self.cached.as_ref().expect("key set implies payload set");
        }
        // MISS — solve, store, and (if we are replacing a prior key) count it.
        let replacing = self.key.is_some();
        let payload = compute();
        self.cached = Some(payload);
        self.key = Some(root.to_string());
        if replacing {
            self.recomputes.fetch_add(1, Ordering::SeqCst);
        }
        self.cached.as_ref().unwrap()
    }

    /// Monotonic count of recomputations forced by a root change (read atomically).
    pub fn recomputes(&self) -> u64 {
        self.recomputes.load(Ordering::SeqCst)
    }
}

impl Default for DecompCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic content-address for a real matrix — FNV-1a 64 over a canonical
/// (row-major, index-framed) byte layout. Two matrices with identical contents
/// yield the same root on every platform/run; any entry change changes the root.
/// Keeps the cache honest and content-addressed without a store handle.
pub fn matrix_content_address(a: &[Vec<f64>]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = OFFSET;
    for (i, row) in a.iter().enumerate() {
        // frame the row index so a value bleeding across rows can't collide
        h ^= (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        h = h.wrapping_mul(PRIME);
        for &x in row {
            h ^= x.to_bits();
            h = h.wrapping_mul(PRIME);
        }
    }
    format!("{:016x}", h)
}

/// Content-addressed `spectral::slem` (second-largest eigenvalue modulus) routed
/// through a `DecompCache`. The expensive `eigenvalues` solve is performed only
/// when `a`'s content-address changes; otherwise the cached spectrum is reused.
pub fn slem_cached(cache: &mut DecompCache, a: &[Vec<f64>]) -> f64 {
    let root = matrix_content_address(a);
    let (_basis, values) = cache.get_or_recompute(&root, || {
        let eigs = crate::spectral::eigenvalues(a);
        // vectorless usage only needs the eigenvalue moduli as the payload.
        (Vec::new(), eigs.iter().map(|e| e.abs()).collect::<Vec<f64>>())
    });
    let mut mags: Vec<f64> = values.clone();
    mags.sort_by(|x, y| y.partial_cmp(x).unwrap_or(core::cmp::Ordering::Equal));
    if mags.len() > 1 {
        mags[1]
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed dummy decomposition used by the falsifier tests.
    fn dummy() -> Decomp {
        (
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![0.5, 0.3],
        )
    }

    /// FALSIFIER (no-thrash): 1000 `get_or_recompute` calls with the SAME root
    /// must NOT increment `recomputes` — the closure runs exactly once (the
    /// initial population) and every subsequent call is a HIT. If the cache
    /// thrashed (recomputed on every call) this would be ~1000, not 0.
    #[test]
    fn decomp_cache_zero_recomputes_on_unchanged_root() {
        let mut cache = DecompCache::new();
        let root = "root::unchanged";
        let mut compute_calls = 0usize;
        for _ in 0..1000 {
            let got = cache.get_or_recompute(root, || {
                compute_calls += 1;
                dummy()
            });
            // the returned decomposition must be the dummy, every time (no stale).
            assert_eq!(got.1, vec![0.5, 0.3]);
        }
        assert_eq!(compute_calls, 1, "compute must run exactly once (warmup)");
        assert_eq!(
            cache.recomputes(),
            0,
            "identical root ⇒ zero recomputes (no thrashing)"
        );
    }

    /// FALSIFIER (no-stale): after the 1000 identical calls, a root CHANGE must
    /// increment `recomputes` by exactly 1 and return the NEW decomposition. If
    /// the cache were stuck/serving-stale it would stay 0 and return the old
    /// payload; if it thrashed it would be >1.
    #[test]
    fn decomp_cache_exactly_one_recompute_on_change() {
        let mut cache = DecompCache::new();
        let root_a = "root::A";
        for _ in 0..1000 {
            cache.get_or_recompute(root_a, dummy);
        }
        assert_eq!(cache.recomputes(), 0);

        let root_b = "root::B";
        // Scope `got` so its `&Decomp` mutable borrow ends before we query
        // `recomputes()` (an immutable borrow). Ending the &Decomp borrow first
        // is the natural usage; the cache stays a `&mut`-only API.
        let vals_b = {
            let got = cache.get_or_recompute(root_b, || {
                (vec![vec![2.0, 0.0], vec![0.0, 2.0]], vec![0.9, 0.7])
            });
            got.1.clone()
        };
        assert_eq!(
            cache.recomputes(),
            1,
            "exactly one recompute on a genuine root change"
        );
        assert_eq!(vals_b, vec![0.9, 0.7], "new decomposition is served");

        // and a second identical call on the new root stays a hit (still 1).
        let vals_b2 = {
            let got2 = cache.get_or_recompute(root_b, dummy);
            got2.1.clone()
        };
        assert_eq!(cache.recomputes(), 1, "no thrash on the new root either");
        assert_eq!(vals_b2, vec![0.9, 0.7]);
    }
}
