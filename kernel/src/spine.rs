//! W2-7 — event-sourced hash-chain knowledge spine.
//!
//! A deterministic, append-only, **tamper-evident** log of knowledge records
//! spanning three kinds — [`RecordKind::Memory`], [`RecordKind::Identity`], and
//! [`RecordKind::Intent`]. Every record links to its predecessor by hash, so the
//! whole spine is a hash chain: re-hashing from genesis and comparing each link
//! detects any mutation of a payload, a link, or a record's own digest.
//!
//! This is intentionally a *plain hash chain*, **not** a signature scheme — it
//! proves integrity/ordering against tampering, not authorship. It touches no
//! money, no auth, no crypto keys. Pure `std` (the hash primitive is the
//! dependency-free [`crate::event_log::sha3_256`]).
//!
//! ## Verified-by-Math
//!
//! * An untouched chain verifies `true` (`verify_chain_tamper_free`).
//! * Mutating one record's `payload_hash` flips `verify_chain()` to `false`
//!   (`verify_chain_detects_payload_tamper`).
//! * [`KnowledgeSpine::query`] returns *only* records of the requested kind
//!   (`query_by_kind_returns_only_that_kind`).

/// Sentinel `prev_hash` for the genesis (first) record — the empty hash.
pub const GENESIS_PREV: [u8; 32] = [0u8; 32];

/// The three knowledge-record kinds tracked by the spine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecordKind {
    /// A stored memory / fact / observation.
    Memory,
    /// An identity assertion / agent or entity handle.
    Identity,
    /// An intent / goal / plan step.
    Intent,
}

impl RecordKind {
    /// Deterministic one-byte discriminant (stable wire/storage encoding).
    fn discriminant(&self) -> u8 {
        match self {
            RecordKind::Memory => 0,
            RecordKind::Identity => 1,
            RecordKind::Intent => 2,
        }
    }

    /// Parse the discriminant back into a kind (inverse of `discriminant`).
    fn from_discriminant(b: u8) -> Option<Self> {
        match b {
            0 => Some(RecordKind::Memory),
            1 => Some(RecordKind::Identity),
            2 => Some(RecordKind::Intent),
            _ => None,
        }
    }
}

/// The canonical, chained form of a knowledge record.
///
/// `record_hash` is the SHA3-256 digest of `(kind, payload_hash, prev_hash, id)`.
/// Because `prev_hash` points at the previous record's `record_hash`, the
/// `records` vector is a hash chain; mutating any field breaks the link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpineRecord {
    /// Stable caller-supplied identifier (e.g. `"mem-001"`).
    pub id: String,
    /// Which kind of knowledge this record carries.
    pub kind: RecordKind,
    /// Hash of the record's *content* (the payload), supplied at append time.
    pub payload_hash: [u8; 32],
    /// Hash of the previous record's `record_hash` (or [`GENESIS_PREV`]).
    pub prev_hash: [u8; 32],
    /// Self-hash of this record — the chain link.
    pub record_hash: [u8; 32],
}

/// Input shape for [`KnowledgeSpine::append`]: the caller-supplied fields that
/// are *not* chain-derived. The spine computes `prev_hash` and `record_hash`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingRecord {
    /// Stable identifier (e.g. `"mem-001"`).
    pub id: String,
    /// Knowledge kind.
    pub kind: RecordKind,
    /// Hash of the record's payload/content.
    pub payload_hash: [u8; 32],
}

/// Deterministically hash the *payload bytes* into a `[u8; 32]`.
///
/// Convenience so callers can hash content without reaching into `event_log`.
pub fn hash_payload(data: &[u8]) -> [u8; 32] {
    crate::event_log::sha3_256(data)
}

impl SpineRecord {
    /// Recompute this record's self-hash from its constituent fields. Used by
    /// [`KnowledgeSpine::verify_chain`] to detect tampering.
    fn compute_hash(&self) -> [u8; 32] {
        compute_record_hash(&self.id, self.kind, &self.payload_hash, &self.prev_hash)
    }
}

/// Pure deterministic digest of a record's fields, in a fixed canonical order:
/// `kind || payload_hash || prev_hash || len(id) || id`.
fn compute_record_hash(
    id: &str,
    kind: RecordKind,
    payload_hash: &[u8; 32],
    prev_hash: &[u8; 32],
) -> [u8; 32] {
    let mut buf = Vec::with_capacity(1 + 32 + 32 + 8 + id.len());
    buf.push(kind.discriminant());
    buf.extend_from_slice(payload_hash);
    buf.extend_from_slice(prev_hash);
    buf.extend_from_slice(&(id.len() as u64).to_le_bytes());
    buf.extend_from_slice(id.as_bytes());
    crate::event_log::sha3_256(&buf)
}

/// An append-only, tamper-evident knowledge spine (hash chain).
#[derive(Clone, Debug, Default)]
pub struct KnowledgeSpine {
    records: Vec<SpineRecord>,
}

impl KnowledgeSpine {
    /// A fresh, empty spine (no records, genesis link pending).
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Number of records currently in the chain.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// `true` iff the chain holds no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Append a pending record. Computes the `prev_hash` (from the current chain
    /// tip, or [`GENESIS_PREV`] if empty) and the `record_hash`, then returns the
    /// fully-chained [`SpineRecord`].
    pub fn append(&mut self, pending: PendingRecord) -> SpineRecord {
        let prev_hash = match self.records.last() {
            Some(tip) => tip.record_hash,
            None => GENESIS_PREV,
        };
        let record_hash =
            compute_record_hash(&pending.id, pending.kind, &pending.payload_hash, &prev_hash);
        let record = SpineRecord {
            id: pending.id,
            kind: pending.kind,
            payload_hash: pending.payload_hash,
            prev_hash,
            record_hash,
        };
        self.records.push(record.clone());
        record
    }

    /// Reconstruct a spine from a previously-persisted, ordered record list
    /// (e.g. loaded from disk by a caller like `bin/spine_snapshot.rs`) and
    /// verify it BEFORE accepting — fail-closed: a tampered, reordered, or
    /// corrupt record list is refused, never silently loaded as if it were
    /// trustworthy. This is the load-side half of tamper evidence: `append`
    /// proves a chain built THIS run is internally consistent; `from_persisted`
    /// is what proves a chain read back from disk on a LATER run still is.
    pub fn from_persisted(records: Vec<SpineRecord>) -> Result<Self, ()> {
        let spine = Self { records };
        if spine.verify_chain() {
            Ok(spine)
        } else {
            Err(())
        }
    }

    /// Borrow the full ordered chain.
    pub fn records(&self) -> &[SpineRecord] {
        &self.records
    }

    /// Borrow a record by position in the chain (0 = genesis).
    pub fn get(&self, index: usize) -> Option<&SpineRecord> {
        self.records.get(index)
    }

    /// Mutable borrow of the chain — test seam only (`cargo test` never ships).
    /// Lets a tamper test flip a field in place and confirm `verify_chain()`
    /// then returns `false`.
    #[cfg(test)]
    pub(crate) fn records_mut_for_test(&mut self) -> &mut Vec<SpineRecord> {
        &mut self.records
    }

    /// Return references to every record whose kind matches `kind`, in chain order.
    pub fn query(&self, kind: RecordKind) -> Vec<&SpineRecord> {
        self.records.iter().filter(|r| r.kind == kind).collect()
    }

    /// Re-walk the entire chain and return `true` iff it is internally
    /// consistent and untampered:
    ///
    /// * the genesis record's `prev_hash` equals [`GENESIS_PREV`];
    /// * every subsequent record's `prev_hash` equals the previous record's
    ///   `record_hash`;
    /// * every record's stored `record_hash` matches the digest recomputed from
    ///   its fields.
    ///
    /// An empty spine verifies `true` (vacuously). Any mutation — of a payload,
    /// a link, or a stored digest — yields `false`.
    pub fn verify_chain(&self) -> bool {
        let mut expected_prev = GENESIS_PREV;
        for rec in &self.records {
            if rec.prev_hash != expected_prev {
                return false;
            }
            if rec.record_hash != rec.compute_hash() {
                return false;
            }
            expected_prev = rec.record_hash;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(id: &str, kind: RecordKind, payload: &[u8]) -> PendingRecord {
        PendingRecord {
            id: id.to_string(),
            kind,
            payload_hash: hash_payload(payload),
        }
    }

    #[test]
    fn verify_chain_tamper_free() {
        // RED->GREEN (1): a freshly appended chain verifies true.
        let mut spine = KnowledgeSpine::new();
        spine.append(pending("mem-1", RecordKind::Memory, b"first memory"));
        spine.append(pending("id-1", RecordKind::Identity, b"agent alice"));
        spine.append(pending("int-1", RecordKind::Intent, b"ship the spine"));
        spine.append(pending("mem-2", RecordKind::Memory, b"second memory"));

        assert_eq!(spine.len(), 4);
        assert!(spine.verify_chain(), "untouched chain must verify true");
    }

    #[test]
    fn verify_chain_detects_payload_tamper() {
        // RED->GREEN (2): mutating one record's payload_hash breaks the chain.
        let mut spine = KnowledgeSpine::new();
        spine.append(pending("mem-1", RecordKind::Memory, b"first memory"));
        spine.append(pending("id-1", RecordKind::Identity, b"agent alice"));
        spine.append(pending("int-1", RecordKind::Intent, b"ship the spine"));

        // Sanity: clean chain verifies.
        assert!(spine.verify_chain());

        // Tamper: flip a byte in the middle record's payload_hash.
        let tampered = {
            let mut h = spine.get(1).unwrap().payload_hash;
            h[0] ^= 0xFF;
            h
        };
        // Mutate the live record in place (simulating a silent edit).
        let rec = spine.records_mut_for_test().get_mut(1).unwrap();
        rec.payload_hash = tampered;

        assert!(
            !spine.verify_chain(),
            "tampered payload must fail verification"
        );
    }

    #[test]
    fn verify_chain_detects_prev_hash_tamper() {
        // Bonus: breaking a link (prev_hash) is also detected.
        let mut spine = KnowledgeSpine::new();
        spine.append(pending("a", RecordKind::Memory, b"a"));
        spine.append(pending("b", RecordKind::Identity, b"b"));
        spine.append(pending("c", RecordKind::Intent, b"c"));

        assert!(spine.verify_chain());
        let rec = spine.records_mut_for_test().get_mut(2).unwrap();
        rec.prev_hash[0] ^= 0x01;
        assert!(!spine.verify_chain());
    }

    #[test]
    fn query_by_kind_returns_only_that_kind() {
        // RED->GREEN (3): query filters to the requested kind exclusively.
        let mut spine = KnowledgeSpine::new();
        spine.append(pending("mem-1", RecordKind::Memory, b"m1"));
        spine.append(pending("id-1", RecordKind::Identity, b"i1"));
        spine.append(pending("mem-2", RecordKind::Memory, b"m2"));
        spine.append(pending("int-1", RecordKind::Intent, b"n1"));
        spine.append(pending("id-2", RecordKind::Identity, b"i2"));

        let mem = spine.query(RecordKind::Memory);
        assert_eq!(mem.len(), 2);
        assert!(mem.iter().all(|r| r.kind == RecordKind::Memory));
        assert_eq!(mem[0].id, "mem-1");
        assert_eq!(mem[1].id, "mem-2");

        let ids = spine.query(RecordKind::Identity);
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().all(|r| r.kind == RecordKind::Identity));

        let intents = spine.query(RecordKind::Intent);
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].id, "int-1");
    }

    #[test]
    fn genesis_prev_is_sentinel_and_empty_verifies() {
        let spine = KnowledgeSpine::new();
        assert!(spine.verify_chain(), "empty spine verifies vacuously");
        assert!(spine.is_empty());

        let mut spine = KnowledgeSpine::new();
        let r0 = spine.append(pending("g", RecordKind::Memory, b"genesis"));
        assert_eq!(r0.prev_hash, GENESIS_PREV);
        assert!(spine.verify_chain());
    }

    #[test]
    fn kind_discriminant_round_trips() {
        for k in [RecordKind::Memory, RecordKind::Identity, RecordKind::Intent] {
            assert_eq!(RecordKind::from_discriminant(k.discriminant()), Some(k));
        }
        assert_eq!(RecordKind::from_discriminant(9), None);
    }

    #[test]
    fn from_persisted_accepts_a_valid_round_tripped_chain() {
        // RED->GREEN: build a chain, "persist" it (clone the record Vec —
        // stands in for a real save-to-disk round trip), reload via
        // from_persisted, and confirm it's accepted and content-identical.
        let mut spine = KnowledgeSpine::new();
        spine.append(pending("doc-1", RecordKind::Memory, b"design doc snapshot"));
        spine.append(pending("doc-2", RecordKind::Memory, b"second doc snapshot"));
        let persisted: Vec<SpineRecord> = spine.records().to_vec();

        let reloaded = KnowledgeSpine::from_persisted(persisted).expect("valid chain must load");
        assert_eq!(reloaded.len(), 2);
        assert!(reloaded.verify_chain());
        assert_eq!(reloaded.get(0).unwrap().id, "doc-1");
        assert_eq!(reloaded.get(1).unwrap().id, "doc-2");
    }

    #[test]
    fn from_persisted_refuses_a_tampered_chain() {
        // RED->GREEN: this is the actual tamper-evidence-across-restarts proof —
        // corrupt one persisted record's payload_hash (simulating an edited-
        // on-disk chain file) and confirm the loader refuses it outright,
        // rather than silently accepting a broken chain.
        let mut spine = KnowledgeSpine::new();
        spine.append(pending("doc-1", RecordKind::Memory, b"design doc snapshot"));
        spine.append(pending("doc-2", RecordKind::Memory, b"second doc snapshot"));
        let mut persisted: Vec<SpineRecord> = spine.records().to_vec();
        persisted[0].payload_hash[0] ^= 0xFF; // simulate a tampered/corrupted file

        assert!(
            KnowledgeSpine::from_persisted(persisted).is_err(),
            "a tampered persisted chain must be refused, not silently loaded"
        );
    }

    #[test]
    fn from_persisted_accepts_empty_vec() {
        let reloaded = KnowledgeSpine::from_persisted(Vec::new()).expect("empty chain is valid");
        assert!(reloaded.is_empty());
    }
}
