//! READING A FEW RECORDS OF A LOG WITHOUT UNPACKING ALL OF THEM (W-LOOPB, R-LOOPS row 4).
//!
//! `entries()` unpacks and hashes every record (`walk_marked`), so `about(kind, subject, 1)`
//! -- a wallet balance, a channel's read mark -- decoded 600 records to keep one. Here the walk
//! is `EvLog::scan`: per record it reads the few payload cells that hold `[klen][kind][slen]
//! [subject]`, and only a record whose bytes MATCH is hashed and unpacked; the walk stops at
//! `limit`. The answer is `entries()` filtered and cut, record for record, seq for seq:
//!
//!   * a failed crc is skipped wherever it sits (before, at or after the match) -- the same
//!     `check_obj` `walk_marked` runs, on the matched record only;
//!   * a record whose subject framing is broken does not decode, on both sides;
//!   * the prefix compares BYTES and `entries()` compares `from_utf8_lossy` strings. Those
//!     agree for every kind and subject without U+FFFD in it (a lossy decode equal to a
//!     string with no replacement character is that string's bytes); one WITH it takes the
//!     long way, so the two cannot differ.
//!
//! `recent_checked(n)` is the console's question: the newest `n`, or the newest record that
//! is quarantined ANYWHERE in the log. It reads the crc verdicts the load already made
//! (`crate::Seen`) and each record's framing bytes, and unpacks only the `n` it returns.

use bebop_store::evlog::{EvLog, Head, Peek};

use super::{decode, Entry, LogImage};
use crate::Quarantined;

impl LogImage {
    /// Newest first, of one kind, optionally about one subject.
    ///
    /// THE FILTER IS THE WHOLE QUERY LANGUAGE HERE and that is deliberate: a
    /// log of a few hundred short records is cheaper to walk than to index, and
    /// an index on an append-only image would have to be rebuilt on every grow.
    pub fn about(&self, kind: &str, subject: Option<&str>, limit: usize) -> Vec<Entry> {
        if kind.contains('\u{FFFD}') || subject.is_some_and(|s| s.contains('\u{FFFD}')) {
            return self.about_all(kind, subject, limit);
        }
        let (k, s) = (kind.as_bytes(), subject.map(str::as_bytes));
        // No record holds a kind or subject longer than its one length byte can say.
        if limit == 0 || k.len() > 255 || s.is_some_and(|s| s.len() > 255) {
            return Vec::new();
        }
        let mut head = vec![k.len() as u8];
        head.extend_from_slice(k);
        if let Some(s) = s {
            head.push(s.len() as u8);
            head.extend_from_slice(s);
        }
        let (head, n) = (Head::new(0, &head), self.len());
        let mut out = Vec::new();
        EvLog::scan(&self.store, |p| {
            if p.has(&head) && p.crc_bad().is_none() {
                if let Some(e) = decode(&p.record().payload, seq(n, p.pos)) {
                    out.push(e);
                }
            }
            out.len() >= limit
        });
        out
    }

    /// `about` the long way: every record unpacked, then filtered. The reference.
    pub(crate) fn about_all(&self, kind: &str, subject: Option<&str>, limit: usize) -> Vec<Entry> {
        self.entries()
            .into_iter()
            .filter(|e| e.kind == kind)
            .filter(|e| subject.map_or(true, |s| e.subject == s))
            .take(limit)
            .collect()
    }

    /// The newest `n` records, or -- when ANY record of the log is quarantined -- the newest
    /// quarantined one: `quarantined().first()` and `entries().take(n)`, in one walk that
    /// hashes nothing and unpacks `n` records.
    pub fn recent_checked(&self, n: usize) -> Result<Vec<Entry>, Quarantined> {
        let total = self.len();
        let mut out = Vec::new();
        let mut refused = None;
        EvLog::scan(&self.store, |p| {
            let bad = match self.seen.is_bad(total, p.pos) {
                Some(true) => Err("crc"),
                Some(false) => framing(p),
                // Not the chain the load scanned (cannot happen: `Seen` moves with every
                // re-lay); hash this one record rather than guess.
                None => if p.crc_bad().is_some() { Err("crc") } else { framing(p) },
            };
            if let Err(reason) = bad {
                refused = Some(Quarantined { id: crate::hex32(&p.record().id), at: p.pos, reason });
                return true;
            }
            if out.len() < n {
                out.extend(decode(&p.record().payload, seq(total, p.pos)));
            }
            false
        });
        refused.map_or(Ok(out), Err)
    }
}

/// The seq `entries()` gives the record at `pos` of a newest-first walk of `n`.
fn seq(n: usize, pos: usize) -> u64 {
    n.saturating_sub(1 + pos) as u64
}

/// `decode_or_reason` from the framing bytes alone: the same three refusals, unpacking nothing.
fn framing(p: &Peek<'_>) -> Result<(), &'static str> {
    let Some(kl) = p.byte(0) else { return Err("empty") };
    let kl = kl as usize;
    if p.len() < 1 + kl {
        return Err("kind-framing");
    }
    match p.byte(1 + kl) {
        Some(sl) if p.len() >= 2 + kl + sl as usize => Ok(()),
        _ => Err("subject-framing"),
    }
}

#[cfg(test)]
mod tests;
