//! The published root and its inclusion proof (LV-PCD-C, R-BEBOP-LEVERS §4.4, AC-LV-PCD-01).
//! Normative text: `docs/design/SPEC-PCD-2026-10-27.md`; this file is one of its readers.
//!
//! THE TREE IS RFC 9162's (Certificate Transparency v2, §2.1): leaves and nodes are
//! DOMAIN-SEPARATED -- `leaf = sha256(0x00 ‖ data)`, `node = sha256(0x01 ‖ left ‖ right)` -- so a
//! node's hash can never be presented as a leaf (`Merkle.lean verify_binding_leaf`'s hypothesis),
//! and the pair hash is injective on (left, right) up to SHA-256 collisions (`hinj`). An odd count is
//! not padded: the tree splits at the largest power of two below `n` (MTH). The empty tree is
//! `sha256("")`.
//!
//! THE PATH IS BOTTOM-UP, leaf first, siblings only; which side each sibling sits on is a function of
//! `(index, n)` alone (RFC 9162 §2.1.3.2), so a proof carries no side bits. `Merkle.lean`'s path is
//! TOP-DOWN with explicit sides: it is `sides(index, n)` zipped with this path, REVERSED (twin test:
//! `merkle/tests.rs lean_twin_verify_path_on_five_leaves`).
//!
//! THE CERTIFICATE (`Cert`) is one nodekey-style frame -- `'V' ‖ field*`, field = u64 LE length ‖
//! bytes -- so every reader that parses node keys parses it: `crates/bebop-wasm/src/verify.rs`
//! (wasm32, the browser), `crates/bebop-wasm/oracle_verify.py`, `bebop-lang/selfhost/prelude/merkle.bp`.
//! Zero dependencies, as the crate: SHA-256 is `nodekey::sha256`.

use crate::nodekey::sha256;

pub type Hash = [u8; 32];

/// The leaf and node domain bytes (RFC 6962 §2.1).
pub const LEAF: u8 = 0x00;
pub const NODE: u8 = 0x01;
/// The certificate frame's tag.
pub const TAG_CERT: u8 = b'V';
/// The longest path a certificate may carry: 2^64 leaves.
pub const MAX_DEPTH: usize = 64;
/// The longest product id a price leaf may name.
pub const MAX_ID: usize = 256;

/// `sha256(0x00 ‖ data)`.
pub fn leaf_hash(data: &[u8]) -> Hash {
    let mut b = Vec::with_capacity(data.len() + 1);
    b.push(LEAF);
    b.extend_from_slice(data);
    sha256(&b)
}

/// `sha256(0x01 ‖ l ‖ r)`.
pub fn node_hash(l: &Hash, r: &Hash) -> Hash {
    let mut b = [0u8; 65];
    b[0] = NODE;
    b[1..33].copy_from_slice(l);
    b[33..].copy_from_slice(r);
    sha256(&b)
}

fn field(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u64).to_le_bytes());
    out.extend_from_slice(b);
}

/// A price leaf's data: `field(id) ‖ field(price as i64 LE)` -- integer minor units, never a float.
pub fn price_leaf(id: &[u8], price: i64) -> Vec<u8> {
    let mut v = Vec::with_capacity(id.len() + 24);
    field(&mut v, id);
    field(&mut v, &price.to_le_bytes());
    v
}

/// The leaf hashes of a price list, in id byte order. `None` on a repeated or over-long id: one id,
/// one price, or the root would prove two prices for one dish.
pub fn price_leaves(rows: &[(Vec<u8>, i64)]) -> Option<Vec<(Vec<u8>, i64, Hash)>> {
    let mut v: Vec<(Vec<u8>, i64)> = rows.to_vec();
    v.sort();
    if v.windows(2).any(|w| w[0].0 == w[1].0) || v.iter().any(|r| r.0.len() > MAX_ID) {
        return None;
    }
    Some(v.into_iter().map(|(id, p)| { let h = leaf_hash(&price_leaf(&id, p)); (id, p, h) }).collect())
}

/// The largest power of two strictly below `n` (n >= 2).
fn split(n: usize) -> usize {
    let mut k = 1;
    while k << 1 < n {
        k <<= 1;
    }
    k
}

/// MTH over leaf hashes.
pub fn root(leaves: &[Hash]) -> Hash {
    match leaves.len() {
        0 => sha256(&[]),
        1 => leaves[0],
        n => {
            let k = split(n);
            node_hash(&root(&leaves[..k]), &root(&leaves[k..]))
        }
    }
}

/// The inclusion path of leaf `index`, bottom-up. `None` when `index` is out of range.
pub fn prove(leaves: &[Hash], index: usize) -> Option<Vec<Hash>> {
    let n = leaves.len();
    if index >= n {
        return None;
    }
    if n == 1 {
        return Some(Vec::new());
    }
    let k = split(n);
    let mut p = if index < k { prove(&leaves[..k], index)? } else { prove(&leaves[k..], index - k)? };
    p.push(if index < k { root(&leaves[k..]) } else { root(&leaves[..k]) });
    Some(p)
}

/// Where a path's sibling sits relative to the running hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// `node(sibling, running)`.
    Left,
    /// `node(running, sibling)`.
    Right,
}

/// The side of each sibling, bottom-up, for leaf `index` of `n` (RFC 9162 §2.1.3.2 steps 1-3).
/// `None` when the shape is impossible: `index >= n` or a path of the wrong length (`len`).
pub fn sides(index: u64, n: u64, len: usize) -> Option<Vec<Side>> {
    if index >= n || len > MAX_DEPTH {
        return None;
    }
    let (mut f, mut s) = (index, n - 1);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        if s == 0 {
            return None; // the path is longer than the tree is deep
        }
        if f & 1 == 1 || f == s {
            out.push(Side::Left);
            if f & 1 == 0 {
                while f & 1 == 0 && f != 0 {
                    f >>= 1;
                    s >>= 1;
                }
            }
        } else {
            out.push(Side::Right);
        }
        f >>= 1;
        s >>= 1;
    }
    if s != 0 {
        return None; // the path stops short of the root
    }
    Some(out)
}

/// Recompute the root from a leaf hash and its bottom-up path. `None` on an impossible shape.
pub fn climb(leaf: &Hash, index: u64, n: u64, path: &[Hash]) -> Option<Hash> {
    let sides = sides(index, n, path.len())?;
    let mut r = *leaf;
    for (sd, sib) in sides.iter().zip(path) {
        r = match sd {
            Side::Left => node_hash(sib, &r),
            Side::Right => node_hash(&r, sib),
        };
    }
    Some(r)
}

/// `verify leaf path root`: the climb, compared. Nothing else is checked, nothing searched.
pub fn verify(leaf: &Hash, index: u64, n: u64, path: &[Hash], root: &Hash) -> bool {
    climb(leaf, index, n, path).as_ref() == Some(root)
}

/// One receipt line's price proof (SPEC-PCD §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cert {
    pub id: Vec<u8>,
    pub price: i64,
    pub index: u64,
    pub n: u64,
    pub root: Hash,
    pub path: Vec<Hash>,
}

/// Why a certificate was not accepted. Each is one status code in every reader (SPEC-PCD §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertError {
    /// The frame does not parse, or its shape is impossible (index >= n, wrong path length).
    Malformed,
    /// The path climbs to `climbed`, which is not the root the certificate names.
    Mismatch { climbed: Hash },
}

impl Cert {
    /// The certificate of leaf `index` of a price list (`price_leaves` order).
    pub fn of(leaves: &[(Vec<u8>, i64, Hash)], index: usize) -> Option<Cert> {
        let hs: Vec<Hash> = leaves.iter().map(|l| l.2).collect();
        let path = prove(&hs, index)?;
        let (id, price, _) = &leaves[index];
        Some(Cert { id: id.clone(), price: *price, index: index as u64, n: hs.len() as u64, root: root(&hs), path })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut v = vec![TAG_CERT];
        field(&mut v, &self.id);
        field(&mut v, &self.price.to_le_bytes());
        field(&mut v, &self.index.to_le_bytes());
        field(&mut v, &self.n.to_le_bytes());
        field(&mut v, &self.root);
        field(&mut v, &self.path.concat());
        v
    }

    /// Strict: every length exact, nothing left over.
    pub fn decode(b: &[u8]) -> Result<Cert, CertError> {
        let mut at = 1usize;
        if b.first() != Some(&TAG_CERT) {
            return Err(CertError::Malformed);
        }
        let mut next = |max: usize| -> Result<&[u8], CertError> {
            let len = b.get(at..at + 8).ok_or(CertError::Malformed)?;
            let len = u64::from_le_bytes(len.try_into().map_err(|_| CertError::Malformed)?);
            if len > max as u64 {
                return Err(CertError::Malformed);
            }
            let s = b.get(at + 8..at + 8 + len as usize).ok_or(CertError::Malformed)?;
            at += 8 + len as usize;
            Ok(s)
        };
        let num = |s: &[u8]| -> Result<u64, CertError> { Ok(u64::from_le_bytes(s.try_into().map_err(|_| CertError::Malformed)?)) };
        let id = next(MAX_ID)?.to_vec();
        let price = num(next(8)?)? as i64;
        let index = num(next(8)?)?;
        let n = num(next(8)?)?;
        let root: Hash = next(32)?.try_into().map_err(|_| CertError::Malformed)?;
        let raw = next(32 * MAX_DEPTH)?;
        if raw.len() % 32 != 0 {
            return Err(CertError::Malformed);
        }
        let path = raw.chunks_exact(32).map(|c| c.try_into().unwrap_or([0; 32])).collect();
        if at != b.len() {
            return Err(CertError::Malformed);
        }
        Ok(Cert { id, price, index, n, root, path })
    }

    /// Climb from this line's leaf; `Ok(root)` when it lands on the named root.
    pub fn check(&self) -> Result<Hash, CertError> {
        let leaf = leaf_hash(&price_leaf(&self.id, self.price));
        let climbed = climb(&leaf, self.index, self.n, &self.path).ok_or(CertError::Malformed)?;
        if climbed == self.root { Ok(climbed) } else { Err(CertError::Mismatch { climbed }) }
    }
}

/// The ML-DSA-65 signed message over a root (SPEC-PCD §4): one frame, so no two (slug, gen, n, root)
/// tuples share bytes. `crates/dowiz-core/src/pq/dsa.rs` signs it in the venue's object.
pub fn signed_message(slug: &[u8], gen: i64, n: u64, root: &Hash) -> Vec<u8> {
    let mut v = vec![b'R'];
    field(&mut v, b"dowiz-price-root-v1");
    field(&mut v, slug);
    field(&mut v, &gen.to_le_bytes());
    field(&mut v, &n.to_le_bytes());
    field(&mut v, root);
    v
}

#[cfg(test)]
mod tests;
