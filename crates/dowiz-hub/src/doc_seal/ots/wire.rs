//! The `.ots` wire codec (W-MG-M1B): bytes in (`parse`) and bytes out (`serialize`), the
//! encoding re-derived from python-opentimestamps and held to a hand-written vector in
//! `doc_seal/tests.rs`. Split out of `ots.rs` for the 300-line gate; `ots::{parse, serialize}`
//! stay where they were.

use super::{Attestation, Op, Proof, Ts, MAGIC, MAX_MSG, OP_APPEND, OP_HEXLIFY, OP_KECCAK256, OP_PREPEND, OP_REVERSE, OP_RIPEMD160, OP_SHA1, OP_SHA256, TAG_BITCOIN, TAG_PENDING};

// ── bytes in ─────────────────────────────────────────────────────────────────────────────

struct Cur<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Cur<'a> {
    fn byte(&mut self) -> Result<u8, String> {
        let x = *self.b.get(self.at).ok_or_else(|| format!("truncated at byte {}", self.at))?;
        self.at += 1;
        Ok(x)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self.b.get(self.at..self.at + n).ok_or_else(|| format!("truncated: {n} bytes wanted at {}", self.at))?;
        self.at += n;
        Ok(s)
    }
    fn varint(&mut self) -> Result<u64, String> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let x = self.byte()?;
            v |= ((x & 0x7f) as u64) << shift;
            if x & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err("a varint longer than 64 bits".to_string())
    }
    fn varbytes(&mut self, max: usize) -> Result<&'a [u8], String> {
        let n = self.varint()?;
        if n as usize > max {
            return Err(format!("varbytes of {n} past the bound {max}"));
        }
        self.take(n as usize)
    }
}

fn parse_attestation(c: &mut Cur) -> Result<Attestation, String> {
    let tag: [u8; 8] = c.take(8)?.try_into().map_err(|_| "tag")?;
    let payload = c.varbytes(MAX_MSG)?;
    let mut p = Cur { b: payload, at: 0 };
    Ok(if tag == TAG_BITCOIN {
        Attestation::Bitcoin { height: p.varint()? }
    } else if tag == TAG_PENDING {
        let uri = p.varbytes(MAX_MSG)?;
        Attestation::Pending { uri: String::from_utf8(uri.to_vec()).map_err(|_| "a pending uri that is not UTF-8".to_string())? }
    } else {
        Attestation::Unknown { tag }
    })
}

fn parse_op(c: &mut Cur, tag: u8) -> Result<Op, String> {
    Ok(match tag {
        OP_SHA256 => Op::Sha256,
        OP_APPEND => Op::Append(c.varbytes(MAX_MSG)?.to_vec()),
        OP_PREPEND => Op::Prepend(c.varbytes(MAX_MSG)?.to_vec()),
        OP_REVERSE => Op::Reverse,
        OP_HEXLIFY => Op::Hexlify,
        OP_SHA1 | OP_RIPEMD160 | OP_KECCAK256 => Op::Unsupported(tag),
        other => return Err(format!("unknown op tag 0x{other:02x}")),
    })
}

fn parse_ts(c: &mut Cur, depth: usize) -> Result<Ts, String> {
    if depth > 256 {
        return Err("a timestamp nested past 256".to_string());
    }
    let mut ts = Ts::default();
    let mut tag = c.byte()?;
    loop {
        let last = tag != 0xff;
        let item = if last { tag } else { c.byte()? };
        if item == 0x00 {
            ts.attestations.push(parse_attestation(c)?);
        } else {
            let op = parse_op(c, item)?;
            let sub = parse_ts(c, depth + 1)?;
            ts.ops.push((op, sub));
        }
        if last {
            return Ok(ts);
        }
        tag = c.byte()?;
    }
}

/// Parse a `.ots` file.
pub fn parse(file: &[u8]) -> Result<Proof, String> {
    let mut c = Cur { b: file, at: 0 };
    if c.take(MAGIC.len())? != &MAGIC[..] {
        return Err("not an OpenTimestamps proof (magic)".to_string());
    }
    let v = c.varint()?;
    if v != 1 {
        return Err(format!("proof version {v}; this reader knows 1"));
    }
    let digest_op = c.byte()?;
    if digest_op != OP_SHA256 {
        return Err(format!("the file's digest op is 0x{digest_op:02x}, not sha256"));
    }
    let digest = c.take(32)?.to_vec();
    let ts = parse_ts(&mut c, 0)?;
    if c.at != file.len() {
        return Err(format!("{} trailing bytes after the timestamp", file.len() - c.at));
    }
    Ok(Proof { digest, ts })
}

// ── bytes out ────────────────────────────────────────────────────────────────────────────

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let x = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(x);
            return;
        }
        out.push(x | 0x80);
    }
}

fn put_varbytes(out: &mut Vec<u8>, b: &[u8]) {
    put_varint(out, b.len() as u64);
    out.extend_from_slice(b);
}

fn put_attestation(out: &mut Vec<u8>, a: &Attestation) {
    let mut payload = Vec::new();
    match a {
        Attestation::Bitcoin { height } => {
            out.extend_from_slice(&TAG_BITCOIN);
            put_varint(&mut payload, *height);
        }
        Attestation::Pending { uri } => {
            out.extend_from_slice(&TAG_PENDING);
            put_varbytes(&mut payload, uri.as_bytes());
        }
        Attestation::Unknown { tag } => out.extend_from_slice(tag),
    }
    put_varbytes(out, &payload);
}

fn put_op(out: &mut Vec<u8>, op: &Op) {
    match op {
        Op::Sha256 => out.push(OP_SHA256),
        Op::Append(b) => {
            out.push(OP_APPEND);
            put_varbytes(out, b);
        }
        Op::Prepend(b) => {
            out.push(OP_PREPEND);
            put_varbytes(out, b);
        }
        Op::Reverse => out.push(OP_REVERSE),
        Op::Hexlify => out.push(OP_HEXLIFY),
        Op::Unsupported(t) => out.push(*t),
    }
}

fn put_ts(out: &mut Vec<u8>, ts: &Ts) -> Result<(), String> {
    let n = ts.attestations.len() + ts.ops.len();
    if n == 0 {
        return Err("a timestamp with neither attestation nor op cannot be written".to_string());
    }
    let mut i = 0;
    for a in &ts.attestations {
        i += 1;
        if i < n {
            out.push(0xff);
        }
        out.push(0x00);
        put_attestation(out, a);
    }
    for (op, sub) in &ts.ops {
        i += 1;
        if i < n {
            out.push(0xff);
        }
        put_op(out, op);
        put_ts(out, sub)?;
    }
    Ok(())
}

/// Serialize a proof to `.ots` bytes.
pub fn serialize(p: &Proof) -> Result<Vec<u8>, String> {
    if p.digest.len() != 32 {
        return Err("the digest is not 32 bytes".to_string());
    }
    let mut out = Vec::with_capacity(64 + 256);
    out.extend_from_slice(&MAGIC);
    put_varint(&mut out, 1);
    out.push(OP_SHA256);
    out.extend_from_slice(&p.digest);
    put_ts(&mut out, &p.ts)?;
    Ok(out)
}

/// A bare serialized timestamp -- what a calendar replies with -- read from the start of `b`.
pub(super) fn parse_ts_bytes(b: &[u8]) -> Result<Ts, String> {
    parse_ts(&mut Cur { b, at: 0 }, 0)
}
