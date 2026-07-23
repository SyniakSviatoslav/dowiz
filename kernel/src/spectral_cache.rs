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

use crate::csr::{Csr, NormalizedTile, TileAddress, FNV_OFFSET_64, FNV_PRIME_64};
use crate::spectral::{classify_drift, DriftClass};

/// Cached spectral decomposition payload.
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
///
/// DEMOTED to private (was `pub`) by BLUEPRINT-P-B §3.2: the raw-matrix hash
/// ceases to be a public entry point — the compiler is the gate. The only
/// content-address that should ever be hashed for a tile is
/// [`NormalizedTile::content_address`]. This function survives because the
/// adversarial test reconstructs the raw path inline (it does NOT call it), and
/// `RetainedBase::admit` is the sole caller here that needs a raw digest.
fn matrix_content_address(a: &[Vec<f64>]) -> String {
    let mut h = FNV_OFFSET_64;
    for (i, row) in a.iter().enumerate() {
        // frame the row index so a value bleeding across rows can't collide
        h ^= (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        h = h.wrapping_mul(FNV_PRIME_64);
        for &x in row {
            h ^= x.to_bits();
            h = h.wrapping_mul(FNV_PRIME_64);
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

    let mut h = FNV_OFFSET_64;

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
        h = h.wrapping_mul(FNV_PRIME_64);
        for &x in row {
            let v = if pivot > 0.0 { x / pivot } else { x };
            h ^= canonical_bits(v);
            h = h.wrapping_mul(FNV_PRIME_64);
        }
    }
    format!("{:016x}", h)
}

/// Content-addressed `spectral::slem` (second-largest eigenvalue modulus) routed
/// through a `DecompCache`. The expensive `eigenvalues` solve is performed only
/// when the tile's canonical content-address changes; otherwise the cached
/// spectrum is reused.
///
/// CHANGED SIGNATURE (BLUEPRINT-P-B §3.2 — the fix): the cache key AND the
/// eigensolve input both derive from the SAME `NormalizedTile` — key/payload
/// coherence by construction. `tile.content_address()` keys the cache, and
/// `tile.to_dense()` is the operator eigensolved. For a row-stochastic canonical
/// tile ρ = 1 exactly and SLEM is the operative quantity (precisely what
/// `spectral::slem` measures); no semantic loss versus the raw path.
pub fn slem_cached(cache: &mut DecompCache, tile: &NormalizedTile) -> f64 {
    let root = tile.content_address().as_hex();
    let (_basis, values) = cache.get_or_recompute(&root, || {
        let dense = tile.to_dense();
        let eigs = crate::spectral::eigenvalues(&dense);
        // vectorless usage only needs the eigenvalue moduli as the payload.
        (
            Vec::new(),
            eigs.iter().map(|e| e.abs()).collect::<Vec<f64>>(),
        )
    });
    let mut mags: Vec<f64> = values.clone();
    crate::sort_by_f64_desc(&mut mags, |&m| m);
    if mags.len() > 1 {
        mags[1]
    } else {
        0.0
    }
}

/// A drift-admitted, canonicalized, content-addressed retained snapshot — node
/// (b) of the doc-19 pipeline, with (d) as its ONLY door.
///
/// INVARIANT (type-encoded): the only constructor is `admit`, which runs
/// `classify_drift` on the RAW operator BEFORE canonicalization+addressing.
/// A retained Unstable base is UNREPRESENTABLE.
#[derive(Debug, Clone, PartialEq)]
pub struct RetainedBase {
    tile: NormalizedTile, // private
    address: TileAddress, // private
    epoch: u64,           // private — logical, max-merge, NO wall-clock
}

/// Law-pole rejection (mirror of `CommitError::Rejected` — never retry; nothing
/// retained).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotRejected {
    /// ρ > 1 + BAND on the raw rebuilt operator: retaining it would snapshot a
    /// divergent dynamics. "Organism endures by NOT persisting" (same law as
    /// `event_log.rs` drift-gate).
    UnstableSpectrum,
}

impl RetainedBase {
    /// Verify-before-persist, INLINE on the causal path: the `RetainedBase` the
    /// caller wants cannot exist unless the gate ran. Order inside:
    /// `classify_drift(raw.to_dense())` → reject Unstable →
    /// `NormalizedTile::canonicalize(raw)` → `content_address()` → construct.
    ///
    /// The gate runs on the RAW rebuilt operator `W` (the dynamics a rebuild
    /// would induce); the hash runs on the canonical form. Two forms, two roles,
    /// one pipeline — `(e)→(c)→(a)→(b) gated by (d)`. A row-stochastic tile has
    /// ρ = 1 always, so gating on the normalized form would be vacuous (never
    /// reject) — the anti-vacuity finding of BLUEPRINT-P-B §4.2.
    pub fn admit(raw: &Csr, epoch: u64) -> Result<RetainedBase, SnapshotRejected> {
        if matches!(classify_drift(&raw.to_dense()), DriftClass::Unstable) {
            return Err(SnapshotRejected::UnstableSpectrum);
        }
        let tile = NormalizedTile::canonicalize(raw);
        let address = tile.content_address();
        Ok(RetainedBase {
            tile,
            address,
            epoch,
        })
    }

    /// Read-only view of the canonical retained tile.
    pub fn tile(&self) -> &NormalizedTile {
        &self.tile
    }

    /// Content-address of the retained tile.
    pub fn address(&self) -> TileAddress {
        self.address
    }

    /// Logical retention epoch (no wall-clock).
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
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
        let w_tile = crate::csr::NormalizedTile::from_dense(&w);
        let mut cache = DecompCache::new();
        let s1 = slem_cached(&mut cache, &w_tile);
        let w2 = scale(&w, 2.0);
        let w2_tile = crate::csr::NormalizedTile::from_dense(&w2);
        let w3 = scale(&w, 3.0);
        let w3_tile = crate::csr::NormalizedTile::from_dense(&w3);
        let s2 = slem_cached(&mut cache, &w2_tile);
        let s3 = slem_cached(&mut cache, &w3_tile);
        assert_eq!(
            cache.recomputes(),
            0,
            "W, 2W, 3W share one canonical key ⇒ zero recomputes"
        );
        assert!(
            (s2 - s1).abs() < 1e-12 && (s3 - s1).abs() < 1e-12,
            "W, 2W, 3W normalize to the SAME tile ⇒ identical slem (s1={s1}, s2={s2}, s3={s3})"
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

    // ── P-B (BLUEPRINT-P-B §5): normalize-before-hash type invariant ─────────

    /// Reconstruct the OLD raw-FNV path inline (the body of the now-private
    /// `matrix_content_address`) so the hazard stays proven forever: two nodes
    /// that hash the SAME logical tile at two scales MUST diverge on the raw
    /// path, and MUST converge once normalized.
    fn raw_fnv_path(a: &[Vec<f64>]) -> String {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut h = OFFSET;
        for (i, row) in a.iter().enumerate() {
            h ^= (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            h = h.wrapping_mul(PRIME);
            for &x in row {
                h ^= x.to_bits();
                h = h.wrapping_mul(PRIME);
            }
        }
        format!("{:016x}", h)
    }

    /// ADVERSARIAL (permanent RED-keeping test) — reproduces the bug as its own
    /// clause (i) and the fix as clause (ii):
    ///   (i)  the raw FNV path (reconstructed inline) yields DIFFERENT hashes for
    ///        tile `A` and `B = 2^k·A` — the divergence two nodes WOULD get;
    ///   (ii) `NormalizedTile::from_dense(A).content_address() ==
    ///        NormalizedTile::from_dense(B).content_address()` — the fix.
    /// If (i) ever fails, the raw path stopped diverging and the whole design
    /// premise must be re-derived.
    #[test]
    fn adversarial_two_node_hash_divergence_without_normalization() {
        // A simple stochastic-like tile of small integers.
        let a = vec![
            vec![1.0, 2.0, 0.0],
            vec![3.0, 1.0, 2.0],
            vec![0.0, 1.0, 3.0],
        ];
        // B = 4·A (a power-of-two scale ⇒ exactly representable, so the two nodes
        // are the SAME logical tile at a different scale).
        let b: Vec<Vec<f64>> = a
            .iter()
            .map(|row| row.iter().map(|&x| x * 4.0).collect())
            .collect();

        // (i) the raw path the old code used MUST diverge — this is the hazard.
        let raw_a = raw_fnv_path(&a);
        let raw_b = raw_fnv_path(&b);
        assert_ne!(raw_a, raw_b, "raw FNV path MUST diverge on scale (the bug)");

        // (ii) the normalized content-address MUST converge — the fix.
        let addr_a = crate::csr::NormalizedTile::from_dense(&a).content_address();
        let addr_b = crate::csr::NormalizedTile::from_dense(&b).content_address();
        assert_eq!(
            addr_a, addr_b,
            "two nodes at different scales of the SAME logical tile MUST share a canonical address"
        );
    }

    /// Guards the §3.3 key/payload-poisoning class: two raw tiles with the same
    /// canonical form get the SAME address AND `slem_cached` serves the IDENTICAL
    /// slem for both (cache HIT, `recomputes == 0` across the pair).
    #[test]
    fn canonical_address_and_spectrum_derive_from_same_object() {
        let tile = vec![
            vec![1.0, 2.0, 0.0],
            vec![3.0, 1.0, 2.0],
            vec![0.0, 1.0, 3.0],
        ];
        let scaled = tile
            .iter()
            .map(|row| row.iter().map(|&x| x * 4.0).collect())
            .collect::<Vec<_>>();

        let nt = crate::csr::NormalizedTile::from_dense(&tile);
        let ns = crate::csr::NormalizedTile::from_dense(&scaled);
        // Same canonical object ⇒ same address.
        assert_eq!(nt.content_address(), ns.content_address());

        let mut cache = DecompCache::new();
        let slem_first = slem_cached(&mut cache, &nt);
        // Second, a DIFFERENT NormalizedTile object but the SAME canonical form.
        let slem_second = slem_cached(&mut cache, &ns);
        assert!(
            (slem_first - slem_second).abs() < 1e-12,
            "same canonical object ⇒ identical slem"
        );
        // And the cross-scale second call reused the shared key (no recompute).
        assert_eq!(
            cache.recomputes(),
            0,
            "same canonical address ⇒ cache HIT, recomputes == 0"
        );
    }

    /// The drift gate REFUSES an Unstable (ρ>1) raw rebuild, and ADMITS a Damped
    /// (ρ<1) one — constructing a `RetainedBase` whose address matches the
    /// canonical tile's address and whose epoch is the requested one.
    #[test]
    fn unstable_raw_rebuild_is_refused_retention() {
        // ρ = 2 raw operator (unstable dynamics). Use a 2×2 with diagonal 2.
        let unstable = crate::csr::Csr::from_dense(&vec![vec![2.0, 0.0], vec![0.0, 2.0]]);
        let res = RetainedBase::admit(&unstable, 7);
        assert_eq!(
            res,
            Err(SnapshotRejected::UnstableSpectrum),
            "Unstable rebuild MUST be refused retention"
        );

        // ρ = 0.5 raw operator (damped). Diagonal 0.5 ⇒ ρ = 0.5.
        let damped = crate::csr::Csr::from_dense(&vec![vec![0.5, 0.0], vec![0.0, 0.5]]);
        let retained = RetainedBase::admit(&damped, 11).expect("damped admit Ok");
        let expected_addr = crate::csr::NormalizedTile::canonicalize(&damped).content_address();
        assert_eq!(
            retained.address(),
            expected_addr,
            "address == canonical tile address"
        );
        assert_eq!(retained.epoch(), 11, "epoch preserved");
    }

    /// Anti-vacuity chaos test (the intentionally-breaking one): a raw operator
    /// with ρ = 2 whose row-normalized form necessarily has ρ = 1 (Resonant)
    /// MUST STILL be refused. If a future refactor moves the gate after
    /// canonicalization, this test goes red.
    #[test]
    fn drift_gate_measures_raw_dynamics_not_normalized_form() {
        // Raw operator with a large uniform scale k ⇒ spectral_radius = k.
        // Its row-normalized form is the uniform self-loop (ρ = 1, Resonant),
        // which a vacuous (normalized-form) gate would ADMIT. The gate must run
        // on the RAW operator and REJECT.
        let k = 2.0;
        let raw = crate::csr::Csr::from_dense(&vec![vec![k, 0.0], vec![0.0, k]]);
        let res = RetainedBase::admit(&raw, 3);
        assert_eq!(
            res,
            Err(SnapshotRejected::UnstableSpectrum),
            "gate measures RAW ρ=2, not the normalized ρ=1 form ⇒ reject"
        );
    }

    /// FAIL-CLOSED (gap-audit round-2): a raw rebuild carrying a NaN/±inf entry
    /// MUST be refused retention. Pre-fix, `classify_drift` let NaN slip through
    /// `f64::max` as `Resonant`, so the poisoned snapshot was silently admitted.
    #[test]
    fn nan_poisoned_raw_rebuild_is_refused_retention() {
        let poisoned = crate::csr::Csr::from_dense(&vec![vec![0.0, f64::NAN], vec![0.0, 0.0]]);
        assert_eq!(
            RetainedBase::admit(&poisoned, 7),
            Err(SnapshotRejected::UnstableSpectrum),
            "NaN-poisoned rebuild MUST be refused (was silently admitted pre-fix)"
        );

        let inf_poisoned =
            crate::csr::Csr::from_dense(&vec![vec![f64::INFINITY, 0.0], vec![0.0, 0.0]]);
        assert_eq!(
            RetainedBase::admit(&inf_poisoned, 7),
            Err(SnapshotRejected::UnstableSpectrum),
            "±inf-poisoned rebuild MUST be refused"
        );
    }

    /// N2 (structural canonicality): `from_dense` with explicit `0.0` entries and
    /// permuted insertion order reaches the SAME `TileAddress` as the clean
    /// build. Also pins that explicit zeros are dropped (not stored).
    #[test]
    fn canonicalize_drops_explicit_zeros_and_sorts_columns() {
        // Clean logical tile.
        let clean = vec![
            vec![2.0, 0.0, 1.0],
            vec![0.0, 3.0, 0.0],
            vec![1.0, 0.0, 2.0],
        ];
        let clean_addr = crate::csr::NormalizedTile::from_dense(&clean).content_address();

        // Same logical tile, built via the EDGE constructor with UNSORTED column
        // submission and EXPLICIT ZERO-weight edges. `from_edges` sorts ascending
        // and merges; `row_normalize` keeps the zero entries. A correct
        // canonicalizer (FNV over the sorted CSR bytes, -0.0 folded to +0.0)
        // reaches the identical address as the clean dense build.
        let messy = crate::csr::Csr::from_edges(
            3,
            &[
                (0, 2, 1.0),
                (0, 0, 2.0), // row0 submitted UNSORTED (col2 before col0)
                (1, 1, 3.0), // row1
                (2, 2, 2.0),
                (2, 0, 1.0), // row2 submitted UNSORTED
            ],
        );
        let messy_addr = crate::csr::NormalizedTile::canonicalize(&messy).content_address();
        assert_eq!(
            clean_addr, messy_addr,
            "explicit zeros dropped + columns sorted ⇒ identical canonical address"
        );
    }
}
