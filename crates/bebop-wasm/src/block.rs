//! The wasm32 reader of the columnar block (row DG9; SPEC-DATALOG-AND-CODEC §B.2-B.6).
//!
//! A DECODER AND ENCODER OF ITS OWN, not `dowiz_hub::block` (that one is gate.sh's native
//! reader; re-exporting it would count one reader twice). The schema strings are parsed here
//! into columns, `check` refuses in DG7's `decode::check` order, and `encode` writes the block
//! again from the decoded VALUES, so `encode(decode(b)) == b` is established, not copied. Shared
//! with the crate: `bebop_store::crc32` (zlib) and `bebop_store::nodekey::sha256` (KAT-tested).
//! `line(b)` is what gate.sh compares across the four readers:
//!   block <schema> n=<n> nnz=<nnz> vals=<fnv64> rt=<ok|diff> k256=<sha256 of the re-encoding>
//!   block refused=<code> col=<column|->
//! `vals` = FNV-1a 64 over each column's count, then its elements, 8 LE bytes each.
//! EVERYTHING REFUSES, NOTHING PANICS. `bw_block` is exported only under `--cfg bw_block`
//! (a cfg gate.sh passes in RUSTFLAGS, building the module apart), so `bytes.baseline` stays.

use bebop_store::nodekey::sha256;

/// §B.4, the order of DG7's `schema::TABLE` (the order matters only for printing).
pub const SCHEMAS: [&str; 4] = [
    "menu_prices:v1(dish:i64:0,price:i64:1,tax_ppm:i64:5,mods_ptr:u32rp:0,mods_col:u32:0,mods_val:i64:1)",
    "bom:v1(dish_ptr:u32rp:0,supply:u32:0,qty:i64:0)",
    "stock_levels:v1(supply:i64:0,qty:i64:0,gen:i64:7)",
    "names:v1(id:i64:0,bytes:u8:0,off:u32off:0)",
];

const I64: u8 = 1;
const I32: u8 = 2;
const BYTES: u8 = 3;
const OFFS: u8 = 4;
const ROW_PTR: u8 = 5;
const CSR_COL: u8 = 6;
const CSR_VAL: u8 = 7;
const FIXED: usize = 24;
const DESC: usize = 16;
const MAX: usize = 96 * 1024;

/// One schema, parsed: its name, `(column, type, unit)` in order, and its header key.
pub struct Spec {
    pub name: &'static str,
    pub cols: Vec<(&'static str, u8, u8)>,
    pub k64: u64,
}

/// RT §2.2 `K64 = (crc32 << 32) | len`.
pub fn k64(b: &[u8]) -> u64 {
    (u64::from(bebop_store::crc32(b)) << 32) | (b.len() as u64 & 0xffff_ffff)
}

/// `name:vN(col:type:unit,...)`; an `i64` right after a CSR `u32` is the CSR val (type 7).
pub fn parse(s: &'static str) -> Option<Spec> {
    let open = s.find('(')?;
    let name = s[..open].split(":v").next()?;
    let mut cols: Vec<(&'static str, u8, u8)> = Vec::new();
    for part in s[open + 1..s.len().checked_sub(1)?].split(',') {
        let mut f = part.split(':');
        let (c, tok, unit) = (f.next()?, f.next()?, f.next()?.parse().ok()?);
        let typ = match tok {
            "i64" if cols.last().is_some_and(|l| l.1 == CSR_COL) => CSR_VAL,
            "i64" => I64,
            "i32" => I32,
            "u8" => BYTES,
            "u32off" => OFFS,
            "u32rp" => ROW_PTR,
            "u32" => CSR_COL,
            _ => return None,
        };
        cols.push((c, typ, unit));
    }
    Some(Spec { name, cols, k64: k64(s.as_bytes()) })
}

/// The table, parsed. A string that does not parse is a defect `table_parses` catches.
pub fn table() -> Vec<Spec> {
    SCHEMAS.iter().filter_map(|s| parse(s)).collect()
}

/// A refusal: the check's code and the column it names (`-` for a header check).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refused {
    pub code: &'static str,
    pub col: &'static str,
}

#[rustfmt::skip]
fn no(code: &'static str, col: &'static str) -> Refused { Refused { code, col } }

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// A decoded block: the schema's index in `table()`, and every column as i64 values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub schema: usize,
    pub n: u32,
    pub nnz: u32,
    pub cols: Vec<Vec<i64>>,
}

/// Validate `b`: `(schema index, n, nnz, [(offset, len)])`, or the first check that refuses.
pub fn check(b: &[u8], t: &[Spec]) -> Result<(usize, u32, u32, Vec<(usize, usize)>), Refused> {
    if b.len() < FIXED + 4 {
        return Err(no("too_short", "-"));
    }
    if b.len() > MAX {
        return Err(no("too_big", "-"));
    }
    if &b[..4] != b"DWB1" {
        return Err(no("bad_magic", "-"));
    }
    if u16_at(b, 4) != 1 {
        return Err(no("bad_version", "-"));
    }
    let end = b.len() - 4;
    if u32_at(b, end) != bebop_store::crc32(&b[..end]) {
        return Err(no("bad_crc", "-"));
    }
    let key = u64::from_le_bytes(b[16..24].try_into().map_err(|_| no("too_short", "-"))?);
    let si = t.iter().position(|s| s.k64 == key).ok_or(no("unknown_schema", "-"))?;
    let cols = &t[si].cols;
    let ncols = usize::from(u16_at(b, 6));
    if ncols > 4096 || ncols != cols.len() {
        return Err(no("bad_ncols", "-"));
    }
    if FIXED + DESC * ncols > end {
        return Err(no("too_short", "-"));
    }
    let (n, nnz) = (u32_at(b, 8), u32_at(b, 12));
    if nnz != 0 && !cols.iter().any(|c| c.1 == ROW_PTR) {
        return Err(no("bad_nnz", "-"));
    }
    let mut lay = Vec::with_capacity(ncols);
    let mut prev = FIXED + DESC * ncols;
    for (i, &(c, typ, unit)) in cols.iter().enumerate() {
        let at = FIXED + DESC * i;
        if b[at] != typ || b[at + 1] != 0 {
            return Err(no("bad_type", c));
        }
        if b[at + 2] != unit {
            return Err(no("bad_unit", c));
        }
        if b[at + 3] != 0 || u32_at(b, at + 12) != 0 {
            return Err(no("bad_reserved", c));
        }
        let (clen, off) = (u32_at(b, at + 4) as usize, u32_at(b, at + 8) as usize);
        if off % 8 != 0 {
            return Err(no("bad_alignment", c));
        }
        if off != prev.div_ceil(8) * 8 || off + clen > end {
            return Err(no("bad_offsets", c));
        }
        let (n64, z64) = (u64::from(n), u64::from(nnz));
        let want = match typ {
            I64 => Some(n64 * 8),
            I32 => Some(n64 * 4),
            OFFS | ROW_PTR => Some((n64 + 1) * 4),
            CSR_COL => Some(z64 * 4),
            CSR_VAL => Some(z64 * 8),
            _ => None,
        };
        if want.is_some_and(|w| w != clen as u64) {
            return Err(no("bad_length", c));
        }
        if b[prev..off].iter().any(|x| *x != 0) {
            return Err(no("bad_padding", c));
        }
        lay.push((off, clen));
        prev = off + clen;
    }
    if prev != end {
        return Err(no("bad_offsets", cols.last().map_or("", |c| c.0)));
    }
    offsets(b, cols, n, nnz, &lay)?;
    Ok((si, n, nnz, lay))
}

/// The columns whose VALUES are offsets (`row_ptr`, text `off`), and the text they cut.
fn offsets(b: &[u8], cols: &[(&'static str, u8, u8)], n: u32, nnz: u32, lay: &[(usize, usize)]) -> Result<(), Refused> {
    for (i, &(c, typ, _)) in cols.iter().enumerate() {
        let last = match typ {
            ROW_PTR => nnz as usize,
            OFFS if i > 0 && cols[i - 1].1 == BYTES => lay[i - 1].1,
            OFFS => return Err(no("bad_type", c)),
            _ => continue,
        };
        let v: Vec<usize> = (0..=n as usize).map(|k| u32_at(b, lay[i].0 + 4 * k) as usize).collect();
        if v[0] != 0 || v.windows(2).any(|w| w[1] < w[0]) || v[n as usize] != last {
            return Err(no("bad_offsets", c));
        }
        if typ == OFFS {
            let (base, text) = (lay[i - 1].0, cols[i - 1].0);
            for w in v.windows(2) {
                let s = &b[base + w[0]..base + w[1]];
                if core::str::from_utf8(s).is_err() || s.contains(&0) {
                    return Err(no("not_utf8", text));
                }
            }
        }
    }
    Ok(())
}

/// Validate and copy every column out as values.
pub fn decode(b: &[u8], t: &[Spec]) -> Result<Decoded, Refused> {
    let (schema, n, nnz, lay) = check(b, t)?;
    let cols = t[schema]
        .cols
        .iter()
        .zip(&lay)
        .map(|(&(_, typ, _), &(off, len))| {
            let raw = &b[off..off + len];
            match typ {
                I64 | CSR_VAL => raw.chunks_exact(8).map(|w| i64::from_le_bytes(w.try_into().unwrap_or([0; 8]))).collect(),
                I32 => raw.chunks_exact(4).map(|w| i64::from(u32_at(w, 0) as i32)).collect(),
                BYTES => raw.iter().map(|x| i64::from(*x)).collect(),
                _ => raw.chunks_exact(4).map(|w| i64::from(u32_at(w, 0))).collect(),
            }
        })
        .collect();
    Ok(Decoded { schema, n, nnz, cols })
}

/// The canonical bytes of `d` (§B.2), written from its values alone.
pub fn encode(d: &Decoded, t: &[Spec]) -> Vec<u8> {
    let spec = &t[d.schema];
    let mut out = Vec::with_capacity(MAX);
    out.extend_from_slice(b"DWB1");
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(spec.cols.len() as u16).to_le_bytes());
    out.extend_from_slice(&d.n.to_le_bytes());
    out.extend_from_slice(&d.nnz.to_le_bytes());
    out.extend_from_slice(&spec.k64.to_le_bytes());
    out.resize(FIXED + DESC * spec.cols.len(), 0);
    for (i, (&(_, typ, unit), vals)) in spec.cols.iter().zip(&d.cols).enumerate() {
        out.resize(out.len().div_ceil(8) * 8, 0);
        let off = out.len();
        let width = match typ {
            I64 | CSR_VAL => 8,
            BYTES => 1,
            _ => 4,
        };
        for v in vals {
            out.extend_from_slice(&v.to_le_bytes()[..width]);
        }
        let at = FIXED + DESC * i;
        out[at] = typ;
        out[at + 2] = unit;
        let clen = (out.len() - off) as u32;
        out[at + 4..at + 8].copy_from_slice(&clen.to_le_bytes());
        out[at + 8..at + 12].copy_from_slice(&(off as u32).to_le_bytes());
    }
    let crc = bebop_store::crc32(&out);
    out.extend_from_slice(&crc.to_le_bytes());
    out
}

/// FNV-1a 64 over each column's count, then its values, 8 LE bytes each.
pub fn vals(d: &Decoded) -> u64 {
    let step = |h: u64, v: u64| v.to_le_bytes().iter().fold(h, |h, x| (h ^ u64::from(*x)).wrapping_mul(0x0000_0100_0000_01b3));
    d.cols.iter().fold(0xcbf2_9ce4_8422_2325, |h, c| c.iter().fold(step(h, c.len() as u64), |h, v| step(h, *v as u64)))
}

/// The gate's line for one block.
pub fn line(b: &[u8]) -> String {
    let t = table();
    let d = match decode(b, &t) {
        Ok(d) => d,
        Err(r) => return format!("block refused={} col={}", r.code, r.col),
    };
    let again = encode(&d, &t);
    let k256: String = sha256(&again).iter().map(|x| format!("{x:02x}")).collect();
    let rt = if again == b { "ok" } else { "diff" };
    format!("block {} n={} nnz={} vals={:016x} rt={rt} k256={k256}", t[d.schema].name, d.n, d.nnz, vals(&d))
}

/// Write `line(ptr..ptr+len)` into `out` (at most `cap` bytes); returns its length, or
/// `-abi::NULL_ARG` for a null pointer and `-1` when `cap` is too small.
///
/// # Safety
/// `ptr..ptr+len` is readable (or `len` is 0) and `out..out+cap` is writable.
#[cfg_attr(bw_block, no_mangle)]
pub unsafe extern "C" fn bw_block(ptr: *const u8, len: usize, out: *mut u8, cap: usize) -> i32 {
    if out.is_null() || (ptr.is_null() && len != 0) {
        return -crate::abi::NULL_ARG;
    }
    let bytes = if len == 0 { &[][..] } else { core::slice::from_raw_parts(ptr, len) };
    let s = line(bytes);
    if s.len() > cap {
        return -1;
    }
    core::ptr::copy_nonoverlapping(s.as_ptr(), out, s.len());
    s.len() as i32
}

#[cfg(test)]
mod tests;
