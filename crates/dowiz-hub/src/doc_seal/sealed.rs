//! `SealedHub` (W-MG-M1B): a hub image under a DOC root -- the order log's records with one
//! ML-DSA-65 seal per commit, grown by `Doc::copy_bytes`, verified with this crate's ids
//! (`HubIds`) and verifier (`MlDsa65`). Split out of `doc_seal.rs` for the 300-line gate; the
//! public names stay at `doc_seal::{SealedHub, KIND_KEYOP, MIN_SEALED_BYTES}`.

use bebop_store::evlog::doc::seal::read_heads;
use bebop_store::evlog::doc::seg::{frontier_of, read_segs};
use bebop_store::evlog::doc::{self, AnchorCheck, Checked, Doc, DocError, KeyOp, Report, Trust};
use bebop_store::evlog::{EvLog, Record};
use bebop_store::{Store, StoreError};

use super::history::KIND_DECL;
use super::{DeviceKey, HubIds, MlDsa65};
use crate::chain::{chain_id, flags_for, with_nonce, Entropy};
use crate::{e_is_full, ChainCheck, Event, EventKind, Hub, HubError};

/// The kind byte of a record that carries a KEY OP (ADMIT / REVOKE) instead of an order
/// event: `b'K'` (0x4B), outside `EventKind`'s 1..=8, so `events()` skips it by name and an
/// old `Hub` reader that ever met one would quarantine it as an unknown kind, never decode
/// it as an order.
pub const KIND_KEYOP: u8 = b'K';

/// The smallest image a sealed hub is born into: a seal is 429 cells and a key 253, so the
/// order log's 64 KiB floor would hold a dozen commits before the first grow.
pub const MIN_SEALED_BYTES: usize = 256 * 1024;

/// A hub image under a DOC root: the order log's records, one ML-DSA-65 seal per commit.
pub struct SealedHub {
    pub(super) store: Store,
    pub(super) entropy: Entropy,
}

pub(super) fn doc_err(e: DocError) -> HubError {
    match e {
        DocError::NotDoc => HubError::NotAHub,
        DocError::Store(e) => HubError::Store(e),
        DocError::BadCrc(b) => HubError::BadCrc(b),
        DocError::Corrupt(what) => HubError::Store(StoreError::Corrupt(what)),
        DocError::Actor => HubError::Store(StoreError::Corrupt("the record's actor is not the signer")),
        DocError::UnknownActor(_) => HubError::Store(StoreError::Corrupt("the signer's key is not in KEYS")),
        DocError::Revoked { .. } => HubError::Store(StoreError::Corrupt("the signer's key is revoked at this seq")),
        DocError::Seq { .. } => HubError::Store(StoreError::Corrupt("the device's seq is not the next one")),
        DocError::CutUndeclared { .. } => HubError::Store(StoreError::Corrupt("a log segment was cut without a signed TRUNCATE")),
        DocError::NothingToCut => HubError::Store(StoreError::Corrupt("nothing to cut: keep is not below the log's length")),
        DocError::CutsAdmit(_) => HubError::Store(StoreError::Corrupt("the cut would take a device's ADMIT record")),
        DocError::NoTree => HubError::Store(StoreError::Corrupt("the id scheme has no tree root to name a segment by")),
        DocError::Carries(what) => HubError::Store(StoreError::Corrupt(what)),
    }
}

impl SealedHub {
    /// A REAL hub image (what `Hub` writes), sealed: its chain copied verbatim under a DOC
    /// whose genesis key is `key`. Refused by name when the hub itself is refused
    /// (`Hub::load`'s rules, which include refusing a DOC as `UnknownVersion`) and when it
    /// holds a quarantined record -- a copy would re-seal that record's crc, which is the
    /// laundering `EvLog::copy_chain_bytes` exists to avoid, and a DOC refuses a failed
    /// crc whole in any case.
    pub fn seal_hub(hub_bytes: &[u8], key: &DeviceKey) -> Result<Self, HubError> {
        let hub = Hub::load(hub_bytes)?;
        if !hub.crc_quarantined().is_empty() {
            return Err(HubError::Store(StoreError::Corrupt("a hub with a quarantined record is not sealed")));
        }
        let mut recs = EvLog::walk(&hub.store);
        recs.reverse();
        let mut store = Store::create_bytes(hub_bytes.len().saturating_mul(2).max(MIN_SEALED_BYTES))?;
        Doc::init_over_bytes(&mut store, doc::FLAG_SHA256_IDS, &[key.key()], &recs).map_err(doc_err)?;
        Ok(SealedHub { store, entropy: Entropy::default() })
    }

    /// A fresh sealed hub with no history.
    pub fn create(key: &DeviceKey) -> Result<Self, HubError> {
        Self::create_sized(key, MIN_SEALED_BYTES)
    }

    /// `create` at a chosen size (a test that wants to see the image grow).
    pub fn create_sized(key: &DeviceKey, bytes: usize) -> Result<Self, HubError> {
        let mut store = Store::create_bytes(bytes)?;
        Doc::init_bytes(&mut store, doc::FLAG_SHA256_IDS, &[key.key()]).map_err(doc_err)?;
        Ok(SealedHub { store, entropy: Entropy::default() })
    }

    /// Load a sealed image: the superblock, the DOC root's crc, the log chain's crc and its
    /// count. Not a DOC = `NotAHub`; a changed byte = `BadCrc`.
    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        let view = Doc::open(&store).map_err(doc_err)?;
        let claimed = EvLog::len_at(&store, view.log);
        match EvLog::chain_crc_at(&store, view.log).map_err(HubError::BadCrc)? {
            Some(n) if n == claimed => {}
            chained => return Err(HubError::Corrupt { claimed, chained }),
        }
        Ok(SealedHub { store, entropy: Entropy::default() })
    }

    pub fn to_bytes_trimmed(&self) -> Vec<u8> {
        self.store.to_bytes_trimmed()
    }

    /// The image at full capacity (`Hub::to_bytes`): what `grow` sizes from.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.store.to_bytes()
    }

    /// THE ENTROPY SEAM (M0): see `Hub::set_entropy`.
    pub fn set_entropy(&mut self, seed: [u8; 32]) {
        self.entropy.set(seed);
    }

    fn log(&self) -> Option<usize> {
        Doc::open(&self.store).ok().and_then(|v| v.log)
    }

    pub fn len(&self) -> usize {
        EvLog::len_at(&self.store, self.log())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The log's tip: what a seal signs and an anchor is taken over.
    pub fn tip(&self) -> Option<[u8; 32]> {
        EvLog::tip_at(&self.store, self.log())
    }

    /// The device's next seq: one past its HEADS row, 1 for its first commit.
    pub fn next_seq(&self, key: &DeviceKey) -> u64 {
        let heads = Doc::open(&self.store).ok().and_then(|v| v.heads);
        read_heads(&self.store, heads).iter().find(|h| h.actor_id == key.id).map_or(1, |h| h.seq + 1)
    }

    /// ONE HUB COMMIT: the order event appended (M0's nonce and sha256 id, the device as
    /// actor, its next seq) AND the seal over the new tip, in one transaction. On a full
    /// arena the image doubles (`Doc::copy_bytes`) and the commit is tried once more.
    pub fn append(&mut self, kind: EventKind, order_id: &str, order_json: &str, key: &DeviceKey, claimed_ms: i64) -> Result<i64, HubError> {
        let idb = order_id.as_bytes();
        if idb.len() > 255 {
            return Err(HubError::OrderIdTooLong);
        }
        let mut body = Vec::with_capacity(2 + idb.len() + order_json.len());
        body.push(kind as u8);
        body.push(idb.len() as u8);
        body.extend_from_slice(idb);
        body.extend_from_slice(order_json.as_bytes());
        self.commit_body(body, key, claimed_ms, None)
    }

    /// A KEY OP as a sealed commit of `key`'s chain: `[b'K'] ‖ op.body()` behind the nonce.
    /// ADMIT of a new device, REVOKE after a seq; rotation = the two in a row. The format's
    /// ability, not an operation anyone runs (memory `no-secret-rotation.md`).
    pub fn commit_keyop(&mut self, op: &KeyOp, key: &DeviceKey, claimed_ms: i64) -> Result<i64, HubError> {
        let mut body = vec![KIND_KEYOP];
        body.extend_from_slice(&op.body());
        self.commit_body(body, key, claimed_ms, Some(op))
    }

    /// `prev` of the device's next record: ITS OWN last record (its HEADS tip -- `verify`'s
    /// rule is per device, so a second device chaining onto the log's tip would be broken,
    /// which W-MG-M1B's two-device test left uncounted), else where a cut left it (the SEG
    /// frontier row, W-MG-M3b), else the log's tip (a device's first record after `seal_hub`
    /// chains onto the pre-seal records), else 0^32. One device: the log's tip, as before.
    fn device_prev(&self, key: &DeviceKey) -> [u8; 32] {
        let view = Doc::open(&self.store).ok();
        if let Some(h) = read_heads(&self.store, view.as_ref().and_then(|v| v.heads)).iter().find(|h| h.actor_id == key.id) {
            return h.tip;
        }
        let segs = view.as_ref().and_then(|v| read_segs(&self.store, v.seg, EvLog::step_cap(&self.store)).ok()).unwrap_or_default();
        if let Some(f) = frontier_of(&segs, &key.id) {
            return f.tip;
        }
        self.tip().unwrap_or([0u8; 32])
    }

    pub(super) fn commit_body(&mut self, body: Vec<u8>, key: &DeviceKey, claimed_ms: i64, op: Option<&KeyOp>) -> Result<i64, HubError> {
        let prev = self.device_prev(key);
        let (nonce, derived) = self.entropy.nonce(&prev, &body);
        let payload = with_nonce(&nonce, &body);
        let id = chain_id(&prev, &payload);
        let rec = Record { id, prev, actor_pubkey: key.id, actor_seq: self.next_seq(key), flags: flags_for(derived), payload };
        match Doc::commit_bytes(&mut self.store, &rec, key, claimed_ms, op) {
            Ok(gen) => Ok(gen),
            Err(DocError::Store(e)) if e_is_full(&e) => {
                self.grow()?;
                Doc::commit_bytes(&mut self.store, &rec, key, claimed_ms, op).map_err(doc_err)
            }
            Err(e) => Err(doc_err(e)),
        }
    }

    /// Double the image: the DOC copied whole (`Doc::copy_bytes`), nothing re-signed.
    fn grow(&mut self) -> Result<(), HubError> {
        let bigger = self.store.to_bytes().len().saturating_mul(2).max(MIN_SEALED_BYTES);
        let mut fresh = Store::create_bytes(bigger)?;
        Doc::copy_bytes(&self.store, &mut fresh).map_err(doc_err)?;
        self.store = fresh;
        Ok(())
    }

    /// The eight counters, this crate's ids, ML-DSA-65, the file's genesis keys as trust.
    pub fn verify(&self) -> Result<Report, HubError> {
        doc::verify(&self.store, &HubIds, &MlDsa65, Trust::Genesis).map_err(doc_err)
    }

    /// `verify` with the trust the caller names (a CLI's `--trust <actor>`).
    pub fn verify_trusting(&self, trust: Trust<'_>) -> Result<Report, HubError> {
        doc::verify(&self.store, &HubIds, &MlDsa65, trust).map_err(doc_err)
    }

    /// M0's walk over the DOC's log: the same rules `Hub::chain_check` applies.
    pub fn chain_check(&self) -> ChainCheck {
        let log = self.log();
        crate::chain::walk_check(&EvLog::walk_until_at(&self.store, log, |_| false), EvLog::tip_at(&self.store, log), |_, _, _| false)
    }

    /// Every order event, newest first; key-op records (`KIND_KEYOP`) and declarations
    /// (`KIND_DECL`: FORGOTTEN, TRUNCATE) are skipped by name.
    pub fn events(&self) -> Vec<Event> {
        EvLog::walk_until_at(&self.store, self.log(), |_| false)
            .iter()
            .filter(|r| !matches!(crate::body_of(r).first(), Some(&KIND_KEYOP) | Some(&KIND_DECL)))
            .filter_map(crate::decode)
            .collect()
    }

    /// Record one anchor over `over` (normally `tip()`): the proof is the adapter's blob
    /// (`tsa::mark`, `ots::stamp`, `roughtime::proof`); `time_ms` is the writer's claim of
    /// the anchor's moment. One transaction; the log and seals untouched.
    pub fn anchor(&mut self, kind: i64, over: &[u8; 32], time_ms: i64, proof: &[u8]) -> Result<i64, HubError> {
        Doc::anchor_bytes(&mut self.store, kind, over, time_ms, proof).map_err(doc_err)
    }

    /// Every anchor, checked by `ac` (the adapters the auditor brings keys to).
    pub fn anchors(&self, ac: &dyn AnchorCheck) -> Result<Vec<Checked>, HubError> {
        doc::check_anchors(&self.store, ac).map_err(doc_err)
    }
}
