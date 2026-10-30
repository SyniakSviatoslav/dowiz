//! KV v4 (W-ES-P8D): the copy-on-write B+tree (FORMAT §9.1 v4; `bebop_kv::tree`), read to the
//! same `n` and fold every other KV version answers with.
//!
//! AN OWN WALK, NOT THE LIBRARY'S `TreeIn`. Measured 2026-10-30: routing v4 through
//! `TreeIn::verify` + `TreeIn::fold` grew the wasm32 module 45979 -> 56903 bytes (+10924), nine
//! times the ~1.2 KB the operator allowed for v4 -- `items`, `Iter`, `Cow` and their `Vec`s come
//! along. This walk allocates nothing: a key is a slice of the image, the last key seen is kept
//! as that slice, and the fold runs over the same slices. The rules are `TreeIn::at` + `verify`
//! re-derived from FORMAT §9.1 v4, so every fixture gets the same word:
//!   root   8 cells {n, 0, 0, 0, 0, 4, ref TREE, DEPTH}; DEPTH 0 is the empty map (TREE 0, n 0)
//!   LEAF   [META = 1<<56 | m] [K_i, ref VAL_i]... [key bytes]        (reached at level 1)
//!   NODE   [META = 2<<56 | hc<<48 | m] [ref C_i, K_i, hc cells]... [key bytes]
//!   VAL    [vlen] [bytes, 8 to a cell]
//!   K_i    = koff << 16 | klen into the node's key area; every ref strictly BACKWARD (rule 1);
//!   a separator equals its child's first key; keys strictly ascending over the whole tree; a
//!   reachable node has >= 2 children; the root's n is the leaves' count.
//! Every object's crc is checked BEFORE its cells are decoded, and a crc that holds bounds the
//! object inside the image, so no length check repeats that bound. `TreeIn::walk` decodes a whole
//! node before descending; this walk decodes slot by slot, which can differ only in WHICH refusal
//! an image with two defects gets, never in whether it is refused.
//! The status vocabulary is docs/STATUS-CODES.md: an object's crc -> `BadCrc` (10); any structural
//! rule -- a v4 CLAIM that does not hold, fixture kv4stub included -- is `NotAKv` (2), the same word
//! a v3 chain that does not hold gets.

use bebop_store::format::MAX_VALUE_BYTES;
use bebop_store::kv::tree::{DIGEST_KV4_LEAF, DIGEST_KV4_NODE, DIGEST_KV4_VAL, KIND_LEAF, KIND_NODE, MAX_FANOUT, MAX_TREE_DEPTH, ROOT_V4};
use bebop_store::Store;

use crate::{fnv1a, KvView, Refusal, FNV_OFFSET};

const NO: Refusal = Refusal::NotAKv;

/// The v4 tree under the KV root at `root`, fully checked and folded. `bytes` is the image `st`
/// was read from (cell i = bytes[8i..8i+8]): keys and values are sliced out of it.
pub fn kv4_view(st: &Store, bytes: &[u8], root: usize) -> Result<KvView, Refusal> {
    if st.obj_len(root) != ROOT_V4 as i64 || (1..=4).any(|i| st.get(root, i) != 0) {
        return Err(NO);
    }
    let (n, depth) = (st.get(root, 0), st.get(root, 7));
    if n < 0 || !(0..=MAX_TREE_DEPTH as i64).contains(&depth) {
        return Err(NO);
    }
    let top = if depth == 0 {
        if st.get(root, 6) != 0 || n != 0 {
            return Err(NO);
        }
        0
    } else {
        back(st, root, 6)?
    };
    crc(st, root)?;
    let mut w = Walk { st, b: bytes, h: FNV_OFFSET, n: 0, last: None };
    if depth > 0 {
        w.walk(top, depth as usize, None)?;
    }
    if w.n != n {
        return Err(NO);
    }
    Ok(KvView { n, root: w.h as i64 })
}

struct Walk<'a> {
    st: &'a Store,
    b: &'a [u8],
    h: u64,
    n: i64,
    last: Option<&'a [u8]>,
}

/// A node's decoded header: `kbase` is the BYTE address of its key area, `kbytes` its size.
struct Node {
    obj: usize,
    m: usize,
    stride: usize,
    kbase: usize,
    kbytes: u64,
    leaf: bool,
}

/// `obj_crc_ok`, not `check_obj`: measured 2026-10-30, the latter's `BadCrc` plumbing costs 8 B more.
fn crc(st: &Store, obj: usize) -> Result<(), Refusal> {
    if st.obj_crc_ok(obj) { Ok(()) } else { Err(Refusal::BadCrc { obj }) }
}

/// The ref in payload cell `cell` of `obj`: strictly backward (rule 1) and inside the image.
fn back(st: &Store, obj: usize, cell: usize) -> Result<usize, Refusal> {
    let off = st.get(obj, cell);
    let t = (obj as i64).wrapping_add(off);
    if off >= 0 || t < 0 { Err(NO) } else { Ok(t as usize) }
}

/// The node at `obj` (its crc already held), which must be a leaf iff `leaf`.
fn node_at(st: &Store, obj: usize, leaf: bool) -> Result<Node, Refusal> {
    let (want, kind) = if leaf { (DIGEST_KV4_LEAF, KIND_LEAF) } else { (DIGEST_KV4_NODE, KIND_NODE) };
    let len = st.obj_len(obj) as usize;
    let meta = st.get(obj, 0) as u64;
    let (hc, m) = ((meta >> 48) & 0xFF, (meta & 0xFFFF_FFFF_FFFF) as usize);
    if st.obj_digest(obj) != want || meta >> 56 != kind || (leaf && hc != 0) || m == 0 || m > MAX_FANOUT {
        return Err(NO);
    }
    let stride = 2 + hc as usize;
    let karea = 1 + stride * m;
    if karea > len {
        return Err(NO);
    }
    Ok(Node { obj, m, stride, kbase: (obj + 2 + karea) * 8, kbytes: ((len - karea) * 8) as u64, leaf })
}

impl<'a> Walk<'a> {
    /// Slot i: its key, a slice of the image bounded by the node's key area, and its ref (VAL in a
    /// leaf, a child in a node).
    fn slot(&self, nd: &Node, i: usize) -> Result<(&'a [u8], usize), Refusal> {
        let at = 1 + nd.stride * i;
        let k = self.st.get(nd.obj, at + usize::from(!nd.leaf)) as u64;
        let (koff, klen) = (k >> 16, k & 0xFFFF);
        if koff + klen > nd.kbytes {
            return Err(NO);
        }
        let from = nd.kbase + koff as usize;
        let key = self.b.get(from..from + klen as usize).ok_or(NO)?;
        Ok((key, back(self.st, nd.obj, at + usize::from(nd.leaf))?))
    }

    /// The value object at `obj` (its crc already held), as a slice of the image; its length is exact.
    fn val(&self, obj: usize) -> Result<&'a [u8], Refusal> {
        let vlen = self.st.get(obj, 0);
        if self.st.obj_digest(obj) != DIGEST_KV4_VAL
            || !(0..=MAX_VALUE_BYTES as i64).contains(&vlen)
            || self.st.obj_len(obj) != 1 + (vlen + 7) / 8
        {
            return Err(NO);
        }
        self.b.get((obj + 3) * 8..(obj + 3) * 8 + vlen as usize).ok_or(NO)
    }

    /// `TreeIn::walk`: the node at `obj`, reached `level` hops above the leaves, whose first key
    /// must be `sep` (its parent's separator) when there is one.
    fn walk(&mut self, obj: usize, level: usize, sep: Option<&[u8]>) -> Result<(), Refusal> {
        crc(self.st, obj)?;
        let nd = node_at(self.st, obj, level == 1)?;
        if level > 1 && nd.m < 2 {
            return Err(NO);
        }
        for i in 0..nd.m {
            let (k, r) = self.slot(&nd, i)?;
            if i == 0 && sep.is_some_and(|s| s != k) {
                return Err(NO);
            }
            if level > 1 {
                self.walk(r, level - 1, Some(k))?;
                continue;
            }
            if self.last.is_some_and(|l| l >= k) {
                return Err(NO);
            }
            crc(self.st, r)?;
            let v = self.val(r)?;
            self.h = fnv1a(self.h, &(k.len() as u64).to_le_bytes());
            self.h = fnv1a(self.h, k);
            self.h = fnv1a(self.h, &(v.len() as u64).to_le_bytes());
            self.h = fnv1a(self.h, v);
            self.last = Some(k);
            self.n += 1;
        }
        Ok(())
    }
}
