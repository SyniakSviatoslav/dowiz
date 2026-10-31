//! The writer's side of a live anchor (W-MG-M6b). `stamp_multi` submits ONE digest to several
//! calendars and merges the pending replies into ONE `.ots` -- every calendar's branch under the
//! same digest, as `ots-cli` does -- so a calendar that dies later costs nothing. `upgrade_file`
//! asks each pending calendar and says, by name, whether a Bitcoin attestation is now PRESENT;
//! present is not verified: only `verify` with the block headers says that. `stamp_image` /
//! `upgrade_image` do the same over a sealed image's tip, one ANCH per step, the log and the
//! seals untouched. A calendar that errors is reported, never dropped in silence; every calendar
//! failing is `Err` and there is NOTHING to write -- an offline run leaves the image as it was
//! (the kill test `an_offline_run_writes_no_anch`).

use std::collections::BTreeSet;

use bebop_store::evlog::doc::anch::NoAdapters;
use bebop_store::evlog::doc::KIND_OTS;

use crate::doc_seal::ots::{self, Attestation, Calendar, Proof, Reached, Ts};
use crate::doc_seal::{hex32, SealedHub};

/// One stamp: the merged `.ots` and, per calendar, the pending uri(s) it answered with or the
/// error it gave. At least one calendar accepted, or this value does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamped {
    pub ots: Vec<u8>,
    pub replies: Vec<(String, Result<String, String>)>,
}

/// The pending calendars a proof still waits on, newest branch first.
pub fn pending_uris(p: &Proof) -> Vec<String> {
    let mut reached = Vec::new();
    ots::reach(&p.ts, &p.digest, &mut reached);
    reached
        .into_iter()
        .filter_map(|r| match r {
            Reached::At { attestation: Attestation::Pending { uri }, .. } => Some(uri),
            _ => None,
        })
        .collect()
}

/// The Bitcoin blocks a proof attests (present in the file; checked only against headers).
pub fn bitcoin_heights(p: &Proof) -> Vec<u64> {
    let mut reached = Vec::new();
    ots::reach(&p.ts, &p.digest, &mut reached);
    reached
        .into_iter()
        .filter_map(|r| match r {
            Reached::At { attestation: Attestation::Bitcoin { height }, .. } => Some(height),
            _ => None,
        })
        .collect()
}

/// Submit `digest` to every calendar named; merge what accepted into one proof.
pub fn stamp_multi(cal: &dyn Calendar, calendars: &[&str], digest: &[u8; 32]) -> Result<Stamped, String> {
    if calendars.is_empty() {
        return Err("no calendar named".to_string());
    }
    let mut merged = Ts::default();
    let mut replies = Vec::with_capacity(calendars.len());
    let mut accepted = 0usize;
    for c in calendars {
        match ots::stamp(cal, c, digest).and_then(|b| ots::parse(&b)) {
            Ok(p) => {
                let uris = pending_uris(&p);
                merged.attestations.extend(p.ts.attestations);
                merged.ops.extend(p.ts.ops);
                accepted += 1;
                replies.push((c.to_string(), Ok(uris.join(","))));
            }
            Err(e) => replies.push((c.to_string(), Err(e))),
        }
    }
    if accepted == 0 {
        let why: Vec<String> = replies.iter().map(|(c, r)| format!("{c}: {}", r.clone().err().unwrap_or_default())).collect();
        return Err(format!("no calendar accepted the digest; nothing to write: {}", why.join("; ")));
    }
    let bytes = ots::serialize(&Proof { digest: digest.to_vec(), ts: merged })?;
    Ok(Stamped { ots: bytes, replies })
}

/// What an upgrade found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Upgraded {
    /// The proof now carries a Bitcoin attestation for `heights` (PRESENT, not checked), and
    /// may still wait on `still_pending`. `ots` is the new file to keep.
    Attested { ots: Vec<u8>, heights: Vec<u64>, still_pending: Vec<String> },
    /// Nothing attested yet; the calendars still pending. Nothing to write.
    Pending { uris: Vec<String> },
}

/// Ask each pending calendar; a changed file with no Bitcoin attestation is still `Pending`.
pub fn upgrade_file(cal: &dyn Calendar, file: &[u8]) -> Result<Upgraded, String> {
    let before = ots::parse(file)?;
    if !bitcoin_heights(&before).is_empty() {
        return Ok(Upgraded::Attested { ots: file.to_vec(), heights: bitcoin_heights(&before), still_pending: pending_uris(&before) });
    }
    match ots::upgrade(cal, file)? {
        None => Ok(Upgraded::Pending { uris: pending_uris(&before) }),
        Some(new) => {
            let p = ots::parse(&new)?;
            let heights = bitcoin_heights(&p);
            if heights.is_empty() {
                Ok(Upgraded::Pending { uris: pending_uris(&p) })
            } else {
                Ok(Upgraded::Attested { ots: new, heights, still_pending: pending_uris(&p) })
            }
        }
    }
}

/// Stamp a sealed image's tip: the new image bytes (one kind-2 ANCH appended; the log and the
/// seals untouched) and the proof. On any `Err` there are no bytes: the caller writes nothing.
pub fn stamp_image(image: &[u8], cal: &dyn Calendar, calendars: &[&str], time_ms: i64) -> Result<(Vec<u8>, Stamped), String> {
    let mut sh = SealedHub::load(image).map_err(|e| format!("not a sealed image this tool anchors: {e:?}"))?;
    let tip = sh.tip().ok_or("an empty log has no tip to anchor")?;
    let stamped = stamp_multi(cal, calendars, &tip)?;
    sh.anchor(KIND_OTS, &tip, time_ms, &stamped.ots).map_err(|e| format!("the ANCH was refused: {e:?}"))?;
    Ok((sh.to_bytes(), stamped))
}

/// Upgrade every pending kind-2 anchor of a sealed image: each one that now attests a block
/// gets a NEW ANCH over the same `over` with the same `time_ms` (anchors are never edited in
/// place). `None` bytes = nothing attested, nothing to write. The lines say what happened.
pub fn upgrade_image(image: &[u8], cal: &dyn Calendar) -> Result<(Option<Vec<u8>>, Vec<String>), String> {
    let mut sh = SealedHub::load(image).map_err(|e| format!("not a sealed image this tool upgrades: {e:?}"))?;
    let anchors = sh.anchors(&NoAdapters).map_err(|e| format!("anchors refused: {e:?}"))?;
    let mut lines = Vec::new();
    let mut changed = false;
    let attests = |proof: &[u8]| ots::parse(proof).map(|p| !bitcoin_heights(&p).is_empty()).unwrap_or(false);
    // The tips an attested ANCH already covers: an older pending ANCH over the same tip is
    // SUPERSEDED, not asked again (the old object stays pending forever; this is what makes
    // a second run write nothing).
    let covered: BTreeSet<[u8; 32]> = anchors.iter().filter(|c| c.anchor.kind == KIND_OTS && attests(&c.anchor.proof)).map(|c| c.anchor.over).collect();
    for c in anchors.iter().filter(|c| c.anchor.kind == KIND_OTS) {
        let a = &c.anchor;
        if covered.contains(&a.over) && !attests(&a.proof) {
            lines.push(format!("anchor over={}: pending, superseded by an attested ANCH over the same tip", hex32(&a.over)));
            continue;
        }
        match upgrade_file(cal, &a.proof)? {
            Upgraded::Attested { ots, heights, .. } if ots != a.proof => {
                sh.anchor(KIND_OTS, &a.over, a.time_ms, &ots).map_err(|e| format!("the ANCH was refused: {e:?}"))?;
                changed = true;
                lines.push(format!("anchor over={}: attestation PRESENT for Bitcoin block(s) {heights:?}; new ANCH written; verify it with --headers", hex32(&a.over)));
            }
            Upgraded::Attested { heights, .. } => lines.push(format!("anchor over={}: already attests Bitcoin block(s) {heights:?}", hex32(&a.over))),
            Upgraded::Pending { uris } => lines.push(format!("anchor over={}: still pending at {}", hex32(&a.over), uris.join(","))),
        }
    }
    Ok((changed.then(|| sh.to_bytes()), lines))
}
