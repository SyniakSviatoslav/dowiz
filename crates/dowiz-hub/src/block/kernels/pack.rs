//! THE VENUE'S SALES IMAGE: day blocks one after another, nothing but §B.2 blocks.
//!
//!   [names: the archives folded in]  [names: every dish id a row names]  [sales: one day]...
//!
//! No container format: each block's length is its own (the last column's end + the crc), and
//! every block goes through `decode::check`. The FIRST block makes the image idempotent -- an
//! archive already folded is refused, as `Cube::absorb` refuses it -- so the image catches up on its
//! own, independently of the cube's JSON head. The SECOND resolves a row's `dish` K64 to its id,
//! CONFIRMED by its bytes (RT K-1). Day blocks follow in ascending day order, one per day that has
//! at least one row; every row of a day block carries that day, ONE ROW PER DISH (`aggregate`),
//! in ascending dish order -- the canonical form, so a day refolded from its archives in the trace
//! encodes to the same bytes.

use std::collections::BTreeMap;

use super::super::decode::{check, decode};
use super::super::encode::encode;
use super::super::{header_size, schema, Block, Col, Refusal, CRC};
use super::{aggregate, sales_block, sales_of, Sale};

/// One image, decoded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pack {
    /// The archives folded in, in rotation order.
    pub folded: Vec<String>,
    /// `K64` -> dish id, every id a row names.
    pub dishes: BTreeMap<i64, String>,
    /// `yyyymmdd` -> that day's rows, one per dish, ascending dish.
    pub days: BTreeMap<i32, Vec<Sale>>,
}

/// A `names` block of `ids`, in order; two ids with one `K64` are refused (RT K-1).
fn names(ids: &[&str]) -> Result<Block, Refusal> {
    let mut seen: BTreeMap<u64, &str> = BTreeMap::new();
    let (mut bytes, mut off, mut keys) = (Vec::new(), vec![0u32], Vec::new());
    for s in ids.iter().copied() {
        let k = schema::k64(s.as_bytes());
        if let Some(a) = seen.insert(k, s) {
            return Err(Refusal::KeyCollision { a: a.to_string(), b: s.to_string() });
        }
        keys.push(k as i64);
        bytes.extend_from_slice(s.as_bytes());
        off.push(bytes.len() as u32);
    }
    Ok(Block { schema: &schema::NAMES, n: ids.len() as u32, nnz: 0, cols: vec![Col::I64(keys), Col::Bytes(bytes), Col::U32(off)] })
}

fn strings(b: &Block) -> Result<Vec<String>, Refusal> {
    let (Col::Bytes(bytes), Col::U32(off)) = (&b.cols[1], &b.cols[2]) else { return Err(Refusal::Mismatch("not a names block")) };
    Ok(off.windows(2).map(|w| String::from_utf8_lossy(&bytes[w[0] as usize..w[1] as usize]).into_owned()).collect())
}

/// The byte length of the block that starts `b`, from its own header; `None` when it cannot say.
fn block_len(b: &[u8]) -> Option<usize> {
    let ncols = usize::from(u16::from_le_bytes([*b.get(6)?, *b.get(7)?]));
    if ncols == 0 {
        return None;
    }
    let at = header_size(ncols - 1);
    let u32_at = |p: usize| Some(u32::from_le_bytes(b.get(p..p + 4)?.try_into().ok()?) as usize);
    Some(u32_at(at + 8)?.checked_add(u32_at(at + 4)?)?.checked_add(CRC)?)
}

impl Pack {
    /// Fold one archive's order lines in, summed into each day's dish rows. An archive already
    /// folded is refused (`Ok(false)`); two dish ids with one `K64` -- in the image and the archive,
    /// or within the archive -- or a sum leaving its type refuse the archive WHOLE, before anything
    /// is changed (RT K-1), so a refusal never folds half an archive.
    pub fn absorb(&mut self, archive: &str, rows: &[(Sale, String)]) -> Result<bool, Refusal> {
        if self.folded.iter().any(|a| a == archive) {
            return Ok(false);
        }
        let mut seen: BTreeMap<i64, &str> = BTreeMap::new();
        for (s, id) in rows {
            let known = self.dishes.get(&s.dish).map(String::as_str).or_else(|| seen.get(&s.dish).copied());
            if let Some(a) = known.filter(|a| *a != id.as_str()) {
                return Err(Refusal::KeyCollision { a: a.to_string(), b: id.clone() });
            }
            seen.insert(s.dish, id);
        }
        let mut days = self.days.clone();
        for (day, lines) in rows.iter().fold(BTreeMap::<i32, Vec<Sale>>::new(), |mut m, (s, _)| {
            m.entry(s.day).or_default().push(*s);
            m
        }) {
            let e = days.entry(day).or_default();
            e.extend(lines);
            *e = aggregate(e)?;
        }
        self.days = days;
        for (s, id) in rows {
            self.dishes.entry(s.dish).or_insert_with(|| id.clone());
        }
        self.folded.push(archive.to_string());
        Ok(true)
    }

    /// The image's bytes. A day too big for one block (§B.2, 96 KiB) is refused, never split.
    pub fn encode(&self) -> Result<Vec<u8>, Refusal> {
        let folded: Vec<&str> = self.folded.iter().map(String::as_str).collect();
        let dishes: Vec<&str> = self.dishes.values().map(String::as_str).collect();
        let mut out = encode(&names(&folded)?)?;
        out.extend(encode(&names(&dishes)?)?);
        for rows in self.days.values().filter(|r| !r.is_empty()) {
            out.extend(encode(&sales_block(rows))?);
        }
        Ok(out)
    }

    /// The image, every block checked; no bytes is an empty pack.
    pub fn decode(bytes: &[u8]) -> Result<Pack, Refusal> {
        let blocks = split(bytes)?;
        let mut p = Pack::default();
        let mut it = blocks.iter();
        match (it.next(), it.next()) {
            (None, _) => return Ok(p),
            (Some(f), Some(d)) if f.schema == &schema::NAMES && d.schema == &schema::NAMES => {
                p.folded = strings(f)?;
                let Col::I64(keys) = &d.cols[0] else { return Err(Refusal::Mismatch("names")) };
                p.dishes = keys.iter().copied().zip(strings(d)?).collect();
            }
            _ => return Err(Refusal::Mismatch("a sales image opens with two names blocks")),
        }
        for b in it {
            let rows = sales_of(b)?;
            let day = rows.first().map(|s| s.day).ok_or(Refusal::Mismatch("an empty day block"))?;
            if rows.iter().any(|s| s.day != day) || p.days.keys().next_back().is_some_and(|d| *d >= day) || rows.windows(2).any(|w| w[0].dish >= w[1].dish) {
                return Err(Refusal::Mismatch("day blocks are one day each, ascending, one row per dish"));
            }
            if rows.iter().any(|s| !p.dishes.contains_key(&s.dish)) {
                return Err(Refusal::Mismatch("a row names a dish the image does not"));
            }
            p.days.insert(day, rows);
        }
        Ok(p)
    }
}

/// Every block of an image, in order, each through `decode::check`.
pub fn split(bytes: &[u8]) -> Result<Vec<Block>, Refusal> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let rest = &bytes[at..];
        let len = block_len(rest).filter(|l| *l <= rest.len()).ok_or(Refusal::TooShort { len: rest.len() })?;
        check(&rest[..len])?;
        out.push(decode(&rest[..len])?);
        at += len;
    }
    Ok(out)
}

/// The day blocks of `from..=to` (`yyyymmdd`) only, CHECKED, without decoding the others: the
/// year scan's read. Each day block's first `day` value is read in place after its check.
pub fn days_in(bytes: &[u8], from: i32, to: i32) -> Result<Vec<Block>, Refusal> {
    let mut out = Vec::new();
    let mut at = 0;
    let mut k = 0;
    while at < bytes.len() {
        let rest = &bytes[at..];
        let len = block_len(rest).filter(|l| *l <= rest.len()).ok_or(Refusal::TooShort { len: rest.len() })?;
        if k >= 2 {
            let l = check(&rest[..len])?;
            if l.known.schema != &schema::SALES || l.n == 0 {
                return Err(Refusal::Mismatch("day blocks are one day each, ascending"));
            }
            let (off, _) = l.cols[0];
            let day = i32::from_le_bytes(rest[off..off + 4].try_into().map_err(|_| Refusal::TooShort { len })?);
            if day >= from && day <= to {
                out.push(decode(&rest[..len])?);
            }
        }
        at += len;
        k += 1;
    }
    Ok(out)
}
