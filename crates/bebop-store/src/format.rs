//! THE FORMAT'S NAMES AND LIMITS (W-ES-P0, 2026-10-27): every cell index, offset and bound
//! the readers and the writer use, named once, with the rule that picks the live superblock
//! and says WHY it refused. The normative text is `docs/FORMAT-BEBOP-IMAGE-v2.md`; every
//! constant below is re-derived there and by `format/tests.rs` (T-ES-08), and the document's
//! constant table is read back by a test, so the two cannot drift apart silently.

use crate::{crc32_in, Cells, Store, Superblock, ARENA, MAGIC, SB_A, SB_B};

/// The container version every writer in the tree writes (superblock cell 1).
pub const FORMAT_VERSION: i64 = 2;
/// Cells in one superblock.
pub const SB_CELLS: usize = 16;

// Superblock cells, relative to the superblock's first cell.
/// Cell 1: the container version.
pub const SB_VERSION: usize = 1;
/// Cell 2: the generation.
pub const SB_GEN: usize = 2;
/// Cell 3: the PartTab's cell index (absolute), 0 = an empty store with no root.
pub const SB_PARTTAB: usize = 3;
/// Cell 4: `arena_used`, the ABSOLUTE index of the first cell past the arena's last object.
pub const SB_USED: usize = 4;
/// Cell 5: the projection table (DG5), 0 = none.
pub const SB_PROJTAB: usize = 5;
/// Cell 6: the migration table (bebop T114), 0 = none; carried forward unchanged.
pub const SB_MIGRATION: usize = 6;
/// Cell 7: live cells (object cells reachable from this generation, header cells included).
pub const SB_LIVE: usize = 7;
/// Cell 8: superseded cells (dead since the last compaction).
pub const SB_SUPERSEDED: usize = 8;
/// Cell 9: the commit anchor `(mark << 32) | crc32(cells[mark..arena_used))`, 0 = none.
pub const SB_ANC: usize = 9;
/// Cell 10: bebop C3's commit-object ref (`st_sb_write_ch`); the Rust writer writes 0.
pub const SB_COMMIT: usize = 10;
/// Cell 11: bebop C3's heads table ref; the Rust writer writes 0.
pub const SB_HEADS: usize = 11;
/// Cell 12: the arena's capacity, in cells counted from `ARENA`.
pub const SB_CAPACITY: usize = 12;
/// Cell 15: crc32 of cells 0..14 (cells 13 and 14 must be zero, S-1).
pub const SB_CRC: usize = 15;

// PartTab (P = 1), relative to the PartTab object's first header cell.
/// Where a commit writes its PartTab: this many cells into the OTHER superblock's page.
pub const PT_PAGE_OFFSET: usize = SB_CELLS;
/// The partition's data root (payload cell 16).
pub const PT_ROOT: usize = 2 + SB_CELLS;
/// The partition's write cursor, `used_p` (payload cell 17).
pub const PT_CURSOR: usize = PT_ROOT + 1;
/// The partition's generation, `gen_p` (payload cell 18).
pub const PT_GEN: usize = PT_ROOT + 2;
/// Cells a P=1 PartTab object occupies, header included: 2 + 16 + 3.
pub const PARTTAB_OBJ_CELLS: usize = 2 + crate::PARTTAB_CELLS as usize;

// Limits (FORMAT §10). Each is a refusal at the bound, never a silent wrap.
/// An object's length field is the low 32 bits of `h0`.
pub const MAX_OBJ_CELLS: i64 = u32::MAX as i64;
/// An object's generation is the low 32 bits of `h1` (D7): past this, `proj`'s
/// `input_gen` comparison would wrap, so `begin` refuses the commit that would cross it.
pub const MAX_GENERATION: i64 = u32::MAX as i64;
/// The anchor's mark field is 32 bits; a commit starting past it writes `anc` = 0 (no anchor).
pub const MAX_ANCHOR_MARK: i64 = u32::MAX as i64;
/// The longest key a KV writer accepts, in bytes.
pub const MAX_KEY_BYTES: usize = 65_535;
/// The longest value a KV writer accepts, in bytes.
pub const MAX_VALUE_BYTES: usize = 1 << 30;

/// Why no superblock was picked. Every variant names the cell it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickError {
    /// Neither superblock has the magic and a matching cell-15 crc.
    NoValidSuperblock,
    /// The live superblock's `arena_used` (cell 4) is negative or past the image's end:
    /// a TRUNCATED image, never a reason to read the older generation.
    ArenaUsedOutside { at: usize, used: i64, have: usize },
    /// Cell 3 names a PartTab whose header or partition entry is outside the image.
    PartTabOutside { at: usize, pt: i64 },
    /// Cell 3 names a PartTab that is not at its home cell `at + PT_PAGE_OFFSET` (W-F1): the
    /// next commit from this superblock writes ITS PartTab at the other page's home, so a
    /// live PartTab anywhere else could be overwritten before the commit point, and a crash
    /// there would lose generation k instead of leaving it readable (FORMAT §3.1, §8).
    PartTabMisplaced { at: usize, pt: i64 },
    /// The PartTab's own crc (h1 high half) does not match its payload (D8, AC-ES-03), or it
    /// holds fewer than the 19 payload cells a P=1 entry needs (`got: None`).
    PartTabBad { at: usize, pt: usize, want: u32, got: Option<u32> },
    /// The PartTab's root cell names an object whose header is outside the image.
    RootOutside { at: usize, root: i64 },
}

/// The live superblock, or why there is none. The order of the checks is the order a
/// reader depends on them: the crc first (to choose), then every offset the choice will be
/// asked to follow, then the PartTab's crc, then the root it names.
pub fn pick_checked_in<C: Cells + ?Sized>(c: &C) -> Result<Superblock, PickError> {
    let ok = |at: usize| crate::sb_valid_in(c, at);
    let at = match (ok(SB_A), ok(SB_B)) {
        (true, true) if c.cell_at(SB_B + SB_GEN) > c.cell_at(SB_A + SB_GEN) => SB_B,
        (true, _) => SB_A,
        (false, true) => SB_B,
        (false, false) => return Err(PickError::NoValidSuperblock),
    };
    let have = c.n_cells();
    let n = have as i64;
    let pt = c.cell_at(at + SB_PARTTAB);
    let used = c.cell_at(at + SB_USED);
    if used < 0 || used > n {
        return Err(PickError::ArenaUsedOutside { at, used, have });
    }
    if pt < 0 || (pt != 0 && pt.checked_add(PT_GEN as i64).is_none_or(|end| end >= n)) {
        return Err(PickError::PartTabOutside { at, pt });
    }
    if pt != 0 && pt != (at + PT_PAGE_OFFSET) as i64 {
        return Err(PickError::PartTabMisplaced { at, pt });
    }
    if pt != 0 {
        let p = pt as usize;
        let len = crate::obj_len_in(c, p) as usize;
        let want = ((c.cell_at(p + 1) >> 32) & 0xFFFF_FFFF) as u32;
        if len < crate::PARTTAB_CELLS as usize || p + 2 + len > have {
            return Err(PickError::PartTabBad { at, pt: p, want, got: None });
        }
        let got = crc32_in(c, p + 2, len);
        if got != want {
            return Err(PickError::PartTabBad { at, pt: p, want, got: Some(got) });
        }
        let r = c.cell_at(p + PT_ROOT);
        if r < 0 || (r != 0 && r.checked_add(2).is_none_or(|end| end >= n)) {
            return Err(PickError::RootOutside { at, root: r });
        }
    }
    Ok(Superblock {
        at,
        generation: c.cell_at(at + SB_GEN),
        root: pt,
        arena_used: used,
        live_cells: c.cell_at(at + SB_LIVE),
        superseded_cells: c.cell_at(at + SB_SUPERSEDED),
    })
}

/// What the live superblock's anchor (cell 9) says about the last commit's cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// `anc` = 0: no anchor (every image before W-ES-P0's Rust writer, and any commit whose
    /// mark is past `MAX_ANCHOR_MARK`). A reader that wants proof walks the arena instead.
    Absent,
    /// The range `[mark, arena_used)` hashes to the crc the commit recorded.
    Holds { mark: usize, used: usize },
    /// It does not, or the mark is not inside `[ARENA, arena_used]`: the last commit is torn.
    Torn { mark: i64, used: i64, want: u32, got: Option<u32> },
}

/// bebop's `st_anc_ok` rule (store.bp), over `Cells`, for the superblock `sb` names. The
/// PartTab half of that rule is `pick_checked_in`'s. The mark is read LOGICALLY (`>>` in
/// bebop is logical): `anc as u64 >> 32`.
pub fn anchor_in<C: Cells + ?Sized>(c: &C, sb: &Superblock) -> Anchor {
    let a = c.cell_at(sb.at + SB_ANC);
    if a == 0 {
        return Anchor::Absent;
    }
    let mark = ((a as u64) >> 32) as i64;
    let want = (a as u64 & 0xFFFF_FFFF) as u32;
    let used = sb.arena_used;
    if mark < ARENA as i64 || mark > used || used as usize > c.n_cells() {
        return Anchor::Torn { mark, used, want, got: None };
    }
    let got = crc32_in(c, mark as usize, (used - mark) as usize);
    if got == want {
        Anchor::Holds { mark: mark as usize, used: used as usize }
    } else {
        Anchor::Torn { mark, used, want, got: Some(got) }
    }
}

/// The anchor a commit of the cells `[mark, cursor)` writes. 0 when the mark does not fit
/// its 32-bit field -- "no anchor", which every reader already accepts.
pub fn anchor_of<C: Cells + ?Sized>(c: &C, mark: i64, cursor: i64) -> i64 {
    if !(ARENA as i64..=MAX_ANCHOR_MARK).contains(&mark) || cursor < mark {
        return 0;
    }
    let crc = crc32_in(c, mark as usize, (cursor - mark) as usize);
    (((mark as u64) << 32) | crc as u64) as i64
}

/// FOR GOLDEN TESTS (bebop-store's KV golden, and dowiz-hub's image-hash goldens when they
/// adopt it): the image as the pre-anchor writer left it: `anc` = 0 in every valid superblock and in every
/// PartTab's copy of one, each crc recomputed exactly as `stage_commit` computes it.
pub fn without_anchor(img: &[u8]) -> Vec<u8> {
    let mut st = Store::from_bytes(img);
    let n = img.len() / 8;
    for at in [crate::SB_A, crate::SB_B] {
        if !st.sb_valid(at) {
            continue;
        }
        let pt = st.cells[at + SB_PARTTAB].max(0) as usize;
        st.cells[at + SB_ANC] = 0;
        st.cells[at + SB_CRC] = crate::crc32_cells(&st.cells, at, 15) as i64;
        if pt != 0 && pt + PARTTAB_OBJ_CELLS <= n {
            st.cells[pt + 2 + SB_ANC] = 0;
            // Payload 15 is first the copied superblock's own crc (as `stage_commit` copies
            // it), then the crc over all 19 payload cells, which covers that first value.
            st.cells[pt + 2 + SB_CRC] = crate::crc32_cells(&st.cells, pt + 2, 15) as i64;
            st.cells[pt + 2 + SB_CRC] = crate::crc32_cells(&st.cells, pt + 2, crate::PARTTAB_CELLS as usize) as i64;
            st.seal(pt);
        }
    }
    st.cells[..n].iter().flat_map(|c| c.to_le_bytes()).collect()
}

/// `MAGIC` is "BEBOPST1" read as a little-endian i64 (re-derived by the tests).
pub const MAGIC_BYTES: [u8; 8] = *b"BEBOPST1";
const _: () = assert!(i64::from_le_bytes(MAGIC_BYTES) == MAGIC);

#[cfg(test)]
mod tests;
#[cfg(test)]
mod catalogue_tests;
#[cfg(test)]
mod catalogue_kv_tests;
