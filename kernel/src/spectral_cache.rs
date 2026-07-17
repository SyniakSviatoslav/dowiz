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
    pub fn get_or_recompute(&mut self, root: &str, compute: impl FnOnce() -> Decomp) -> &Decomp {
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

/// Bit pattern of a canonical quiet-NaN used by [`canonical_content_address`] to
/// map any NaN input to a single deterministic id (so corrupted tiles are
/// content-addressed identically across runs/platforms).
pub const CANONICAL_QUIET_NAN: u64 = 0x7ff8_0000_0000_0000;

/// Normalise one entry to its hashable bit pattern on the canonical path:
/// * `-0.0` and `+0.0` both collapse to the `+0.0` bit pattern (value-identical
///   tiles must share a key);
/// * any NaN folds to [`CANONICAL_QUIET_NAN`] (deterministic corruption id).
/// No transcendental appears here — only `f64::to_bits`, which is exact.
#[inline]
fn canonical_bits(v: f64) -> u64 {
    if v == 0.0 {
        // collapses both -0.0 and +0.0 to +0.0 bits
        0.0f64.to_bits()
    } else if v.is_nan() {
        CANONICAL_QUIET_NAN
    } else {
        v.to_bits()
    }
}

/// In debug builds, flag NaN input up front (the canonical hash is for
/// well-formed numeric tiles; NaN is corruption that must be surfaced, not
/// silently canonicalised). Release builds keep NaN → `CANONICAL_QUIET_NAN`
/// for deterministic cross-run ids (adversarial determinism test).
#[cfg(debug_assertions)]
fn assert_no_nan(a: &[Vec<f64>]) {
    for row in a {
        for &x in row {
            assert!(
                !x.is_nan(),
                "canonical_content_address: NaN-bearing tile is flagged in debug (release folds to CANONICAL_QUIET_NAN)"
            );
        }
    }
}

/// Canonical (scale-invariant) content-address for `slem_cached`.
///
/// Global-pivot scaling, NOT row-stochastic normalisation: row-normalising
/// would equate matrices that differ by *per-row* scale and have genuinely
/// different spectra (a worse bug than the one being fixed). Global-pivot
/// scaling equates exactly the uniform-scale family `{c·W : c > 0}`, whose
/// spectra differ only by the known factor `c`.
///
/// Mechanics (hazard-safety-as-math, blueprint item 6): let `p` = |first
/// nonzero entry| in row-major order. Hash the entries `x / p` with the same
/// FNV-1a framing as [`matrix_content_address`], mapping `-0.0 → +0.0` and any
/// NaN → [`CANONICAL_QUIET_NAN`]. IEEE-754 mandates correctly-rounded division,
/// so for an exactly-scaled family (`c·x` representable exactly — integer
/// counts, power-of-two `c`) `fl((c·x)/(c·p)) ≡ fl(x/p)` bitwise on every
/// conforming target. No summation and no transcendental appear on this path
/// (both would break scale-commutativity), so the two V2 §A constraints hold
/// structurally. Zero matrix ⇒ no pivot ⇒ hash raw entries (well-defined).
pub fn canonical_content_address(a: &[Vec<f64>]) -> String {
    #[cfg(debug_assertions)]
    assert_no_nan(a);

    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = OFFSET;

    // pivot p = |first nonzero entry| in row-major order
    let mut pivot = 0.0f64;
    'outer: for row in a {
        for &x in row {
            if x != 0.0 {
                pivot = x.abs();
                break 'outer;
            }
        }
    }

    for (i, row) in a.iter().enumerate() {
        // frame the row index so a value bleeding across rows can't collide
        h ^= (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        h = h.wrapping_mul(PRIME);
        for &x in row {
            let v = if pivot > 0.0 { x / pivot } else { x };
            h ^= canonical_bits(v);
            h = h.wrapping_mul(PRIME);
        }
    }
    format!("{:016x}", h)
}

/// Content-addressed `spectral::slem` (second-largest eigenvalue modulus) routed
/// through a `DecompCache`. The expensive `eigenvalues` solve is performed only
/// when `a`'s content-address changes; otherwise the cached spectrum is reused.
pub fn slem_cached(cache: &mut DecompCache, a: &[Vec<f64>]) -> f64 {
    let root = canonical_content_address(a);
    // pivot p = |first nonzero entry| (row-major); the canonical operator is
    // a / p, whose spectrum is the scale-free family member shared by every
    // uniform scale of `a`, so the cached payload is cross-node meaningful.
    let mut p = 0.0f64;
    'outer: for row in a {
        for &x in row {
            if x != 0.0 {
                p = x.abs();
                break 'outer;
            }
        }
    }
    // Build the pivot-scaled canonical operator once (moved into the closure).
    let canon_op: Vec<Vec<f64>> = if p > 0.0 {
        a.iter()
            .map(|row| row.iter().map(|&x| x / p).collect::<Vec<f64>>())
            .collect()
    } else {
        a.to_vec()
    };
    let (_basis, values) = cache.get_or_recompute(&root, || {
        let eigs = crate::spectral::eigenvalues(&canon_op);
        // vectorless usage only needs the eigenvalue moduli as the payload.
        (
            Vec::new(),
            eigs.iter().map(|e| e.abs()).collect::<Vec<f64>>(),
        )
    });
    let mut mags: Vec<f64> = values.clone();
    mags.sort_by(|x, y| y.partial_cmp(x).unwrap_or(core::cmp::Ordering::Equal));
    let slem_canonical = if mags.len() > 1 {
        mags[1]
    } else {
        0.0
    };
    // Scale the scale-free canonical SLEM back by the pivot to honour the
    // caller-facing contract: slem_cached(a) == slem(a) == slem(a/p) · p.
    slem_canonical * p
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed dummy decomposition used by the falsifier tests.
    fn dummy() -> Decomp {
        (vec![vec![1.0, 0.0], vec![0.0, 1.0]], vec![0.5, 0.3])
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

    // ── T3/A5 (W1-L10, doc-19 bridge-gap #1) RED tests ────────────────────────
    /// Integer-valued fixture tile whose uniform-scale family is exactly
    /// representable (power-of-two and small-int scales). This is the canonical
    /// "logical tile" two nodes may build at different scale.
    fn fixture_tile() -> Vec<Vec<f64>> {
        // A simple stochastic-like matrix of small integers.
        vec![
            vec![1.0, 2.0, 0.0],
            vec![3.0, 1.0, 2.0],
            vec![0.0, 1.0, 3.0],
        ]
    }

    /// Multiply every entry of `a` by `c` (uniform scale).
    fn scale(a: &[Vec<f64>], c: f64) -> Vec<Vec<f64>> {
        a.iter()
            .map(|row| row.iter().map(|&x| x * c).collect::<Vec<f64>>())
            .collect()
    }

    /// Build a 1×1 tile containing exactly `x`.
    fn tile_with(x: f64) -> Vec<Vec<f64>> {
        vec![vec![x]]
    }

    /// RED (scale-invariance bug): `W`, `2·W`, `3·W` are the same logical tile;
    /// the cache must fill once and HIT twice → `recomputes() == 0`. Today the
    /// raw hash keyed on `to_bits` makes each scale a distinct key ⇒ 2 recomputes.
    /// TODAY this FAILS (recomputes == 2) — proving the bug live before the fix.
    #[test]
    fn slem_cached_scale_invariant_key_and_payload() {
        let w = fixture_tile();
        let mut cache = DecompCache::new();
        let s1 = slem_cached(&mut cache, &w);
        let w2 = scale(&w, 2.0);
        let w3 = scale(&w, 3.0);
        let s2 = slem_cached(&mut cache, &w2);
        let s3 = slem_cached(&mut cache, &w3);
        assert_eq!(
            cache.recomputes(),
            0,
            "W, 2W, 3W share one canonical key ⇒ zero recomputes"
        );
        assert!(
            (s2 - 2.0 * s1).abs() < 1e-12 && (s3 - 3.0 * s1).abs() < 1e-12,
            "slem scales linearly with pivot p: s2≈2·s1, s3≈3·s1 (got s1={s1}, s2={s2}, s3={s3})"
        );
    }

    /// RED (`-0.0`/`+0.0`): value-identical but bit-distinct tiles must canonicalise
    /// to the SAME id. Today `to_bits` differs for -0.0 vs +0.0 ⇒ distinct roots.
    /// TODAY this FAILS (ids differ) — proving the latent case live before the fix.
    #[test]
    fn neg_zero_and_pos_zero_are_the_same_tile() {
        assert_eq!(
            canonical_content_address(&tile_with(0.0)),
            canonical_content_address(&tile_with(-0.0)),
            "-0.0 and +0.0 must canonicalise to the same content id"
        );
    }

    // ── T3/A5 adversarial guards (designed to break a wrong fix) ──────────────

    /// ADVERSARIAL (i) over-normalization guard: row-stochastic normalisation
    /// would equate `W` with `D·W` for DISTINCT per-row factors D — but those
    /// matrices have genuinely different spectra, so they MUST keep distinct
    /// canonical keys. This is the test that goes RED if an implementer
    /// "helpfully" switches to per-row normalisation.
    #[test]
    fn distinct_per_row_scale_does_not_collide() {
        let w = fixture_tile();
        // D·W with DISTINCT per-row factors (2,1,3): NOT a uniform scale.
        let dw = vec![
            vec![2.0 * w[0][0], 2.0 * w[0][1], 2.0 * w[0][2]],
            vec![1.0 * w[1][0], 1.0 * w[1][1], 1.0 * w[1][2]],
            vec![3.0 * w[2][0], 3.0 * w[2][1], 3.0 * w[2][2]],
        ];
        assert_ne!(
            canonical_content_address(&w),
            canonical_content_address(&dw),
            "per-row scaling changes the spectrum ⇒ must NOT share a canonical key"
        );
        // Sanity: it is still scale-invariant under a UNIFORM scale (2·W == 4·W key).
        assert_eq!(
            canonical_content_address(&scale(&w, 2.0)),
            canonical_content_address(&scale(&w, 4.0)),
            "uniform scale family MUST collide"
        );
    }

    /// ADVERSARIAL (iii) all-zero tile: id is stable (same across calls) and
    /// distinct from a nonzero tile. No pivot ⇒ raw-hash branch must be sound.
    #[test]
    fn all_zero_tile_stable_and_distinct() {
        let zero = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        assert_eq!(
            canonical_content_address(&zero),
            canonical_content_address(&zero),
            "zero tile id is stable across calls"
        );
        assert_ne!(
            canonical_content_address(&zero),
            canonical_content_address(&tile_with(1.0)),
            "zero tile id is distinct from a nonzero tile"
        );
        // -0.0 in a zero tile must still equal a +0.0 zero tile.
        let nzero = vec![vec![-0.0, 0.0], vec![0.0, -0.0]];
        assert_eq!(
            canonical_content_address(&zero),
            canonical_content_address(&nzero),
            "zero tile with -0.0 entries == zero tile with +0.0 entries"
        );
    }

    /// ADVERSARIAL (ii, release) NaN-bearing tile: deterministic id across runs.
    /// Release builds fold NaN → `CANONICAL_QUIET_NAN`, so two NaNs hash to the
    /// SAME id (corruption is content-addressed identically across runs).
    #[cfg(not(debug_assertions))]
    #[test]
    fn nan_tile_deterministic_id_across_runs() {
        let nan_tile = vec![vec![f64::NAN, 1.0], vec![2.0, f64::NAN]];
        let id_a = canonical_content_address(&nan_tile);
        let id_b = canonical_content_address(&nan_tile);
        assert_eq!(id_a, id_b, "NaN tile ⇒ deterministic id across runs");
        // A different layout must not accidentally collide with the NaN id.
        assert_ne!(
            id_a,
            canonical_content_address(&vec![vec![1.0, 1.0], vec![2.0, 1.0]]),
        );
    }

    /// ADVERSARIAL (ii, debug) NaN-bearing tile is flagged. In debug builds the
    /// `debug_assert!` must FIRE on NaN input (corruption surfaced, not masked).
    #[cfg(debug_assertions)]
    #[test]
    fn nan_tile_is_flagged_in_debug() {
        let nan_tile = vec![vec![f64::NAN, 1.0], vec![2.0, 3.0]];
        let res = std::panic::catch_unwind(|| {
            canonical_content_address(&nan_tile);
        });
        assert!(
            res.is_err(),
            "debug build must panic (debug_assert fires) on NaN input"
        );
    }
}
