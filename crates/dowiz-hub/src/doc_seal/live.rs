//! LEVEL-1 ANCHORS, LIVE (W-MG-M6b; R-MAINGOAL-A §3, §7 row M6): the seams of `ots.rs` filled
//! for an operator. A block-headers FILE the auditor brings (`FileHeaders`), one checker over
//! the three anchor kinds (`Adapters`), the verifier's reading of a sealed image or of a bare
//! `.ots` (`verify_image`, `verify_ots`), and in `stamp.rs` the multi-calendar stamp and the
//! upgrade. The LIVE HTTP side (`curl.rs`) sits behind the `anchor-live` feature and is the only
//! code here that talks to a service; the verifier never does -- the file plus what the auditor
//! brings, and nothing else (`bin/anchor.rs` is the operator's door).
//!
//! WHAT LEVEL 1 IS TODAY (2026-10-31): the seams with fakes in the tests, a real `Calendar` over
//! HTTPS behind the feature, a real headers file written by `fetch-headers`. NO platform mark is
//! issued by anyone (the platform key does not exist), and the hub is not on DOC in production,
//! so no cron has an input yet. A Bitcoin attestation takes hours: a fresh stamp is PENDING, and
//! pending is `Unchecked`, never ok.
//!
//! THE HEADERS FILE is `doc_verify`'s format, one block per line:
//! `<height> <merkle_root hex, 32 B in RAW HEADER BYTE ORDER> <time_ms>`; `#` starts a comment.
//! Explorers show a Merkle root byte-REVERSED (big-endian display); `header_line_from_explorer`
//! turns an explorer's block JSON into a line, reversing it, so the auditor copies nothing by
//! hand. The OTS attestation's message is the header's raw field (python-opentimestamps
//! `BitcoinBlockHeaderAttestation.verify_against_blockheader`: `hashMerkleRoot != msg`).

use bebop_store::evlog::doc::{Anchor, AnchorCheck, AnchorCounts, Checked, Report, Verdict, KIND_OTS, KIND_PLATFORM_TSA, KIND_ROUGHTIME};

use super::ots::{self, BlockHeaders, Ots};
use super::roughtime::Roughtime;
use super::tsa::PlatformTsa;
use super::SealedHub;
use crate::HubError;

pub mod stamp;
#[cfg(feature = "anchor-live")]
pub mod curl;
#[cfg(test)]
mod tests;

/// Block headers from a file the auditor brings. Nothing here reaches a network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHeaders {
    rows: Vec<(u64, [u8; 32], i64)>,
}

impl FileHeaders {
    /// Parse the file; one bad line refuses the whole file by its number (an auditor's input
    /// with a typo must not silently lose the one block the proof needs).
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut rows = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let body = line.split('#').next().unwrap_or("").trim();
            if body.is_empty() {
                continue;
            }
            let f: Vec<&str> = body.split_whitespace().collect();
            let row = (|| {
                let h: u64 = f.first()?.parse().ok()?;
                let root: [u8; 32] = crate::crypto::unhex(f.get(1)?)?.try_into().ok()?;
                let t: i64 = f.get(2)?.parse().ok()?;
                (f.len() == 3).then_some((h, root, t))
            })();
            match row {
                Some(r) => rows.push(r),
                None => return Err(format!("headers line {}: cannot read {line:?} (want `<height> <merkle_root raw hex 32 B> <time_ms>`)", n + 1)),
            }
        }
        Ok(FileHeaders { rows })
    }

    /// One line of the file.
    pub fn line(height: u64, root_raw: &[u8; 32], time_ms: i64) -> String {
        format!("{height} {} {time_ms}", crate::crypto::hex(root_raw))
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

impl BlockHeaders for FileHeaders {
    fn merkle_root(&self, height: u64) -> Option<[u8; 32]> {
        self.rows.iter().find(|r| r.0 == height).map(|r| r.1)
    }
    fn time_ms(&self, height: u64) -> Option<i64> {
        self.rows.iter().find(|r| r.0 == height).map(|r| r.2)
    }
}

/// An explorer's block JSON (blockstream.info / mempool.space `GET /api/block/<hash>`:
/// `"height"`, `"merkle_root":"<display hex>"`, `"timestamp":<unix seconds>`) as one headers
/// line for `height`: the root reversed into raw header order, the time in ms.
pub fn header_line_from_explorer(height: u64, block_json: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(block_json).map_err(|e| format!("block json: {e}"))?;
    if let Some(h) = v.get("height").and_then(|h| h.as_u64()) {
        if h != height {
            return Err(format!("the block json is height {h}, not {height}"));
        }
    }
    let display = v.get("merkle_root").and_then(|m| m.as_str()).ok_or("block json: no merkle_root")?;
    let mut raw = crate::crypto::unhex(display).ok_or("block json: merkle_root is not hex")?;
    if raw.len() != 32 {
        return Err(format!("block json: merkle_root is {} bytes, not 32", raw.len()));
    }
    raw.reverse();
    let secs = v.get("timestamp").and_then(|t| t.as_i64()).ok_or("block json: no timestamp")?;
    let root: [u8; 32] = raw.try_into().map_err(|_| "32".to_string())?;
    Ok(FileHeaders::line(height, &root, secs.saturating_mul(1000)))
}

/// One checker over the three kinds: the platform keys the auditor brings (kind 1), the block
/// headers the auditor brings (kind 2), Roughtime parse-only (kind 3, never ok here). A kind
/// with no adapter is `Unchecked` by name.
pub struct Adapters<'a> {
    pub keys: &'a [Vec<u8>],
    pub headers: Option<&'a dyn BlockHeaders>,
}

impl AnchorCheck for Adapters<'_> {
    fn check(&self, a: &Anchor) -> Verdict {
        match a.kind {
            KIND_PLATFORM_TSA => PlatformTsa { keys: self.keys }.check(a),
            KIND_OTS => Ots { headers: self.headers }.check(a),
            KIND_ROUGHTIME => Roughtime.check(a),
            k => Verdict::Unchecked(format!("no adapter for anchor kind {k}")),
        }
    }
}

/// What the verifier read: the eight counters and heads, every anchor with its verdict.
pub struct Outcome {
    pub report: Report,
    pub checked: Vec<Checked>,
    pub counts: AnchorCounts,
}

impl Outcome {
    /// The report holds and no anchor is bad. An `Unchecked` anchor neither holds nor breaks:
    /// it is a question the auditor did not bring the means to answer.
    pub fn holds(&self) -> bool {
        self.report.holds() && self.counts.bad == 0
    }
}

/// The verifier's reading of a sealed image: `load` (its refusals by name -- `BadCrc` for a
/// changed cell, `NotAHub`, `UnknownVersion`), `verify`, every anchor checked by `Adapters`.
pub fn verify_image(bytes: &[u8], keys: &[Vec<u8>], headers: Option<&dyn BlockHeaders>) -> Result<Outcome, HubError> {
    let sh = SealedHub::load(bytes)?;
    let report = sh.verify()?;
    let checked = sh.anchors(&Adapters { keys, headers })?;
    let counts = AnchorCounts::of(&checked);
    Ok(Outcome { report, checked, counts })
}

/// A bare `.ots` -- what `stamp <file>` leaves beside a plain file -- read as a kind-2 anchor
/// over its own digest: the same adapter, the same verdicts, no image around it.
pub fn verify_ots(file: &[u8], headers: Option<&dyn BlockHeaders>) -> Verdict {
    let digest = match ots::parse(file) {
        Ok(p) => p.digest,
        Err(e) => return Verdict::Bad(format!("the .ots does not parse: {e}")),
    };
    let over: [u8; 32] = match digest.try_into() {
        Ok(d) => d,
        Err(_) => return Verdict::Bad("the .ots digest is not 32 bytes".to_string()),
    };
    Ots { headers }.check(&Anchor { at: 0, kind: KIND_OTS, over, time_ms: 0, prev: None, proof: file.to_vec() })
}

/// One word and one reason per verdict for a line of output. `Unchecked` is never dressed as
/// ok: the word is `unchecked` and the reason names what was missing.
pub fn describe(v: &Verdict) -> (&'static str, String) {
    match v {
        Verdict::Ok { not_after_ms, not_before_ms, what } => ("ok", format!("{what}; not_after_ms={not_after_ms:?} not_before_ms={not_before_ms:?}")),
        Verdict::Bad(w) => ("BAD", w.clone()),
        Verdict::Unchecked(w) => ("unchecked", w.clone()),
    }
}
