//! constants.rs — Phase A branchless constants: single authority for every
//! magic number, threshold, and mathematical constant in the kernel.
//!
//! Every number that was previously a bare literal in a method body is now a
//! named `pub const` here. This serves two rewrite-law goals:
//!
//! 1. **n(0) access** — a constant is always available, never looked up.
//! 2. **Branchless** — named constants feed directly into LUTs without
//!    runtime branching or pointer indirection.
//!
//! # Sources (from mining report, 111 patterns):
//!
//! - `ktg2/fractal.rs:8` — ZERO = -64 (absolute zero anchor)
//! - `spectral.rs:724` — DRIFT_BAND = 1e-6 (single-authority tolerance)
//! - `mat.rs:31` — MAT_BLOCK_SIZE = 128 (cache-aware tile, L2-tuned)
//! - `spectral_cache.rs:121-140` — CANONICAL_QUIET_NAN = 0x7ff8_0000_0000_0000
//! - `money.rs:22` — MONEY_SCALE_MICRO = 1_000_000
//! - `pixel_snapshot.rs:39` — DOT_OFFSET braille mapping
//! - `householder.rs:78-125` — Matrix32x32 N=32 fixed size
//! - `simd.rs:109` — SIMD lane width = 4
//! - `token_bucket.rs` — refill rate / burst constants
//! - `csr.rs` — FNV_PRIME_64 = 0x00000100000001B3

// ─── Fractal / geometric constants ──────────────────────────────────────

/// Absolute zero anchor for the fractal word system.
/// Every Bit position is measured relative to this value.
/// Inversion mirrors through ZERO: `pos → 2 * ZERO - pos`.
pub const ZERO: i32 = -64;

/// Half the word width, used for power-of-two position stepping.
pub const HALF_WIDTH: i32 = 12;

/// Full 24-bit word width for Fractal Manchester Architecture.
pub const WORD_WIDTH: i32 = 24;

/// FMA optical carrier frequency (fractal power position base).
pub const CARRIER_FREQ: f64 = 1.0;

/// Fractal depth limit — recursive power-position stepping.
pub const MAX_FRACTAL_DEPTH: u32 = 8;

// ─── Trigonometric constants ────────────────────────────────────────────

/// Re-export PI for zero-dep access. Use `core::f64::consts::PI` in std
/// code; this alias is for `no_std` paths and test consistency.
/// NOTE: PI/TAU are also re-exported in lib.rs; this module's versions
/// are qualified as `constants::PI` to avoid ambiguity.
pub use core::f64::consts::PI;

/// Full circle in radians.
pub use core::f64::consts::TAU;

/// Half circle.
pub const HALF_PI: f64 = PI / 2.0;

/// Golden ratio φ = (1 + √5) / 2 ≈ 1.6180339887.
/// Used by golden-angle embeddings in crystal lattice (academia_p2p.rs:5108).
pub const PHI: f64 = 1.6180339887498948482;

/// Golden angle = 2π / φ² ≈ 2.3999632 rad.
/// Used for sin(i * GOLDEN_ANGLE) in MoE expert embeddings.
pub const GOLDEN_ANGLE: f64 = core::f64::consts::TAU / (PHI * PHI);

// ─── Numerical tolerances ───────────────────────────────────────────────

/// Single-authority tolerance around ρ=1 for drift classification.
/// Replaces function-local `BAND` in `classify_drift` (spectral.rs:724).
pub const DRIFT_BAND: f64 = 1e-6;

/// Minimal denominator to avoid division by zero.
pub const EPSILON: f64 = 1e-12;

/// Minimal non-zero float value used as a safe division floor.
pub const F64_MIN_POSITIVE: f64 = f64::MIN_POSITIVE;

/// Default convergence threshold for iterative methods.
pub const CONVERGENCE_TOL: f64 = 1e-12;

// ─── Architecture / cache constants ─────────────────────────────────────

/// Cache-aware matrix tile size tuned to EPYC-Milan L2 cache.
/// Three 128×128 f64 blocks = 384 KiB < 512 KiB L2.
/// Documented upgrade trigger: `cpuid` L2 size check (mat.rs:24-31).
pub const MAT_BLOCK_SIZE: usize = 128;

/// Fixed 32×32 stack matrix dimension for Householder eigensolver.
/// `[f64; 1024]` fits in L1D; Copy, no heap (householder.rs:78-125).
pub const FIXED_MATRIX_DIM: usize = 32;
pub const FIXED_MATRIX_ELEMS: usize = FIXED_MATRIX_DIM * FIXED_MATRIX_DIM; // 1024

/// SIMD lane width: 4 independent rows per register (simd.rs:109).
pub const SIMD_LANE_WIDTH: usize = 4;

// ─── Canonical bit representations ──────────────────────────────────────

/// Canonical quiet NaN bits: 0x7ff8_0000_0000_0000.
/// Used by canonical_bits() to fold any NaN to a single deterministic
/// representation for content-addressing (spectral_cache.rs:121-140).
pub const CANONICAL_QUIET_NAN: u64 = 0x7ff8_0000_0000_0000;

/// FNV-1a 64-bit prime (csr.rs, spectral_cache.rs, hypergraph.rs).
pub const FNV_PRIME_64: u64 = 0x00000100000001B3;

/// FNV-1a 64-bit offset basis.
pub const FNV_OFFSET_64: u64 = 0xcbf29ce484222325;

/// PCG-style LCG multiplier (simd.rs:407, csr.rs:1402, spectral.rs:311).
pub const LCG_MULTIPLIER: u64 = 6364136223846793005;

/// PCG-style LCG increment.
pub const LCG_INCREMENT: u64 = 1442695040888963407;

// ─── Money / financial constants ────────────────────────────────────────

/// Micro-scale for integer money arithmetic (money.rs:22).
/// All amounts are stored as `i64` minor units; this scale is the
/// canonical authority for conversion between display and storage.
pub const MONEY_SCALE_MICRO: i128 = 1_000_000;

// ─── Token / rate-limit constants ───────────────────────────────────────

/// Default token bucket capacity (token_bucket.rs).
pub const DEFAULT_TOKEN_CAPACITY: u64 = 1000;

/// Default token refill rate per second.
pub const DEFAULT_TOKEN_REFILL_RATE: u64 = 100;

// ─── PID controller defaults ────────────────────────────────────────────

/// Minimum allowed integral term to avoid division by zero (PID anti-windup).
/// Single authority — `pid.rs` reads this const instead of its own copy.
pub const KI_EPSILON: f64 = 0.001;

/// Default PID proportional gain.
pub const PID_DEFAULT_KP: f64 = 1.0;

/// Default PID integral gain.
pub const PID_DEFAULT_KI: f64 = 0.1;

/// Default PID derivative gain.
pub const PID_DEFAULT_KD: f64 = 0.01;

// ─── Hypervector / VSA constants ────────────────────────────────────────

/// Default hypervector dimension in bits (hypervector.rs).
pub const HYPERVECTOR_DIM: usize = 1024;

/// Number of u64 words needed to pack 1024 bits.
pub const HYPERVECTOR_WORDS: usize = HYPERVECTOR_DIM / 64; // 16

// ─── Crystal lattice constants ──────────────────────────────────────────

/// Number of cells in the crystal lattice hash-space (academia.rs:41-57).
pub const CRYSTAL_CELLS: usize = 65536;

/// Maximum neighbours to scan per query in crystal lattice search.
pub const CRYSTAL_MAX_NEIGHBOURS: usize = 27;

// ─── Braille / rendering constants ──────────────────────────────────────

/// Braille Unicode block base (U+2800). pixel_snapshot.rs:39.
pub const BRAILLE_BASE: u32 = 0x2800;

/// Braille dot-to-offset mapping (1-indexed dots 1..8).
/// DOT_OFFSET[dot] = Unicode bit offset for that dot.
pub const DOT_OFFSET: [u32; 8] = [0x01, 0x02, 0x04, 0x40, 0x08, 0x10, 0x20, 0x80];

/// Sparkline glyphs: 8-level intensity from U+2581 (▁) to U+2588 (█).
pub const SPARKLINE_BASE: u32 = 0x2581;

// ─── Hash / crypto constants ────────────────────────────────────────────

/// SHA3-256 output length in bytes.
pub const SHA3_256_LEN: usize = 32;

/// Keccak-f[1600] state size in bytes.
pub const KECCAK_STATE_BYTES: usize = 200;

/// Keccak-f[1600] rate for SHA3-256 (1088 bits = 136 bytes).
pub const KECCAK_RATE: usize = 136;

// ─── Breaker / safety constants ─────────────────────────────────────────

/// Default minimum cooldown (microseconds).
pub const BREAKER_MIN_COOLDOWN_US: u64 = 1_000_000; // 1 sec

/// Default maximum cooldown (microseconds) after repeated trips.
pub const BREAKER_MAX_COOLDOWN_US: u64 = 120_000_000; // 2 min

/// Default minimum tripped spells before the breaker escalates cooldown.
pub const BREAKER_MIN_TRIPS: u32 = 3;

// ─── FSM / order-machine constants ──────────────────────────────────────

/// Number of lifecycle states in the order FSM (order_machine.rs).
pub const LIFECYCLE_STATE_COUNT: usize = 12;

/// Proven spectral radius for a nilpotent DAG adjacency.
/// Perron-Frobenius: ρ = 0 for a nilpotent transition matrix.
/// (order_machine.rs:375-393)
pub const NILPOTENT_SPECTRAL_RADIUS: f64 = 0.0;

// ─── Sanitization ───────────────────────────────────────────────────────

/// Sentinel value returned by `sanitize_f64` for NaN/Inf inputs.
pub const SANITIZED_DEFAULT: f64 = 0.0;

/// Sentinel for `sanitize_f32`.
pub const SANITIZED_F32_DEFAULT: f32 = 0.0;

// ─── Branchless combined flag constants ─────────────────────────────────

/// No flags set.
pub const FLAG_NONE: u8 = 0;

/// Overflow flag (bit 0).
pub const FLAG_OVERFLOW: u8 = 1 << 0;

/// Underflow flag (bit 1).
pub const FLAG_UNDERFLOW: u8 = 1 << 1;

/// Division-by-zero flag (bit 2).
pub const FLAG_DIV_ZERO: u8 = 1 << 2;

/// Invalid-operation flag (bit 3).
pub const FLAG_INVALID: u8 = 1 << 3;

/// Inexact-result flag (bit 4).
pub const FLAG_INEXACT: u8 = 1 << 4;

// ─── Test helpers ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_anchor_is_negative_64() {
        assert_eq!(ZERO, -64);
    }

    #[test]
    fn word_width_is_24() {
        assert_eq!(WORD_WIDTH, 24);
    }

    #[test]
    fn drift_band_is_positive() {
        assert!(DRIFT_BAND > 0.0);
    }

    #[test]
    fn mat_block_size_is_power_of_two() {
        assert!(MAT_BLOCK_SIZE.is_power_of_two());
    }

    #[test]
    fn canonical_nan_has_quiet_bit_set() {
        // Quiet NaN has bit 51 set (0x0008_...).
        assert!(CANONICAL_QUIET_NAN & (1u64 << 51) != 0);
    }

    #[test]
    fn fnv_prime_is_known() {
        assert_eq!(FNV_PRIME_64, 0x00000100000001B3);
    }

    #[test]
    fn braille_dot_offset_length() {
        assert_eq!(DOT_OFFSET.len(), 8);
    }
}