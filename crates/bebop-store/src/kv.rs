//! The KV layout `bebop-lang/selfhost/std/kv.bp` creates, shaped to dowiz's `MemoryStore`.
//!
//! Root `KV{n, ref KIDX, ref KBLOB, ref VIDX, ref VBLOB}`:
//!   KIDX  2n cells, (offset, len) per key into KBLOB, keys SORTED
//!   KBLOB key bytes, one per cell
//!   VIDX  2n cells, (offset, len) per value into VBLOB
//!   VBLOB value bytes, one per cell
//!
//! bebop creates the schema (the layout digests come from sha256, which stays on that side);
//! this module reads and writes the data through the documented pointer-free format.

use crate::{Store, StoreError};

/// FNV-1a 64-bit offset basis — the same constant dowiz-core uses.
pub const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
pub const FNV_PRIME: u64 = 0x100_0000_01b3;

fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// An in-memory view of every entry, read out of the store.
pub struct Kv {
    pub entries: Vec<(String, Vec<u8>)>,
}

impl Kv {
    /// Read all entries out of a store.
    pub fn load(st: &Store) -> Option<Kv> {
        let root = st.root()?;
        let n = st.get(root, 0) as usize;
        let kidx = st.follow(root, 1)?;
        let kblob = st.follow(root, 2)?;
        let vidx = st.follow(root, 3)?;
        let vblob = st.follow(root, 4)?;
        let mut entries = Vec::with_capacity(n);
        for i in 0..n {
            let ko = st.get(kidx, 2 * i) as usize;
            let kl = st.get(kidx, 2 * i + 1) as usize;
            let vo = st.get(vidx, 2 * i) as usize;
            let vl = st.get(vidx, 2 * i + 1) as usize;
            let k: String = (0..kl).map(|j| (st.get(kblob, ko + j) as u8) as char).collect();
            let v: Vec<u8> = (0..vl).map(|j| st.get(vblob, vo + j) as u8).collect();
            entries.push((k, v));
        }
        Some(Kv { entries })
    }

    /// Fetch a value by key.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    /// All keys, in the sorted order the store holds them in.
    pub fn keys(&self) -> Vec<String> {
        self.entries.iter().map(|(k, _)| k.clone()).collect()
    }

    /// Insert or overwrite, keeping keys sorted.
    pub fn put(&mut self, key: &str, value: &[u8]) {
        match self.entries.binary_search_by(|(k, _)| k.as_str().cmp(key)) {
            Ok(i) => self.entries[i].1 = value.to_vec(),
            Err(i) => self.entries.insert(i, (key.to_string(), value.to_vec())),
        }
    }

    /// FNV-1a 64 over frame-delimited (key, value) pairs -- `len || bytes` for each -- which is
    /// byte-for-byte what `InMemoryStore::snapshot_root` folds in dowiz-core, and what
    /// `kv_snapshot` folds in kv.bp.
    pub fn snapshot_root_u64(&self) -> u64 {
        let mut h = FNV_OFFSET;
        for (k, v) in &self.entries {
            h = fnv1a(h, &(k.len() as u64).to_le_bytes());
            h = fnv1a(h, k.as_bytes());
            h = fnv1a(h, &(v.len() as u64).to_le_bytes());
            h = fnv1a(h, v);
        }
        h
    }

    /// The same root, formatted the way dowiz-core formats it.
    pub fn snapshot_root(&self) -> String {
        format!("{:016x}", self.snapshot_root_u64())
    }

    /// Write every entry back as a new generation. Eager: it rewrites all four arrays and the
    /// root, mirroring what wlog.bp's update phase does. A tiered append is ROADMAP B4.
    pub fn commit_into(&self, st: &mut Store, path: &str) -> Result<i64, StoreError> {
        let old_root = st.root().ok_or(StoreError::NoSuperblock)?;
        let arr_dig = st.obj_digest(st.follow(old_root, 1).unwrap());
        let root_dig = st.obj_digest(old_root);

        let n = self.entries.len();
        let kbytes: usize = self.entries.iter().map(|(k, _)| k.len()).sum();
        let vbytes: usize = self.entries.iter().map(|(_, v)| v.len()).sum();

        let mut tx = st.begin()?;
        let kidx = st.alloc(&mut tx, (2 * n).max(1) as i64, arr_dig)?;
        let kblob = st.alloc(&mut tx, kbytes.max(1) as i64, arr_dig)?;
        let vidx = st.alloc(&mut tx, (2 * n).max(1) as i64, arr_dig)?;
        let vblob = st.alloc(&mut tx, vbytes.max(1) as i64, arr_dig)?;

        let (mut ko, mut vo) = (0usize, 0usize);
        for (i, (k, v)) in self.entries.iter().enumerate() {
            st.put_cell(kidx, 2 * i, ko as i64);
            st.put_cell(kidx, 2 * i + 1, k.len() as i64);
            for (j, b) in k.as_bytes().iter().enumerate() {
                st.put_cell(kblob, ko + j, *b as i64);
            }
            ko += k.len();
            st.put_cell(vidx, 2 * i, vo as i64);
            st.put_cell(vidx, 2 * i + 1, v.len() as i64);
            for (j, b) in v.iter().enumerate() {
                st.put_cell(vblob, vo + j, *b as i64);
            }
            vo += v.len();
        }
        st.seal(kidx); st.seal(kblob); st.seal(vidx); st.seal(vblob);

        let root = st.alloc(&mut tx, 5, root_dig)?;
        st.put_cell(root, 0, n as i64);
        st.link(root, 1, kidx);
        st.link(root, 2, kblob);
        st.link(root, 3, vidx);
        st.link(root, 4, vblob);
        st.seal(root);
        st.commit(&tx, root, path)
    }
}
