//! Rust side of the bebop store format.
//!
//! Layout (bebop-lang `selfhost/prelude/store.bp`):
//!   cells 0..15    superblock A
//!   cells 512..527 superblock B
//!   cells 1024..   append-only bump arena of objects
//!
//! Superblock (16 cells): 0 magic "BEBOPST1", 1 version, 2 generation, 3 root,
//! 4 arena_used, 5 layout_table, 6 migration_table, 7 live_cells, 8 superseded_cells,
//! 9 reserved, 10 commit-object, 11 heads-table, 12..14 zero, 15 crc32 of cells 0..14.
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

impl Store {
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
