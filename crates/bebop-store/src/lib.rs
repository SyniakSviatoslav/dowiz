//! Rust side of the bebop store format.
//!
//! Layout (bebop-lang `selfhost/prelude/store.bp`):
//!   cells 0..15    superblock A
//!   cells 512..527 superblock B
//!   cells 1024..   append-only bump arena of objects
//!
//! Superblock (16 cells): 0 magic "BEBOPST1", 1 version, 2 generation, 3 root,
//! 4 arena_used, 5 layout_table, 6 migration_table, 7 live_cells, 8 superseded_cells,
//! 9 reserved, 10 commit-object, 11 heads-table, 12 arena capacity in cells,
//! 13..14 zero, 15 crc32 of cells 0..14.
//! A reader picks the VALID superblock with the higher generation.
//!
//! Object: h0 = (layout_digest_lo32 << 32) | length_in_cells,
//!         h1 = (crc32(payload LE bytes) << 32) | generation,
//!         then `length_in_cells` payload cells.
//! A `ref` field is an OBJECT-relative signed cell offset: target = this_object_start + off,
//! 0 = null.
//!
//! CRC is zlib CRC-32 over the raw little-endian bytes of the cells --
//! bebop's `crc32x` builtin documents itself as equal to
//! `zlib.crc32(struct.pack('<%dq' % n, cells))`.

use std::io;

/// Superblock magic, the little-endian i64 spelling of "BEBOPST1".
pub const MAGIC: i64 = 3554557610294396226;
/// Cell index of superblock A.
pub const SB_A: usize = 0;
/// Cell index of superblock B.
pub const SB_B: usize = 512;
/// First cell of the arena.
pub const ARENA: usize = 1024;

/// zlib CRC-32, table-free (bitwise), over raw bytes.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        c ^= b as u32;
        for _ in 0..8 {
            let m = (c & 1).wrapping_neg();
            c = (c >> 1) ^ (0xEDB8_8320 & m);
        }
    }
    !c
}

/// CRC-32 over `n` cells starting at `off`, taken as little-endian bytes.
pub fn crc32_cells(cells: &[i64], off: usize, n: usize) -> u32 {
    let mut buf = Vec::with_capacity(n * 8);
    for i in 0..n {
        buf.extend_from_slice(&cells[off + i].to_le_bytes());
    }
    crc32(&buf)
}

/// A bebop store file loaded into memory as cells.
pub struct Store {
    pub cells: Vec<i64>,
}

/// One superblock's fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Superblock {
    pub at: usize,
    pub generation: i64,
    pub root: i64,
    pub arena_used: i64,
    pub live_cells: i64,
    pub superseded_cells: i64,
}

/// The largest image `from_bytes` will pad a trimmed one back up to: 512 MiB.
/// Far above anything dowiz stores (a hub that has traded for a year is tens of
/// megabytes) and far below what would hurt to allocate by accident.
pub const MAX_PAD_CELLS: usize = (512 << 20) / 8;

impl Store {
    /// Create a FRESH store file of `size_bytes`, initialised exactly as `st_open` does on a
    /// file with no valid superblock: superblock A only, generation 0, root 0, cursor at the
    /// first arena cell, capacity = size/8 - 1024.
    pub fn create(path: &str, size_bytes: usize) -> io::Result<Self> {
        let n = size_bytes / 8;
        let mut st = Store { cells: vec![0i64; n] };
        let capacity = (n - ARENA) as i64;
        st.cells[SB_A] = MAGIC;
        st.cells[SB_A + 1] = 2;
        st.cells[SB_A + 2] = 0;
        st.cells[SB_A + 3] = 0;
        st.cells[SB_A + 4] = ARENA as i64;
        st.cells[SB_A + 12] = capacity;
        st.cells[SB_A + 15] = crc32_cells(&st.cells, SB_A, 15) as i64;
        let mut buf = Vec::with_capacity(size_bytes);
        for c in &st.cells { buf.extend_from_slice(&c.to_le_bytes()); }
        std::fs::write(path, &buf)?;
        Ok(st)
    }

    /// Create a fresh store IN MEMORY, with no filesystem at all.
    ///
    /// The whole point of the pointer-free format is that nothing in it is an
    /// address, so the byte image and the in-memory image are the same thing.
    /// That is what lets the store live somewhere with no `open()` -- an R2
    /// object, a Durable Object's storage, a Worker's heap -- rather than only
    /// on a disk.
    pub fn create_bytes(size_bytes: usize) -> Self {
        let n = size_bytes / 8;
        let mut st = Store { cells: vec![0i64; n] };
        let capacity = (n - ARENA) as i64;
        st.cells[SB_A] = MAGIC;
        st.cells[SB_A + 1] = 2;
        st.cells[SB_A + 2] = 0;
        st.cells[SB_A + 3] = 0;
        st.cells[SB_A + 4] = ARENA as i64;
        st.cells[SB_A + 12] = capacity;
        st.cells[SB_A + 15] = crc32_cells(&st.cells, SB_A, 15) as i64;
        st
    }

    /// Load a store from a byte image. Trailing bytes that do not fill a whole
    /// cell are ignored rather than padded: a truncated image must not silently
    /// become a valid one.
    ///
    /// A TRIMMED image is padded back to capacity; a TRUNCATED one is not, and
    /// the difference is the whole guarantee. `to_bytes_trimmed` drops the tail
    /// of zeros past `arena_used`, which a reader can re-create exactly. An
    /// image cut BELOW `arena_used` is missing objects the superblock still
    /// names, and padding it would hand the caller a store whose arena reads as
    /// zeros where records used to be -- valid-looking and wrong. So the padding
    /// is allowed only up from `arena_used`, never up to it.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let n = bytes.len() / 8;
        let mut cells = Vec::with_capacity(n);
        for i in 0..n {
            let mut w = [0u8; 8];
            w.copy_from_slice(&bytes[i * 8..i * 8 + 8]);
            cells.push(i64::from_le_bytes(w));
        }

        // The live superblock names both the arena's end (cell 4, an ABSOLUTE
        // cell index) and its capacity (cell 12, a count of cells from ARENA).
        // Pick the valid one with the higher generation, the same rule every
        // reader uses.
        let mut best: Option<(i64, i64, i64)> = None; // (generation, arena_used, capacity)
        for at in [SB_A, SB_B] {
            if at + 15 < cells.len()
                && cells[at] == MAGIC
                && crc32_cells(&cells, at, 15) as i64 == cells[at + 15]
            {
                let gen = cells[at + 2];
                if best.is_none_or(|(g, _, _)| gen > g) {
                    best = Some((gen, cells[at + 4], cells[at + 12]));
                }
            }
        }
        if let Some((_, arena_used, capacity)) = best {
            let full = ARENA + capacity.max(0) as usize;
            // EVERY CELL THE ARENA CLAIMS MUST ALREADY BE HERE. Below that the
            // image is truncated, not trimmed, and it is left exactly as it
            // arrived so whatever reads it next fails on the real bytes.
            //
            // AND THE CLAIM IS NOT TRUSTED FOR ITS SIZE. A restore hands this
            // bytes the caller chose: a superblock whose capacity cell says a
            // hundred million cells would have the reader allocate 800 MB for
            // an image that is a few kilobytes long, which on a Worker is the
            // isolate, not an error message. Past the bound the image is left
            // as it arrived, exactly like a truncated one -- padding is a
            // convenience for images we wrote, never an instruction we follow.
            let bound = MAX_PAD_CELLS.max(cells.len());
            if cells.len() >= arena_used.max(0) as usize && cells.len() < full && full <= bound {
                cells.resize(full, 0);
            }
        }

        Store { cells }
    }

    /// The byte image. Little-endian cells, identical to what `create`/`commit`
    /// write to a file -- a store written here opens with `Store::open`, and one
    /// written by `commit` loads with `from_bytes`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.cells.len() * 8);
        for c in &self.cells {
            buf.extend_from_slice(&c.to_le_bytes());
        }
        buf
    }

    /// The image without its unused tail. The arena is a bump allocator, so every
    /// cell past `arena_used` is a zero that the reader re-creates from the
    /// capacity in the superblock. A freshly doubled image is mostly nothing.
    ///
    /// The cut is at `arena_used` (superblock cell 4), which is an ABSOLUTE cell
    /// index rather than a count from `ARENA` -- `create_bytes` seeds it with
    /// `ARENA` itself and every commit writes `tx.cursor` into it. Reopen with
    /// `Store::from_bytes`, which pads the zeros back.
    ///
    /// Without a readable superblock nothing may be dropped: a store that cannot
    /// say where its arena ends is handed back whole.
    pub fn to_bytes_trimmed(&self) -> Vec<u8> {
        let trim_at = match self.pick() {
            Some(sb) => sb.arena_used.max(ARENA as i64),
            None => self.cells.len() as i64,
        };
        let trim_len = (trim_at as usize).min(self.cells.len());
        let mut buf = Vec::with_capacity(trim_len * 8);
        for i in 0..trim_len {
            buf.extend_from_slice(&self.cells[i].to_le_bytes());
        }
        buf
    }

    /// Commit with no filesystem: stage the new PartTab and superblock, and hand
    /// the caller the new generation. The caller persists `to_bytes()`.
    ///
    /// The on-disk `commit` orders three fsync'd writes -- objects, PartTab,
    /// superblock LAST -- so a crash between any two leaves generation k-1
    /// intact. A whole-image PUT to object storage gives the same property by a
    /// shorter route: the old image stays readable until the new one lands, and
    /// there is no partial state in between. Stated rather than assumed, because
    /// the guarantee is the reason the ordering exists.
    pub fn commit_bytes(&mut self, tx: &Tx, root: usize) -> i64 {
        let _ = self.stage_commit(tx, root);
        tx.next_gen
    }

    /// Read a store file from disk.
    pub fn open(path: &str) -> io::Result<Self> {
        let bytes = std::fs::read(path)?;
        let n = bytes.len() / 8;
        let mut cells = Vec::with_capacity(n);
        for i in 0..n {
            let mut w = [0u8; 8];
            w.copy_from_slice(&bytes[i * 8..i * 8 + 8]);
            cells.push(i64::from_le_bytes(w));
        }
        Ok(Store { cells })
    }

    /// Is the superblock at `at` valid? Magic must match and cell 15 must be the
    /// CRC-32 of cells 0..14.
    pub fn sb_valid(&self, at: usize) -> bool {
        if at + 16 > self.cells.len() || self.cells[at] != MAGIC {
            return false;
        }
        let want = self.cells[at + 15] as u32;
        crc32_cells(&self.cells, at, 15) == want
    }

    /// Pick the valid superblock with the higher generation, exactly as a bebop reader does.
    /// How many arena cells this image was created with.
    ///
    /// READ FROM THE SUPERBLOCK, not derived from the image's length, because
    /// the two can legitimately differ: a reader given a longer buffer than the
    /// store was built for must not conclude it has room the allocator does not
    /// know about.
    pub fn capacity_cells(&self) -> i64 {
        let Some(sb) = self.pick() else { return 0 };
        self.cells.get(sb.at + 12).copied().unwrap_or(0)
    }

    pub fn pick(&self) -> Option<Superblock> {
        let mut best: Option<Superblock> = None;
        for at in [SB_A, SB_B] {
            if !self.sb_valid(at) {
                continue;
            }
            let sb = Superblock {
                at,
                generation: self.cells[at + 2],
                root: self.cells[at + 3],
                arena_used: self.cells[at + 4],
                live_cells: self.cells[at + 7],
                superseded_cells: self.cells[at + 8],
            };
            if best.map_or(true, |b| sb.generation > b.generation) {
                best = Some(sb);
            }
        }
        best
    }

    /// Length in cells of the object starting at `obj`.
    pub fn obj_len(&self, obj: usize) -> i64 {
        self.cells[obj] & 0xFFFF_FFFF
    }

    /// Layout digest (low 32 bits) of the object at `obj`.
    pub fn obj_digest(&self, obj: usize) -> i64 {
        (self.cells[obj] >> 32) & 0xFFFF_FFFF
    }

    /// Generation the object at `obj` was written in.
    pub fn obj_generation(&self, obj: usize) -> i64 {
        self.cells[obj + 1] & 0xFFFF_FFFF
    }

    /// Does the object's stored CRC match its payload?
    pub fn obj_crc_ok(&self, obj: usize) -> bool {
        let len = self.obj_len(obj) as usize;
        let want = ((self.cells[obj + 1] >> 32) & 0xFFFF_FFFF) as u32;
        crc32_cells(&self.cells, obj + 2, len) == want
    }

    /// Payload cell `i` of the object at `obj`.
    pub fn get(&self, obj: usize, i: usize) -> i64 {
        self.cells[obj + 2 + i]
    }

    /// The DATA root, resolved the way `st_root` does: the superblock's cell 3 names a
    /// PartTab object (B5 step 1), and the partition's root is the raw cell `pt + 18`
    /// -- i.e. PartTab payload cell 16, the first of the `[root_p, used_p, gen_p]` entry.
    pub fn root(&self) -> Option<usize> {
        let sb = self.pick()?;
        let pt = self.cells[sb.at + 3];
        if pt == 0 { return None; }
        let r = self.cells[pt as usize + 18];
        if r == 0 { None } else { Some(r as usize) }
    }

    /// The PartTab offset the live superblock names.
    pub fn parttab(&self) -> Option<usize> {
        let sb = self.pick()?;
        let pt = self.cells[sb.at + 3];
        if pt == 0 { None } else { Some(pt as usize) }
    }

    /// Follow the object-relative ref in payload cell `i`. Returns None for a null (0) ref.
    pub fn follow(&self, obj: usize, i: usize) -> Option<usize> {
        let off = self.get(obj, i);
        if off == 0 {
            None
        } else {
            Some((obj as i64 + off) as usize)
        }
    }
}

// ---------------------------------------------------------------------------
// Write path.
//
// The commit protocol, mirrored from `st_begin` / `st_alloc` / `st_seal` /
// `st_commit_d_m`:
//   1. pick the live superblock; the cursor and generation come from the PartTab it names
//   2. bump-allocate objects from that cursor, writing h0 then h1, sealing each with its CRC
//   3. write a NEW PartTab into the OTHER superblock's page free tail, at (512 - sb) + 16 --
//      NOT into the arena, so consecutive generations' PartTabs land on different pages and a
//      single page tear cannot destroy both
//   4. write the OTHER superblock last; that write IS the commit point
//
// Writes go to disk in that same order, so a crash between any two of them leaves the
// previous generation's superblock intact and the store readable at gen k-1.
// ---------------------------------------------------------------------------

use std::io::{Seek, SeekFrom, Write};

/// An open write transaction.
#[derive(Debug, Clone, Copy)]
pub struct Tx {
    pub sb: usize,
    pub mark: i64,
    pub cursor: i64,
    pub live_delta: i64,
    pub sup_delta: i64,
    pub next_gen: i64,
}

/// Number of cells in a P=1 PartTab: 16 + 3*1.
pub const PARTTAB_CELLS: i64 = 19;

#[derive(Debug)]
pub enum StoreError {
    NoSuperblock,
    ArenaFull { need: i64, capacity: i64 },
    Io(io::Error),
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self { StoreError::Io(e) }
}

impl Store {
    /// Open a transaction: cursor and generation come from the live PartTab, exactly as
    /// `st_begin` reads `used_p` and `gen_p` rather than the superblock's own cells.
    pub fn begin(&self) -> Result<Tx, StoreError> {
        let sb = self.pick().ok_or(StoreError::NoSuperblock)?;
        let pt = self.cells[sb.at + 3];
        let (used, gen) = if pt == 0 {
            (self.cells[sb.at + 4], self.cells[sb.at + 2])
        } else {
            (self.cells[pt as usize + 19], self.cells[pt as usize + 20])
        };
        Ok(Tx { sb: sb.at, mark: used, cursor: used, live_delta: 0, sup_delta: 0, next_gen: gen + 1 })
    }

    /// Bump-allocate an object of `len` payload cells. Writes h0 and h1; the CRC half of h1
    /// stays zero until `seal`.
    pub fn alloc(&mut self, tx: &mut Tx, len: i64, digest: i64) -> Result<usize, StoreError> {
        let off = tx.cursor;
        let capacity = self.cells[tx.sb + 12];
        if off + 2 + len > ARENA as i64 + capacity {
            return Err(StoreError::ArenaFull { need: off + 2 + len, capacity });
        }
        let o = off as usize;
        if o + 2 + len as usize > self.cells.len() {
            self.cells.resize(o + 2 + len as usize, 0);
        }
        self.cells[o] = ((digest & 0xFFFF_FFFF) << 32) | len;
        self.cells[o + 1] = tx.next_gen;
        tx.cursor = off + 2 + len;
        tx.live_delta += 2 + len;
        Ok(o)
    }

    /// Set payload cell `i` of `obj`.
    pub fn put_cell(&mut self, obj: usize, i: usize, v: i64) {
        self.cells[obj + 2 + i] = v;
    }

    /// Write an object-relative ref into payload cell `i`, as `st_link` does.
    pub fn link(&mut self, obj: usize, i: usize, target: usize) {
        self.cells[obj + 2 + i] = target as i64 - obj as i64;
    }

    /// Seal an object: CRC-32 of its payload into the high half of h1.
    pub fn seal(&mut self, obj: usize) {
        let len = self.obj_len(obj) as usize;
        let crc = crc32_cells(&self.cells, obj + 2, len) as i64;
        self.cells[obj + 1] = (crc << 32) | (self.cells[obj + 1] & 0xFFFF_FFFF);
    }

    /// Stage a commit in memory: write the new PartTab and the other superblock.
    /// Returns (parttab_offset, other_superblock_offset).
    pub fn stage_commit(&mut self, tx: &Tx, root: usize) -> (usize, usize) {
        let sb = tx.sb;
        let live = self.cells[sb + 7] + tx.live_delta - tx.sup_delta;
        let sup = self.cells[sb + 8] + tx.sup_delta;
        let mig = self.cells[sb + 6];
        let capacity = self.cells[sb + 12];
        // the PartTab digest is reused from the live PartTab rather than recomputed, so the
        // Rust side never needs sha256; schema creation stays bebop's job.
        let old_pt = self.cells[sb + 3] as usize;
        let ptdig = if old_pt != 0 { self.obj_digest(old_pt) } else { 0 };

        let pt = (SB_B - sb) + 16;
        self.cells[pt] = ((ptdig & 0xFFFF_FFFF) << 32) | PARTTAB_CELLS;
        self.cells[pt + 1] = tx.next_gen;
        for i in 0..16 {
            self.cells[pt + 2 + i] = self.cells[sb + i];
        }
        self.cells[pt + 18] = root as i64;
        self.cells[pt + 19] = tx.cursor;
        self.cells[pt + 20] = tx.next_gen;
        // payload cell 15 carries a CRC over all 19 payload cells, written after the fold --
        // the same shape st_parttab_write uses.
        self.cells[pt + 2 + 15] = crc32_cells(&self.cells, pt + 2, PARTTAB_CELLS as usize) as i64;
        self.seal(pt);

        let osb = SB_B - sb;
        self.cells[osb] = MAGIC;
        self.cells[osb + 1] = 2;
        self.cells[osb + 2] = tx.next_gen;
        self.cells[osb + 3] = pt as i64;
        self.cells[osb + 4] = tx.cursor;
        self.cells[osb + 5] = 0;
        self.cells[osb + 6] = mig;
        self.cells[osb + 7] = live;
        self.cells[osb + 8] = sup;
        for k in 9..12 { self.cells[osb + k] = 0; }
        self.cells[osb + 12] = capacity;
        self.cells[osb + 13] = 0;
        self.cells[osb + 14] = 0;
        self.cells[osb + 15] = crc32_cells(&self.cells, osb, 15) as i64;
        (pt, osb)
    }

    /// Commit to disk in protocol order: new objects, then the PartTab page, then the
    /// superblock LAST. A crash between any two leaves generation k-1 intact.
    pub fn commit(&mut self, tx: &Tx, root: usize, path: &str) -> Result<i64, StoreError> {
        let (pt, osb) = self.stage_commit(tx, root);
        let mut f = std::fs::OpenOptions::new().write(true).open(path)?;
        self.write_cells(&mut f, tx.mark as usize, (tx.cursor - tx.mark) as usize)?;
        f.sync_data()?;
        self.write_cells(&mut f, pt, PARTTAB_CELLS as usize + 2)?;
        f.sync_data()?;
        self.write_cells(&mut f, osb, 16)?;
        f.sync_data()?;
        Ok(tx.next_gen)
    }

    fn write_cells(&self, f: &mut std::fs::File, off: usize, n: usize) -> io::Result<()> {
        if n == 0 { return Ok(()); }
        let mut buf = Vec::with_capacity(n * 8);
        for i in 0..n {
            buf.extend_from_slice(&self.cells[off + i].to_le_bytes());
        }
        f.seek(SeekFrom::Start((off * 8) as u64))?;
        f.write_all(&buf)
    }
}
pub mod kv;
pub mod evlog;

#[cfg(test)]
mod bytes_tests {
    use super::*;

    /// A fresh store built with no filesystem must be byte-identical to one
    /// `create()` writes. If these ever diverge, the format has two definitions.
    #[test]
    fn create_bytes_matches_create_on_disk() {
        let p = std::env::temp_dir().join("bebop_bytes_create.store");
        let p = p.to_str().unwrap();
        let on_disk = Store::create(p, 1 << 20).unwrap();
        let in_mem = Store::create_bytes(1 << 20);
        assert_eq!(on_disk.cells, in_mem.cells, "same format, two constructors");
        assert_eq!(std::fs::read(p).unwrap(), in_mem.to_bytes(), "byte image matches the file");
        let _ = std::fs::remove_file(p);
    }

    /// A store committed WITHOUT a filesystem, written out as bytes, must open
    /// with the ordinary reader — that is what lets the same store live in an R2
    /// object on a Worker and in a file on a hub.
    #[test]
    fn commit_bytes_is_readable_by_the_file_reader() {
        let mut st = Store::create_bytes(1 << 20);
        let mut tx = st.begin().unwrap();
        let obj = st.alloc(&mut tx, 4, 0x1234).unwrap();
        for i in 0..4 {
            st.put_cell(obj, i, (i as i64 + 1) * 11);
        }
        st.seal(obj);
        let gen = st.commit_bytes(&tx, obj);
        assert_eq!(gen, 1, "first commit is generation 1");

        let p = std::env::temp_dir().join("bebop_bytes_commit.store");
        let p = p.to_str().unwrap();
        std::fs::write(p, st.to_bytes()).unwrap();

        let reopened = Store::open(p).unwrap();
        let root = reopened.root().expect("the committed root must resolve");
        assert_eq!(reopened.obj_len(root), 4);
        assert!(reopened.obj_crc_ok(root), "payload CRC survives the byte trip");
        for i in 0..4 {
            assert_eq!(reopened.get(root, i), (i as i64 + 1) * 11);
        }
        let _ = std::fs::remove_file(p);
    }

    /// from_bytes must not invent cells out of a truncated image.
    #[test]
    fn from_bytes_ignores_a_partial_trailing_cell() {
        let st = Store::create_bytes(1 << 16);
        let mut b = st.to_bytes();
        b.extend_from_slice(&[0xAB, 0xCD, 0xEF]); // three stray bytes, not a cell
        let back = Store::from_bytes(&b);
        assert_eq!(back.cells.len(), st.cells.len(), "a partial cell is dropped, not padded");
        assert_eq!(back.cells, st.cells);
    }

    /// The cut lands exactly where the superblock says the arena ends: one cell
    /// more would keep a zero nobody needs, one cell less would drop a record.
    #[test]
    fn the_trim_lands_on_the_arena_cursor() {
        let mut st = Store::create_bytes(4 << 20);
        let mut tx = st.begin().unwrap();
        let root = st.alloc(&mut tx, 3, 0x1111).unwrap();
        st.seal(root);
        st.commit_bytes(&tx, root);
        let used = st.pick().unwrap().arena_used;
        assert_eq!(st.to_bytes_trimmed().len(), used as usize * 8);
        assert!(used > ARENA as i64, "the arena moved past its base");
    }

    /// A TRUNCATED image is not a trimmed one and must not be padded into
    /// looking valid: the cells the superblock claims are simply not there.
    #[test]
    fn a_truncated_image_is_left_as_it_arrived() {
        let mut st = Store::create_bytes(1 << 20);
        let mut tx = st.begin().unwrap();
        let root = st.alloc(&mut tx, 8, 0x2222).unwrap();
        for i in 0..8 { st.put_cell(root, i, 7); }
        st.seal(root);
        st.commit_bytes(&tx, root);

        let trimmed = st.to_bytes_trimmed();
        assert_eq!(Store::from_bytes(&trimmed).cells.len(), st.cells.len(), "trimmed pads back");

        // Cut one cell BELOW the arena cursor: an object the superblock names
        // is now missing, so nothing may be invented in its place.
        let cut = &trimmed[..trimmed.len() - 8];
        let back = Store::from_bytes(cut);
        assert_eq!(back.cells.len(), cut.len() / 8, "a truncated image is not padded");
        assert!(back.cells.len() < st.cells.len());
    }

    /// A trimmed image reopens identical to the full image.
    #[test]
    fn a_trimmed_image_reopens_identical() {
        let mut st = Store::create_bytes(4 << 20);
        let mut tx = st.begin().unwrap();

        // Write a few objects to move the cursor forward
        for i in 0..5 {
            let obj = st.alloc(&mut tx, 4, 0x1234 + i).unwrap();
            for j in 0..4 {
                st.put_cell(obj, j, (i as i64 + 1) * 11 + j as i64);
            }
            st.seal(obj);
        }

        let root = st.alloc(&mut tx, 2, 0x5678).unwrap();
        st.put_cell(root, 0, 42);
        st.put_cell(root, 1, 99);
        st.seal(root);

        st.commit_bytes(&tx, root);

        // Get both the full and trimmed images
        let full = st.to_bytes();
        let trimmed = st.to_bytes_trimmed();

        // Trimmed must be smaller
        assert!(trimmed.len() < full.len(), "trimmed image must be smaller");

        // Reopen both
        let st_full = Store::from_bytes(&full);
        let st_trimmed = Store::from_bytes(&trimmed);

        // They must produce the same root
        let root_full = st_full.root();
        let root_trimmed = st_trimmed.root();
        assert_eq!(root_full, root_trimmed, "root must resolve the same");

        // The payload cells must be identical
        if let (Some(r_full), Some(r_trimmed)) = (root_full, root_trimmed) {
            for i in 0..2 {
                assert_eq!(st_full.get(r_full, i), st_trimmed.get(r_trimmed, i),
                          "payload cell {} must match", i);
            }
        }
    }

    /// A superblock that claims an absurd capacity is not obeyed. The image is
    /// left as it arrived: the alternative is a restore choosing how much
    /// memory the reader allocates.
    #[test]
    fn an_absurd_capacity_is_not_padded_to() {
        let mut st = Store::create_bytes(1 << 20);
        let mut tx = st.begin().unwrap();
        let root = st.alloc(&mut tx, 2, 0x3333).unwrap();
        st.seal(root);
        st.commit_bytes(&tx, root);

        let mut cells = st.cells.clone();
        let at = st.pick().unwrap().at;
        cells[at + 12] = (MAX_PAD_CELLS as i64) * 4; // a capacity nothing could hold
        cells[at + 15] = crc32_cells(&cells, at, 15) as i64; // and a CRC that agrees
        let mut bytes = Vec::new();
        for c in &cells[..st.pick().unwrap().arena_used as usize] {
            bytes.extend_from_slice(&c.to_le_bytes());
        }

        let back = Store::from_bytes(&bytes);
        assert_eq!(back.cells.len(), bytes.len() / 8, "no padding on an absurd claim");
    }

    /// A trimmed image is much smaller than the full one.
    #[test]
    fn a_trimmed_image_is_much_smaller() {
        let mut st = Store::create_bytes(4 << 20);
        let mut tx = st.begin().unwrap();

        // Write a small object to a huge store
        let obj = st.alloc(&mut tx, 4, 0x1234).unwrap();
        for j in 0..4 {
            st.put_cell(obj, j, 42 + j as i64);
        }
        st.seal(obj);

        st.commit_bytes(&tx, obj);

        let full = st.to_bytes();
        let trimmed = st.to_bytes_trimmed();

        // The trimmed image should be much smaller -- much less than 1/20th the size
        // since we only used a handful of cells in a 4MiB store
        println!("Full: {} bytes, Trimmed: {} bytes", full.len(), trimmed.len());
        assert!(
            (trimmed.len() * 20) < full.len(),
            "trimmed ({}) should be < 1/20th of full ({})",
            trimmed.len(),
            full.len()
        );
    }
}
