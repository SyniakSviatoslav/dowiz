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

/// Byte `b` of a blob, in version `v`.
fn blob_byte(st: &Store, blob: usize, v: i64, b: usize) -> u8 {
    if v >= 2 {
        (st.get(blob, b >> 3) >> (8 * (b & 7))) as u8
    } else {
        st.get(blob, b) as u8
    }
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

    /// Read all entries out of a store, believing every payload cell: the CRC is checked
    /// by `load` / `load_checked` (verify.rs, W-CRC) before this runs, and nothing else calls it.
    ///
    /// EVERY NUMBER IN HERE CAME OUT OF THE IMAGE, and an image arrives over a
    /// network from a Durable Object. The count, the four offsets and the four
    /// lengths are all claims; none of them was checked, and one flipped bit in
    /// a key's length -- 9 with bit 33 set is 8589934601 -- turned the collect
    /// below into `memory allocation of 8589934601 bytes failed`. An allocation
    /// that large does not return an error: the process ABORTS. On a Worker
    /// that is the isolate, for every tenant sharing it, from one corrupt byte.
    ///
    /// So a slice that does not fit the array it names is not a slice, and an
    /// image holding one is not a KV image. `None` here is `HubError::NotAHub`
    /// at the caller -- a refusal it can report, which is the answer a caller
    /// can act on. Truncating to what fits would be the other failure this
    /// crate keeps finding: a shorter image that still looks valid.
    pub(crate) fn decode(st: &Store) -> Option<Kv> {
        let root = st.root()?;
        let ver = Self::version(st);
        if ver > VERSION {
            return None;
        }
        // A blob of c cells holds c bytes in v1 and 8c in v2; every slice below is a
        // BYTE slice and is bounded by this.
        let bytes_of = |cells: usize| -> Option<usize> {
            if ver >= 2 { cells.checked_mul(8) } else { Some(cells) }
        };
        let n = st.get(root, 0);
        let kidx = st.follow(root, 1)?;
        let kblob = st.follow(root, 2)?;
        let vidx = st.follow(root, 3)?;
        let vblob = st.follow(root, 4)?;
        // Each entry owns two cells in each index, so the count is bounded by
        // the index arrays that are really there -- not by the root's word.
        let (kidx_cells, vidx_cells) = (st.obj_cells(kidx), st.obj_cells(vidx));
        let (kblob_cells, vblob_cells) = (bytes_of(st.obj_cells(kblob))?, bytes_of(st.obj_cells(vblob))?);
        if n < 0 {
            return None;
        }
        let n = n as usize;
        if n.checked_mul(2)? > kidx_cells.min(vidx_cells) {
            return None;
        }
        let fits = |off: i64, len: i64, cells: usize| -> Option<usize> {
            if off < 0 || len < 0 {
                return None;
            }
            let end = (off as usize).checked_add(len as usize)?;
            if end > cells {
                return None;
            }
            Some(len as usize)
        };
        let mut entries = Vec::with_capacity(n);
        // EVERY ENTRY OWNS ITS OWN BYTES, so the entries together cannot be
        // larger than the two blobs (W-AUDIT S2, 2026-09-27). Each entry was
        // bounded by its blob; n entries that all name the whole blob read
        // n × blob, and a crafted index made one `load` allocate quadratically
        // in the image. An index that over-claims the blobs is refused.
        let mut budget = kblob_cells.checked_add(vblob_cells)?;
        for i in 0..n {
            let ko = st.get(kidx, 2 * i);
            let kl = st.get(kidx, 2 * i + 1);
            let vo = st.get(vidx, 2 * i);
            let vl = st.get(vidx, 2 * i + 1);
            let kl = fits(ko, kl, kblob_cells)?;
            let vl = fits(vo, vl, vblob_cells)?;
            budget = budget.checked_sub(kl.checked_add(vl)?)?;
            let (ko, vo) = (ko as usize, vo as usize);
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
            let kb: Vec<u8> = (0..kl).map(|j| blob_byte(st, kblob, ver, ko + j)).collect();
            let k: String = String::from_utf8_lossy(&kb).into_owned();
            let v: Vec<u8> = (0..vl).map(|j| blob_byte(st, vblob, ver, vo + j)).collect();
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
        let kidx = st.follow(old_root, 1).ok_or(StoreError::Corrupt("KV root names no key index"))?;
        let arr_dig = st.obj_digest(kidx);
        let root_dig = st.obj_digest(old_root);
        // THE ROOT'S OWN VERSION, never this module's: a v1 image keeps being written as
        // v1 (a mixed image would be unreadable), a fresh one is v2.
        let ver = Self::version(st);
        if ver > VERSION {
            return Err(StoreError::Corrupt("KV root names a version this code does not know"));
        }
        // SUPERSEDED (W-CRC, D.1 #4): this commit rewrites the root and all four arrays, so
        // the old five objects -- header cells included -- are dead from the new generation
        // on. `stage_commit` moves them from `live_cells` to `superseded_cells`.
        let mut dead = 2 + st.obj_cells(old_root) as i64;
        for i in 1..=4 {
            if let Some(a) = st.follow(old_root, i) {
                dead += 2 + st.obj_cells(a) as i64;
            }
        }
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

#[cfg(test)]
mod tests;
