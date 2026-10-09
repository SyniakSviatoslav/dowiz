//! The KV layout `bebop-lang/selfhost/std/kv.bp` creates, shaped to dowiz's `MemoryStore`.
//!
//! ── TWO VERSIONS, AND THE ROOT SAYS WHICH (evlog.rs's rule, DG3 2026-09-28) ──
//!
//! v1 root, 5 cells `KV{n, ref KIDX, ref KBLOB, ref VIDX, ref VBLOB}`:
//!   KIDX  2n cells, (offset, len) per key into KBLOB, keys SORTED
//!   KBLOB key bytes, ONE PER CELL -- eight bytes of arena for one byte of key
//!   VIDX  2n cells, (offset, len) per value into VBLOB
//!   VBLOB value bytes, one per cell
//!
//! v2 root, 6 cells `{n, ref KIDX, ref KBLOB, ref VIDX, ref VBLOB, VERSION=2}`, same digest:
//!   KIDX/VIDX  (BYTE offset, BYTE len) per entry
//!   KBLOB/VBLOB the bytes PACKED EIGHT TO A CELL, little-endian, `ceil(bytes/8)` cells
//!              (at least one), the last cell zero-filled
//!
//! WHY v2: the catalogue's shape (165 entries x 3.2 KB) was a 4,256,224-byte image for
//! ~530 KB of content (docs/research/2026-09-28-bebop-dag.md §4). The fold does not
//! change: `snapshot_root` is over BYTES, never over cells, so a v1 and a v2 image of the
//! same entries give the same root.
//!
//! HOW THE TWO LIVE TOGETHER: a root with no version cell (or a zero there) is v1, which
//! is every image written before DG3. `commit_into` writes the version the root already
//! has, so an old image stays v1; every FRESH image (`init`, `compacted_bytes`) is v2, so a
//! hub's image becomes v2 the next time it is compacted. A version this code does not know
//! is refused, not guessed at.
//!
//! v3 (W-DELTA, 2026-10-06) is a v2 base plus a chain of delta records, 8-cell root
//! `{.. v2 .., VERSION=3, ref DELTA, D}`: a write appends instead of rewriting. Only
//! `delta::append_delta` writes it; compaction writes v2. See `kv/delta.rs`.
//!
//! bebop creates the schema (the layout digests come from sha256, which stays on that side);
//! this module reads and writes the data through the documented pointer-free format.

use crate::{Store, StoreError};

/// FNV-1a 64-bit offset basis — the same constant dowiz-core uses.
pub const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
pub const FNV_PRIME: u64 = 0x100_0000_01b3;

/// FNV-1a, one byte per step. W-KVDEC tried eight bytes per step (one cell's worth) and
/// MEASURED no gain: release 721.5 us vs 721.7 us over the 548 KB catalogue, and 3.5x
/// SLOWER in debug (14,370 vs 4,077 us). FNV-1a is a serial xor-multiply chain, so the
/// multiply's latency is the floor and loop overhead was never the cost. The root is
/// stored, so any rewrite must stay bit-identical (kv/decode/tests.rs pins it).
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

/// The version a FRESH image is created as. A store says its own version in its root.
pub const VERSION: i64 = 2;
/// Cells in a v1 root and in a v2 one; the extra cell is the version itself.
const ROOT_V1: usize = 5;
const ROOT_V2: usize = 6;

/// Cells a blob of `bytes` bytes takes in version `v`: one per byte in v1, eight bytes
/// to a cell in v2. Never zero: an empty blob is still one (zero) cell, as it always was.
fn blob_cells(v: i64, bytes: usize) -> usize {
    let c = if v >= 2 { bytes.div_ceil(8) } else { bytes };
    c.max(1)
}

/// Write a whole blob -- every cell of it, including v2's zero-filled tail and the one
/// zero cell of an empty blob. `alloc` does not zero what it hands out, so a cell this
/// does not write is whatever an aborted transaction left there.
fn blob_write(st: &mut Store, blob: usize, v: i64, bytes: &[u8]) {
    if v >= 2 {
        for (c, chunk) in bytes.chunks(8).enumerate() {
            let mut w = [0u8; 8];
            w[..chunk.len()].copy_from_slice(chunk);
            st.put_cell(blob, c, i64::from_le_bytes(w));
        }
    } else {
        for (j, &b) in bytes.iter().enumerate() {
            st.put_cell(blob, j, b as i64);
        }
    }
    if bytes.is_empty() {
        st.put_cell(blob, 0, 0);
    }
}

impl Kv {
    /// Create the empty KV schema in a fresh store -- the same four zero-length arrays and
    /// root that `kv.bp`'s init phase writes.
    fn stage_init(st: &mut Store) -> Result<(crate::Tx, usize), StoreError> {
        Self::stage_init_v(st, VERSION)
    }

    /// Stage the empty schema in version `v`. v1 exists for the tests that prove an old
    /// image still reads and still writes as v1, and for nothing else.
    fn stage_init_v(st: &mut Store, v: i64) -> Result<(crate::Tx, usize), StoreError> {
        let mut tx = st.begin()?;
        let kidx = st.alloc(&mut tx, 1, DIGEST_ARR_I64)?;
        let kblob = st.alloc(&mut tx, 1, DIGEST_ARR_I64)?;
        let vidx = st.alloc(&mut tx, 1, DIGEST_ARR_I64)?;
        let vblob = st.alloc(&mut tx, 1, DIGEST_ARR_I64)?;
        st.seal(kidx); st.seal(kblob); st.seal(vidx); st.seal(vblob);
        let root = st.alloc(&mut tx, if v >= 2 { ROOT_V2 } else { ROOT_V1 } as i64, DIGEST_KV_ROOT)?;
        st.put_cell(root, 0, 0);
        st.link(root, 1, kidx);
        st.link(root, 2, kblob);
        st.link(root, 3, vidx);
        st.link(root, 4, vblob);
        if v >= 2 {
            st.put_cell(root, 5, v);
        }
        st.seal(root);
        Ok((tx, root))
    }

    /// Which version this store's KV is written in. READ OFF THE ROOT: a root of six or
    /// more cells whose cell 5 is positive says its version; anything else is v1, which is
    /// exactly what every image written before DG3 looks like.
    pub fn version(st: &Store) -> i64 {
        let Some(root) = st.root() else { return VERSION };
        if st.obj_cells(root) >= ROOT_V2 {
            let v = st.get(root, 5);
            if v > 0 {
                return v;
            }
        }
        1
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

    /// Fetch a value by key: BINARY SEARCH (W-ZC), since every writer keeps the keys
    /// sorted. A miss falls back to the old linear scan, so an image whose keys are not
    /// sorted (none is written here; `zc::KvIn` checks) still answers what it holds.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        match self.entries.binary_search_by(|(k, _)| k.as_str().cmp(key)) {
            Ok(i) => Some(self.entries[i].1.clone()),
            Err(_) => self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()),
        }
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
        let kidx = st.follow(old_root, 1).ok_or(StoreError::Corrupt("KV root names no key index"))?;
        let arr_dig = st.obj_digest(kidx);
        let root_dig = st.obj_digest(old_root);
        // THE ROOT'S OWN VERSION, never this module's: a v1 image keeps being written as
        // v1 (a mixed image would be unreadable), a fresh one is v2.
        let ver = Self::version(st);
        if ver > delta::VERSION_DELTA {
            return Err(StoreError::Corrupt("KV root names a version this code does not know"));
        }
        // A v3 image rewritten whole is a v2 image (W-DELTA): the rewrite holds every entry,
        // so its chain is dead with the rest of the old generation.
        let chain = delta::chain_in(st, old_root).ok_or(StoreError::Corrupt("the KV delta chain does not hold"))?;
        let ver = ver.min(VERSION);
        // SUPERSEDED (W-CRC, D.1 #4): this commit rewrites the root and all four arrays, so
        // the old five objects -- header cells included -- are dead from the new generation
        // on. `stage_commit` moves them from `live_cells` to `superseded_cells`.
        let mut dead = 2 + st.obj_cells(old_root) as i64;
        for i in 1..=4 {
            if let Some(a) = st.follow(old_root, i) {
                dead += 2 + st.obj_cells(a) as i64;
            }
        }
        dead += chain.iter().map(|r| 2 + st.obj_cells(r.obj) as i64).sum::<i64>();
        let (mut tx, root) = self.stage_write(st, ver, arr_dig, root_dig)?;
        tx.sup_delta += dead;
        Ok((tx, root))
    }

    /// Write the entries as a new root and four new arrays, superseding nothing.
    fn stage_write(&self, st: &mut Store, ver: i64, arr_dig: i64, root_dig: i64) -> Result<(crate::Tx, usize), StoreError> {
        let n = self.entries.len();
        let kbytes: usize = self.entries.iter().map(|(k, _)| k.len()).sum();
        let vbytes: usize = self.entries.iter().map(|(_, v)| v.len()).sum();

        let mut tx = st.begin()?;
        let kidx = st.alloc(&mut tx, (2 * n).max(1) as i64, arr_dig)?;
        let kblob = st.alloc(&mut tx, blob_cells(ver, kbytes) as i64, arr_dig)?;
        let vidx = st.alloc(&mut tx, (2 * n).max(1) as i64, arr_dig)?;
        let vblob = st.alloc(&mut tx, blob_cells(ver, vbytes) as i64, arr_dig)?;

        let (mut kall, mut vall) = (Vec::with_capacity(kbytes), Vec::with_capacity(vbytes));
        for (i, (k, v)) in self.entries.iter().enumerate() {
            st.put_cell(kidx, 2 * i, kall.len() as i64);
            st.put_cell(kidx, 2 * i + 1, k.len() as i64);
            kall.extend_from_slice(k.as_bytes());
            st.put_cell(vidx, 2 * i, vall.len() as i64);
            st.put_cell(vidx, 2 * i + 1, v.len() as i64);
            vall.extend_from_slice(v);
        }
        if n == 0 {
            st.put_cell(kidx, 0, 0);
            st.put_cell(vidx, 0, 0);
        }
        blob_write(st, kblob, ver, &kall);
        blob_write(st, vblob, ver, &vall);
        st.seal(kidx); st.seal(kblob); st.seal(vidx); st.seal(vblob);

        let root = st.alloc(&mut tx, if ver >= 2 { ROOT_V2 } else { ROOT_V1 } as i64, root_dig)?;
        st.put_cell(root, 0, n as i64);
        st.link(root, 1, kidx);
        st.link(root, 2, kblob);
        st.link(root, 3, vidx);
        st.link(root, 4, vblob);
        if ver >= 2 {
            st.put_cell(root, 5, ver);
        }
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
        let mut fresh = Store::create_bytes(capacity)?;
        // STRAIGHT INTO THE EMPTY STORE, no `init_bytes` first (W-CRC): the empty schema
        // that init wrote is superseded by the very next commit -- 20 cells dead in every
        // compacted image, and, now that superseded cells are counted, a dead figure that
        // could never return to 0 after a compaction. Same
        // digests and version `init` uses, so the image reads exactly as before.
        let (tx, root) = self.stage_write(&mut fresh, VERSION, DIGEST_ARR_I64, DIGEST_KV_ROOT)?;
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
}

/// The zero-copy reader (W-ZC): one value straight from the image cells.
pub mod zc;
/// The delta chain (W-DELTA): a write appends one record per key and a new root.
pub mod delta;
/// `decode`'s blob copy, a cell at a time (W-KVDEC).
mod decode;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod golden_tests;
