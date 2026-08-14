//! fft.rs — radix-2 Cooley–Tukey FFT / IFFT. O(N log N), zero-dep.
//!
//! Item #7 of screenshot-batch-2 ("Fast Fourier Transform — from time to
//! frequency in O(N log N)"). The kernel already had `resonance::goertzel`
//! (single-frequency DFT) but no full FFT — this closes that gap.
//!
//! # Why it fits the rewrite law
//! A twiddle factor is exactly a rotation on the unit circle:
//! `W_N^k = e^{-2πi k/N} = cos(θ) − i·sin(θ)`. So "geometry over algebra"
//! is literal here: the FFT is a cascade of phase rotations, and the twiddle
//! table is a precomputed LUT (n(0) access — the butterfly reads `twiddle[j]`
//! instead of recomputing a transcendental per stage).
//!
//! # Determinism
//! Fixed summation/combination order, no SIMD, no fast-math — bit-reproducible
//! across native / wasm32. Float is fine here (signal math, never money).
//!
//! # Zero-dep
//! Reuses `crate::spectral::Complex` (the kernel's hand-rolled complex, no
//! `num-complex`).

use crate::complex::Complex;
use alloc::vec::Vec;

/// Bit-reversal permutation of `x` in place (length must be a power of two).
fn bit_reverse_in_place(x: &mut [Complex]) {
    let n = x.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            x.swap(i, j);
        }
    }
}

/// Radix-2 Cooley–Tukey forward FFT. Returns `None` if `len` is not a power
/// of two (or zero). Twiddle factors are precomputed once into a LUT.
pub fn fft(x: &[Complex]) -> Option<Vec<Complex>> {
    let n = x.len();
    if n == 0 || !n.is_power_of_two() {
        return None;
    }
    let mut a = x.to_vec();
    bit_reverse_in_place(&mut a);

    // Twiddle LUT: W_n^k for k in [0, n/2). Precomputed once (n(0) access).
    let mut twiddle = Vec::with_capacity(n / 2);
    for k in 0..(n / 2) {
        let theta = -2.0 * core::f64::consts::PI * (k as f64) / (n as f64);
        twiddle.push(Complex::new(crate::math::cos(theta), crate::math::sin(theta)));
    }

    let mut len = 2usize;
    while len <= n {
        let half = len / 2;
        let step = n / len; // twiddle index stride
        for start in (0..n).step_by(len) {
            for j in 0..half {
                let w = twiddle[j * step];
                let u = a[start + j];
                let v = a[start + j + half].mul(w);
                a[start + j] = u.add(v);
                a[start + j + half] = u.sub(v);
            }
        }
        len <<= 1;
    }
    Some(a)
}

/// Radix-2 inverse FFT (unnormalized — callers scale by `1/N` if they need the
/// true inverse). `ifft(fft(x)) == N·x`. Returns `None` on non-power-of-two.
pub fn ifft(x: &[Complex]) -> Option<Vec<Complex>> {
    let n = x.len();
    if n == 0 || !n.is_power_of_two() {
        return None;
    }
    // IFFT = conjugate, forward FFT, conjugate, scale by 1/N.
    let conj: Vec<Complex> = x.iter().map(|c| c.conj()).collect();
    let mut y = fft(&conj)?;
    for c in y.iter_mut() {
        let d = c.conj();
        *c = Complex::new(d.re / n as f64, d.im / n as f64);
    }
    Some(y)
}

/// Convenience: forward FFT of a real signal, returning complex spectrum.
/// `len` must be a power of two.
pub fn fft_real(x: &[f64]) -> Option<Vec<Complex>> {
    if x.len() == 0 || !x.len().is_power_of_two() {
        return None;
    }
    let c: Vec<Complex> = x.iter().map(|&v| Complex::new(v, 0.0)).collect();
    fft(&c)
}

/// Round-trip energy check: Parseval's theorem — the total energy (sum of
/// |x|²) equals `1/N` times the sum of |X|². Useful as a falsifiable probe.
pub fn parseval_error(x: &[Complex]) -> Option<f64> {
    let n = x.len();
    let spectrum = fft(x)?;
    let time_energy: f64 = x.iter().map(|c| c.re * c.re + c.im * c.im).sum();
    let freq_energy: f64 = spectrum
        .iter()
        .map(|c| c.re * c.re + c.im * c.im)
        .sum();
    Some((freq_energy / n as f64 - time_energy).abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_power_of_two() {
        assert_eq!(fft(&[Complex::new(1.0, 0.0); 3]), None);
        assert_eq!(fft(&[]), None);
        assert_eq!(ifft(&[]), None);
    }

    #[test]
    fn dc_impulse() {
        // Constant signal → single DC bin at k=0, zero elsewhere.
        let n = 8;
        let x: Vec<Complex> = (0..n).map(|_| Complex::new(1.0, 0.0)).collect();
        let y = fft(&x).unwrap();
        assert!((y[0].re - n as f64).abs() < 1e-9, "DC bin = N, got {}", y[0].re);
        for k in 1..n {
            assert!(y[k].abs() < 1e-9, "bin {k} should be 0, got {}", y[k].abs());
        }
    }

    #[test]
    fn single_sinusoid_bin() {
        // x[n] = cos(2π·k0·n/N) → energy concentrated in bins ±k0.
        let n = 16;
        let k0 = 2;
        let x: Vec<Complex> = (0..n)
            .map(|i| {
                let v = (2.0 * core::f64::consts::PI * k0 as f64 * i as f64 / n as f64).cos();
                Complex::new(v, 0.0)
            })
            .collect();
        let y = fft(&x).unwrap();
        let mut peak = 0.0f64;
        for c in &y {
            peak = peak.max(c.abs());
        }
        assert!((y[k0].abs() - peak).abs() < 1e-9 || (y[n - k0].abs() - peak).abs() < 1e-9,
            "peak must be at bin ±k0");
    }

    #[test]
    fn roundtrip_ifft_scales_back() {
        let n = 16;
        let x: Vec<Complex> = (0..n)
            .map(|i| Complex::new((i as f64).sin(), (i as f64 * 0.5).cos()))
            .collect();
        let y = fft(&x).unwrap();
        let back = ifft(&y).unwrap();
        for i in 0..n {
            assert!((back[i].re - x[i].re).abs() < 1e-9, "re[{i}] mismatch");
            assert!((back[i].im - x[i].im).abs() < 1e-9, "im[{i}] mismatch");
        }
    }

    #[test]
    fn parseval_holds() {
        let n = 32;
        let x: Vec<Complex> = (0..n)
            .map(|i| Complex::new((i as f64 * 0.3).cos(), (i as f64 * 0.7).sin()))
            .collect();
        let err = parseval_error(&x).unwrap();
        assert!(err < 1e-8, "Parseval error too large: {err}");
    }

    #[test]
    fn fft_real_matches_complex() {
        let n = 8;
        let real = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let via_real = fft_real(&real).unwrap();
        let complex: Vec<Complex> = real.iter().map(|&v| Complex::new(v, 0.0)).collect();
        let via_complex = fft(&complex).unwrap();
        for i in 0..n {
            assert!((via_real[i].re - via_complex[i].re).abs() < 1e-9);
            assert!((via_real[i].im - via_complex[i].im).abs() < 1e-9);
        }
    }
}
