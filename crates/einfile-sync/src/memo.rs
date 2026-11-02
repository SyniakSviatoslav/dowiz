//! `Memo` (§3): a session's `verify` runs on the file and again on the union, and a file of years
//! holds thousands of seals, each one ML-DSA. A `Verifier` in front of the real one remembers a
//! signature that verified, so the key fold runs again and the cryptography does not. Format and
//! `verify` do not move. Only `true` is remembered: a forged seal pays every time, and gets nothing.
//!
//! The key is a hash over everything the answer depends on, length-prefixed so two fields cannot
//! trade bytes. With no hash in `ids` nothing is remembered and every call goes through.
//! Whether this is enough is a HYPOTHESIS: ML-DSA's time on this box was never measured (§3).

#[allow(unused_imports)]
use alloc::{vec, vec::Vec};
use alloc::collections::BTreeSet;
use bebop_log::doc::{Ids, Verifier};
use core::cell::{Cell, RefCell};

const TAG: &[u8] = b"ein-memo/1";

pub struct Memo<'a> {
    inner: &'a dyn Verifier,
    ids: &'a dyn Ids,
    seen: RefCell<BTreeSet<[u8; 32]>>,
    calls: Cell<u64>,
}

impl<'a> Memo<'a> {
    pub fn new(inner: &'a dyn Verifier, ids: &'a dyn Ids) -> Self {
        Memo { inner, ids, seen: RefCell::new(BTreeSet::new()), calls: Cell::new(0) }
    }

    /// How many times the real verifier was asked.
    pub fn calls(&self) -> u64 {
        self.calls.get()
    }

    fn key(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> Option<[u8; 32]> {
        let mut m = Vec::with_capacity(TAG.len() + 32 + pk.len() + msg.len() + sig.len());
        m.extend_from_slice(TAG);
        m.extend_from_slice(&alg.to_le_bytes());
        for field in [pk, msg, sig] {
            m.extend_from_slice(&(field.len() as u64).to_le_bytes());
            m.extend_from_slice(field);
        }
        self.ids.hash(&m)
    }
}

impl Verifier for Memo<'_> {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        let key = self.key(alg, pk, msg, sig);
        if key.is_some_and(|k| self.seen.borrow().contains(&k)) {
            return true;
        }
        self.calls.set(self.calls.get() + 1);
        let ok = self.inner.verify(alg, pk, msg, sig);
        if let (true, Some(k)) = (ok, key) {
            self.seen.borrow_mut().insert(k);
        }
        ok
    }
    fn checks_signatures(&self) -> bool {
        self.inner.checks_signatures()
    }
    fn actor_of(&self, pk: &[u8]) -> Option<[u8; 32]> {
        self.inner.actor_of(pk)
    }
}

#[cfg(test)]
mod tests;
