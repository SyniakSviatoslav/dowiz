//! ntt.rs — Number-Theoretic Transform (NTT) + exact integer convolution.
//!
//! The exact-integer counterpart to `crate::fft` (float Cooley–Tukey). Where the
//! FFT runs over `Complex` (f64) and accumulates rounding error, the NTT runs
//! over `Z/pZ` for the NTT-friendly prime `p = 998244353 = 2^23·119 + 1` and is
//! **exact**: every intermediate product stays in `[0, p)` with no float noise.
//!
//! This is what "geometry over algebra" becomes when the geometry is a
//! hypervector: a 1024-bit binary vector is a polynomial over `F_p`, and its
//! circular cross-correlation (shift-invariant similarity) is a polynomial
//! multiplication — O(D²) naïve, O(D log D) via NTT. For D = 1024 the transform
//! length is 2048 = 2^11, well under the prime's 2^23 root-of-unity ceiling, so
//! every butterfly is a single `mulmod` — no multi-prime CRT, no float fallback.
//!
//! # Zero-dep invariant
//! Pure `core` + `alloc`. Deterministic (fixed summation/combination order, no
//! SIMD, no fast-math) — bit-reproducible across native / wasm32.

use alloc::vec;
use alloc::vec::Vec;

/// NTT-friendly prime: `998244353 = 2^23 · 119 + 1`. Supports transform
/// lengths up to `2^23`.
pub const MOD: u64 = 998_244_353;
/// Primitive root of unity modulo [`MOD`] (3 is a primitive root).
pub const ROOT: u64 = 3;

/// Modular exponentiation `base^exp mod m` (square-and-multiply).
pub fn mod_pow(mut base: u64, mut exp: u64, m: u64) -> u64 {
    base %= m;
    let mut result = 1u64;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * base % m;
        }
        base = base * base % m;
        exp >>= 1;
    }
    result
}

/// Modular inverse via Fermat's little theorem (`m` must be prime).
pub fn mod_inv(a: u64, m: u64) -> u64 {
    mod_pow(a, m - 2, m)
}

/// In-place iterative NTT / inverse-NTT over `MOD`. `a.len()` must be a power
/// of two. All elements are reduced into `[0, MOD)` on input.
pub fn ntt(a: &mut [u64], invert: bool) {
    let n = a.len();
    debug_assert!(n.is_power_of_two(), "NTT length must be a power of two");

    // Reduce inputs into the field (callers may pass raw bit counts).
    for x in a.iter_mut() {
        *x %= MOD;
    }

    // Bit-reversal permutation.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            a.swap(i, j);
        }
    }

    // Butterfly stages.
    let mut len = 2usize;
    while len <= n {
        let root = mod_pow(ROOT, (MOD - 1) / len as u64, MOD);
        let wlen = if invert { mod_inv(root, MOD) } else { root };
        let mut i = 0;
        while i < n {
            let mut w = 1u64;
            for j in 0..(len / 2) {
                let u = a[i + j];
                let v = a[i + j + len / 2] * w % MOD;
                a[i + j] = if u + v >= MOD { u + v - MOD } else { u + v };
                a[i + j + len / 2] = if u >= v { u - v } else { u + MOD - v };
                w = w * wlen % MOD;
            }
            i += len;
        }
        len <<= 1;
    }

    if invert {
        let inv_n = mod_inv(n as u64, MOD);
        for x in a.iter_mut() {
            *x = *x * inv_n % MOD;
        }
    }
}

/// Linear convolution of two integer sequences (exact, mod `MOD`).
/// `result[k] = Σ_i a[i]·b[k−i]` for `k in 0..a.len()+b.len()−1`, reduced into
/// `[0, MOD)`. Complexity O(N log N) with `N` the next power of two ≥
/// `a.len()+b.len()−1`.
pub fn convolve(a: &[u64], b: &[u64]) -> Vec<u64> {
    let n = a.len() + b.len() - 1;
    let mut size = 1usize;
    while size < n {
        size <<= 1;
    }
    let mut fa = vec![0u64; size];
    let mut fb = vec![0u64; size];
    fa[..a.len()].copy_from_slice(a);
    fb[..b.len()].copy_from_slice(b);
    ntt(&mut fa, false);
    ntt(&mut fb, false);
    for i in 0..size {
        fa[i] = fa[i] * fb[i] % MOD;
    }
    ntt(&mut fa, true);
    fa.truncate(n);
    fa
}

/// Circular convolution of two equal-length sequences. `a.len() == b.len()`
/// must be a power of two. `result[k] = Σ_i a[i]·b[(k−i) mod n]`, exact mod
/// `MOD`. This is the NTT primitive behind shift-invariant hypervector
/// similarity (a cyclic cross-correlation is a circular convolution against
/// the reversed sequence).
pub fn circular_convolve(a: &[u64], b: &[u64]) -> Vec<u64> {
    debug_assert_eq!(a.len(), b.len());
    debug_assert!(a.len().is_power_of_two());
    let n = a.len();
    let mut fa = a.to_vec();
    let mut fb = b.to_vec();
    ntt(&mut fa, false);
    ntt(&mut fb, false);
    for i in 0..n {
        fa[i] = fa[i] * fb[i] % MOD;
    }
    ntt(&mut fa, true);
    fa
}

/// Map a value in `[0, MOD)` back to the signed range `(−MOD/2, MOD/2]`, so a
/// correlation result (true value in `[−n, n]`, `n ≪ MOD`) round-trips its
/// sign through the NTT.
pub fn centered(v: u64) -> i64 {
    let v = v % MOD;
    if v > MOD / 2 {
        v as i64 - MOD as i64
    } else {
        v as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Naïve O(n²) reference convolution (for parity checks).
    fn naive_convolve(a: &[u64], b: &[u64]) -> Vec<u64> {
        let n = a.len() + b.len() - 1;
        (0..n)
            .map(|k| {
                let mut acc = 0u64;
                for i in 0..=k.min(a.len() - 1) {
                    if k - i < b.len() {
                        acc += a[i] * b[k - i];
                    }
                }
                acc % MOD
            })
            .collect()
    }

    fn naive_circular(a: &[u64], b: &[u64]) -> Vec<u64> {
        let n = a.len();
        (0..n)
            .map(|k| {
                let mut acc = 0u64;
                for i in 0..n {
                    acc += a[i] * b[(k + n - i) % n];
                }
                acc % MOD
            })
            .collect()
    }

    #[test]
    fn mod_pow_matches_reference() {
        assert_eq!(mod_pow(2, 10, MOD), 1024);
        assert_eq!(mod_pow(3, 0, MOD), 1);
        // Fermat: a^(p-1) ≡ 1 (mod p).
        assert_eq!(mod_pow(12345, MOD - 1, MOD), 1);
        assert_eq!(mod_pow(7, MOD - 1, MOD), 1);
    }

    #[test]
    fn mod_inv_rounds_trip() {
        for a in [1u64, 2, 3, 42, 999_999, MOD - 1] {
            assert_eq!(a * mod_inv(a, MOD) % MOD, 1, "inverse of {a}");
        }
    }

    #[test]
    fn ntt_roundtrip() {
        let n = 256;
        let mut a: Vec<u64> = (0..n).map(|i| (i * i + 7) % MOD).collect();
        let orig = a.clone();
        ntt(&mut a, false);
        ntt(&mut a, true);
        assert_eq!(a, orig, "NTT→INTT must be identity");
    }

    #[test]
    fn convolution_matches_naive() {
        let a = [1u64, 2, 3, 4, 5];
        let b = [6u64, 7, 8];
        let fast = convolve(&a, &b);
        let slow = naive_convolve(&a, &b);
        assert_eq!(fast, slow);
    }

    #[test]
    fn circular_convolution_matches_naive() {
        let a = [1u64, 2, 3, 4, 5, 6, 7, 8];
        let b = [9u64, 8, 7, 6, 5, 4, 3, 2];
        let fast = circular_convolve(&a, &b);
        let slow = naive_circular(&a, &b);
        assert_eq!(fast, slow);
    }

    #[test]
    fn circular_convolution_of_shifted_delta_recovers_shift() {
        // Circular-convolving with a shifted delta shifts the signal.
        let n = 8;
        let mut delta = vec![0u64; n];
        delta[3] = 1; // δ(t − 3)
        let signal = [5u64, 1, 4, 2, 6, 3, 7, 9];
        let fast = circular_convolve(&delta, &signal);
        // result[k] = signal[(k − 3) mod n]
        for k in 0..n {
            assert_eq!(fast[k], signal[(k + n - 3) % n], "shift mismatch at {k}");
        }
    }

    #[test]
    fn centered_recovers_signed_correlation() {
        // +1 and −1 encoded as 1 and MOD−1; a perfect alignment sums to n,
        // an anti-alignment to −n.
        let n = 8usize;
        let ones = vec![1u64; n];
        let neg_ones = vec![MOD - 1; n];
        // corr of ones with ones = n everywhere.
        let corr = circular_convolve(&ones, &ones);
        assert_eq!(centered(corr[0]), n as i64);
        // corr of ones with neg_ones = −n everywhere.
        let corr2 = circular_convolve(&ones, &neg_ones);
        assert_eq!(centered(corr2[0]), -(n as i64));
    }
}
