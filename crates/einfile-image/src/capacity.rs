//! WHAT A WRITER MAY SIZE BY (W-MG-TIDY). The superblock's capacity is a cell of the image, and
//! rotate, forget, cut and every grow hand it to `create_bytes`: a valid superblock claiming 2^50
//! cells on 64 KiB of bytes aborted the process in the allocation. `from_bytes` stopped padding by
//! it (W-MG-HOLES2); this is the writers' half.

use super::{idx, Store, StoreError, ARENA, MAX_PAD_CELLS};

impl Store {
    /// The size of the image this store was created as, in bytes. REFUSED, as `StoreError::Limit`
    /// "image capacity" in capacity cells: a capacity past `MAX_PAD_CELLS * 8` that the cells
    /// held do not bear out. (An image really that large, `Options::max_image_bytes`, holds them.)
    pub fn writer_bytes(&self) -> Result<usize, StoreError> {
        let claim = self.pick().and_then(|sb| self.cells.get(sb.at + 12).copied()).unwrap_or(0);
        let full = ARENA.saturating_add(idx(claim).unwrap_or(0));
        let held = self.cells.len();
        if full > MAX_PAD_CELLS && full > held {
            return Err(StoreError::Limit { what: "image capacity", got: claim, max: (MAX_PAD_CELLS - ARENA) as i64 });
        }
        Ok(full.max(held).saturating_mul(8))
    }
}

#[cfg(test)]
mod tests;
