//! THE ZERO-COPY KV READER (W-ZC, R-BEBOPDB D.1 #1, 2026-10-06).
//!
//! `Kv::load` copies the image into a `Store` and decodes EVERY entry into a
//! `Vec<(String, Vec<u8>)>` before one value can be read: FRESH 368-787 us for a
//! 165-dish catalogue (docs/research/2026-10-05-bebop-db-gaps-strengths-tensor-graph.md
//! F.3 W1), on every Durable Object handler that wanted one dish. This module reads
//! the SAME bytes in place, through `Cells` (`View` borrows them): root -> KIDX
//! binary search -> the one value. The format does not change.
//!
//! TWO WAYS IN, because the crc and the speed pull apart (W-CRC's policy holds):
//!
//!   * `KvIn::open` -- the FULL check: the crc of the root and of all four arrays,
//!     every index slice bounded by its blob, the byte budget, the key order. It
//!     refuses exactly the images `Kv::load_checked` refuses (`NotKv` / `BadCrc`
//!     naming the object). The value blob is one object, and the format has no
//!     per-value crc, so verifying ONE value costs the crc of the WHOLE blob (~370 us
//!     at 165 x 2.5 KB): `open` costs that once.
//!   * `KvIn::reopen(cells, Checked)` -- for a holder that already opened THESE bytes
//!     (the DO keeps a `Checked` per image generation): O(1) shape checks, no crc.
//!     A `Checked` can only come out of a successful `open`. Every slice a read touches
//!     is still bounded by the image, so a wrong `Checked` gives a wrong answer at
//!     worst, never a panic and never an allocation beyond the blobs.
//!
//! KEYS ARE SORTED by every writer in the tree (`Kv::put` inserts by binary search,
//! `stage_write` writes `entries` in order; `kv.bp` writes only the empty schema), and
//! the order is raw-byte order, which is Rust's `str` order. `open` CHECKS it anyway:
//! an image whose keys are not strictly ascending is read by a linear scan (`sorted()`
//! says which), so an image nobody here wrote still answers like `Kv::get`.
//!
//! LIMIT, stated: a key is compared as raw bytes; `Kv` decodes keys lossily. They
//! differ only for a key that is not valid UTF-8, which no writer here produces.

use std::borrow::Cow;
use std::cmp::Ordering;

use super::{FNV_OFFSET, ROOT_V2, VERSION};
use crate::verify::KvError;
use crate::{follow_in, get_in, obj_cells_in, root_in};

mod image;
pub use image::{check_obj_in, Image};

/// Proof that `KvIn::open` passed on some bytes, and what it learned (the key order).
/// The field is private: only `open` makes one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checked {
    sorted: bool,
}

/// A KV image opened in place.
pub struct KvIn<C: Image> {
    c: C,
    ver: i64,
    n: usize,
    kidx: usize,
    kblob: usize,
    vidx: usize,
    vblob: usize,
    /// Bytes the two blobs really hold (cells behind the object x 1 or 8).
    kbytes: usize,
    vbytes: usize,
    sorted: bool,
}

/// One entry's (offset, len), bounded by the blob: `None` is a slice the image does not hold.
fn slice_of(off: i64, len: i64, bytes: usize) -> Option<(usize, usize)> {
    if off < 0 || len < 0 {
        return None;
    }
    let end = (off as usize).checked_add(len as usize)?;
    if end > bytes { None } else { Some((off as usize, len as usize)) }
}

impl<C: Image> KvIn<C> {
    /// The full check. See the module.
    pub fn open(c: C) -> Result<Self, KvError> {
        let root = root_in(&c).ok_or(KvError::NotKv)?;
        check_obj_in(&c, root).map_err(KvError::BadCrc)?;
        for i in 1..=4 {
            if let Some(arr) = follow_in(&c, root, i) {
                check_obj_in(&c, arr).map_err(KvError::BadCrc)?;
            }
        }
        let mut kv = Self::shape(c, true)?;
        kv.sorted = kv.index_pass()?;
        Ok(kv)
    }

    /// For bytes this holder already `open`ed. See the module.
    pub fn reopen(c: C, checked: Checked) -> Result<Self, KvError> {
        Self::shape(c, checked.sorted)
    }

    /// What `open` learned, for `reopen`.
    pub fn checked(&self) -> Checked {
        Checked { sorted: self.sorted }
    }

    /// Whether the keys were found strictly ascending (binary search) or not (linear scan).
    pub fn sorted(&self) -> bool {
        self.sorted
    }

    /// Entries in the image.
    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Root, version, count and the four arrays, every number bounded by the image --
    /// the O(1) half of `Kv::decode`'s checks.
    fn shape(c: C, sorted: bool) -> Result<Self, KvError> {
        let root = root_in(&c).ok_or(KvError::NotKv)?;
        let ver = if obj_cells_in(&c, root) >= ROOT_V2 && get_in(&c, root, 5) > 0 { get_in(&c, root, 5) } else { 1 };
        if ver > VERSION {
            return Err(KvError::NotKv);
        }
        let n = get_in(&c, root, 0);
        let arr = |i| follow_in(&c, root, i).ok_or(KvError::NotKv);
        let (kidx, kblob, vidx, vblob) = (arr(1)?, arr(2)?, arr(3)?, arr(4)?);
        let bytes = |o: usize| -> Result<usize, KvError> {
            let cells = obj_cells_in(&c, o);
            if ver >= 2 { cells.checked_mul(8).ok_or(KvError::NotKv) } else { Ok(cells) }
        };
        let (kbytes, vbytes) = (bytes(kblob)?, bytes(vblob)?);
        if n < 0 || (n as usize).checked_mul(2).is_none_or(|t| t > obj_cells_in(&c, kidx).min(obj_cells_in(&c, vidx))) {
            return Err(KvError::NotKv);
        }
        Ok(KvIn { ver, n: n as usize, kidx, kblob, vidx, vblob, kbytes, vbytes, sorted, c })
    }

    /// Every slice bounded, the budget (entries together no larger than the blobs:
    /// W-AUDIT S2's quadratic guard), and the key order. `Err` = what `decode` refuses.
    fn index_pass(&self) -> Result<bool, KvError> {
        let mut budget = self.kbytes.checked_add(self.vbytes).ok_or(KvError::NotKv)?;
        let mut sorted = true;
        for i in 0..self.n {
            let (_, kl) = self.key_slice(i)?;
            let (_, vl) = self.val_slice(i)?;
            budget = budget.checked_sub(kl + vl).ok_or(KvError::NotKv)?;
            if sorted && i > 0 && self.cmp_keys(i - 1, i)? != Ordering::Less {
                sorted = false;
            }
        }
        Ok(sorted)
    }

    fn key_slice(&self, i: usize) -> Result<(usize, usize), KvError> {
        slice_of(get_in(&self.c, self.kidx, 2 * i), get_in(&self.c, self.kidx, 2 * i + 1), self.kbytes).ok_or(KvError::NotKv)
    }

    fn val_slice(&self, i: usize) -> Result<(usize, usize), KvError> {
        slice_of(get_in(&self.c, self.vidx, 2 * i), get_in(&self.c, self.vidx, 2 * i + 1), self.vbytes).ok_or(KvError::NotKv)
    }

    /// Byte `b` of a blob.
    #[inline]
    fn byte(&self, blob: usize, b: usize) -> u8 {
        if self.ver >= 2 {
            (get_in(&self.c, blob, b >> 3) >> (8 * (b & 7))) as u8
        } else {
            get_in(&self.c, blob, b) as u8
        }
    }

    /// `len` bytes of a blob from byte `off` -- borrowed when the image is bytes and
    /// the blob is packed (v2), copied otherwise. The caller bounded the slice.
    fn blob(&self, blob: usize, off: usize, len: usize) -> Cow<'_, [u8]> {
        if self.ver >= 2 {
            if let Some(b) = (blob + 2).checked_mul(8).and_then(|a| a.checked_add(off)).and_then(|a| self.c.bytes_at(a, len)) {
                return Cow::Borrowed(b);
            }
        }
        Cow::Owned((0..len).map(|j| self.byte(blob, off + j)).collect())
    }

    /// Key `i` against `key`.
    fn cmp_key(&self, i: usize, key: &[u8]) -> Result<Ordering, KvError> {
        let (ko, kl) = self.key_slice(i)?;
        for (j, &b) in key.iter().enumerate().take(kl) {
            let c = self.byte(self.kblob, ko + j);
            if c != b {
                return Ok(c.cmp(&b));
            }
        }
        Ok(kl.cmp(&key.len()))
    }

    fn cmp_keys(&self, a: usize, b: usize) -> Result<Ordering, KvError> {
        let (bo, bl) = self.key_slice(b)?;
        let kb = self.blob(self.kblob, bo, bl).into_owned();
        self.cmp_key(a, &kb)
    }

    /// The index of `key`, by binary search on a sorted image, by scan otherwise.
    fn find(&self, key: &[u8]) -> Result<Option<usize>, KvError> {
        if !self.sorted {
            for i in 0..self.n {
                if self.cmp_key(i, key)? == Ordering::Equal {
                    return Ok(Some(i));
                }
            }
            return Ok(None);
        }
        let (mut lo, mut hi) = (0usize, self.n);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match self.cmp_key(mid, key)? {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return Ok(Some(mid)),
            }
        }
        Ok(None)
    }

    /// Key `i`'s bytes.
    pub fn key(&self, i: usize) -> Result<Cow<'_, [u8]>, KvError> {
        let (o, l) = self.key_slice(i)?;
        Ok(self.blob(self.kblob, o, l))
    }

    /// Value `i`'s bytes.
    pub fn value(&self, i: usize) -> Result<Cow<'_, [u8]>, KvError> {
        let (o, l) = self.val_slice(i)?;
        Ok(self.blob(self.vblob, o, l))
    }

    /// The value under `key`: O(log n) keys and the one value are read.
    pub fn get(&self, key: &[u8]) -> Result<Option<Cow<'_, [u8]>>, KvError> {
        match self.find(key)? {
            Some(i) => self.value(i).map(Some),
            None => Ok(None),
        }
    }

    /// The indices of every key starting with `prefix`, in stored order. On a sorted
    /// image: a lower-bound search, then a walk while the prefix holds.
    pub fn prefix_range(&self, prefix: &[u8]) -> Result<Vec<usize>, KvError> {
        let starts = |i: usize| -> Result<bool, KvError> {
            let (ko, kl) = self.key_slice(i)?;
            Ok(kl >= prefix.len() && (0..prefix.len()).all(|j| self.byte(self.kblob, ko + j) == prefix[j]))
        };
        let mut out = Vec::new();
        if !self.sorted {
            for i in 0..self.n {
                if starts(i)? {
                    out.push(i);
                }
            }
            return Ok(out);
        }
        let (mut lo, mut hi) = (0usize, self.n);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.cmp_key(mid, prefix)? == Ordering::Less { lo = mid + 1 } else { hi = mid }
        }
        let mut i = lo;
        while i < self.n && starts(i)? {
            out.push(i);
            i += 1;
        }
        Ok(out)
    }

    /// FNV-1a 64 over `len||key||len||value`, in stored order: `Kv::snapshot_root_u64`,
    /// without building the entries.
    pub fn snapshot_root_u64(&self) -> Result<u64, KvError> {
        let mut h = FNV_OFFSET;
        for i in 0..self.n {
            let (k, v) = (self.key(i)?, self.value(i)?);
            h = super::fnv1a(h, &(k.len() as u64).to_le_bytes());
            h = super::fnv1a(h, &k);
            h = super::fnv1a(h, &(v.len() as u64).to_le_bytes());
            h = super::fnv1a(h, &v);
        }
        Ok(h)
    }
}

/// One value straight from the image cells, fully checked (`KvIn::open`).
pub fn kv_get_in<C: Image + ?Sized>(cells: &C, key: &[u8]) -> Result<Option<Vec<u8>>, KvError> {
    let kv = KvIn::open(cells)?;
    Ok(kv.get(key)?.map(Cow::into_owned))
}

/// Every `(key, value)` whose key starts with `prefix`, fully checked, in key order.
pub fn kv_prefix_in<C: Image + ?Sized>(cells: &C, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, KvError> {
    let kv = KvIn::open(cells)?;
    kv.prefix_range(prefix)?
        .into_iter()
        .map(|i| Ok((kv.key(i)?.into_owned(), kv.value(i)?.into_owned())))
        .collect()
}

#[cfg(test)]
mod tests;
