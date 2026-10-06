//! The reads `zc.rs` wants fast over any image (split out of `zc.rs`, W-ZC): a crc over a
//! run of cells, and the bytes of a run in place when the image is a byte slice.

use crate::verify::BadCrc;
use crate::{crc32_in, obj_len_in, Cells, Store, View};

/// `Cells`, plus the two reads this module wants fast: a crc over a run of cells, and
/// the bytes of a run IN PLACE where the image is a byte slice.
pub trait Image: Cells {
    fn crc_cells(&self, off: usize, n: usize) -> u32 {
        crc32_in(self, off, n)
    }
    /// `len` bytes from byte `at` of the image, borrowed, if this image is bytes.
    fn bytes_at(&self, _at: usize, _len: usize) -> Option<&[u8]> {
        None
    }
}

impl Image for View<'_> {
    fn crc_cells(&self, off: usize, n: usize) -> u32 {
        match off.checked_add(n).and_then(|e| e.checked_mul(8)).and_then(|e| self.bytes.get(off * 8..e)) {
            // Slice-by-8, a whole cell per step, exactly as `crc32_cells` does for a
            // `Store`: `crate::crc32` is one byte per step, measured 1.3x slower than
            // `Catalog::load`'s whole check at 165 x 2.5 KB (zc-bench run 1).
            Some(b) => !b.chunks_exact(8).fold(0xFFFF_FFFF, |h, w| {
                crate::crc::step_cell(h, i64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]))
            }),
            None => crc32_in(self, off, n),
        }
    }
    fn bytes_at(&self, at: usize, len: usize) -> Option<&[u8]> {
        self.bytes.get(at..at.checked_add(len)?)
    }
}

impl Image for Store {
    fn crc_cells(&self, off: usize, n: usize) -> u32 {
        if off.checked_add(n).is_some_and(|e| e <= self.cells.len()) {
            crate::crc32_cells(&self.cells, off, n)
        } else {
            crc32_in(self, off, n)
        }
    }
}

impl<T: Cells + ?Sized> Cells for &T {
    fn cell_at(&self, i: usize) -> i64 {
        (**self).cell_at(i)
    }
    fn n_cells(&self) -> usize {
        (**self).n_cells()
    }
}

impl<T: Image + ?Sized> Image for &T {
    fn crc_cells(&self, off: usize, n: usize) -> u32 {
        (**self).crc_cells(off, n)
    }
    fn bytes_at(&self, at: usize, len: usize) -> Option<&[u8]> {
        (**self).bytes_at(at, len)
    }
}

/// `Store::check_obj` over `Cells`.
pub fn check_obj_in<C: Image + ?Sized>(c: &C, obj: usize) -> Result<(), BadCrc> {
    let want = ((c.cell_at(obj.saturating_add(1)) >> 32) & 0xFFFF_FFFF) as u32;
    let len = obj_len_in(c, obj) as usize;
    if obj.saturating_add(2).saturating_add(len) > c.n_cells() {
        return Err(BadCrc { obj, want, got: None });
    }
    let got = c.crc_cells(obj + 2, len);
    if got == want { Ok(()) } else { Err(BadCrc { obj, want, got: Some(got) }) }
}
