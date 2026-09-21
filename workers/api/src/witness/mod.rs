//! The census: the few numbers about a venue's history that a later night can
//! CONTRADICT.
//!
//! WHAT THE CHAIN CANNOT SEE. Every record's id commits to the id before it, so
//! `chain_check` detects an EDITED record and every record after it. It is
//! blind, by construction, to a record REMOVED FROM THE END: what is left is a
//! shorter chain that verifies perfectly. It is equally blind to a log rebuilt
//! from scratch, because anyone who can write the image can recompute every id
//! in it. Against the threat that actually matters here — editing the ledger
//! after the fact — an unwitnessed chain proves nothing at all.
//!
//! SO THE TIP IS WRITTEN DOWN SOMEWHERE ELSE. Two elsewheres, in fact, and the
//! difference between them is the whole point:
//!
//!   * the PLATFORM object's `witness` log — a different object from the
//!     venue's, so a venue-scoped compromise cannot quietly rewrite it, and
//!     itself an append log whose own records are hash-chained;
//!   * the nightly S3 copy — written off-site, by a different principal, at a
//!     time the editor cannot revisit.
//!
//! A TRUNCATION THEN CONTRADICTS SOMETHING. That is all a witness is: not a
//! proof of correctness, but a second account that has to be edited too.
//!
//! AND IT IS AN ALARM ONLY WHEN IT IS RIGHT. A rotation moves records into an
//! archive VERBATIM — same ids, same links — so last night's tip is still held,
//! by a different image. A witness that could not tell a rotation from a
//! truncation would be switched off inside a week, so the seals below carry
//! the archives and the tip is looked for in both places.

use serde::{Deserialize, Serialize};

/// The image on the platform object. An append log: a census is evidence, and
/// evidence is not updated in place.
pub const IMAGE: &str = "witness";

/// The record kind inside it. The subject is the venue, so one venue's history
/// is `about(KIND, Some(venue), n)`.
pub const KIND: &str = "census";

/// One immutable archive, sealed the night it first appeared.
///
/// SEALED ONCE, because an archive is written once by construction — the object
/// refuses to overwrite one — so reading it every night would be paying for the
/// same answer. What a later night checks is that it is still THERE and still
/// counted, which costs nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seal {
    pub id: String,
    pub records: usize,
    #[serde(default)]
    pub tip: Option<String>,
}

/// What the venue's history looked like at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Census {
    pub at_ms: i64,
    pub venue: String,
    /// The hot log: what a request reads.
    pub records: usize,
    pub tip: Option<String>,
    pub generation: i64,
    /// Every archive this venue has ever had, oldest first.
    #[serde(default)]
    pub seals: Vec<Seal>,
    /// `seals` plus the hot log. THE NUMBER THAT MAY NEVER FALL: a rotation
    /// moves records between the two halves and leaves this untouched, an
    /// append raises it, and nothing legitimate lowers it.
    pub total: usize,
    /// What this census found when it was compared with the one before it.
    ///
    /// CARRIED IN THE EVIDENCE ITSELF, not only shouted into the error log. A
    /// verdict that lives somewhere other than the record it is about is a
    /// verdict that can be lost while the record survives — and the off-site
    /// copy would then be a census with nothing to say about the night it was
    /// taken. Empty is the good answer.
    #[serde(default)]
    pub found: Vec<String>,
}

impl Census {
    pub fn archived(&self) -> usize {
        self.seals.iter().map(|s| s.records).sum()
    }

    /// Recompute `total` from its parts, so the stored figure is never the only
    /// place it exists.
    pub fn recount(&mut self) {
        self.total = self.archived() + self.records;
    }
}

/// What last night's census says about tonight's, in plain sentences.
///
/// PURE, and that is not an aesthetic choice: this is the half that decides
/// whether somebody edited the ledger, it can never be run in this box's tests
/// if it needs a Durable Object, and a judgement nobody can test is a judgement
/// nobody should trust. The two facts it cannot compute for itself — whether
/// the old tip is still held, and by what — are arguments.
///
/// An empty list means the two accounts agree.
pub fn contradictions(prev: &Census, now: &Census, tip_held_by: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();

    // 1. TOTAL MAY NOT FALL. The one line that catches a plain truncation,
    //    whichever half of the history it was taken from.
    if now.total < prev.total {
        out.push(format!(
            "the history shrank: {} records on {}, {} now",
            prev.total, prev.at_ms, now.total
        ));
    }

    // 2. AN ARCHIVE MAY NOT VANISH. Deleting a cold image is the cheapest way
    //    to lose a year of orders, and it would otherwise leave the hot log
    //    looking perfect.
    for s in &prev.seals {
        match now.seals.iter().find(|t| t.id == s.id) {
            None => out.push(format!("archive {} was witnessed on {} and is gone", s.id, prev.at_ms)),
            Some(t) if t.records != s.records => out.push(format!(
                "archive {} held {} records on {} and holds {} now",
                s.id, s.records, prev.at_ms, t.records
            )),
            Some(t) if t.tip != s.tip => {
                out.push(format!("archive {} is sealed to a different tip than on {}", s.id, prev.at_ms))
            }
            Some(_) => {}
        }
    }

    // 3. THE TIP MUST STILL BE IN THERE, and this is the one that catches a
    //    log rebuilt from scratch — every id recomputed, the chain perfect,
    //    and last night's newest record simply not in it.
    if let Some(tip) = prev.tip.as_deref() {
        if tip_held_by.is_none() {
            out.push(format!(
                "the record witnessed as the tip on {} ({}…) is in neither the log nor any archive",
                prev.at_ms,
                &tip[..tip.len().min(16)]
            ));
        }
    }

    out
}

/// The half that needs the objects. The line between this and everything above
/// it is where the tests stop: a judgement about whether somebody edited the
/// ledger must be decidable without a Durable Object, or it is a judgement
/// nobody can check.
mod night;
pub use night::{last, nightly};

#[cfg(test)]
mod tests;
