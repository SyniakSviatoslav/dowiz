// SPDX-License-Identifier: MIT OR Apache-2.0
//! The DOC half of `open` (W-MG-M5, FORMAT §9.4-9.5): `open_doc`, the counters `doc::verify`
//! answers under this reader's seams (`ids.rs`), and the SPEC blobs. Split from lib.rs so each
//! file stays small; the rules are still the library's, called.

use bebop_image::{Store, Superblock};
use bebop_kv::tree::VERSION_TREE;
use bebop_log::doc::anch::read_anchors;
use bebop_log::doc::seal::read_heads;
use bebop_log::doc::seg::read_segs;
use bebop_log::doc::spec::{read_blob, read_spec};
use bebop_log::doc::{self, Doc as DocRoot, DocError, Trust};
use bebop_log::EvLog;

use crate::{blank, ids, log_fold, log_records, tree_entries, Doc, Kind, Refusal};

/// `doc::verify`'s answer as the ABI hands it out (`abi::bw_counts`), sixteen numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DocCounts {
    pub records: i64,
    pub chained: i64,
    pub legacy: i64,
    pub redacted: i64,
    pub broken: i64,
    /// Every seal reachable from HEADS (ok + bad under a verifier that knows no algorithm).
    pub seals: i64,
    pub forks: i64,
    pub heads: i64,
    pub keys: i64,
    pub admitted: i64,
    /// 1 if this reader checked the signatures; 0 -- always, in this crate (`ids::NoVerifier`).
    pub checked: i64,
    /// `Report::holds()` as the library computed it under that verifier (0 here: unchecked
    /// seals are bad seals to the library; printed so a host cannot mistake a count for a proof).
    pub holds: i64,
    /// W-MG-M3/M4 (appended, so the twelve above keep their cells): segments under the DOC, the
    /// records they say left the file, the tombstones under them, what effective declarations say.
    pub segments: i64,
    pub cut: i64,
    pub cut_redacted: i64,
    pub declared: i64,
}

/// A DOC (FORMAT §9.4): the root's crc (`Doc::open`), its log as `open_log` reads one, HEADS
/// and KEYS crc-checked, the SPEC crc-checked when named, a v4 TREE read as `open_kv` reads
/// one. A TREE of another version is refused NOT_A_KV: this reader has no loader for a v1-v3
/// root that is not the image's root, and says so in words rather than guess.
pub(crate) fn open_doc(st: Store, sb: Superblock) -> Result<Doc, Refusal> {
    let view = DocRoot::open(&st).map_err(doc_refusal)?;
    let records = log_records(&st, view.log)?;
    for t in [view.heads, view.keys].into_iter().flatten() {
        st.check_obj(t).map_err(|b| Refusal::BadCrc { obj: b.obj })?;
    }
    let spec = match view.spec {
        Some(at) => {
            st.check_obj(at).map_err(|b| Refusal::BadCrc { obj: b.obj })?;
            Some(read_spec(&st, at).ok_or(Refusal::NotAKv)?)
        }
        None => None,
    };
    let entries = match view.tree {
        Some(t) => {
            st.check_obj(t).map_err(|b| Refusal::BadCrc { obj: b.obj })?;
            if st.get(t, 5) != VERSION_TREE {
                return Err(Refusal::NotAKv);
            }
            tree_entries(&st, t)?.0
        }
        None => Vec::new(),
    };
    // Every segment and anchor the DOC reaches, crc-checked here (verify re-reads the segments).
    read_segs(&st, view.seg, EvLog::step_cap(&st)).map_err(doc_refusal)?;
    let anchors = read_anchors(&st, view.anch);
    for a in &anchors {
        st.check_obj(a.at).map_err(|b| Refusal::BadCrc { obj: b.obj })?;
    }
    let heads = read_heads(&st, view.heads);
    Ok(Doc { fold: log_fold(&records), entries, records, heads, spec, anchors, ..blank(Kind::Doc, 0, st, sb) })
}

/// A DOC refusal in the one vocabulary: a crc is 10, a chain that never ends is 4, a segment no
/// TRUNCATE names is 75 (its own name: the file is whole, but records left it undeclared),
/// anything else the library refuses about the layout is NOT_A_KV (2).
fn doc_refusal(e: DocError) -> Refusal {
    match e {
        DocError::BadCrc(b) => Refusal::BadCrc { obj: b.obj },
        DocError::Corrupt("the log chain never ends") => Refusal::Truncated { claimed: 0, chained: None },
        DocError::CutUndeclared { .. } => Refusal::CutUndeclared,
        _ => Refusal::NotAKv,
    }
}

impl Doc {
    /// DOC only: `doc::verify` under this reader's seams (`ids.rs`), computed once and kept.
    /// `None` for a KV or a log.
    pub fn verify_doc(&mut self) -> Option<Result<DocCounts, Refusal>> {
        if self.kind != Kind::Doc {
            return None;
        }
        if let Some(c) = self.counts {
            return Some(Ok(c));
        }
        let rep = match doc::verify(&self.st, &ids::ReaderIds, &ids::NoVerifier, Trust::Genesis) {
            Ok(r) => r,
            Err(e) => return Some(Err(doc_refusal(e))),
        };
        let c = rep.counters;
        let counts = DocCounts {
            records: c.records as i64,
            chained: c.chained as i64,
            legacy: c.legacy as i64,
            redacted: c.redacted as i64,
            broken: c.broken as i64,
            seals: (c.seals_ok + c.seals_bad) as i64,
            forks: c.forks as i64,
            heads: rep.heads.len() as i64,
            keys: rep.keys as i64,
            admitted: rep.admitted as i64,
            checked: 0,
            holds: rep.holds() as i64,
            segments: c.segments as i64,
            cut: c.cut as i64,
            cut_redacted: c.cut_redacted as i64,
            declared: c.declared as i64,
        };
        self.counts = Some(counts);
        Some(Ok(counts))
    }

    /// DOC with a SPEC: the embedded reader's bytes, or `None` (no SPEC, or a blob that does
    /// not read -- its crc, its digest, its length).
    pub fn spec_wasm(&self) -> Option<Vec<u8>> {
        read_blob(&self.st, self.spec?.wasm?)
    }
    /// DOC with a SPEC: the format text, likewise.
    pub fn spec_text(&self) -> Option<Vec<u8>> {
        read_blob(&self.st, self.spec?.text?)
    }
}
