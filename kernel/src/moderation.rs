//! BLUEPRINT-P74 (M1 / M3) — per-hub moderation **reports**.
//!
//! P74 ships a thin, honest moderation surface. A *report* is a customer or
//! courier flagging abusive/illegal *content* or an abusive *actor* on a
//! **specific hub**. The report rides the EXISTING content-addressed event log
//! (`event_log.rs`) via `commit_after_decide` — it adds NO new store, no new
//! hot path. This module owns only the report payload types + the abuse-category
//! `decide` validator (the Law pole that rejects malformed/quality-smuggled
//! reports before anything persists).
//!
//! # Anti-scope (load-bearing, type-level)
//!
//! The `ReportReason` enum has **no quality/service variant**. A "slow
//! service" / "cold food" / "wrong order" complaint is UNREPRESENTABLE here
//! and routes to the §16.29 vendor+payment-provider dispute channel instead.
//! This is how §16.59's no-vendor-quality-bar red line becomes a *type*, not a
//! rule: quality data has no shape in this module, so it can never be confused
//! with abuse data.
//!
//! The REPORTER is the event's own `actor_pubkey` (a customer or courier) — no
//! separate reporter field, no reporter reputation (§16.59). A report is a fact
//! ("actor X flagged for reason R by actor Y"), never a verdict. The hub
//! operator's human judgment is the only adjudication; Wave-0 does not auto-act,
//! does not rate reporters, does not verify a report's truth.
//!
//! # No-scoring guarantee (the point of P74)
//!
//! A report is neutral plumbing: it carries an identity (`actor_pubkey`), never a
//! score. It cannot reach the HRW matcher or any discovery signal — that is
//! enforced structurally (M4): the matching path lives in a different crate with
//! no dependency edge to this module, and `ReportTarget`/`ReportReason` carry no
//! `score/rating/reputation/rank` field (the field-name CI guard).

use crate::event_log::MeshEvent;
use std::fmt;

/// Bounded free-text note on a report (opaque UTF-8). Scaling axis: report
/// volume (event-log rows); this bound keeps one report O(constant)-sized.
pub const MAX_REPORT_TEXT_BYTES: usize = 2048;

/// Domain-separated, prefix-tagged TLV signing/encoding namespace for reports.
/// A report's bytes are content-addressed only within this domain; a payload
/// that does not carry this prefix is `Undecodable` (no cross-type reuse).
pub const DOMAIN_REPORT: &[u8] = b"dowiz.report\x01";

/// Abuse-only report reasons. There is deliberately NO quality/service variant:
/// a "slow"/"cold"/"wrong order" complaint is UNREPRESENTABLE here and routes to
/// the §16.29 dispute channel instead. This enum IS the type-level enforcement of
/// §16.59's no-quality-bar red line — moderation data and quality data cannot be
/// confused because quality data has no shape in this type.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportReason {
    IllegalContent = 0,
    Fraud = 1,
    Harassment = 2,
    Impersonation = 3,
    /// CSAM / exploitation — decode-flagged for legal escalation (M3). This is the
    /// single honest lever dowiz's architecture permits: a *prompt to the hub
    /// operator* (never dowiz) to escalate through their own legal channel.
    ExploitativeContent = 4,
    Spam = 5,
    Other = 255,
}

impl ReportReason {
    /// Decode a reason byte; `None` for any byte outside the abuse enum (this is
    /// exactly the §16.59 quality-code smuggle attempt → `UnknownReason`).
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::IllegalContent),
            1 => Some(Self::Fraud),
            2 => Some(Self::Harassment),
            3 => Some(Self::Impersonation),
            4 => Some(Self::ExploitativeContent),
            5 => Some(Self::Spam),
            255 => Some(Self::Other),
            _ => None,
        }
    }

    /// Legal-escalation flag (M3): only `ExploitativeContent` decode-flags for
    /// the *hub operator* to escalate through their own legal channel. dowiz
    /// never reads hub content (§16.14), so this is the only honest lever — a
    /// prompt to a human, never an automated action or a content read.
    pub fn is_legal_escalation(&self) -> bool {
        matches!(self, Self::ExploitativeContent)
    }
}

impl fmt::Display for ReportReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::IllegalContent => "IllegalContent",
            Self::Fraud => "Fraud",
            Self::Harassment => "Harassment",
            Self::Impersonation => "Impersonation",
            Self::ExploitativeContent => "ExploitativeContent",
            Self::Spam => "Spam",
            Self::Other => "Other",
        };
        f.write_str(s)
    }
}

/// What a report is *about*. Content targets (hub/vendor/item) and actor targets
/// (a courier or customer identity). No target carries a score or count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportTarget {
    Hub,
    /// vendor_id (P62's row-scoping key).
    Vendor([u8; 16]),
    /// content-hash of the flagged item.
    CatalogItem([u8; 32]),
    /// courier/customer pubkey — identity, NOT a rating subject.
    Actor([u8; 32]),
}

/// The report body carried in a `MeshEvent.payload`. The REPORTER is the event's
/// own `actor_pubkey` (a customer or courier) — no separate reporter field, no
/// reporter reputation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPayload {
    pub target: ReportTarget,
    pub reason: ReportReason,
    pub note: Vec<u8>, // len <= MAX_REPORT_TEXT_BYTES
}

/// Typed rejection from the report `decide` validator (the Law pole, never
/// retried). A rejected report persists nothing (see `commit_after_decide`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportRejected {
    /// Payload bytes do not parse to a `ReportPayload` (missing domain prefix,
    /// truncated TLV, unknown target tag).
    Undecodable,
    /// Reason byte outside the abuse enum (a quality-code smuggle attempt).
    UnknownReason(u8),
    /// Note exceeds `MAX_REPORT_TEXT_BYTES`.
    NoteTooLong(usize),
}

impl fmt::Display for ReportRejected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Undecodable => write!(f, "report payload undecodable"),
            Self::UnknownReason(b) => {
                write!(f, "unknown report reason byte {b} (quality-code smuggle)")
            }
            Self::NoteTooLong(n) => write!(f, "report note too long: {n} > MAX_REPORT_TEXT_BYTES"),
        }
    }
}

impl ReportPayload {
    /// Canonical, domain-separated TLV encoding. Deterministic and injective in
    /// the struct fields, so two distinct reports always encode to distinct
    /// bytes (the event-log content-id is a hash of these bytes → idempotency).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN_REPORT);
        match &self.target {
            ReportTarget::Hub => out.push(0),
            ReportTarget::Vendor(id) => {
                out.push(1);
                out.extend_from_slice(id);
            }
            ReportTarget::CatalogItem(h) => {
                out.push(2);
                out.extend_from_slice(h);
            }
            ReportTarget::Actor(k) => {
                out.push(3);
                out.extend_from_slice(k);
            }
        }
        out.push(self.reason as u8);
        out.extend_from_slice(&(self.note.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.note);
        out
    }

    /// Decode canonical TLV → `ReportPayload`, or a typed `ReportRejected`.
    /// This is the byte-boundary enforcement of the type-level anti-scope: a
    /// reason byte outside the abuse enum is `UnknownReason`, never coerced.
    pub fn decode(bytes: &[u8]) -> Result<Self, ReportRejected> {
        if bytes.len() < DOMAIN_REPORT.len() || &bytes[..DOMAIN_REPORT.len()] != DOMAIN_REPORT {
            return Err(ReportRejected::Undecodable);
        }
        let mut cursor = DOMAIN_REPORT.len();
        let tag = *bytes.get(cursor).ok_or(ReportRejected::Undecodable)?;
        cursor += 1;
        let target = match tag {
            0 => ReportTarget::Hub,
            1 => {
                let id = bytes
                    .get(cursor..cursor + 16)
                    .ok_or(ReportRejected::Undecodable)?
                    .try_into()
                    .unwrap();
                cursor += 16;
                ReportTarget::Vendor(id)
            }
            2 => {
                let h = bytes
                    .get(cursor..cursor + 32)
                    .ok_or(ReportRejected::Undecodable)?
                    .try_into()
                    .unwrap();
                cursor += 32;
                ReportTarget::CatalogItem(h)
            }
            3 => {
                let k = bytes
                    .get(cursor..cursor + 32)
                    .ok_or(ReportRejected::Undecodable)?
                    .try_into()
                    .unwrap();
                cursor += 32;
                ReportTarget::Actor(k)
            }
            _ => return Err(ReportRejected::Undecodable),
        };
        let reason_byte = *bytes.get(cursor).ok_or(ReportRejected::Undecodable)?;
        cursor += 1;
        let reason =
            ReportReason::from_u8(reason_byte).ok_or(ReportRejected::UnknownReason(reason_byte))?;
        let note_len = {
            let l = bytes
                .get(cursor..cursor + 4)
                .ok_or(ReportRejected::Undecodable)?;
            cursor += 4;
            u32::from_le_bytes(l.try_into().unwrap()) as usize
        };
        let note = bytes
            .get(cursor..cursor + note_len)
            .ok_or(ReportRejected::Undecodable)?
            .to_vec();
        cursor += note_len;
        if note.len() > MAX_REPORT_TEXT_BYTES {
            return Err(ReportRejected::NoteTooLong(note.len()));
        }
        let _ = cursor;
        Ok(ReportPayload {
            target,
            reason,
            note,
        })
    }
}

/// The report `decide` Law: validate a candidate event's payload as a well-formed
/// abuse report BEFORE it is committed to the hub's event log. Used as the
/// `decide` closure to `EventLog::commit_after_decide`.
///
/// Returns `Err(ReportRejected)` on any malformed/quality-smuggled payload; the
/// event log then persists NOTHING (the Law pole — never retry). The reporter is
/// `ev.actor_pubkey`; this function never inspects or asserts on the reporter.
pub fn decide_report(ev: &MeshEvent) -> Result<(), ReportRejected> {
    ReportPayload::decode(&ev.payload)?;
    Ok(())
}

/// P74 (M1/M3) production caller — commit an abuse-report `MeshEvent` to a hub's
/// event log with [`decide_report`] wired as the validate-before-persist Law pole
/// of [`EventLog::commit_after_decide`]. This is the ONE kernel-local production
/// path that rides reports on the existing content-addressed log: a malformed or
/// quality-smuggled payload is Law-rejected and NOTHING persists; a well-formed
/// report is appended idempotently (a replay of the same content is a `Duplicate`
/// no-op and never re-runs the Law).
///
/// Before this, `decide_report` was only ever invoked from unit tests — this
/// closes the P74 wiring so the hub moderation surface actually gates commits.
pub fn commit_report<S: crate::event_log::EventStore>(
    log: &mut crate::event_log::EventLog<S>,
    ev: MeshEvent,
) -> Result<crate::event_log::AppendOutcome, crate::event_log::CommitError> {
    let (outcome, _decision) = log.commit_after_decide(ev, |e| decide_report(e))?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::{CommitError, EventLog, MemEventStore};

    fn reporter() -> [u8; 32] {
        [7u8; 32]
    }

    fn report_event(payload: &ReportPayload) -> MeshEvent {
        MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: reporter(),
            actor_seq: 1,
            payload: payload.encode(),
        }
    }

    #[test]
    fn m1_valid_report_commits_and_idempotent() {
        let mut log = EventLog::new(MemEventStore::new());
        let p = ReportPayload {
            target: ReportTarget::Actor([9u8; 32]),
            reason: ReportReason::Harassment,
            note: b"flagging repeated abuse".to_vec(),
        };
        let ev = report_event(&p);
        let (out, _dec) = log
            .commit_after_decide(ev.clone(), |e| decide_report(e))
            .expect("valid abuse report must commit");
        assert!(matches!(out, crate::event_log::AppendOutcome::Committed(_)));
        // Replay the SAME content: structural no-op (Duplicate), decide NOT re-run.
        let (out2, dec2) = log
            .commit_after_decide(ev, |e| decide_report(e))
            .expect("replay must be a no-op");
        assert!(matches!(
            out2,
            crate::event_log::AppendOutcome::Duplicate(_)
        ));
        assert!(dec2.is_none(), "duplicate must not re-run decide");
        assert_eq!(log.len(), 1, "one logical report = one row");
    }

    #[test]
    fn m1_quality_code_rejected_nothing_persists() {
        // Adversarial: reason byte 100 is outside the abuse enum — a §16.59
        // quality-code smuggle attempt. Must be Law-rejected and persist nothing.
        let mut log = EventLog::new(MemEventStore::new());
        let mut bytes = DOMAIN_REPORT.to_vec();
        bytes.push(0); // Hub target
        bytes.push(100); // UnknownReason
        bytes.extend_from_slice(&0u32.to_le_bytes()); // empty note
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: reporter(),
            actor_seq: 1,
            payload: bytes,
        };
        let res = log.commit_after_decide(ev, |e| decide_report(e));
        assert!(
            matches!(res, Err(CommitError::Rejected(_))),
            "quality-code report must be Law-rejected"
        );
        assert_eq!(log.len(), 0, "rejected report persists nothing");
    }

    #[test]
    fn m1_note_too_long_rejected() {
        let mut log = EventLog::new(MemEventStore::new());
        let p = ReportPayload {
            target: ReportTarget::Hub,
            reason: ReportReason::Spam,
            note: vec![0u8; MAX_REPORT_TEXT_BYTES + 1],
        };
        let ev = report_event(&p);
        let res = log.commit_after_decide(ev, |e| decide_report(e));
        assert!(matches!(res, Err(CommitError::Rejected(_))));
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn m1_decode_rejects_quality_reason() {
        // Direct decode-level assertion (the rejection pole, exercised at bytes).
        let mut bytes = DOMAIN_REPORT.to_vec();
        bytes.push(3); // Actor target
        bytes.extend_from_slice(&[1u8; 32]); // actor key
        bytes.push(100); // unknown reason
        bytes.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            ReportPayload::decode(&bytes),
            Err(ReportRejected::UnknownReason(100))
        );
    }

    #[test]
    fn m3_exploitative_content_decode_flags_operator_escalation() {
        // M3: only `ExploitativeContent` decode-flags for the hub operator to
        // escalate through their OWN legal channel. dowiz never reads hub content.
        assert!(ReportReason::ExploitativeContent.is_legal_escalation());
        assert!(!ReportReason::IllegalContent.is_legal_escalation());
        assert!(!ReportReason::Fraud.is_legal_escalation());
        assert!(!ReportReason::Spam.is_legal_escalation());
    }
}
