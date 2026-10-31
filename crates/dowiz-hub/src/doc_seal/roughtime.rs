//! ANCHOR KIND 3: ROUGHTIME (R-MAINGOAL-A §2d, "level ½"): one UDP request per commit when
//! there is a network, the signed `MIDP ± RADI` kept as the proof, and it is the ONE lower
//! bound the file can carry -- "these bytes existed no EARLIER than MIDP − RADI", because
//! the nonce the server signed over was derived from them.
//!
//! WHAT THIS BUILD CAN AND CANNOT CHECK, said plainly (card: no anchor verdict without a
//! real check). Roughtime's response is Ed25519 over an SREP that holds a SHA-512 Merkle
//! root over the nonces; neither Ed25519 nor SHA-512 exists in this tree (every primitive
//! here is ML-DSA/ML-KEM/SHA-256), and a verifier that did not check the signature would
//! be taking the server's word. So this adapter PARSES the response -- the tag map, MIDP,
//! RADI, that the proof's nonce is `nonce_for(over)` -- and returns `Unchecked` naming
//! what it read and what it did not check. It never says `Ok`. The day an Ed25519 verifier
//! and SHA-512 are in the tree, `check` grows the two calls and the verdict changes; the
//! proof format does not.
//!
//! Message format (https://roughtime.googlesource.com/roughtime/+/HEAD/PROTOCOL.md): u32 LE
//! tag count, u32 LE offsets (count − 1), u32 LE tags (ascending), then the values; tags
//! are the ASCII bytes LE-packed (`SIG\0`, `PATH`, `SREP`, `CERT`, `INDX`; inside SREP:
//! `RADI` u32 µs, `MIDP` u64 µs, `ROOT`). The proof an ANCH carries: `nonce (64) ‖ response`.

use bebop_store::evlog::doc::{Anchor, AnchorCheck, Verdict, KIND_ROUGHTIME};

pub const NONCE_BYTES: usize = 64;

/// The seam to a live server (UDP, port 2002 in the Google deployment): no implementation
/// in this crate -- the Worker or a device supplies one; the tests supply a fake.
pub trait RoughtimeServer {
    /// Send `nonce`, return the raw response message.
    fn query(&self, nonce: &[u8; NONCE_BYTES]) -> Result<Vec<u8>, String>;
}

/// The 64-byte nonce for a tip: `over ‖ sha256("bebop-roughtime/1" ‖ over)`. Binding, not
/// secret: the lower bound is on the bytes the nonce was derived from.
pub fn nonce_for(over: &[u8; 32]) -> [u8; NONCE_BYTES] {
    let mut m = Vec::with_capacity(18 + 32);
    m.extend_from_slice(b"bebop-roughtime/1");
    m.extend_from_slice(over);
    let h = bebop_store::nodekey::sha256(&m);
    let mut n = [0u8; NONCE_BYTES];
    n[..32].copy_from_slice(over);
    n[32..].copy_from_slice(&h);
    n
}

/// The proof blob: `nonce ‖ response`.
pub fn proof(nonce: &[u8; NONCE_BYTES], response: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(NONCE_BYTES + response.len());
    p.extend_from_slice(nonce);
    p.extend_from_slice(response);
    p
}

/// Query `server` for `over` and build the proof.
pub fn stamp(server: &dyn RoughtimeServer, over: &[u8; 32]) -> Result<Vec<u8>, String> {
    let n = nonce_for(over);
    Ok(proof(&n, &server.query(&n)?))
}

const fn tag(s: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*s)
}
pub const TAG_SREP: u32 = tag(b"SREP");
pub const TAG_MIDP: u32 = tag(b"MIDP");
pub const TAG_RADI: u32 = tag(b"RADI");

/// A Roughtime message's tag -> value slices. `None` on any claim the bytes cannot hold.
pub fn parse_map(m: &[u8]) -> Option<Vec<(u32, &[u8])>> {
    let u32_at = |i: usize| m.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let n = u32_at(0)? as usize;
    if n == 0 || n > 64 {
        return None;
    }
    let head = 4 + 4 * (n - 1) + 4 * n;
    let values = m.get(head..)?;
    let mut offsets = Vec::with_capacity(n + 1);
    offsets.push(0usize);
    for i in 0..n - 1 {
        let o = u32_at(4 + 4 * i)? as usize;
        if o % 4 != 0 || o < *offsets.last()? || o > values.len() {
            return None;
        }
        offsets.push(o);
    }
    offsets.push(values.len());
    let mut out: Vec<(u32, &[u8])> = Vec::with_capacity(n);
    for i in 0..n {
        let t = u32_at(4 + 4 * (n - 1) + 4 * i)?;
        if i > 0 && t <= out[i - 1].0 {
            return None;
        }
        out.push((t, values.get(offsets[i]..offsets[i + 1])?));
    }
    Some(out)
}

/// `(MIDP µs, RADI µs)` of a response, from its SREP.
pub fn midp_radi(response: &[u8]) -> Option<(u64, u32)> {
    let outer = parse_map(response)?;
    let srep = outer.iter().find(|(t, _)| *t == TAG_SREP)?.1;
    let inner = parse_map(srep)?;
    let midp = inner.iter().find(|(t, _)| *t == TAG_MIDP)?.1;
    let radi = inner.iter().find(|(t, _)| *t == TAG_RADI)?.1;
    let midp = u64::from_le_bytes(midp.get(..8)?.try_into().ok()?);
    let radi = u32::from_le_bytes(radi.get(..4)?.try_into().ok()?);
    Some((midp, radi))
}

/// The checker. Parses; never passes (see the header).
pub struct Roughtime;

impl AnchorCheck for Roughtime {
    fn check(&self, a: &Anchor) -> Verdict {
        if a.kind != KIND_ROUGHTIME {
            return Verdict::Unchecked(format!("kind {} is not a Roughtime response", a.kind));
        }
        let Some((nonce, response)) = a.proof.split_at_checked(NONCE_BYTES) else {
            return Verdict::Bad("the proof is shorter than a 64-byte nonce".to_string());
        };
        if nonce != nonce_for(&a.over) {
            return Verdict::Bad("the proof's nonce is not derived from `over`".to_string());
        }
        let Some((midp, radi)) = midp_radi(response) else {
            return Verdict::Bad("the response has no SREP with MIDP and RADI".to_string());
        };
        let lower_ms = (midp / 1000).saturating_sub((radi as u64).div_ceil(1000));
        Verdict::Unchecked(format!(
            "parsed: MIDP {midp} µs, RADI {radi} µs (no earlier than {lower_ms} ms IF the signature held); Ed25519 and SHA-512 are not in this build, so the server's signature and the Merkle path were NOT checked"
        ))
    }
}
