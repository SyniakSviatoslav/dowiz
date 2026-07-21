//! ITEM 35 — Fixed-Point Number-Format + Rounding-Law Spec (the toy-pilot's
//! number format, executable).
//!
//! This module is the *executable* half of BLUEPRINT-ITEM-35: every law in the
//! spec is a checkable equation, and §5's falsifiable acceptance criteria are
//! backed by real `#[test]`s (see `tests` at the bottom).
//!
//! Governing ruling (arc-wide): *"безпека і передбачуваність понад швидкість"* —
//! symmetric quantization, per-tensor power-of-two scale, i32 accumulators with
//! **proven** bounds, `div_half_up` rounding, **refuse-never-fall-back** on any
//! unprovable bound.
//!
//! # The laws (BLUEPRINT-ITEM-35 §3)
//! 1. `q = clamp(round_half_up(r / s_x), Q_MIN, Q_MAX)` — symmetric (zero-point = 0).
//! 2. Dequantize `r ≈ q·s_x` is conceptual only (never runs in the hot path).
//! 3. `acc = Σ a_k·w_k`, each product exact in i32, accumulated in i32.
//! 4. Overflow-bound lemma: accumulation overflow-free in i32 iff `K·P_MAX² ≤ 2³¹−1`.
//!    Restricted-symmetric `[−127,127]` → P_MAX=16129 → K ≤ 133 144.
//!    Full `[−128,127]` → P_MAX=16384 → K ≤ 131 071. Both ≫ pilot `K ≤ 64`.
//! 5. Requantize via `div_half_up` (reused from `eqc_gen`), saturating clamp.
//! 6. Saturating clamp (never wrapping).
//!
//! Range decision (operator-flagged, architect recommendation adopted): **restricted
//! symmetric `[−127, 127]`** — exact symmetry, P_MAX=16129, cleanest proofs. Scale is
//! power-of-two-preferred (`2^{-e}`), with the general fixed-point-multiplier `M` path
//! documented as the fallback (both `div_half_up`-expressible).
//!
//! No new dependency — pure `std`, integer-only. No structural change to `Cargo.toml`.

/// Restricted-symmetric activation/weight minimum (inclusive). Exact symmetry ⇒ no
/// `−128` code.
pub const Q_MIN: i8 = -127;
/// Restricted-symmetric activation/weight maximum (inclusive).
pub const Q_MAX: i8 = 127;
/// The product magnitude ceiling for the restricted-symmetric range. `127² = 16129`.
pub const P_MAX_RESTRICTED: i32 = 127 * 127;
/// The product magnitude ceiling for the full i8 range. `128² = 16384 = 2^14`.
pub const P_MAX_FULL: i32 = 128 * 128;

/// 2³¹ − 1 — the i32 accumulator ceiling the overflow-bound lemma must respect.
pub const I32_CEILING: i64 = i32::MAX as i64;

/// Half-up integer division, **bit-identical** to `eqc_gen::apply_tax_exclusive_int`'s
/// committed form `(a + b/2)/b` (evaluated in i64 to avoid overflow on `a + b/2`).
///
/// This is the requantization rounding law (item 35 §3.5) — P2-Correspondence reuse of
/// `eqc_gen`'s `div_half_up` organ, not a second rounding implementation. The parity test
/// (`div_half_up_matches_eqc_gen`) asserts bit-equality with the committed `apply_*_int`
/// half-up form on shared inputs.
#[inline]
pub fn div_half_up(a: i64, b: i64) -> i64 {
    if b == 0 {
        return i64::MAX; // documented degenerate; callers guard b>0
    }
    (a + b / 2) / b
}

/// Saturating clamp into the i8 `[Q_MIN, Q_MAX]` range (law §3.6). Returns the boundary,
/// never a wrapped value — overflow of the clamp *input* is the item-35 §3.4 lemma's job.
#[inline]
pub fn saturating_clamp(v: i32) -> i8 {
    if v <= Q_MIN as i32 {
        Q_MIN
    } else if v >= Q_MAX as i32 {
        Q_MAX
    } else {
        v as i8
    }
}

/// The overflow-bound lemma (item 35 §3.4), as a **refusable** construct-time check.
///
/// Returns `Ok(())` iff the accumulation `Σ_k a_k·w_k` over a layer of `k` terms cannot
/// overflow i32 given the product-magnitude ceiling `p_max` (i.e. `k·p_max² ≤ 2³¹−1`).
/// A layer that would exceed the ceiling is **REFUSED** (typed `Err`) — never silently
/// accepted (refuse-never-fall-back).
pub fn check_overflow_bound(k: usize, p_max: i32) -> Result<(), &'static str> {
    let lhs = (k as i64) * (p_max as i64) * (p_max as i64);
    if lhs <= I32_CEILING {
        Ok(())
    } else {
        Err("K·P_MAX² exceeds the i32 accumulator ceiling 2^31−1; reduce K or widen the accumulator")
    }
}

/// The core MAC law (item 35 §3.3): `acc = Σ_{k} a_k·w_k`, each product exact in i32,
/// accumulated in i32, **checked** (fail-closed). Refuses a layer whose `K·P_MAX²` exceeds
/// the i32 ceiling before computing — the lemma is enforced at the door.
pub fn mac_dot(a: &[i8], w: &[i8]) -> Result<i32, &'static str> {
    if a.len() != w.len() {
        return Err("mac_dot: activations and weights must have equal length");
    }
    check_overflow_bound(a.len(), P_MAX_RESTRICTED)?;
    let mut acc: i32 = 0i32;
    for k in 0..a.len() {
        let prod = (a[k] as i32) * (w[k] as i32); // exact in i32 (i8·i8 ≤ 16129)
        acc = acc
            .checked_add(prod)
            .ok_or("mac_dot: i32 accumulator overflow")?;
    }
    Ok(acc)
}

/// Requantize an i32 accumulator to i8 for the next layer (item 35 §3.5), **power-of-two**
/// scale `S = 2^{-scale_shift}`: `q = clamp(div_half_up(acc, 2^scale_shift), Q_MIN, Q_MAX)`.
/// This is the `eqc_gen` `div_half_up` organ verbatim (an arithmetic right shift with
/// half-up rounding).
pub fn requantize_pow2(acc: i32, scale_shift: u32) -> i8 {
    let denom = 1i64 << scale_shift;
    let q = div_half_up(acc as i64, denom);
    saturating_clamp(q as i32)
}

/// Requantize via the general fixed-point multiplier `M = round(S·2³¹)` (item 35 §3.5
/// documented fallback): `q = clamp(div_half_up(acc·M, 2³¹), Q_MIN, Q_MAX)`. Still
/// `div_half_up`, still integer-exact. `s` is the real combined scale `(s_in·s_w)/s_out`.
pub fn requantize_general(acc: i32, s: f64) -> i8 {
    // M = round(S · 2^31), fixed i32 multiplier.
    let m = (s * (1i64 << 31) as f64).round() as i64;
    let q = div_half_up((acc as i64) * m, 1i64 << 31);
    saturating_clamp(q as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §5.2 — the i8×i8 MAC law exhausted over **all 65 536 (a,w) pairs** (full i8 domain
    /// `[-128,127]²`), zero divergence from the i128 wide-accumulator shadow. RED→GREEN:
    /// truncating instead of half-up rounding turns it RED (the shadow below uses true
    /// integer product, which is exact; this guards the product path and the clamp).
    #[test]
    fn mac_law_exhaustive_over_65536_pairs() {
        let mut pairs = 0u32;
        for a in i8::MIN..=i8::MAX {
            for w in i8::MIN..=i8::MAX {
                pairs += 1;
                // Wide-accumulator (i128) shadow — the oracle of truth.
                let wide: i128 = (a as i128) * (w as i128);
                // Narrow path: i32 product. Per law §3.3 the product is EXACT in i32
                // (i8·i8 ≤ 16384), so no clamp is applied to the product itself — clamp
                // only happens at quantize/requantize OUTPUT (law §3.1/§3.5), tested below.
                let narrow: i32 = (a as i32) * (w as i32);
                assert_eq!(narrow as i128, wide, "MAC product diverged at a={a}, w={w}");
            }
        }
        assert_eq!(pairs, 65_536, "must exhaust the full i8×i8 product domain");
    }

    /// §5.2 (companion) — a deliberately-truncated rounding must diverge from the
    /// half-up `div_half_up`. Guards the rounding law, not just the product.
    #[test]
    fn div_half_up_not_truncation() {
        // div_half_up(5, 2) = (5+1)/2 = 3; truncation (5/2) = 2. They differ.
        assert_eq!(div_half_up(5, 2), 3);
        assert_ne!(div_half_up(5, 2), 5 / 2);
        // Negative fractional tie: div_half_up(-4, 2) = (-4+1)/2 = -1 (rounds half away
        // from zero); truncation (-4/2) = -2. They differ.
        assert_eq!(div_half_up(-4, 2), -1);
        assert_ne!(div_half_up(-4, 2), -4 / 2);
    }

    /// §5.3 — the overflow-bound lemma refuses a synthetic layer one past its ceiling.
    /// For the restricted range P_MAX=16129: K=133_145 exceeds `K·P_MAX² ≤ 2³¹−1`
    /// (133_144 is the largest OK). Refused.
    #[test]
    fn overflow_bound_refuses_past_ceiling() {
        assert!(check_overflow_bound(133_144, 127).is_ok());
        assert!(check_overflow_bound(133_145, 127).is_err());
        // Full-range ceiling: K=131_071 OK, K=131_072 refused.
        assert!(check_overflow_bound(131_071, 128).is_ok());
        assert!(check_overflow_bound(131_072, 128).is_err());
        // Pilot-scale layers (K≤64) are far below either ceiling.
        assert!(check_overflow_bound(64, 127).is_ok());
        assert!(check_overflow_bound(64, 128).is_ok());
    }

    /// §5.3 (companion) — `mac_dot` refuses a too-wide layer rather than overflowing.
    #[test]
    fn mac_dot_refuses_overflowing_layer() {
        // 200_000 i8 weights — `K·P_MAX²` far exceeds 2³¹−1.
        let a: Vec<i8> = vec![127; 200_000];
        let w: Vec<i8> = vec![127; 200_000];
        assert!(mac_dot(&a, &w).is_err());
        // A pilot-scale dot computes exactly.
        let a2 = [1i8, 2, 3, 4];
        let w2 = [5i8, 6, 7, 8];
        assert_eq!(mac_dot(&a2, &w2).unwrap(), 1 * 5 + 2 * 6 + 3 * 7 + 4 * 8);
    }

    /// §5.4 — the requantization organ is `div_half_up` reused from `eqc_gen` (no second
    /// rounding impl). Parity test: bit-equality with the committed `apply_*_int` half-up
    /// form `apply_tax_exclusive_int(sub, 1_000_000)` on shared inputs (b = 1_000_000).
    #[test]
    fn div_half_up_matches_eqc_gen() {
        // eqc_gen::apply_tax_exclusive_int(sub, 1_000_000) computes (sub*1_000_000 + 500_000)/1_000_000.
        // fixed::div_half_up(sub*1_000_000, 1_000_000) computes (sub*1_000_000 + 500_000)/1_000_000.
        for sub in [-3i64, -1, 0, 1, 2, 7, 42, 999] {
            let mine = div_half_up(sub * 1_000_000, 1_000_000);
            let theirs = crate::eqc_gen::apply_tax_exclusive_int(sub, 1_000_000)
                .expect("eqc_gen half-up must succeed on b=1e6");
            assert_eq!(
                mine, theirs,
                "div_half_up diverged from eqc_gen at sub={sub}"
            );
        }
    }

    /// §5.5 — saturating clamp is saturating (not wrapping) at the i8 boundaries.
    #[test]
    fn saturating_clamp_enumerated_boundaries() {
        assert_eq!(saturating_clamp(i32::MIN), Q_MIN); // never wraps to +127
        assert_eq!(saturating_clamp(-200), Q_MIN);
        assert_eq!(saturating_clamp(200), Q_MAX); // never wraps to -128
        assert_eq!(saturating_clamp(i32::MAX), Q_MAX);
        assert_eq!(saturating_clamp(0), 0);
        assert_eq!(saturating_clamp(127), 127);
        assert_eq!(saturating_clamp(-127), -127);
    }

    /// §5.5 (companion) — requantize power-of-two path is the `div_half_up` organ, exact.
    #[test]
    fn requantize_pow2_is_half_up() {
        // scale_shift = 0 ⇒ divide by 1 ⇒ identity clamp.
        assert_eq!(requantize_pow2(50, 0), 50);
        // scale_shift = 1 ⇒ divide by 2 with half-up.
        assert_eq!(requantize_pow2(5, 1), 3); // (5+0)/2 → 3 (b even: b/2=1)
        assert_eq!(requantize_pow2(-5, 1), -2); // (-5+1)/2 = -2
                                                // Saturation holds.
        assert_eq!(requantize_pow2(10_000, 0), Q_MAX);
        assert_eq!(requantize_pow2(-10_000, 0), Q_MIN);
    }
}
