//! One v4 object read in place: a node's header, one slot's key and ref, a value. Zero allocation
//! except `items`, which the writer and `verify` use to read a whole node at once.

use alloc::vec::Vec;
use core::cmp::Ordering;

use super::{TreeError, DIGEST_KV4_LEAF, DIGEST_KV4_NODE, DIGEST_KV4_VAL, KIND_LEAF, KIND_NODE};
use crate::bytes::Bytes;
use crate::zc::Image;
use crate::{get_in, obj_digest_in, obj_len_in};

/// A node's decoded header. `karea` is the payload cell where the key area starts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NodeRef {
    pub obj: usize,
    pub leaf: bool,
    pub m: usize,
    pub stride: usize,
    pub len: usize,
    pub karea: usize,
}

impl NodeRef {
    /// Payload cell of slot i's K: a leaf's slot is (K, V), a node's (C, K, hash cells).
    pub fn k_cell(&self, i: usize) -> usize {
        1 + self.stride * i + usize::from(!self.leaf)
    }
    /// Payload cell of slot i's ref (V in a leaf, C in a node).
    pub fn r_cell(&self, i: usize) -> usize {
        1 + self.stride * i + usize::from(self.leaf)
    }
    /// Bytes the key area holds.
    pub fn kbytes(&self) -> usize {
        (self.len - self.karea) * 8
    }
}

fn bad(obj: usize, why: &'static str) -> TreeError {
    TreeError::Node { obj, why }
}

/// The node at `obj`, which must be a leaf iff `leaf`. A node of the OTHER kind is `Depth`: the
/// level it was reached at is the root's DEPTH claim minus the hops taken.
pub(crate) fn node_at<C: Image + ?Sized>(c: &C, obj: usize, leaf: bool) -> Result<NodeRef, TreeError> {
    let (want, other) = if leaf { (DIGEST_KV4_LEAF, DIGEST_KV4_NODE) } else { (DIGEST_KV4_NODE, DIGEST_KV4_LEAF) };
    let dig = obj_digest_in(c, obj);
    if dig == other {
        return Err(TreeError::Depth { obj });
    }
    if dig != want {
        return Err(bad(obj, "digest"));
    }
    let len = obj_len_in(c, obj) as usize;
    if obj.checked_add(2 + len).is_none_or(|e| e > c.n_cells()) || len == 0 {
        return Err(bad(obj, "length past the image"));
    }
    let meta = get_in(c, obj, 0) as u64;
    let (kind, hc, m) = (meta >> 56, (meta >> 48) & 0xFF, (meta & 0xFFFF_FFFF_FFFF) as usize);
    if kind != if leaf { KIND_LEAF } else { KIND_NODE } {
        return Err(bad(obj, "META kind"));
    }
    if leaf && hc != 0 {
        return Err(bad(obj, "hash cells in a leaf"));
    }
    // A node with ONE child is legal to read (the writer leaves one only on a chain it is about to
    // collapse, never reachable); `verify` refuses one that is reachable.
    if m == 0 || m > super::MAX_FANOUT {
        return Err(bad(obj, "META m"));
    }
    let stride = 2 + hc as usize;
    let karea = 1 + stride * m;
    if karea > len {
        return Err(bad(obj, "slots past the object"));
    }
    Ok(NodeRef { obj, leaf, m, stride, len, karea })
}

/// Slot i's key as (absolute BYTE address, length), bounded by the node's key area.
pub(crate) fn key_at<C: Image + ?Sized>(c: &C, nd: &NodeRef, i: usize) -> Result<(usize, usize), TreeError> {
    let k = get_in(c, nd.obj, nd.k_cell(i)) as u64;
    let (koff, klen) = ((k >> 16) as usize, (k & 0xFFFF) as usize);
    if koff.checked_add(klen).is_none_or(|e| e > nd.kbytes()) {
        return Err(bad(nd.obj, "key past the key area"));
    }
    Ok(((nd.obj + 2 + nd.karea) * 8 + koff, klen))
}

/// Slot i's ref, which must point STRICTLY BACKWARD (rule 1).
pub(crate) fn ref_at<C: Image + ?Sized>(c: &C, nd: &NodeRef, i: usize) -> Result<usize, TreeError> {
    back(c, nd.obj, nd.r_cell(i))
}

/// The ref in payload cell `cell` of `obj`, strictly backward and inside the image.
pub(crate) fn back<C: Image + ?Sized>(c: &C, obj: usize, cell: usize) -> Result<usize, TreeError> {
    let off = get_in(c, obj, cell);
    if off >= 0 {
        return Err(TreeError::Forward { obj });
    }
    match (obj as i64).checked_add(off) {
        Some(t) if t >= 0 => Ok(t as usize),
        _ => Err(TreeError::Forward { obj }),
    }
}

/// The value object at `obj`: (absolute byte address, length). Its length must be EXACT.
pub(crate) fn val_at<C: Image + ?Sized>(c: &C, obj: usize) -> Result<(usize, usize), TreeError> {
    if obj_digest_in(c, obj) != DIGEST_KV4_VAL {
        return Err(bad(obj, "value digest"));
    }
    let len = obj_len_in(c, obj) as usize;
    if obj.checked_add(2 + len).is_none_or(|e| e > c.n_cells()) {
        return Err(bad(obj, "value past the image"));
    }
    let vlen = get_in(c, obj, 0);
    if !(0..=crate::MAX_VALUE_BYTES as i64).contains(&vlen) || len != 1 + (vlen as usize).div_ceil(8) {
        return Err(bad(obj, "value length"));
    }
    Ok(((obj + 3) * 8, vlen as usize))
}

/// Compare the `len` bytes at `addr` with `probe`, reading each cell once.
pub(crate) fn cmp_at<C: Image + ?Sized>(c: &C, addr: usize, len: usize, probe: &[u8]) -> Ordering {
    if let Some(s) = c.bytes_at(addr, len) {
        return s.cmp(probe);
    }
    let (mut at, mut cell) = (usize::MAX, 0u64);
    for (i, &p) in probe.iter().take(len).enumerate() {
        let b = addr + i;
        if b / 8 != at {
            at = b / 8;
            cell = c.cell_at(at) as u64;
        }
        match ((cell >> (8 * (b % 8))) as u8).cmp(&p) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    len.cmp(&probe.len())
}

/// The `len` bytes at `addr`: borrowed where the image is bytes, gathered otherwise.
pub(crate) fn bytes_of<C: Image + ?Sized>(c: &C, addr: usize, len: usize) -> Bytes<'_> {
    match c.bytes_at(addr, len) {
        Some(s) => Bytes::borrowed(s),
        None => Bytes::owned(gather(c, addr, len)),
    }
}

/// The `len` bytes at `addr`, each cell read ONCE (a byte-at-a-time gather read a 256-byte value's
/// 32 cells 256 times: measured 317 cells per get at 1e4 against 93 after).
fn gather<C: Image + ?Sized>(c: &C, addr: usize, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut b = addr;
    while b < addr + len {
        let cell = c.cell_at(b / 8).to_le_bytes();
        let take = (8 - b % 8).min(addr + len - b);
        out.extend_from_slice(cell.get(b % 8..b % 8 + take).unwrap_or_default());
        b += take;
    }
    out
}

/// Lower bound in a node: the first slot whose key is >= `key`, and whether it is equal.
pub(crate) fn search<C: Image + ?Sized>(c: &C, nd: &NodeRef, key: &[u8]) -> Result<(usize, bool), TreeError> {
    let (mut lo, mut hi) = (0usize, nd.m);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let (a, l) = key_at(c, nd, mid)?;
        match cmp_at(c, a, l, key) {
            Ordering::Less => lo = mid + 1,
            Ordering::Equal => return Ok((mid, true)),
            Ordering::Greater => hi = mid,
        }
    }
    Ok((lo, false))
}

/// The child of an internal node that covers `key`: the last separator <= key, or child 0.
pub(crate) fn child_for<C: Image + ?Sized>(c: &C, nd: &NodeRef, key: &[u8]) -> Result<usize, TreeError> {
    let (i, eq) = search(c, nd, key)?;
    Ok(if eq { i } else { i.saturating_sub(1) })
}

/// Every slot of a node as (key, absolute ref), with the layout checked EXACTLY: keys packed in
/// slot order with no gap, the object exactly `karea + ceil(key bytes / 8)` cells, keys strictly
/// ascending, every ref backward.
pub(crate) fn items<C: Image + ?Sized>(c: &C, nd: &NodeRef) -> Result<Vec<(Vec<u8>, usize)>, TreeError> {
    let mut out: Vec<(Vec<u8>, usize)> = Vec::with_capacity(nd.m);
    let base = (nd.obj + 2 + nd.karea) * 8;
    let mut used = 0usize;
    for i in 0..nd.m {
        let (a, l) = key_at(c, nd, i)?;
        if a != base + used {
            return Err(bad(nd.obj, "keys not packed in slot order"));
        }
        let k = bytes_of(c, a, l).into_owned();
        if out.last().is_some_and(|(p, _)| *p >= k) {
            return Err(TreeError::Order { obj: nd.obj });
        }
        used += l;
        out.push((k, ref_at(c, nd, i)?));
    }
    if nd.len != nd.karea + used.div_ceil(8) {
        return Err(bad(nd.obj, "length is not exact"));
    }
    Ok(out)
}
