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
