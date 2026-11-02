//! KEPT HISTORY ON A SEALED HUB (W-MG-M3b, R-MAINGOAL-A §2b/§2c): the two ways a record stops
//! being plain history, each DECLARED by a record the device SIGNS through its seal --
//!
//!   * `forget`  redacts the person's records in place (`Doc::redact_into`: ids, `prev`, seals,
//!               anchors and segments verbatim; the payload emptied and its nonce erased, the
//!               rule of `forget.rs`) and commits one FORGOTTEN listing each record, and whom;
//!   * `cut`     moves the oldest records out under a segment (`Doc::cut_into`) and commits the
//!               TRUNCATE that names the segment; what left comes back as the archive.
//!
//! Both declarations (and M2's MERGE, `merge.rs`) are records of kind `KIND_DECL` behind M0's nonce, sealed like every hub
//! commit (ML-DSA-65 only, `DeviceKey`; never `pq::hybrid_signing`), so `verify` counts them
//! only while the key is admitted and valid at that seq: a revoked device's FORGOTTEN declares
//! nothing and a dropped segment is refused by name. The new image replaces the old one ONLY
//! with its declaration committed: a refused commit leaves the hub as it was.
//!
//! The plain `Hub` keeps its own paths (`forget.rs`, `rotate.rs`): its log is not a DOC.

use einstore::evlog::doc::keys::read_keys;
use einstore::evlog::doc::{forgotten_body, Doc, DocError};
use einstore::evlog::Record;
use einstore::{Store, StoreError};

use super::sealed::doc_err;
use super::{DeviceKey, HubIds, SealedHub, MIN_SEALED_BYTES};
use crate::forget::{redacted_payload, REDACTED_BIT};
use crate::{body_of, decode, e_is_full, Event, HubError};

/// The kind byte of a DECLARATION record (FORGOTTEN, TRUNCATE): `b'D'` (0x44), outside
/// `EventKind`'s 1..=8 like `KIND_KEYOP`, so `events()` skips it by name and `HubIds::body`
/// finds the declaration behind it.
pub const KIND_DECL: u8 = b'D';

impl SealedHub {
    /// Redact every order record `redact` answers for and declare it: one signed FORGOTTEN
    /// (`subject` -- a hash of the person's key, never the key -- and each tombstone it covers). Returns `n`; nothing
    /// is written for 0. The image is rebuilt (`Doc::redact_into`) at its size, doubling on
    /// the rare redaction that grew a record.
    pub fn forget<F>(&mut self, redact: F, subject: &[u8; 32], key: &DeviceKey, claimed_ms: i64) -> Result<usize, HubError>
    where
        F: Fn(&Event) -> Option<String>,
    {
        let rule = |r: &Record| -> Option<Vec<u8>> {
            if body_of(r).first().map_or(true, |k| k & REDACTED_BIT != 0) {
                return None;
            }
            let ev = decode(r)?;
            let json = redact(&ev)?;
            Some(redacted_payload(r.flags, &ev, &json))
        };
        let mut size = self.store.writer_bytes()?.max(MIN_SEALED_BYTES);
        let (fresh, erased) = loop {
            let mut fresh = Store::create_bytes(size)?;
            match Doc::redact_into(&self.store, &mut fresh, &HubIds, &rule) {
                Ok(erased) => break (fresh, erased),
                Err(DocError::Store(e)) if e_is_full(&e) => size = size.saturating_mul(2),
                Err(e) => return Err(doc_err(e)),
            }
        };
        if erased.is_empty() {
            return Ok(0);
        }
        let mut body = vec![KIND_DECL];
        body.extend_from_slice(&forgotten_body(subject, &erased));
        self.swap_and_declare(fresh, body, key, claimed_ms)?;
        Ok(erased.len())
    }

    /// Cut every record but the newest `keep` out under a segment and declare it: one
    /// signed TRUNCATE naming the segment's root, range and counts. Returns the ARCHIVE --
    /// the whole image as it stood, which recomputes the root (`rotate::segment_root` over
    /// its ids in file order). `keep` counts records, not orders: which orders may leave is
    /// the caller's question, as `Hub::rotate`'s `keep` is.
    ///
    /// ONLY A ROOT CUTS: a TRUNCATE of an admitted key declares nothing (`declare.rs`), so its file
    /// would be refused `CutUndeclared`. Refused here first, with nothing written. After a rotation
    /// (ADMIT of a new key) the new key is no root under `Trust::Genesis`: nobody cuts that file
    /// until a verifier brings that key as a root (`Trust::Only`).
    pub fn cut(&mut self, keep: usize, key: &DeviceKey, claimed_ms: i64) -> Result<Vec<u8>, HubError> {
        let view = Doc::open(&self.store).map_err(doc_err)?;
        if !read_keys(&self.store, view.keys).iter().any(|k| k.actor_id == key.id() && k.admit_rec.is_none()) {
            return Err(HubError::Store(StoreError::Corrupt("only a root may cut: this key was admitted")));
        }
        let archive = self.store.to_bytes_trimmed();
        let mut size = self.store.writer_bytes()?.max(MIN_SEALED_BYTES);
        let (fresh, seg) = loop {
            let mut fresh = Store::create_bytes(size)?;
            match Doc::cut_into(&self.store, &mut fresh, keep, &HubIds) {
                Ok(seg) => break (fresh, seg),
                Err(DocError::Store(e)) if e_is_full(&e) => size = size.saturating_mul(2),
                Err(e) => return Err(doc_err(e)),
            }
        };
        let mut body = vec![KIND_DECL];
        body.extend_from_slice(&seg.truncate_body());
        self.swap_and_declare(fresh, body, key, claimed_ms)?;
        Ok(archive)
    }

    /// The rebuilt image takes the old one's place only with its declaration committed.
    pub(super) fn swap_and_declare(&mut self, fresh: Store, body: Vec<u8>, key: &DeviceKey, claimed_ms: i64) -> Result<(), HubError> {
        let old = core::mem::replace(&mut self.store, fresh);
        match self.commit_body(body, key, claimed_ms, None) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.store = old;
                Err(e)
            }
        }
    }
}
