//! NOTES ON THE STOCK LOG (W-STOCK P5, 2026-10-03): records that ride in the
//! chain and are NEVER a movement of the shelf -- a supplier's card, an order
//! sent to a supplier. They answer "who do we buy it from, and what is already
//! on its way" from the same append-only log the shelf is folded from, so the
//! venue's purchasing history is as tamper-evident as its stock.
//!
//! A NOTE NEVER DECODES AS A `StockEvent`. Its `k` is [`KIND`], which
//! `decode` answers `None` for, so every fold -- `ledger`, `journal`, the cost
//! book, the lots, a checkpoint and `verify_checkpoints` -- walks past it
//! exactly as it walks past a checkpoint. A log written before notes existed
//! is a log with none; a Worker rolled back past this file reads a log WITH
//! notes as if they were not there. That is the old-image rule, kept by
//! construction rather than by a migration.
//!
//! The body is the caller's JSON object (minijson-readable keys); this file
//! only frames it: `{"k":"note","note":"<what>",...body...,"at":<ms>}`.

use super::{StockError, StockLog};
use crate::minijson::{esc, int_field, str_field};

/// The `k` of every note.
pub const KIND: &str = "note";
/// A note's body is at most this long: a card, not a document.
pub const BODY_MAX: usize = 2048;

impl StockLog {
    /// Append one note of kind `what` with `body`, a JSON object (`{...}`).
    /// Stamped with the request clock when one is set. Refused, nothing
    /// written: an empty `what`, a body that is not an object, one too long.
    pub fn append_note(&mut self, what: &str, body: &str) -> Result<(), StockError> {
        let what = what.trim();
        let inner = body.trim().strip_prefix('{').and_then(|b| b.strip_suffix('}')).ok_or(StockError::Malformed)?;
        if what.is_empty() || body.len() > BODY_MAX {
            return Err(StockError::Malformed);
        }
        let mut rec = format!(r#"{{"k":"{KIND}","note":"{}""#, esc(what));
        if !inner.trim().is_empty() {
            rec.push(',');
            rec.push_str(inner.trim());
        }
        if let Some(at) = self.clock {
            rec.push_str(&format!(r#","at":{at}"#));
        }
        rec.push('}');
        self.write_payload(rec.into_bytes())
    }

    /// Every note of kind `what`, OLDEST FIRST, as written.
    pub fn notes(&self, what: &str) -> Vec<String> {
        self.raw().into_iter().filter(|r| is_note(r, what)).collect()
    }
}

/// Is `rec` a note of kind `what`?
pub fn is_note(rec: &str, what: &str) -> bool {
    str_field(rec, "k").as_deref() == Some(KIND) && str_field(rec, "note").as_deref() == Some(what)
}

/// A note's clock, when it carries one.
pub fn at_of(rec: &str) -> Option<i64> {
    int_field(rec, "at")
}

#[cfg(test)]
#[path = "notes/tests.rs"]
mod tests;
