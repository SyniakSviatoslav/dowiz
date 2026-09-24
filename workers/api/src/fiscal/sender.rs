//! How a fiscal document would reach the tax authority — and, today, how it
//! does NOT.
//!
//! `NotConfigured` refuses every send without touching the network; there is
//! no HTTP code in this module at all. The one real sender is the eBills
//! adapter (`ebills_sender.rs`, L70, operator-authorised 2026-09-24): a venue
//! sends only when its owner ARMED it (`ebills_arm.rs`), and its answers are
//! replayed through this trait so the queue's drain is the one that rules.
//!
//! `Mock` exists only under `cfg(test)`. It models the one property the
//! platform promises and the queue relies on: the `uuid` is the idempotency
//! key, so the same uuid sent twice is ONE invoice (TAX §3.7 rule 6).

use serde::{Deserialize, Serialize};

use super::document::Document;

/// The codes a fiscalised invoice gets, recorded in `Noted{fiscal}` (TAX §3.7 rule 5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Codes {
    pub iic: String,
    pub fic: String,
    pub inv_ord_num: String,
}

/// What one attempt came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendResult {
    /// Registered. The codes go on the order as `Noted{fiscal}`.
    Sent(Codes),
    /// It may go later (a timeout, a 5xx, a lost response). Same uuid next time.
    Retriable(String),
    /// The platform REFUSED this document (a problem+json). Retrying the same
    /// bytes gets the same answer: exception row + corrective path.
    Refused(String),
    /// No sender is configured. NOT a failure of the document: the entry stays
    /// where it is, untouched, and the 48 h clock is what reports it (law N).
    NotConfigured,
    /// NOT SENT AGAIN, AND NOT GONE (the eBills sender, L70): a sale that
    /// exists at the platform unfiscalised, a create the platform refused, a
    /// dish with no till item, an unanswered send that could not be
    /// reconciled. The entry stays queued, untouched -- law N keeps naming the
    /// order -- and the reason is an exception row, never an automatic resend.
    Held(String),
}

pub trait FiscalSender {
    fn send(&self, doc: &Document) -> SendResult;
}

/// PRODUCTION. Refuses every send; opens no connection.
pub struct NotConfigured;

impl FiscalSender for NotConfigured {
    fn send(&self, _doc: &Document) -> SendResult {
        SendResult::NotConfigured
    }
}

/// TEST ONLY. A platform that keys invoices by uuid.
#[cfg(test)]
pub struct Mock {
    invoices: std::cell::RefCell<std::collections::BTreeMap<[u8; 16], Codes>>,
    calls: std::cell::Cell<usize>,
    refuse: Vec<[u8; 16]>,
    lose_next_response: std::cell::Cell<bool>,
}

#[cfg(test)]
impl Mock {
    pub(super) fn new() -> Self {
        Mock {
            invoices: Default::default(),
            calls: Default::default(),
            refuse: Vec::new(),
            lose_next_response: Default::default(),
        }
    }
    /// A platform that refuses these uuids with a problem+json.
    pub(super) fn refusing(uuids: &[[u8; 16]]) -> Self {
        Mock { refuse: uuids.to_vec(), ..Mock::new() }
    }
    /// The next send registers the invoice and then loses the answer: the
    /// caller sees `Retriable`, exactly the case the uuid key exists for.
    pub(super) fn lose_next_response(&self) {
        self.lose_next_response.set(true);
    }
    /// Invoices the platform holds — the number the tax authority sees.
    pub(super) fn invoices(&self) -> usize {
        self.invoices.borrow().len()
    }
    /// Sends it received, including repeats.
    pub(super) fn calls(&self) -> usize {
        self.calls.get()
    }
}

#[cfg(test)]
impl FiscalSender for Mock {
    fn send(&self, doc: &Document) -> SendResult {
        self.calls.set(self.calls.get() + 1);
        if self.refuse.contains(&doc.uuid) {
            return SendResult::Refused(format!("{{\"title\":\"refused\",\"uuid\":\"{}\"}}", super::document::uuid_text(&doc.uuid)));
        }
        let n = self.invoices.borrow().len() + 1;
        let codes = self
            .invoices
            .borrow_mut()
            .entry(doc.uuid)
            .or_insert_with(|| Codes {
                iic: format!("IIC{n:04}"),
                fic: format!("FIC{n:04}"),
                inv_ord_num: n.to_string(),
            })
            .clone();
        if self.lose_next_response.replace(false) {
            return SendResult::Retriable("the response was lost".into());
        }
        SendResult::Sent(codes)
    }
}
