//! IMAGES AT REST, COMPRESSED (W-SPACE): the envelope a venue's image wears in a backup
//! bundle (the owner's download, the nightly S3/R2 copy and the archives it carries), never
//! in the object's own storage -- the hot image stays raw so its reads stay zero-copy.
//!
//! THE ENVELOPE, 24 bytes then the payload:
//!   `DWZPACK1` | codec u8 | 3 bytes zero | crc32 (zlib) of the RAW bytes u32 LE | raw length u64 LE
//! `codec` 0 = stored (compression did not pay), 1 = one LZ4 block (`lz4_flex::block`, the
//! crate's safe encoder and checked safe decoder; see `Cargo.toml` for why a crate).
//!
//! OLD BYTES STILL LOAD: an image is a bebop store and begins `BEBOPST1`; anything that does
//! not begin with [`MAGIC`] is returned as it is ([`unpack`]), so every bundle written before
//! this module reads exactly as it did.
//!
//! A RESTORE READS OUTSIDE BYTES, so the header is distrusted before anything is allocated: a
//! raw length past [`MAX_RAW`], or more than an LZ4 block of this size could ever expand to,
//! is refused by name. The crc is over the RAW bytes, so a damaged payload is caught here,
//! before a restore's sha256 check says the same thing in less useful words.
//!
//! COMPRESS BEFORE SEAL: the bundle is packed per image, then gzipped and sealed whole
//! (`workers/api/src/cloud.rs`). Sealed bytes look like noise and do not compress.

use std::borrow::Cow;

pub const MAGIC: &[u8; 8] = b"DWZPACK1";
pub const HEADER: usize = 24;
pub const STORED: u8 = 0;
pub const LZ4: u8 = 1;
/// The largest image an envelope may claim. A venue image is a few MB; the order log's own
/// ceiling is far below this. A header naming more is damaged or hostile, not big.
pub const MAX_RAW: usize = 64 * 1024 * 1024;
/// LZ4's best case is a run: one 255-byte length step buys 255 bytes of output, so a block
/// of `n` bytes cannot expand past ~`255 * n`. Anything claiming more is lying.
const MAX_EXPANSION: usize = 255;

/// Why a packed image does not open. Every one is a damaged or foreign copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    /// Shorter than the header, or its padding is not zero.
    Header,
    Codec(u8),
    /// The raw length the header claims is past [`MAX_RAW`] or past what the payload can hold.
    Length(u64),
    /// The payload does not decode to exactly the claimed length.
    Body(&'static str),
    Crc,
}

/// `raw` in the envelope: LZ4 when it is smaller, stored otherwise.
pub fn pack(raw: &[u8]) -> Vec<u8> {
    let z = lz4_flex::block::compress(raw);
    let (codec, body): (u8, &[u8]) = if z.len() < raw.len() { (LZ4, &z) } else { (STORED, raw) };
    let mut out = Vec::with_capacity(HEADER + body.len());
    out.extend_from_slice(MAGIC);
    out.push(codec);
    out.extend_from_slice(&[0; 3]);
    out.extend_from_slice(&crc32fast::hash(raw).to_le_bytes());
    out.extend_from_slice(&(raw.len() as u64).to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// THE PACK BUDGET of one export (W-SPACE): raw bytes packed per request, smallest image first.
/// MEASURED 2026-10-29, wasm32 release under node on the dev box (ARM, not Cloudflare): LZ4
/// compress ~8-12 ms per raw MB (log 9.0 ms for 839 KB, stock 6.3 ms for 573 KB, the compacted
/// journal 0.75 ms for 70 KB), decompress ~2.9 ms/MB, crc32fast ~0.9 ms/MB. 256 KiB is ~3 ms
/// of compress, the most the Free plan's 10 ms leaves beside the export's own sha256 and
/// base64; what qa-durres packs today (journal + menu + settings, 93 KB) measured ~1 ms. So the menu, its journal (post-compaction ~70 KB) and the
/// settings pack; the order log and stock go raw. RAISING IT WAITS FOR A CLOUDFLARE-SIDE CPU
/// MEASUREMENT of an export, not a number from this box.
pub const PACK_BUDGET: usize = 256 * 1024;

/// What an export stores for each of `raws`, in the same order: `(bytes, packed)`. Images are
/// taken smallest first and packed while the raw bytes packed so far stay within `budget`;
/// every other image is its raw bytes, which [`unpack`] passes through unchanged.
pub fn pack_within(raws: &[&[u8]], budget: usize) -> Vec<(Vec<u8>, bool)> {
    let mut order: Vec<usize> = (0..raws.len()).collect();
    order.sort_by_key(|&i| (raws[i].len(), i));
    let mut out: Vec<Option<(Vec<u8>, bool)>> = vec![None; raws.len()];
    let mut spent = 0usize;
    for i in order {
        let raw = raws[i];
        out[i] = Some(if spent + raw.len() <= budget {
            spent += raw.len();
            (pack(raw), true)
        } else {
            (raw.to_vec(), false)
        });
    }
    out.into_iter().flatten().collect()
}

/// Is this an envelope (as opposed to an image written before W-SPACE)?
pub fn is_packed(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// The raw image: unpacked and crc-checked, or `bytes` itself when it carries no envelope.
pub fn unpack(bytes: &[u8]) -> Result<Cow<'_, [u8]>, PackError> {
    if !is_packed(bytes) {
        return Ok(Cow::Borrowed(bytes));
    }
    let h = bytes.get(..HEADER).ok_or(PackError::Header)?;
    if h[9..12] != [0; 3] {
        return Err(PackError::Header);
    }
    let crc = u32::from_le_bytes([h[12], h[13], h[14], h[15]]);
    let claimed = u64::from_le_bytes([h[16], h[17], h[18], h[19], h[20], h[21], h[22], h[23]]);
    let body = &bytes[HEADER..];
    // BEFORE ANY ALLOCATION: the claim is checked against the cap and against the payload.
    let len = usize::try_from(claimed).ok().filter(|&n| n <= MAX_RAW).ok_or(PackError::Length(claimed))?;
    let raw = match h[8] {
        STORED if body.len() == len => body.to_vec(),
        STORED => return Err(PackError::Length(claimed)),
        LZ4 if len > body.len().saturating_mul(MAX_EXPANSION) => return Err(PackError::Length(claimed)),
        LZ4 => {
            let mut out = vec![0u8; len];
            match lz4_flex::block::decompress_into(body, &mut out) {
                Ok(n) if n == len => out,
                Ok(_) => return Err(PackError::Body("short")),
                Err(_) => return Err(PackError::Body("lz4")),
            }
        }
        c => return Err(PackError::Codec(c)),
    };
    if crc32fast::hash(&raw) != crc {
        return Err(PackError::Crc);
    }
    Ok(Cow::Owned(raw))
}

#[cfg(test)]
mod tests;
