//! zlib CRC-32 on the CPU's own instructions, with the tables as the fallback
//! (R-EMBED-STORE §12 P12, W-SPEED-A, 2026-10-29).
//!
//! The format's crc is slicing-by-8 in software (`sw.rs`, moved here unchanged from bebop-image),
//! and verify spends most of its crc time there: every load checks the crc of every object. Two
//! families of hardware compute THE SAME function, and only those two are used:
//!
//!   * AArch64 `crc32b/crc32x` (`__crc32b`, `__crc32d`, feature `crc`): the zlib polynomial
//!     0x04C11DB7 (reflected 0xEDB88320). Their sibling `crc32c*` is CASTAGNOLI -- a different
//!     polynomial and a different number -- and is never called.
//!   * x86_64 carry-less multiply (`pclmulqdq` + SSE4.1): 128-bit folding of the zlib polynomial,
//!     then a Barrett reduction (Intel, "Fast CRC Computation for Generic Polynomials Using
//!     PCLMULQDQ", 2009; the constants are zlib's / crc32fast's). SSE4.2's `crc32` instruction is
//!     CRC-32C, the wrong polynomial, and is deliberately not used.
//!
//! Everything works on the RUNNING REGISTER, exactly like `step_bytes` / `step_cell`: no pre- or
//! post-inversion inside, so a caller's `!f(!0, ..)` is unchanged and a hardware step and a
//! software step can follow one another on one register.
//!
//! DISPATCH. A path is taken when its feature is on at COMPILE time (`cfg(target_feature)`), else,
//! with the `std` feature, when std detects it at RUN time (one cached atomic load per call).
//! Otherwise -- and always on wasm32, where neither arch module is compiled and the three entry
//! points are `inline(always)`, so the reader module `bytes.baseline` ratchets is the code it was
//! (measured: 46057 bytes with and without this crate wired in) -- the tables run.
//!
//! `unsafe` is denied crate-wide and allowed only in `arm.rs` and `x86.rs`, where each intrinsic
//! block sits under a SAFETY comment behind its feature check. Proof of sameness: `tests.rs`
//! compares every path against the tables on every length 0..=4096 over named patterns.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![deny(unsafe_code)]

mod sw;
pub use sw::{step_bytes, step_cell};

#[cfg(target_arch = "aarch64")]
#[allow(unsafe_code)]
mod arm;
#[cfg(target_arch = "x86_64")]
#[allow(unsafe_code)]
mod x86;

/// The running register over raw bytes.
#[cfg_attr(any(target_arch = "aarch64", target_arch = "x86_64"), inline)]
#[cfg_attr(not(any(target_arch = "aarch64", target_arch = "x86_64")), inline(always))]
pub fn bytes(c: u32, b: &[u8]) -> u32 {
    #[cfg(target_arch = "aarch64")]
    {
        if let Some(r) = arm::bytes(c, b) {
            return r;
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if let Some(r) = x86::bytes(c, b) {
            return r;
        }
    }
    step_bytes(c, b)
}

/// The running register over a slice of cells, each its eight little-endian bytes.
#[cfg_attr(any(target_arch = "aarch64", target_arch = "x86_64"), inline)]
#[cfg_attr(not(any(target_arch = "aarch64", target_arch = "x86_64")), inline(always))]
pub fn cells(c: u32, v: &[i64]) -> u32 {
    #[cfg(target_arch = "aarch64")]
    {
        if let Some(r) = arm::cells(c, v) {
            return r;
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if let Some(r) = x86::cells(c, v) {
            return r;
        }
    }
    v.iter().fold(c, |h, &x| step_cell(h, x))
}

/// The running register over `n` cells produced by `cell(0) .. cell(n - 1)` -- the form a reader
/// over borrowed bytes (bebop-image's `Cells`) needs: `cells_by(c, n, |i| src.cell_at(off + i))`.
#[cfg_attr(any(target_arch = "aarch64", target_arch = "x86_64"), inline)]
#[cfg_attr(not(any(target_arch = "aarch64", target_arch = "x86_64")), inline(always))]
pub fn cells_by<F: FnMut(usize) -> i64>(c: u32, n: usize, mut cell: F) -> u32 {
    #[cfg(target_arch = "aarch64")]
    {
        if arm::active() {
            return arm::cells_by(c, n, cell);
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if x86::active() && n * 8 >= x86::MIN {
            // Gather through the closure into a stack buffer, fold each buffer.
            let mut h = c;
            let mut buf = [0u8; 8 * x86::GATHER];
            let mut i = 0;
            while i < n {
                let k = (n - i).min(x86::GATHER);
                for (j, slot) in buf.chunks_exact_mut(8).take(k).enumerate() {
                    slot.copy_from_slice(&cell(i + j).to_le_bytes());
                }
                h = bytes(h, buf.get(..8 * k).unwrap_or(&[]));
                i += k;
            }
            return h;
        }
    }
    (0..n).fold(c, |h, i| step_cell(h, cell(i)))
}

/// Which path `bytes` / `cells` take on this CPU: for bench lines and test messages.
pub fn path() -> &'static str {
    #[cfg(target_arch = "aarch64")]
    {
        if arm::active() {
            return "aarch64-crc32x";
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if x86::active() {
            return "x86_64-pclmul";
        }
    }
    "sw-slice8"
}

#[cfg(test)]
mod tests;
