//! AArch64 `crc32b` / `crc32x`: the zlib polynomial in one instruction per byte / per cell.
//! (Never `crc32c*`: that is Castagnoli, another polynomial.) The crate's only AArch64 `unsafe`.

use core::arch::aarch64::{__crc32b, __crc32d};

/// The `crc` feature: on at compile time, else (with `std`, and always under test) detected at
/// run time.
#[inline(always)]
pub(crate) fn active() -> bool {
    #[cfg(target_feature = "crc")]
    {
        true
    }
    #[cfg(all(not(target_feature = "crc"), any(feature = "std", test)))]
    {
        std::arch::is_aarch64_feature_detected!("crc")
    }
    #[cfg(all(not(target_feature = "crc"), not(any(feature = "std", test))))]
    {
        false
    }
}

#[target_feature(enable = "crc")]
unsafe fn bytes_hw(mut c: u32, b: &[u8]) -> u32 {
    let mut it = b.chunks_exact(8);
    for w in &mut it {
        c = __crc32d(c, u64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]));
    }
    for &x in it.remainder() {
        c = __crc32b(c, x);
    }
    c
}

/// `crc32x` takes its 64-bit operand least-significant byte first: one cell, one instruction.
#[target_feature(enable = "crc")]
unsafe fn cells_hw(mut c: u32, v: &[i64]) -> u32 {
    for &x in v {
        c = __crc32d(c, x as u64);
    }
    c
}

#[target_feature(enable = "crc")]
unsafe fn cells_by_hw<F: FnMut(usize) -> i64>(mut c: u32, n: usize, mut cell: F) -> u32 {
    for i in 0..n {
        c = __crc32d(c, cell(i) as u64);
    }
    c
}

/// `None` when this CPU has no `crc`.
#[inline]
pub(crate) fn bytes(c: u32, b: &[u8]) -> Option<u32> {
    if !active() {
        return None;
    }
    // SAFETY: `active()` proved the `crc` feature, the only precondition of `bytes_hw`; it reads
    // `b` through safe slice iteration.
    Some(unsafe { bytes_hw(c, b) })
}

/// `None` when this CPU has no `crc`.
#[inline]
pub(crate) fn cells(c: u32, v: &[i64]) -> Option<u32> {
    if !active() {
        return None;
    }
    // SAFETY: as `bytes`: the feature is proved; `v` is read by safe iteration.
    Some(unsafe { cells_hw(c, v) })
}

/// The caller checked `active()`; without the feature this falls back to the tables.
#[inline]
pub(crate) fn cells_by<F: FnMut(usize) -> i64>(c: u32, n: usize, mut cell: F) -> u32 {
    if !active() {
        return (0..n).fold(c, |h, i| crate::step_cell(h, cell(i)));
    }
    // SAFETY: as `bytes`: the feature is proved; the closure is ordinary safe code.
    unsafe { cells_by_hw(c, n, cell) }
}
