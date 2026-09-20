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

/// `st_digest("KV{i64,ref [i64],ref [i64],ref [i64],ref [i64]}")`, the KV root layout.
/// Read out of a store that `kv.bp` created; re-derivable by running `kv.bin i` and
/// inspecting the root object's header. Pinned here so Rust can create a store from
/// scratch without reimplementing sha256.
pub const DIGEST_KV_ROOT: i64 = 610082063;
/// `st_digest("arr i64")`, the layout of the four entry arrays. Same provenance.
pub const DIGEST_ARR_I64: i64 = 4290599237;

impl Kv {
    /// Create the empty KV schema in a fresh store -- the same four zero-length arrays and
    /// root that `kv.bp`'s init phase writes.
    fn stage_init(st: &mut Store) -> Result<(crate::Tx, usize), StoreError> {
        let mut tx = st.begin()?;
        let kidx = st.alloc(&mut tx, 1, DIGEST_ARR_I64)?;
        let kblob = st.alloc(&mut tx, 1, DIGEST_ARR_I64)?;
        let vidx = st.alloc(&mut tx, 1, DIGEST_ARR_I64)?;
        let vblob = st.alloc(&mut tx, 1, DIGEST_ARR_I64)?;
        st.seal(kidx); st.seal(kblob); st.seal(vidx); st.seal(vblob);
        let root = st.alloc(&mut tx, 5, DIGEST_KV_ROOT)?;
        st.put_cell(root, 0, 0);
        st.link(root, 1, kidx);
        st.link(root, 2, kblob);
        st.link(root, 3, vidx);
        st.link(root, 4, vblob);
        st.seal(root);
        Ok((tx, root))
    }

    /// Create the empty KV schema in a fresh store.
    pub fn init(st: &mut Store, path: &str) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_init(st)?;
        st.commit(&tx, root, path)
    }

    /// `init` with no filesystem.
    pub fn init_bytes(st: &mut Store) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_init(st)?;
        Ok(st.commit_bytes(&tx, root))
    }

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
            // KEYS ARE UTF-8 BYTES AND MUST BE DECODED AS UTF-8. This read
            // `(byte as char)`, which is not a decode at all -- in Rust that
            // maps a `u8` to the code point of the same value, which is exactly
            // Latin-1. The write side has always been `k.as_bytes()`, so every
            // non-ASCII key was stored correctly and read back wrong, and the
            // wrong string was then written back as ITS OWN UTF-8 -- so the
            // damage COMPOUNDED on every round trip: `ujë` became `ujÃ«`, then
            // `ujÃÂ«`, then `ujÃÂÃÂ«`, and the dish it identified became a
            // new dish each time. It reached production as duplicate products on
            // an Albanian menu, which is the whole product's alphabet.
            //
            // Lossy rather than strict: a key that is already damaged must still
            // be readable, or this fix would make an affected image unopenable
            // instead of repairable.
            let kb: Vec<u8> = (0..kl).map(|j| st.get(kblob, ko + j) as u8).collect();
            let k: String = String::from_utf8_lossy(&kb).into_owned();
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

    /// Remove a key, reporting whether it was there. The commit rewrites all
    /// four arrays anyway, so a delete costs exactly what a put costs and there
    /// is no tombstone to compact later.
    pub fn remove(&mut self, key: &str) -> bool {
        match self.entries.binary_search_by(|(k, _)| k.as_str().cmp(key)) {
            Ok(i) => {
                self.entries.remove(i);
                true
            }
            Err(_) => false,
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
    fn stage_commit_into(&self, st: &mut Store) -> Result<(crate::Tx, usize), StoreError> {
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
        Ok((tx, root))
    }

    /// Write every entry back as a new generation.
    pub fn commit_into(&self, st: &mut Store, path: &str) -> Result<i64, StoreError> {
        let (tx, root) = self.stage_commit_into(st)?;
        st.commit(&tx, root, path)
    }

    /// `commit_into` with no filesystem.
    /// Commit into a FRESH image of `capacity` bytes and hand it back.
    ///
    /// WHY THIS EXISTS, measured rather than argued. The store is append-only:
    /// every commit allocates five new objects and the previous generation is
    /// never reclaimed. That is exactly right for `EvLog`, whose whole purpose
    /// is an immutable chain -- and exactly wrong for a KV, which rewrites the
    /// same small map over and over. The cost is not per ENTRY, it is per
    /// COMMIT: a roster with one person and no sessions filled its arena after
    /// 313 empty commits, while five hundred sessions written in a single
    /// commit fitted with room to spare.
    ///
    /// For a hub that means the roster stops accepting writes after a few
    /// hundred logins -- and the route it fails is the one everybody needs to
    /// get in. The same held for the catalogue, the settings, the subscriptions
    /// and the posts.
    ///
    /// A `Kv` holds all of its entries in memory, so committing them into a new
    /// store is not a repair or a migration: it is the same map, written once,
    /// with no dead generations behind it. The image is therefore as large as
    /// its CONTENT rather than as large as its history.
    ///
    /// WHAT IS GIVEN UP: the old image's generations. Nothing in dowiz reads
    /// them -- `snapshot_root` folds the entries themselves, not the store --
    /// and an audit trail belongs in the event log, which is append-only on
    /// purpose and is not this.
    pub fn compacted_bytes(&self, capacity: usize) -> Result<Vec<u8>, StoreError> {
        let mut fresh = Store::create_bytes(capacity);
        Kv::init_bytes(&mut fresh)?;
        let (tx, root) = self.stage_commit_into(&mut fresh)?;
        fresh.commit_bytes(&tx, root);
        // TRIMMED. The fresh image was sized by doubling until the content
        // fitted, so most of the arena it ended up with is untouched zeros --
        // the tail `Store::from_bytes` re-creates from the capacity in the
        // superblock. Writing it would send up to half an image of nothing on
        // every settings change and every menu import.
        Ok(fresh.to_bytes_trimmed())
    }

    /// Commit into the SMALLEST image that holds the data, up to `max`.
    ///
    /// The size of a KV image is otherwise an arbitrary constant chosen once,
    /// and a constant chosen once is a constant that turns out to be wrong
    /// somewhere specific. It did: the catalogue's 1 MiB arena is larger than
    /// D1's one-million-byte row limit, so a fifty-two dish menu -- about sixty
    /// kilobytes of actual content -- could not be written to the Worker's
    /// store at all, and the failure was a 500 with no message.
    ///
    /// Doubling from 16 KiB finds the fit in a handful of attempts, each of
    /// which is a commit into a fresh arena and therefore cheap. `max` is still
    /// honoured, so a genuinely large menu grows to the declared ceiling rather
    /// than silently truncating.
    pub fn compacted_bytes_fit(&self, max: usize) -> Result<Vec<u8>, StoreError> {
        let mut cap = 16 * 1024;
        loop {
            match self.compacted_bytes(cap) {
                Ok(b) => return Ok(b),
                Err(StoreError::ArenaFull { .. }) if cap < max => {
                    cap = (cap * 2).min(max);
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn commit_into_bytes(&self, st: &mut Store) -> Result<i64, StoreError> {
        let (tx, root) = self.stage_commit_into(st)?;
        Ok(st.commit_bytes(&tx, root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A store created, written and read back entirely by Rust: the schema, five entries, a
    /// reopen, and the FNV-1a root. The root value is dowiz-core's own — the same constant
    /// `InMemoryStore` folds over these entries — so this test fails if either the store
    /// format handling or the fold drifts.
    /// A NON-ASCII KEY MUST SURVIVE A ROUND TRIP. It did not: keys were written
    /// as UTF-8 and read back as Latin-1, so an Albanian or Ukrainian id came
    /// back as a different string -- and writing that back damaged it further,
    /// so the same dish became a new product on every menu import.
    #[test]
    fn a_non_ascii_key_survives_the_image() {
        let keys = ["pije-ujë-0-5l", "sushi-sets-durrës-set-24", "страва-суші", "ascii-plain"];
        let mut st = Store::create_bytes(64 * 1024);
        Kv::init_bytes(&mut st).expect("init");
        let mut kv = Kv::load(&st).expect("load");
        for k in keys {
            kv.put(k, b"v");
        }
        let bytes = kv.compacted_bytes_fit(256 * 1024).expect("write");

        let back = Kv::load(&Store::from_bytes(&bytes)).expect("reload");
        for k in keys {
            assert!(
                back.get(k).is_some(),
                "key {k:?} did not survive; the image holds {:?}",
                back.keys()
            );
        }
        // And the damage must not compound: a second round trip is identical.
        let mut again = back;
        let twice = again.compacted_bytes_fit(256 * 1024).expect("rewrite");
        let back2 = Kv::load(&Store::from_bytes(&twice)).expect("reload twice");
        assert_eq!(back2.keys(), again.keys(), "a second round trip changed the keys");
    }

    #[test]
    fn rust_roundtrip_matches_dowiz_root() {
        let path = std::env::temp_dir().join("bebop_store_kv_roundtrip.store");
        let path = path.to_str().unwrap();
        let mut st = Store::create(path, 1 << 20).expect("create");
        Kv::init(&mut st, path).expect("init");

        let st = Store::open(path).expect("open");
        let mut kv = Kv::load(&st).expect("load");
        assert_eq!(kv.entries.len(), 0, "a fresh KV must be empty");
        assert_eq!(kv.snapshot_root_u64(), FNV_OFFSET, "empty root is the FNV offset basis");

        for (k, v) in [
            ("order/0001", "pending"),
            ("order/0002", "confirmed"),
            ("courier/alpha", "idle"),
            ("zone/north", "{\"cap\":12}"),
            ("order/0003", "delivered"),
        ] {
            kv.put(k, v.as_bytes());
        }
        let mut st = Store::open(path).expect("reopen for write");
        kv.commit_into(&mut st, path).expect("commit");

        let st = Store::open(path).expect("reopen");
        let kv = Kv::load(&st).expect("reload");
        assert_eq!(kv.entries.len(), 5);
        assert_eq!(
            kv.keys(),
            vec!["courier/alpha", "order/0001", "order/0002", "order/0003", "zone/north"],
            "keys must come back sorted"
        );
        assert_eq!(kv.get("order/0002").unwrap(), b"confirmed");
        assert_eq!(kv.snapshot_root(), "fd11fc93f180ca47", "dowiz-core's snapshot_root");
        let _ = std::fs::remove_file(path);
    }

    /// Overwriting a key must change the root, and restoring the old value must restore it.
    #[test]
    fn root_is_sensitive_to_every_byte() {
        let path = std::env::temp_dir().join("bebop_store_kv_sensitive.store");
        let path = path.to_str().unwrap();
        let mut st = Store::create(path, 1 << 20).expect("create");
        Kv::init(&mut st, path).expect("init");
        let st = Store::open(path).expect("open");
        let mut kv = Kv::load(&st).expect("load");
        kv.put("a", b"one");
        let before = kv.snapshot_root_u64();
        kv.put("a", b"onf");
        assert_ne!(kv.snapshot_root_u64(), before, "a one-bit value change must move the root");
        kv.put("a", b"one");
        assert_eq!(kv.snapshot_root_u64(), before, "restoring the value restores the root");
        let _ = std::fs::remove_file(path);
    }
}
