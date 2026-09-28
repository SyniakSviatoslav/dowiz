//! THE COLUMNAR CATALOGUE BLOCK (row DG7; SPEC-DATALOG-AND-CODEC Part B).
//!
//! One block is at once the CAS unit, the wire format and the in-memory format:
//! a reader views a field with one bounds check and one `from_le_bytes`. It
//! replaces the per-request JSON parse on the catalogue's hot paths (menu
//! prices, `bom_of`); JSON stays at the browser edge.
//!
//! LAYOUT (§B.2, little-endian): a 24-byte fixed header, `16 * ncols` bytes of
//! column descriptors, the columns (each 8-byte aligned, zero padding between),
//! and a zlib crc32 over everything before it. The header size is DERIVED
//! (`header_size`), never a literal. The four files:
//!   `schema` -- the ONE table of schema strings (§B.4, B-2) and their `K64`;
//!   `encode` -- columns -> bytes, and the catalogue projection from product JSON;
//!   `decode` -- the validator every reader goes through (B-1), and bytes -> columns;
//!   `view`   -- zero-copy reads, `bom_of` from the block, checked pricing (B-3),
//!               and the availability kernel with its scalar oracle (B-4).
//!
//! EVERYTHING REFUSES, NOTHING PANICS: a malformed block is a typed `Refusal`
//! that names the region and, where there is one, the column (by its name in
//! the schema string).

pub mod decode;
pub mod encode;
pub mod schema;
pub mod view;

#[cfg(test)]
mod tests;

/// §B.2 offset 0: "DWB1".
pub const MAGIC: [u8; 4] = *b"DWB1";
/// §B.2 offset 4.
pub const VERSION: u16 = 1;
/// §B.2: magic 4 + version 2 + ncols 2 + n 4 + nnz 4 + schema 8.
pub const FIXED: usize = 4 + 2 + 2 + 4 + 4 + 8;
/// §B.2: type 1 + flags 1 + unit 1 + reserved 1 + len 4 + offset 4 + reserved 4.
pub const COLDESC: usize = 1 + 1 + 1 + 1 + 4 + 4 + 4;
/// §B.2 `end-4`: the crc32 trailer.
pub const CRC: usize = 4;
/// §B.2: one block is one Durable Object chunk (`workers/api/src/hubdo.rs` `CHUNK`).
pub const MAX_BLOCK: usize = 96 * 1024;
/// §B.2: `ncols` ≤ 4096.
pub const MAX_COLS: usize = 4096;
/// §B.2: every column starts on an 8-byte boundary.
pub const ALIGN: usize = 8;

/// `24 + 16 * ncols`, as §B.2 says every reader derives it.
pub const fn header_size(ncols: usize) -> usize {
    FIXED + COLDESC * ncols
}

/// §B.2 `type` codes.
pub mod ty {
    pub const I64: u8 = 1;
    pub const I32: u8 = 2;
    pub const BYTES: u8 = 3;
    pub const OFFSETS: u8 = 4;
    pub const ROW_PTR: u8 = 5;
    pub const CSR_COL: u8 = 6;
    pub const CSR_VAL: u8 = 7;
    pub const VALIDITY: u8 = 8;
    pub const MASK16: u8 = 9;
}

/// §B.2 `unit` codes.
pub mod unit {
    pub const NONE: u8 = 0;
    pub const MINOR: u8 = 1;
    pub const GRAMS: u8 = 2;
    pub const ML: u8 = 3;
    pub const UNIT: u8 = 4;
    pub const PPM: u8 = 5;
    pub const MS: u8 = 6;
    pub const GENERATION: u8 = 7;
}

/// Why a block was refused (B-1). `col` is the column's name in the schema string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    TooShort { len: usize },
    TooBig { len: usize },
    BadMagic,
    BadVersion(u16),
    BadCrc { stored: u32, computed: u32 },
    UnknownSchema(u64),
    BadNcols { got: usize, want: usize },
    /// `nnz` must be 0 when the schema has no CSR column.
    BadNnz { got: u32 },
    BadType { col: &'static str },
    BadUnit { col: &'static str },
    BadReserved { col: &'static str },
    BadAlignment { col: &'static str },
    /// A column's place in the block, or an offsets / row_ptr VALUE, is wrong.
    BadOffsets { col: &'static str },
    BadLength { col: &'static str },
    BadPadding { col: &'static str },
    NotUtf8 { col: &'static str },
    /// Two different strings with one `K64` in the same block: refused, not guessed (RT K-1).
    KeyCollision { a: String, b: String },
    /// A checked money operation overflowed (B-3).
    Overflow { col: &'static str },
    /// A rate outside `0..=1_000_000` ppm.
    BadRate { col: &'static str },
    /// The three catalogue blocks do not describe the same rows.
    Mismatch(&'static str),
}

impl Refusal {
    /// One stable code per refusal.
    pub fn code(&self) -> &'static str {
        match self {
            Refusal::TooShort { .. } => "too_short",
            Refusal::TooBig { .. } => "too_big",
            Refusal::BadMagic => "bad_magic",
            Refusal::BadVersion(_) => "bad_version",
            Refusal::BadCrc { .. } => "bad_crc",
            Refusal::UnknownSchema(_) => "unknown_schema",
            Refusal::BadNcols { .. } => "bad_ncols",
            Refusal::BadNnz { .. } => "bad_nnz",
            Refusal::BadType { .. } => "bad_type",
            Refusal::BadUnit { .. } => "bad_unit",
            Refusal::BadReserved { .. } => "bad_reserved",
            Refusal::BadAlignment { .. } => "bad_alignment",
            Refusal::BadOffsets { .. } => "bad_offsets",
            Refusal::BadLength { .. } => "bad_length",
            Refusal::BadPadding { .. } => "bad_padding",
            Refusal::NotUtf8 { .. } => "not_utf8",
            Refusal::KeyCollision { .. } => "key_collision",
            Refusal::Overflow { .. } => "overflow",
            Refusal::BadRate { .. } => "bad_rate",
            Refusal::Mismatch(_) => "mismatch",
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "block refused: {self:?}")
    }
}

/// One column's values, as `decode` copies them out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Col {
    /// types 1 and 7.
    I64(Vec<i64>),
    /// type 2.
    I32(Vec<i32>),
    /// type 3.
    Bytes(Vec<u8>),
    /// types 4, 5 and 6.
    U32(Vec<u32>),
}

/// A block as structured values: what `decode(encode(x)) == x` compares (B-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub schema: &'static schema::Schema,
    pub n: u32,
    pub nnz: u32,
    pub cols: Vec<Col>,
}

/// zlib CRC-32 (§B.2 trailer). The SAME function as `bebop_store::crc32`
/// (bitwise); `schema_table_agrees` holds the two equal on every fixture and on
/// seeded input of every length 0..700.
///
/// WHY NOT ONE LOOP. The crc was 7.66 us of a 10.24 us `menu_prices` decode
/// (release, 2026-09-28): slice-by-8 is bound by the latency of its own xor
/// chain. Four independent lanes over the four quarters run in one loop, and
/// the four crcs are joined with zlib's `crc32_combine` (a GF(2) multiply by
/// x^(8·len)). No `unsafe`, no target feature: the crate forbids the first and
/// wasm32 has no crc instruction.
pub fn crc32(bytes: &[u8]) -> u32 {
    const LANES: usize = 4;
    let quarter = bytes.len() / LANES / 8 * 8;
    if quarter < 64 {
        return !crc_run(0xFFFF_FFFF, bytes);
    }
    let mut c = [0xFFFF_FFFFu32; LANES];
    for i in (0..quarter).step_by(8) {
        for (k, ck) in c.iter_mut().enumerate() {
            let at = k * quarter + i;
            *ck = crc_step8(*ck, &bytes[at..at + 8]);
        }
    }
    c[LANES - 1] = crc_run(c[LANES - 1], &bytes[LANES * quarter..]);
    let tail = bytes.len() - LANES * quarter;
    let shift = x8n(quarter);
    let mut crc = !c[0];
    for (k, ck) in c.iter().enumerate().skip(1) {
        let s = if k == LANES - 1 { x8n(quarter + tail) } else { shift };
        crc = mul_mod_p(s, crc) ^ !ck;
    }
    crc
}

/// One slice-by-8 step over exactly 8 bytes.
fn crc_step8(c: u32, w: &[u8]) -> u32 {
    let t = &CRC_TABLES;
    let lo = c ^ u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
    t[7][(lo & 0xff) as usize]
        ^ t[6][((lo >> 8) & 0xff) as usize]
        ^ t[5][((lo >> 16) & 0xff) as usize]
        ^ t[4][(lo >> 24) as usize]
        ^ t[3][w[4] as usize]
        ^ t[2][w[5] as usize]
        ^ t[1][w[6] as usize]
        ^ t[0][w[7] as usize]
}

/// The running (un-inverted) crc `c` continued over `bytes`.
fn crc_run(mut c: u32, bytes: &[u8]) -> u32 {
    let mut chunks = bytes.chunks_exact(8);
    for w in &mut chunks {
        c = crc_step8(c, w);
    }
    for &b in chunks.remainder() {
        c = CRC_TABLES[0][((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
    }
    c
}

/// zlib `multmodp`: a·b modulo the reflected crc polynomial.
const fn mul_mod_p(a: u32, mut b: u32) -> u32 {
    let mut m = 1u32 << 31;
    let mut p = 0u32;
    loop {
        if a & m != 0 {
            p ^= b;
            if a & (m - 1) == 0 {
                break;
            }
        }
        m >>= 1;
        b = if b & 1 != 0 { (b >> 1) ^ 0xEDB8_8320 } else { b >> 1 };
    }
    p
}

/// x^(2^k) mod p, k = 0..32 (zlib `x2n_table`).
static X2N: [u32; 32] = {
    let mut t = [0u32; 32];
    t[0] = 1 << 30;
    let mut k = 1;
    while k < 32 {
        t[k] = mul_mod_p(t[k - 1], t[k - 1]);
        k += 1;
    }
    t
};

/// x^(8·n) mod p: what `crc32_combine` multiplies the first crc by.
fn x8n(mut n: usize) -> u32 {
    let (mut p, mut k) = (1u32 << 31, 3usize);
    while n != 0 {
        if n & 1 != 0 {
            p = mul_mod_p(X2N[k & 31], p);
        }
        n >>= 1;
        k += 1;
    }
    p
}

static CRC_TABLES: [[u32; 256]; 8] = crc_tables();

const fn crc_tables() -> [[u32; 256]; 8] {
    let mut t = [[0u32; 256]; 8];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        t[0][i] = c;
        i += 1;
    }
    let mut s = 1;
    while s < 8 {
        let mut i = 0;
        while i < 256 {
            let prev = t[s - 1][i];
            t[s][i] = t[0][(prev & 0xff) as usize] ^ (prev >> 8);
            i += 1;
        }
        s += 1;
    }
    t
}
