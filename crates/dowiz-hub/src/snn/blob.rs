//! THE WEIGHTS AS BYTES: one integer blob with a version and a crc (W-CRC policy), refused whole
//! when any of it is wrong -- never half-read, never guessed (W-SNN row 1, row 5).
//!
//! LAYOUT v1 (little-endian), written by the trainer (`snn-train/train.py export`, scratchpad):
//!   0   8  magic  "DWSNN\0\0\x01"
//!   8   2  format version (1)        10  2  stalk dim D (2..=8)    12  2  layers L (1..=4)
//!   14  2  axes (27)                 16  2  group types (3)        18  2  reserved (0)
//!   20  4  model id (the training run: 20261006 = this lane's)     24  4  n params
//!   28  4n params, i32, Q16 fixed point, in the order of `Model` below
//!   ..  4  zlib crc32 of every byte before it (`crate::block::crc32`, the W-CRC function)
//! The param count is DERIVED from the header (`expected_params`), so a header and a body that
//! disagree are refused by name, not read short.

use super::{AXES, GROUP_TYPES, Q};

pub const MAGIC: [u8; 8] = *b"DWSNN\x00\x00\x01";
pub const FORMAT: u16 = 1;
pub const HEADER: usize = 28;

/// Why a blob was not used. Each is a named refusal: the caller falls back to the current ranker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    TooShort { len: usize },
    BadMagic,
    BadFormat(u16),
    BadShape { what: &'static str, got: u16 },
    BadCount { got: u32, want: usize },
    BadLength { got: usize, want: usize },
    BadCrc { stored: u32, computed: u32 },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::TooShort { len } => write!(f, "snn_blob_too_short: {len} bytes"),
            Refusal::BadMagic => write!(f, "snn_blob_bad_magic"),
            Refusal::BadFormat(v) => write!(f, "snn_blob_format: {v}, this build reads {FORMAT}"),
            Refusal::BadShape { what, got } => write!(f, "snn_blob_shape: {what} = {got}"),
            Refusal::BadCount { got, want } => write!(f, "snn_blob_count: {got} params, the header says {want}"),
            Refusal::BadLength { got, want } => write!(f, "snn_blob_length: {got} bytes, want {want}"),
            Refusal::BadCrc { stored, computed } => write!(f, "snn_blob_crc: stored {stored:08x}, computed {computed:08x}"),
        }
    }
}

/// The decoded model. Every value is Q16 (`Q` = 1.0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub id: u32,
    pub d: usize,
    pub layers: usize,
    /// Per-axis weights of the residual cosine (27).
    pub lam: Vec<i64>,
    /// Share of the residual cosine in the score, 0..=Q.
    pub beta: i64,
    /// Axis -> stalk map, `win[a][e]` (27 x D).
    pub win: Vec<Vec<i64>>,
    /// Diagonal restriction maps per group type (category, tag, ingredient), `rho[t][e]`.
    pub rho: Vec<Vec<i64>>,
    /// Per-layer D x D weights, `wl[l][d][e]`.
    pub wl: Vec<Vec<Vec<i64>>>,
    /// The guest's three paths into the stalk space: axes, dishes, groups (`gam[p][e]`).
    pub gam: Vec<Vec<i64>>,
}

pub fn expected_params(d: usize, layers: usize) -> usize {
    AXES + 1 + AXES * d + GROUP_TYPES * d + layers * d * d + 3 * d
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// The blob, checked whole (crc first, then shape), or the named reason it is refused.
pub fn decode(b: &[u8]) -> Result<Model, Refusal> {
    if b.len() < HEADER + 4 {
        return Err(Refusal::TooShort { len: b.len() });
    }
    let body = &b[..b.len() - 4];
    let (stored, computed) = (u32_at(b, b.len() - 4), crate::block::crc32(body));
    if stored != computed {
        return Err(Refusal::BadCrc { stored, computed });
    }
    if b[..8] != MAGIC {
        return Err(Refusal::BadMagic);
    }
    let format = u16_at(b, 8);
    if format != FORMAT {
        return Err(Refusal::BadFormat(format));
    }
    let (d, layers, axes, types) = (u16_at(b, 10), u16_at(b, 12), u16_at(b, 14), u16_at(b, 16));
    for (what, got, ok) in [
        ("stalk dim", d, (2..=8).contains(&d)),
        ("layers", layers, (1..=4).contains(&layers)),
        ("axes", axes, usize::from(axes) == AXES),
        ("group types", types, usize::from(types) == GROUP_TYPES),
        ("reserved", u16_at(b, 18), u16_at(b, 18) == 0),
    ] {
        if !ok {
            return Err(Refusal::BadShape { what, got });
        }
    }
    let (d, layers) = (usize::from(d), usize::from(layers));
    let want = expected_params(d, layers);
    let n = u32_at(b, 24);
    if n as usize != want {
        return Err(Refusal::BadCount { got: n, want });
    }
    if body.len() != HEADER + 4 * want {
        return Err(Refusal::BadLength { got: b.len(), want: HEADER + 4 * want + 4 });
    }
    let mut it = (0..want).map(|i| i64::from(i32::from_le_bytes([b[HEADER + 4 * i], b[HEADER + 4 * i + 1], b[HEADER + 4 * i + 2], b[HEADER + 4 * i + 3]])));
    let mut take = |m: usize| -> Vec<i64> { it.by_ref().take(m).collect() };
    let lam = take(AXES);
    let beta = take(1)[0].clamp(0, Q);
    let win = (0..AXES).map(|_| take(d)).collect();
    let rho = (0..GROUP_TYPES).map(|_| take(d)).collect();
    let wl = (0..layers).map(|_| (0..d).map(|_| take(d)).collect()).collect();
    let gam = (0..3).map(|_| take(d)).collect();
    Ok(Model { id: u32_at(b, 20), d, layers, lam, beta, win, rho, wl, gam })
}

/// The model as bytes again (the tests' round trip and the RED proofs' corrupted copies).
pub fn encode(m: &Model) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    for h in [FORMAT, m.d as u16, m.layers as u16, AXES as u16, GROUP_TYPES as u16, 0] {
        out.extend_from_slice(&h.to_le_bytes());
    }
    let params: Vec<i64> = m
        .lam
        .iter()
        .copied()
        .chain([m.beta])
        .chain(m.win.iter().flatten().copied())
        .chain(m.rho.iter().flatten().copied())
        .chain(m.wl.iter().flatten().flatten().copied())
        .chain(m.gam.iter().flatten().copied())
        .collect();
    out.extend_from_slice(&m.id.to_le_bytes());
    out.extend_from_slice(&(params.len() as u32).to_le_bytes());
    for p in params {
        out.extend_from_slice(&(p as i32).to_le_bytes());
    }
    let crc = crate::block::crc32(&out);
    out.extend_from_slice(&crc.to_le_bytes());
    out
}
