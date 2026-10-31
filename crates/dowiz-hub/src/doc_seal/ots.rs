//! ANCHOR KIND 2: OPENTIMESTAMPS (R-MAINGOAL-A §3, choice 2): the nightly witness root --
//! the hub's tip at the census -- submitted to a public OTS calendar, the `.ots` proof kept
//! in an ANCH, upgraded the next night once the calendar's Bitcoin transaction is in a
//! block. Why OTS and not Rekor: no registration, no instance rotation, no TUF, the longest
//! life (Bitcoin headers are public everywhere); the weakness -- the verifier needs the
//! block header -- is accepted and NAMED: `BlockHeaders` is a seam the auditor fills from a
//! node, a header file, or a block explorer (and then the trust is in the explorer).
//!
//! THE FORMAT, re-derived from python-opentimestamps (`opentimestamps/core/{timestamp,
//! notary,op}.py`), serializer and parser both, and held to a HAND-WRITTEN byte vector in
//! the tests so the parser is checked against the published encoding and not against the
//! serializer beside it:
//!
//! ```text
//! file      = MAGIC (31 B) ‖ varint 1 ‖ 0x08 ‖ digest (32) ‖ timestamp
//! timestamp = (0xff item)* item                 item = 0x00 attestation | op timestamp
//! op        = 0xf0 varbytes (append) | 0xf1 varbytes (prepend) | 0xf2 (reverse)
//!           | 0xf3 (hexlify) | 0x08 (sha256) | 0x02 sha1 | 0x03 ripemd160 | 0x67 keccak256
//! attest.   = TAG (8 B) ‖ varbytes payload;  Bitcoin TAG 0588960d73d71901, payload = varint height
//!                                            Pending TAG 83dfe30d2ef90c8e, payload = varbytes uri
//! varint    = unsigned LEB128;  varbytes = varint len ‖ bytes
//! ```
//!
//! A Bitcoin attestation holds when the message the ops produce at it EQUALS the block's
//! Merkle root (the raw 32 bytes of the header's field). The calendar paths to Bitcoin use
//! sha256, append and prepend only; sha1 / ripemd160 / keccak256 are not in this tree and a
//! proof that needs one is `Unchecked` by that op's name, never guessed.
//!
//! THE LIVE CALLS -- `POST <calendar>/digest` with the 32 raw bytes, `GET <calendar>/
//! timestamp/<hex>` for the upgrade -- sit behind `Calendar`, a trait with NO implementation
//! here: the Worker's `fetch` or a cron script implements it. What was tested offline: the
//! file built from a fake calendar's pending response, the upgrade grafting a fake
//! calendar's Bitcoin branch onto that pending leaf, and the evaluation against a fake
//! header source -- the whole path except the two HTTP calls.

use bebop_store::evlog::doc::{Anchor, AnchorCheck, Verdict, KIND_OTS};
use bebop_store::nodekey::sha256;

pub const MAGIC: [u8; 31] = [
    0x00, b'O', b'p', b'e', b'n', b'T', b'i', b'm', b'e', b's', b't', b'a', b'm', b'p', b's', 0x00, 0x00, b'P', b'r', b'o', b'o', b'f', 0x00, 0xbf, 0x89, 0xe2, 0xe8, 0x84, 0xe8, 0x92, 0x94,
];
pub const TAG_BITCOIN: [u8; 8] = [0x05, 0x88, 0x96, 0x0d, 0x73, 0xd7, 0x19, 0x01];
pub const TAG_PENDING: [u8; 8] = [0x83, 0xdf, 0xe3, 0x0d, 0x2e, 0xf9, 0x0c, 0x8e];
pub const OP_SHA1: u8 = 0x02;
pub const OP_RIPEMD160: u8 = 0x03;
pub const OP_SHA256: u8 = 0x08;
pub const OP_KECCAK256: u8 = 0x67;
pub const OP_APPEND: u8 = 0xf0;
pub const OP_PREPEND: u8 = 0xf1;
pub const OP_REVERSE: u8 = 0xf2;
pub const OP_HEXLIFY: u8 = 0xf3;
/// python-opentimestamps' bounds on a message and an op argument.
pub const MAX_MSG: usize = 4096;
/// The public calendars (opentimestamps.org); the Worker chooses one per night.
pub const CALENDARS: [&str; 3] = ["https://a.pool.opentimestamps.org", "https://b.pool.opentimestamps.org", "https://a.pool.eternitywall.com"];

/// The LIVE seam. `submit` = `POST <calendar>/digest` (body: the 32 raw bytes; reply: a
/// serialized timestamp whose initial message is the digest); `upgrade` = `GET
/// <uri>/timestamp/<hex of msg>` (reply: a serialized timestamp for that message, or 404
/// while the calendar is still pending).
pub trait Calendar {
    fn submit(&self, calendar: &str, digest: &[u8; 32]) -> Result<Vec<u8>, String>;
    fn upgrade(&self, uri: &str, msg: &[u8]) -> Result<Option<Vec<u8>>, String>;
}

/// The seam to Bitcoin's headers, filled by the auditor: the raw 32 bytes of block
/// `height`'s Merkle-root field and the header's time (ms).
pub trait BlockHeaders {
    fn merkle_root(&self, height: u64) -> Option<[u8; 32]>;
    fn time_ms(&self, height: u64) -> Option<i64>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Sha256,
    Append(Vec<u8>),
    Prepend(Vec<u8>),
    Reverse,
    Hexlify,
    /// An op this tree has no implementation of; named so the verdict can name it.
    Unsupported(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attestation {
    Bitcoin { height: u64 },
    Pending { uri: String },
    Unknown { tag: [u8; 8] },
}

/// A timestamp tree: the attestations at this node and the branches below it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ts {
    pub attestations: Vec<Attestation>,
    pub ops: Vec<(Op, Ts)>,
}

/// A parsed `.ots` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    pub digest: Vec<u8>,
    pub ts: Ts,
}

mod wire;
pub use wire::{parse, serialize};

// ── evaluation ───────────────────────────────────────────────────────────────────────────

/// An op applied to a message; `None` for an unsupported op or a bound passed.
pub fn apply(op: &Op, msg: &[u8]) -> Option<Vec<u8>> {
    let out = match op {
        Op::Sha256 => sha256(msg).to_vec(),
        Op::Append(b) => [msg, b.as_slice()].concat(),
        Op::Prepend(b) => [b.as_slice(), msg].concat(),
        Op::Reverse => msg.iter().rev().copied().collect(),
        Op::Hexlify => crate::crypto::hex(msg).into_bytes(),
        Op::Unsupported(_) => return None,
    };
    (out.len() <= MAX_MSG).then_some(out)
}

/// Every attestation of the tree with the message the ops produce at it, or the op that
/// stopped the branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reached {
    At { attestation: Attestation, msg: Vec<u8> },
    Stopped { op: Op },
}

pub fn reach(ts: &Ts, msg: &[u8], out: &mut Vec<Reached>) {
    for a in &ts.attestations {
        out.push(Reached::At { attestation: a.clone(), msg: msg.to_vec() });
    }
    for (op, sub) in &ts.ops {
        match apply(op, msg) {
            Some(m) => reach(sub, &m, out),
            None => out.push(Reached::Stopped { op: op.clone() }),
        }
    }
}

// ── the calendar round trip ──────────────────────────────────────────────────────────────

/// `stamp`: submit `digest` to `calendar`, return the `.ots` bytes (a pending proof).
pub fn stamp(cal: &dyn Calendar, calendar: &str, digest: &[u8; 32]) -> Result<Vec<u8>, String> {
    let reply = cal.submit(calendar, digest)?;
    let ts = wire::parse_ts_bytes(&reply)?;
    serialize(&Proof { digest: digest.to_vec(), ts })
}

/// `upgrade`: ask each pending attestation's calendar for its timestamp and graft what it
/// returns onto that leaf. `Ok(None)` when nothing changed (every calendar still pending).
pub fn upgrade(cal: &dyn Calendar, file: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let mut p = parse(file)?;
    let digest = p.digest.clone();
    let changed = graft(cal, &mut p.ts, &digest)?;
    if !changed {
        return Ok(None);
    }
    serialize(&p).map(Some)
}

fn graft(cal: &dyn Calendar, ts: &mut Ts, msg: &[u8]) -> Result<bool, String> {
    let mut changed = false;
    let olds = std::mem::take(&mut ts.attestations);
    for a in olds {
        if let Attestation::Pending { uri } = &a {
            if let Some(reply) = cal.upgrade(uri, msg)? {
                let sub = wire::parse_ts_bytes(&reply)?;
                ts.attestations.extend(sub.attestations);
                ts.ops.extend(sub.ops);
                changed = true;
                continue;
            }
        }
        ts.attestations.push(a);
    }
    for (op, sub) in ts.ops.iter_mut() {
        if let Some(m) = apply(op, msg) {
            changed |= graft(cal, sub, &m)?;
        }
    }
    Ok(changed)
}

// ── the checker ──────────────────────────────────────────────────────────────────────────

/// The checker: block headers from the auditor, or none (then every Bitcoin attestation is
/// `Unchecked`, naming its block).
pub struct Ots<'a> {
    pub headers: Option<&'a dyn BlockHeaders>,
}

impl AnchorCheck for Ots<'_> {
    fn check(&self, a: &Anchor) -> Verdict {
        if a.kind != KIND_OTS {
            return Verdict::Unchecked(format!("kind {} is not an OpenTimestamps proof", a.kind));
        }
        let p = match parse(&a.proof) {
            Ok(p) => p,
            Err(e) => return Verdict::Bad(format!("the .ots does not parse: {e}")),
        };
        if p.digest != a.over {
            return Verdict::Bad("the .ots is over a different digest than `over`".to_string());
        }
        let mut reached = Vec::new();
        reach(&p.ts, &p.digest, &mut reached);
        let mut why = Vec::new();
        for r in &reached {
            match r {
                Reached::At { attestation: Attestation::Bitcoin { height }, msg } => match self.headers {
                    None => why.push(format!("attests Bitcoin block {height}; no block headers were given, so the block was not checked")),
                    Some(h) => match h.merkle_root(*height) {
                        None => why.push(format!("attests Bitcoin block {height}; the headers given do not hold that block")),
                        Some(root) if root.as_slice() == msg.as_slice() => {
                            return Verdict::Ok {
                                not_after_ms: h.time_ms(*height),
                                not_before_ms: None,
                                what: format!("the ops reach the Merkle root of Bitcoin block {height}"),
                            }
                        }
                        Some(_) => return Verdict::Bad(format!("the ops do not reach the Merkle root of Bitcoin block {height}")),
                    },
                },
                Reached::At { attestation: Attestation::Pending { uri }, .. } => why.push(format!("pending at {uri}: the calendar has not committed it yet")),
                Reached::At { attestation: Attestation::Unknown { tag }, .. } => why.push(format!("an attestation of unknown kind {}", crate::crypto::hex(tag))),
                Reached::Stopped { op } => why.push(format!("a branch needs op {op:?}, which this build does not have")),
            }
        }
        if why.is_empty() {
            why.push("the proof reaches no attestation".to_string());
        }
        Verdict::Unchecked(why.join("; "))
    }
}
