//! THE VALIDATOR EVERY READER GOES THROUGH (B-1), and bytes -> columns.
//!
//! `check` refuses -- never pads, never guesses -- a block whose magic,
//! version, crc, schema, column descriptors, offsets, lengths, alignment or
//! padding are wrong, and names the column by its schema name. It also checks
//! the VALUES that are offsets (a CSR `row_ptr`, a text `off` column), because
//! a reader indexes with them: a `row_ptr` past `nnz` is a read out of bounds
//! waiting for the first reader that trusts it.
//!
//! CANONICAL FORM. Columns sit in schema order, each at the first 8-aligned
//! byte after the previous one, zero padding between, the crc directly after
//! the last column. `check` refuses anything else, which is what makes
//! `encode(decode(b)) == b` (B-6) hold for every block it accepts.

use super::schema::{self, ColSpec, Known};
use super::{header_size, ty, Block, Col, Refusal, ALIGN, COLDESC, CRC, FIXED, MAGIC, MAX_BLOCK, MAX_COLS, VERSION};

/// A validated block's shape: its schema, `n`, `nnz` and each column's byte range.
#[derive(Debug, Clone)]
pub struct Layout {
    pub known: &'static Known,
    pub n: u32,
    pub nnz: u32,
    /// `(offset, len)` in bytes, per column, in schema order.
    pub cols: Vec<(usize, usize)>,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

pub(super) fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

pub(super) fn i64_at(b: &[u8], at: usize) -> i64 {
    let mut w = [0u8; 8];
    w.copy_from_slice(&b[at..at + 8]);
    i64::from_le_bytes(w)
}

/// §B.2 offset 16..24.
fn schema_at(b: &[u8]) -> u64 {
    i64_at(b, FIXED - 8) as u64
}

/// The byte length a column of `typ` must have (`None`: a bytes column, sized by its offsets).
fn want_len(typ: u8, n: u64, nnz: u64) -> Option<u64> {
    match typ {
        ty::I64 => Some(n * 8),
        ty::I32 => Some(n * 4),
        ty::OFFSETS | ty::ROW_PTR => Some((n + 1) * 4),
        ty::CSR_COL => Some(nnz * 4),
        ty::CSR_VAL => Some(nnz * 8),
        _ => None,
    }
}

fn align_up(x: usize) -> usize {
    x.div_ceil(ALIGN) * ALIGN
}

/// Validate `b` and return its layout. See the module header for what is refused.
pub fn check(b: &[u8]) -> Result<Layout, Refusal> {
    let len = b.len();
    if len < header_size(0) + CRC {
        return Err(Refusal::TooShort { len });
    }
    if len > MAX_BLOCK {
        return Err(Refusal::TooBig { len });
    }
    if b[..4] != MAGIC {
        return Err(Refusal::BadMagic);
    }
    let version = u16_at(b, 4);
    if version != VERSION {
        return Err(Refusal::BadVersion(version));
    }
    let end = len - CRC;
    let (stored, computed) = (u32_at(b, end), super::crc32(&b[..end]));
    if stored != computed {
        return Err(Refusal::BadCrc { stored, computed });
    }
    let known = schema::by_k64(schema_at(b)).ok_or(Refusal::UnknownSchema(schema_at(b)))?;
    let ncols = usize::from(u16_at(b, 6));
    if ncols > MAX_COLS || ncols != known.cols.len() {
        return Err(Refusal::BadNcols { got: ncols, want: known.cols.len() });
    }
    if header_size(ncols) > end {
        return Err(Refusal::TooShort { len });
    }
    let (n, nnz) = (u32_at(b, 8), u32_at(b, 12));
    if nnz != 0 && !known.csr {
        return Err(Refusal::BadNnz { got: nnz });
    }
    let mut cols = Vec::with_capacity(ncols);
    let mut prev_end = header_size(ncols);
    for (i, spec) in known.cols.iter().enumerate() {
        let (off, clen) = descriptor(b, i, spec, prev_end, end, u64::from(n), u64::from(nnz))?;
        cols.push((off, clen));
        prev_end = off + clen;
    }
    if prev_end != end {
        let last = known.cols.last().map_or("", |c| c.name);
        return Err(Refusal::BadOffsets { col: last });
    }
    let layout = Layout { known, n, nnz, cols };
    values(b, &layout)?;
    Ok(layout)
}

/// One column descriptor (§B.2 `coldesc[i]`), checked against its schema column.
fn descriptor(b: &[u8], i: usize, spec: &ColSpec, prev_end: usize, end: usize, n: u64, nnz: u64) -> Result<(usize, usize), Refusal> {
    let col = spec.name;
    let at = header_size(i);
    // Flags: no shipped schema has a validity column, so bit 0 set is a type error.
    if b[at] != spec.typ || b[at + 1] != 0 {
        return Err(Refusal::BadType { col });
    }
    if b[at + 2] != spec.unit {
        return Err(Refusal::BadUnit { col });
    }
    if b[at + 3] != 0 || u32_at(b, at + COLDESC - 4) != 0 {
        return Err(Refusal::BadReserved { col });
    }
    let clen = u32_at(b, at + 4) as usize;
    let off = u32_at(b, at + 8) as usize;
    if off % ALIGN != 0 {
        return Err(Refusal::BadAlignment { col });
    }
    if off != align_up(prev_end) || off.checked_add(clen).is_none_or(|e| e > end) {
        return Err(Refusal::BadOffsets { col });
    }
    if want_len(spec.typ, n, nnz).is_some_and(|w| w != clen as u64) {
        return Err(Refusal::BadLength { col });
    }
    if b[prev_end..off].iter().any(|x| *x != 0) {
        return Err(Refusal::BadPadding { col });
    }
    Ok((off, clen))
}

/// The columns whose VALUES are offsets: `row_ptr` and text `off`.
fn values(b: &[u8], l: &Layout) -> Result<(), Refusal> {
    let specs = &l.known.cols;
    for (i, spec) in specs.iter().enumerate() {
        let (off, _) = l.cols[i];
        let last = match spec.typ {
            ty::ROW_PTR => l.nnz as usize,
            // §B.3: the offsets pair with the bytes column before them.
            ty::OFFSETS => match i.checked_sub(1).map(|p| (specs[p].typ, l.cols[p])) {
                Some((ty::BYTES, (_, blen))) => blen,
                _ => return Err(Refusal::BadType { col: spec.name }),
            },
            _ => continue,
        };
        let mut prev = 0usize;
        for k in 0..=l.n as usize {
            let v = u32_at(b, off + 4 * k) as usize;
            if (k == 0 && v != 0) || v < prev || v > last {
                return Err(Refusal::BadOffsets { col: spec.name });
            }
            prev = v;
        }
        if prev != last {
            return Err(Refusal::BadOffsets { col: spec.name });
        }
        if spec.typ == ty::OFFSETS {
            utf8(b, l, i)?;
        }
    }
    Ok(())
}

/// Every string of a text column is UTF-8 (§B.3 "UTF-8 bytes, no NUL").
fn utf8(b: &[u8], l: &Layout, off_col: usize) -> Result<(), Refusal> {
    let (boff, _) = l.cols[off_col - 1];
    let (ooff, _) = l.cols[off_col];
    let col = l.known.cols[off_col - 1].name;
    for k in 0..l.n as usize {
        let (s, e) = (u32_at(b, ooff + 4 * k) as usize, u32_at(b, ooff + 4 * k + 4) as usize);
        let text = std::str::from_utf8(&b[boff + s..boff + e]).map_err(|_| Refusal::NotUtf8 { col })?;
        if text.contains('\0') {
            return Err(Refusal::NotUtf8 { col });
        }
    }
    Ok(())
}

/// Validate and copy every column out: `decode(encode(x)) == x` (B-6).
pub fn decode(b: &[u8]) -> Result<Block, Refusal> {
    let l = check(b)?;
    let cols = l
        .known
        .cols
        .iter()
        .zip(&l.cols)
        .map(|(spec, &(off, len))| {
            let raw = &b[off..off + len];
            match spec.typ {
                ty::I64 | ty::CSR_VAL => Col::I64(raw.chunks_exact(8).map(|w| i64_at(w, 0)).collect()),
                ty::I32 => Col::I32(raw.chunks_exact(4).map(|w| u32_at(w, 0) as i32).collect()),
                ty::BYTES => Col::Bytes(raw.to_vec()),
                _ => Col::U32(raw.chunks_exact(4).map(|w| u32_at(w, 0)).collect()),
            }
        })
        .collect();
    Ok(Block { schema: l.known.schema, n: l.n, nnz: l.nnz, cols })
}
