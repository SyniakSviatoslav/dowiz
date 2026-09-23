//! FORGETTING A PERSON IN THE CONSENT LOG (§3.3 step 2 of
//! `BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22`).
//!
//! NOTHING IS REMOVED. The proof that the venue STOPPED is the one record a
//! complaint asks for, so it must outlive the person: one `w`-less withdrawal
//! `{ state: withdrawn, method: erasure }` is appended for every purpose and
//! channel. The key is a pseudonym, and with the `people` card and the order
//! log's contact gone it identifies nobody.
//!
//! WHAT A PERSON CAN STILL BE FOUND BY IS REDACTED IN PLACE, the order log's
//! way (`crate::forget`): the `evidence` an owner typed ("signed at the counter,
//! Arben, +355 69 ...") and a `via` that is the person's own thread (a
//! WhatsApp peer IS the number). What stays is the ICO's proof -- who (the
//! pseudonym), when, how, what they were shown (`wordingId`), and, for a grant
//! an owner entered, WHICH owner (`via`): that is the venue's accountability,
//! not the subject's data. `id` and `prev` are kept, so the chain's links and
//! its tip do not move; such a record no longer matches its content id, and
//! [`chain_check`] counts it `redacted` when its link holds (`broken` when it
//! does not). The law is that the `f` declarations in this image name exactly
//! that many: an undeclared redaction fails `ConsentCheck::holds`.
//!
//! PURE. The instant is the caller's.

use super::{subject_of, Act, Method, State, CHANNELS, KIND_ACT, PURPOSES};
use crate::logimage::LogImage;
use crate::minijson::int_field;
use crate::{content_id_chained, HubError};
use bebop_store::evlog::{EvLog, Record};
use bebop_store::Store;

/// The declaration: how many acts of `cust:<key>` were redacted in place.
pub const KIND_FORGOTTEN: &str = "f";
/// What an erased `evidence` reads. NOT EMPTY, so an owner-entered grant is
/// still a record `check` accepts -- a grant that existed is part of the proof.
pub const ERASED: &str = "erased";

/// What one erasure did to the consent image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Forgot {
    /// Acts redacted in place by THIS run.
    pub redacted: usize,
    /// Erasure withdrawals appended by THIS run.
    pub withdrawn: usize,
}

/// Forget `key`: redact its acts in place, declare them, and append the
/// withdrawals. IDEMPOTENT: a second run finds nothing left to redact and every
/// purpose/channel already stopped by an erasure, and writes nothing.
pub fn forget(log: &mut LogImage, key: &str, at_ms: i64) -> Result<Forgot, String> {
    let redacted = redact_acts(log, key).map_err(|e| format!("consent redaction: {e:?}"))?;
    if redacted > 0 {
        log.append(KIND_FORGOTTEN, &subject_of(key), &format!("{{\"records\":{redacted},\"atMs\":{at_ms}}}"))
            .map_err(|e| format!("consent image refused the declaration: {e:?}"))?;
    }
    let entries = log.entries();
    let mut withdrawn = 0;
    for purpose in PURPOSES {
        for channel in CHANNELS {
            if newest_is_erasure(&entries, key, purpose, channel) {
                continue;
            }
            let act = Act {
                key: key.to_string(),
                purpose: purpose.into(),
                channel: channel.into(),
                state: State::Withdrawn,
                at_ms,
                method: Method::Erasure,
                evidence: String::new(),
                wording_id: String::new(),
                via: String::new(),
            };
            super::log::write(log, &act)?;
            withdrawn += 1;
        }
    }
    Ok(Forgot { redacted, withdrawn })
}

fn newest_is_erasure(entries: &[crate::logimage::Entry], key: &str, purpose: &str, channel: &str) -> bool {
    // `entries` is newest first; the newest act for the triple decides.
    entries
        .iter()
        .filter(|e| e.kind == KIND_ACT)
        .filter_map(|e| Act::parse(&e.json))
        .find(|a| a.key == key && a.purpose == purpose && a.channel == channel)
        .is_some_and(|a| a.state == State::Withdrawn && a.method == Method::Erasure)
}

/// `[kind_len][kind][subject_len][subject][json]` -- `logimage.rs`'s payload.
fn split(p: &[u8]) -> Option<(&[u8], &[u8], &[u8])> {
    let kl = *p.first()? as usize;
    let k = p.get(1..1 + kl)?;
    let sl = *p.get(1 + kl)? as usize;
    let s = p.get(2 + kl..2 + kl + sl)?;
    Some((k, s, p.get(2 + kl + sl..)?))
}

/// The act with the person taken out, or `None` when nothing in it names them.
fn scrubbed(act: &Act) -> Option<Act> {
    let mut a = act.clone();
    if !a.evidence.is_empty() && a.evidence != ERASED {
        a.evidence = ERASED.into();
    }
    // An order id and an owner id are not the subject's data; a thread is.
    if !matches!(a.method, Method::CheckoutBox | Method::OwnerEntered) {
        a.via.clear();
    }
    (a != *act).then_some(a)
}

fn redact_acts(log: &mut LogImage, key: &str) -> Result<usize, HubError> {
    let store = Store::from_bytes(&log.to_bytes());
    let tip = EvLog::tip(&store);
    let mut records: Vec<Record> = EvLog::walk(&store);
    records.reverse();
    let subject = subject_of(key);
    let mut n = 0;
    for r in records.iter_mut() {
        let Some((k, s, j)) = split(&r.payload) else { continue };
        if k != KIND_ACT.as_bytes() || s != subject.as_bytes() {
            continue;
        }
        let Some(act) = Act::parse(&String::from_utf8_lossy(j)) else { continue };
        let Some(clean) = scrubbed(&act).filter(|a| a.key == key) else { continue };
        let json = clean.to_json();
        let mut p = Vec::with_capacity(2 + k.len() + s.len() + json.len());
        p.push(k.len() as u8);
        p.extend_from_slice(k);
        p.push(s.len() as u8);
        p.extend_from_slice(s);
        p.extend_from_slice(json.as_bytes());
        r.payload = p;
        n += 1;
    }
    if n == 0 {
        return Ok(0);
    }
    let mut size = store.to_bytes().len().max(crate::logimage::MIN_LOG_BYTES);
    let fresh = loop {
        match crate::forget::rebuilt(&records, tip, size) {
            Ok(fresh) => break fresh,
            Err(HubError::Store(e)) if crate::e_is_full(&e) => size = size.saturating_mul(2),
            Err(e) => return Err(e),
        }
    };
    *log = LogImage::load(&fresh.to_bytes())?;
    Ok(n)
}

/// The consent image's chain, with redaction declared. `redacted == declared`
/// and `broken == 0` is the law; `LogImage::chain_check` knows neither side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConsentCheck {
    pub records: usize,
    pub chained: usize,
    /// Content changed, link held.
    pub redacted: usize,
    /// What the `f` declarations name.
    pub declared: usize,
    pub broken: usize,
}

impl ConsentCheck {
    pub fn holds(&self) -> bool {
        self.broken == 0 && self.redacted == self.declared
    }
}

pub fn chain_check(log: &LogImage) -> ConsentCheck {
    let store = Store::from_bytes(&log.to_bytes());
    let tip = EvLog::tip(&store);
    let walked = EvLog::walk(&store);
    let mut out = ConsentCheck::default();
    for (at, r) in walked.iter().enumerate() {
        out.records += 1;
        let linked = match at {
            0 => tip == Some(r.id),
            n => walked[n - 1].prev == r.id,
        };
        let is_act = split(&r.payload).is_some_and(|(k, _, _)| k == KIND_ACT.as_bytes());
        if r.id == content_id_chained(&r.prev, &r.payload) {
            out.chained += 1;
        } else if linked && is_act {
            out.redacted += 1;
        } else {
            out.broken += 1;
        }
    }
    out.declared = log
        .entries()
        .iter()
        .filter(|e| e.kind == KIND_FORGOTTEN)
        .filter_map(|e| int_field(&e.json, "records"))
        .map(|n| n.max(0) as usize)
        .sum();
    out
}

#[cfg(test)]
mod tests;
