//! complex.rs — minimal complex number (avoids a `num-complex` dependency —
//! kernel is zero-dep). Extracted from `spectral.rs` so the geometry modules
//! (fft, spherical, modular) can use it in the `no_std` core. All operations
//! route through `crate::math` (correctly-rounded `sqrt`/`hypot`/`atan2`).

/// Minimal complex number (avoids a `num-complex` dependency — kernel is zero-dep).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    /// Modulus |z|.
    pub fn abs(self) -> f64 {
        crate::math::hypot(self.re, self.im)
    }
    /// Argument arg(z) ∈ (−π, π].
    pub fn arg(self) -> f64 {
        crate::math::atan2(self.im, self.re)
    }
    /// Complex conjugate.
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }
    pub fn add(self, o: Complex) -> Complex {
        Complex::new(self.re + o.re, self.im + o.im)
    }
    pub fn sub(self, o: Complex) -> Complex {
        Complex::new(self.re - o.re, self.im - o.im)
    }
    pub fn mul(self, o: Complex) -> Complex {
        Complex::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
    pub fn div(self, o: Complex) -> Complex {
        let d = o.re * o.re + o.im * o.im;
        Complex::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
    /// Complex square root (principal branch).
    pub fn sqrt(self) -> Complex {
        let r = self.abs();
        let re = crate::math::sqrt((r + self.re) / 2.0);
        let im = crate::math::sqrt((r - self.re) / 2.0);
        // choose sign of im to match arg (so sqrt matches the half-angle)
        if self.im < 0.0 {
            Complex::new(re, -im)
        } else {
            Complex::new(re, im)
        }
    }
    /// Integer power (repeated complex multiply).
    pub fn powu(self, k: u32) -> Complex {
        let mut r = Complex::new(1.0, 0.0);
        for _ in 0..k {
            r = r.mul(self);
        }
        r
    }
    /// True when both parts are exactly zero.
    pub fn is_zero(self) -> bool {
        self.re == 0.0 && self.im == 0.0
    }
}
