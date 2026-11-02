//! THE MERGE ON A SEALED HUB (W-MG-M2, M2-min of docs/design/M2-merge-design.md): two copies of one
//! file, no server. `Doc::merge_into` lays the union, nothing re-signed; this names the conflict
//! and seals one MERGE under the merging device's key. GLUING DETECTS AND NEVER RESOLVES: both
//! sides' movements stay in the file, an obstruction is a sentence for a person (§7).
//!
//! THE CONFLICT IS `glue::delta`'s, as it is on the hub's own stock log: `Conflict`, in
//! `glue::merge`'s words. `hub` / `kitchen` in those words read here as ours / theirs.
//!
//! THE CONFLICT LIST IS A SIGNED CLAIM, AND IT CAN BE CHECKED AGAINST ITS OWN ROWS. `verify` has no
//! stock semantics. But a MERGE's two lists of rows fix both sides, so whoever holds the file folds
//! the same two sides and asks the same question: `merge_lines` does, and a key holder who signed
//! "no conflict" over the rows of the last salmon reads `recheck=DIFFERS`, which `doc_verify`
//! refuses. ONE FILE CANNOT PROVE WHAT THE SIGNER SAW. The rows are the signer's too: rows that name
//! less than the file holds make a smaller question with a true answer. All `recheck` asks of them
//! is that each list holds a record the other lacks, which every honest MERGE does; past that, it
//! is the limit of a stripped MERGE (`atk5_silent`). `same` says the list follows from the rows,
//! never that the rows are all there was. Under a cut the base level may be gone: `unchecked`,
//! which `doc_verify` never reads as clean, because a one-record cut would launder a lie.
//!
//! WHAT THE MERGE SAYS (§2d's frame): `MERGE_SENTENCE`, which `doc_verify` prints.

use einstore::evlog::doc::{merge_body, parse_merge, Doc, DocError, Ids, MergeView, Trust, STATE_COMMUTES, STATE_OBSTRUCTED};
use einstore::evlog::Record;
use einstore::Store;

use super::history::KIND_DECL;
use super::sides::{canonical, conflicts_at, is_stock, keyed, two_tails};
use super::{hex32, DeviceKey, HubIds, MlDsa65, SealedHub, MIN_SEALED_BYTES};
use crate::glue::merge::conflict_body;
use crate::glue::Conflict;
use crate::minijson::{int_field, str_field};
use crate::{e_is_full, HubError};

/// What a verifier prints under a file holding a MERGE (design §7, §2d of the plan).
pub const MERGE_SENTENCE: &str = "Цей файл звів власник ключа K. Він стверджує, що бачив обидві історії повністю. Це не означає, що рухи товару справді сталися, і не доводить часу.";

/// What a verifier prints under a MERGE it could not re-derive (rc 4, `doc_verify`).
pub const UNCHECKED_SENTENCE: &str = "Цей файл обрізано, тож конфлікти MERGE не перевірено: це не означає, що їх немає. Перевірте за архівом.";

/// Why a merge wrote nothing.
#[derive(Debug, PartialEq)]
pub enum MergeError {
    /// The library's named refusal: nothing written.
    Refused(DocError),
    /// The file would not load, or the commit of the MERGE was refused.
    Hub(String),
    /// The quantities of the two sides do not fit the arithmetic: a level past `i64`. Nothing else:
    /// a side is read with the door open (`StockLedger::history`), so no conflict and no shelf
    /// below zero ever lands here, and only a key holder's absurd quantity does.
    Ledger(String),
}

impl From<HubError> for MergeError {
    fn from(e: HubError) -> Self {
        MergeError::Hub(format!("{e:?}"))
    }
}

/// What a merge did.
#[derive(Debug, PartialEq, Eq)]
pub enum Merged {
    /// This file already held everything: not a byte moved.
    Nothing,
    /// One history (or their tail held only MERGEs): their records taken in, no MERGE owed.
    Adopted { records: usize },
    /// Two histories glued under one MERGE of this device. `conflicts` empty = they commute.
    Sealed { records: usize, conflicts: Vec<Conflict> },
}

/// A conflict as the bytes a MERGE carries.
fn named(c: &Conflict) -> Vec<u8> {
    conflict_body(c).into_bytes()
}

fn oldest_first(sh: &SealedHub) -> Vec<Record> {
    let mut recs = sh.records();
    recs.reverse();
    recs
}

impl SealedHub {
    /// Glue the file `theirs` onto this one, sealing as `key` if two histories met. A refusal
    /// changes nothing: the rebuilt image takes this one's place only with its MERGE committed.
    /// Our roots are this file's genesis keys, and `theirs` answers to them.
    pub fn merge(&mut self, theirs: &[u8], key: &DeviceKey, claimed_ms: i64) -> Result<Merged, MergeError> {
        let other = SealedHub::load(theirs)?;
        let (a, b) = (self.store.writer_bytes().map_err(HubError::from)?, other.store.writer_bytes().map_err(HubError::from)?);
        let mut size = a.saturating_add(b).max(MIN_SEALED_BYTES);
        let (fresh, u) = loop {
            let mut fresh = Store::create_bytes(size).map_err(HubError::from)?;
            match Doc::merge_into(&self.store, &other.store, &mut fresh, &HubIds, &MlDsa65, Trust::Genesis) {
                Ok(u) => break (fresh, u),
                Err(DocError::Store(e)) if e_is_full(&e) => size = size.saturating_mul(2),
                Err(e) => return Err(MergeError::Refused(e)),
            }
        };
        if !u.wrote {
            return Ok(Merged::Nothing);
        }
        let records = u.their_tail.len();
        if !u.both_wrote {
            self.store = fresh;
            return Ok(Merged::Adopted { records });
        }
        let probe = SealedHub { store: fresh, entropy: Default::default() };
        let conflicts = conflicts_at(&canonical(oldest_first(&probe)), &keyed(&probe), &u.ours, &u.theirs).map_err(MergeError::Ledger)?;
        let state = if conflicts.is_empty() { STATE_COMMUTES } else { STATE_OBSTRUCTED };
        let view = MergeView { ours: u.ours, theirs: u.theirs, state, seg_root: u.seg_root, conflicts: conflicts.iter().map(named).collect() };
        let mut body = vec![KIND_DECL];
        body.extend_from_slice(&merge_body(&view));
        self.swap_and_declare(probe.store, body, key, claimed_ms)?;
        Ok(Merged::Sealed { records, conflicts })
    }

    /// A declaration record with any body, as `key` signs it: what a key holder can always write
    /// (`Doc::commit_bytes` takes any record), so a test and the gate can write a MERGE that lies.
    pub fn declare(&mut self, body: &[u8], key: &DeviceKey, claimed_ms: i64) -> Result<i64, HubError> {
        let mut framed = vec![KIND_DECL];
        framed.extend_from_slice(body);
        self.commit_body(framed, key, claimed_ms, None)
    }

    /// Every MERGE in the file, oldest first, each re-derived where the records allow it.
    pub fn merge_lines(&self) -> Vec<MergeLine> {
        let recs = oldest_first(self);
        let (order, known) = (canonical(recs.clone()), keyed(self));
        let cut = Doc::open(&self.store).ok().is_some_and(|v| v.seg.is_some());
        let mut out = Vec::new();
        for r in &recs {
            let body = HubIds.body(r.flags, &r.payload);
            if !body.starts_with(einstore::evlog::doc::MERGE_MARK) {
                continue;
            }
            let view = parse_merge(body);
            let recheck = match &view {
                None => Recheck::Differs,
                Some(_) if cut => Recheck::Unchecked,
                Some(v) if !two_tails(&order, &known, &v.ours, &v.theirs) => Recheck::Differs,
                Some(v) => match conflicts_at(&order, &known, &v.ours, &v.theirs) {
                    Ok(cs) if cs.iter().map(named).collect::<Vec<_>>() == v.conflicts => Recheck::Same,
                    _ => Recheck::Differs,
                },
            };
            out.push(MergeLine { actor: r.actor_pubkey, seq: r.actor_seq, view, recheck });
        }
        out
    }
}

/// `merge_lines`' answer about a conflict list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recheck {
    Same,
    Differs,
    /// The file was cut: the records of a side may have left. Not clean: `doc_verify` exits 4.
    Unchecked,
}

/// One MERGE as read. `view` `None`: it carries the mark and is not a MERGE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeLine {
    pub actor: [u8; 32],
    pub seq: u64,
    pub view: Option<MergeView>,
    pub recheck: Recheck,
}

/// A named conflict as a sentence for a person (design §7). The text of `glue::merge::conflict_body`
/// is the key list of one flat object.
pub fn conflict_words(text: &str) -> String {
    let json = format!("{{{text}}}");
    let item = str_field(&json, "item").unwrap_or_default();
    let qty = int_field(&json, "qty").unwrap_or(0);
    match str_field(&json, "kind").as_deref() {
        Some("oversold") => format!("{item}: на двох пристроях використано більше, ніж було; разом вийшло {qty}. Перерахуйте полицю."),
        Some("recount") => format!("{item}: на одному пристрої полицю перерахували, а на іншому її тим часом рухали. Який залишок правильний, вирішує лише перерахунок."),
        Some("double") if str_field(&json, "what").as_deref() == Some("served") => format!("{item}: замовлення {} продано на двох пристроях. Перевірте, що насправді видали.", str_field(&json, "order").unwrap_or_default()),
        Some("double") => format!("{item}: одну резервацію замовлення {} використано двічі. Перевірте, що насправді видали.", str_field(&json, "order").unwrap_or_default()),
        Some("provisional") => format!("{item}: продано понад частку ({qty}). Підтвердьте або скасуйте."),
        _ => format!("{item}: дві історії не сходяться. Потрібне рішення людини."),
    }
}

/// `doc_verify`'s lines about the MERGEs, and whether every conflict list re-derives. Not clean also
/// when one could not be asked (`Unchecked`): nobody has checked it, so nobody may read it as
/// checked. `holds` false asks nothing: a file a stranger laid gets no fold per MERGE (HOLE 12), and
/// it is refused already. No MERGE, no line: a file that never merged prints what it always printed.
pub fn merge_report(sh: &SealedHub, holds: bool) -> (Vec<String>, bool) {
    if !holds {
        return (vec!["merge recheck skipped: the file does not hold".to_string()], false);
    }
    let (mut out, mut clean) = (Vec::new(), true);
    let lines = sh.merge_lines();
    for l in &lines {
        let recheck = match l.recheck {
            Recheck::Same => "same",
            Recheck::Differs => "DIFFERS",
            Recheck::Unchecked => "unchecked",
        };
        clean &= l.recheck == Recheck::Same;
        let Some(v) = &l.view else {
            out.push(format!("merge actor={} seq={} state=unreadable recheck={recheck}", hex32(&l.actor), l.seq));
            continue;
        };
        let state = if v.state == STATE_OBSTRUCTED { "obstructed" } else { "commutes" };
        out.push(format!("merge actor={} seq={} state={state} ours={} theirs={} conflicts={} recheck={recheck}", hex32(&l.actor), l.seq, v.ours.len(), v.theirs.len(), v.conflicts.len()));
        for c in &v.conflicts {
            let text = String::from_utf8_lossy(c);
            out.push(format!("conflict seq={} {{{text}}} -> {}", l.seq, conflict_words(&text)));
        }
    }
    if !lines.is_empty() {
        let read = sh.stock_events().len();
        out.push(format!("stock movements={read} unreadable={}", sh.records().iter().filter(|r| is_stock(r)).count().saturating_sub(read)));
        out.push(MERGE_SENTENCE.to_string());
    }
    if lines.iter().any(|l| l.recheck == Recheck::Unchecked) {
        out.push(UNCHECKED_SENTENCE.to_string());
    }
    (out, clean)
}

/// The refusal's name, for a program's last line: the words `tools/gates/maingoal.sh` greps.
pub fn refusal_name(e: &MergeError) -> String {
    match e {
        MergeError::Refused(DocError::ForeignRoot) => "FOREIGN_ROOT: these are files of different documents".to_string(),
        MergeError::Refused(DocError::Fork { actor, seq, .. }) => format!("FORK: device {} holds two histories from record {seq}", hex32(actor)),
        MergeError::Refused(DocError::KeyClash(a)) => format!("KEY_CLASH: {}", hex32(a)),
        MergeError::Refused(DocError::CutOneSide { .. }) => "CUT_ONE_SIDE: the two files do not share one cut".to_string(),
        MergeError::Refused(DocError::ForgetOneSide) => "FORGET_ONE_SIDE: an erasure is on one side only".to_string(),
        MergeError::Refused(DocError::RevokedTail { actor, after_seq }) => format!("REVOKED_TAIL: device {} wrote past its REVOKE after {after_seq}", hex32(actor)),
        MergeError::Refused(DocError::LegacyTail { seq, .. }) => format!("LEGACY_TAIL: record {seq}"),
        MergeError::Refused(DocError::UnionNotWhole) => "UNION_NOT_WHOLE: each file holds and the two together do not".to_string(),
        MergeError::Refused(DocError::SealFork { actor, seq }) => format!("SEAL_FORK: device {} sealed one history twice at record {seq}", hex32(actor)),
        MergeError::Refused(DocError::NotWhole { ours }) => format!("NOT_WHOLE: {} file does not hold", if *ours { "our" } else { "their" }),
        MergeError::Ledger(why) => format!("LEDGER: the quantities do not fit: {why}"),
        other => format!("REFUSED: {other:?}"),
    }
}
