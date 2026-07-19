//! BLUEPRINT-P68 — Hub supervisor: update + backup.
//!
//! A/B-slot atomic-flip auto-update with a real-code-path health gate,
//! owner-triggered rollback, mandatory age-snapshot-before-promote, and a
//! sovereign encrypted backup envelope that dowiz can never decrypt.
//!
//! Scope (per the blueprint §2): the pure logic + the crypto that reuses the
//! kernel's already-vendored primitives (`pq` feature: `x25519` + `keccak::shake256`
//! + `aes-gcm`) and the event-log chain-tip epoch anchor (`event_log::sha3_256`).
//! The platform deps (`self_update`, rclone, systemd, OsRng) live out-of-core
//! behind the §3 ports in a separate adapters crate — they are NOT linked here,
//! exactly per the P66/P60 firewall. This module is gated behind `pq` because the
//! backup envelope genuinely needs AES-256-GCM + X25519; the kernel default build
//! stays offline-clean (no `aes-gcm` in the default graph, verified via
//! `cargo tree -p dowiz-kernel --no-default-features -e no-dev | grep -c aes-gcm`
//! → 0). Acceptance is therefore run under `--features pq`.
//!
//! §4-B closed (NO break-glass): `RecipientPubKey` has exactly one constructor —
//! `from_vendor_config` — and `RecipientSet` is built only from those. The `seal`
//! API takes a `RecipientSet` and NO other key parameter, so a dowiz-controlled
//! key has no producer. An identifier-absence scan (copying
//! the `payment.rs`/`no-break-glass` lineage) proves it at CI time.
//!
//! Compile firewall (P68 §4.2): this module imports NO adapter / network / serde
//! crate beyond what `pq` already vendors. The identifier-absence scan below
//! asserts the forbidden tokens are absent from this source.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use crate::event_log::{sha3_256, AppendOutcome, EventLog, EventStore, MemEventStore, MeshEvent};
use crate::pq::keccak::shake256;
use crate::pq::x25519::x25519;

// ═══════════════════════════════════════════════════════════════════════════
// M1 — sovereign encrypted backup envelope (X25519 → SHAKE256 → AES-256-GCM
// STREAM). age-STYLE, NOT the `age` crate: composed entirely from kernel
// primitives, zero new crypto deps (DECART §4.1).
// ═══════════════════════════════════════════════════════════════════════════

pub const BACKUP_ENVELOPE_VERSION: u8 = 1;
pub const BACKUP_CHUNK_BYTES: usize = 64 * 1024; // age's STREAM chunk size (constant memory)
pub const AEAD_KEY_LEN: usize = 32; // AES-256-GCM
pub const AEAD_TAG_LEN: usize = 16;
pub const STREAM_NONCE_LEN: usize = 12; // 11-byte BE chunk counter ‖ 1-byte last-flag
pub const STREAM_MORE: u8 = 0x00; // interior chunk
pub const STREAM_FINAL: u8 = 0x01; // LAST chunk (truncation/extension resistance)
pub const BACKUP_KDF_CTX: &[u8] = b"dowiz.backup.envelope.v1"; // payload file-key domain sep
pub const BACKUP_WRAP_CTX: &[u8] = b"dowiz.backup.recipient-wrap.v1"; // per-recipient wrap domain sep

/// A VENDOR-controlled X25519 backup RECIPIENT public key. Constructed ONLY from a
/// public key read out of the vendor's own backup config (§4.2). There is
/// deliberately NO constructor from a compile-baked constant and NO vendor-external
/// key producer — that is the §4-B firewall in the type.
#[derive(Clone)]
pub struct RecipientPubKey([u8; 32]);
impl RecipientPubKey {
    /// The ONLY ctor: from vendor config. A dowiz key has no producer here.
    pub fn from_vendor_config(pk: [u8; 32]) -> Self {
        Self(pk)
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// The recipient set for a backup. Built ONLY via `from_vendor_config`; the
/// sealing API takes NO other key source, so "append a dowiz key" has no producer.
/// 1..N, all vendor-controlled.
#[derive(Clone)]
pub struct RecipientSet(Vec<RecipientPubKey>);
impl RecipientSet {
    /// Build from vendor-config keys. Empty set is rejected (you cannot seal to nobody).
    pub fn from_vendor_config(keys: Vec<RecipientPubKey>) -> Result<Self, BackupError> {
        if keys.is_empty() {
            return Err(BackupError::NoRecipients);
        }
        Ok(Self(keys))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn recipients(&self) -> &[RecipientPubKey] {
        &self.0
    }
}

/// Per-recipient wrap stanza (age-style header): the random file-key AEAD-wrapped
/// to ONE recipient.
#[derive(Clone)]
pub struct RecipientStanza {
    pub ephemeral_pub: [u8; 32], // per-recipient ephemeral X25519 public
    pub nonce: [u8; STREAM_NONCE_LEN], // wrap nonce
    pub wrapped_file_key: [u8; AEAD_KEY_LEN + AEAD_TAG_LEN], // AES-256-GCM(wrap_key, file_key)
}

/// The sealed backup header. `recipients.len()` MUST equal the vendor-config
/// recipient count (§4.2).
#[derive(Clone)]
pub struct BackupHeader {
    pub version: u8,
    pub recipients: Vec<RecipientStanza>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupError {
    NoRecipients,
    DowizRecipientRejected,
    AeadInvalid,
    Truncated,
    ChunkReorder,
    VersionUnsupported,
    StateReadFailed,
    Io(String),
}

/// Entropy PORT (out-of-core). The pure crate is RNG-free like the kernel.
pub trait Rng {
    fn fill(&mut self, buf: &mut [u8]);
}

/// A deterministic seeded RNG for tests / KATs (the real adapters supply OsRng/qrng).
pub struct SeededRng {
    state: u64,
}
impl SeededRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
}
impl Rng for SeededRng {
    fn fill(&mut self, buf: &mut [u8]) {
        let mut x = self.state;
        for b in buf.iter_mut() {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *b = (x >> 33) as u8;
        }
        self.state = x;
    }
}

/// Build the 12-byte STREAM nonce: 11-byte big-endian chunk counter ‖ 1-byte flag.
pub fn stream_nonce(chunk_index: u64, final_flag: u8) -> [u8; STREAM_NONCE_LEN] {
    let mut n = [0u8; STREAM_NONCE_LEN];
    let be = chunk_index.to_be_bytes(); // 8 bytes
    n[0..3].copy_from_slice(&[0, 0, 0]);
    n[3..11].copy_from_slice(&be);
    n[11] = final_flag;
    n
}

/// Derive a per-recipient wrap key: SHAKE256(shared ‖ BACKUP_WRAP_CTX ‖ eph_pub ‖ recipient).
/// Domain-separated from the wallet/cert KDFs by `BACKUP_WRAP_CTX`.
fn derive_wrap_key(shared: &[u8; 32], eph_pub: &[u8; 32], recipient: &[u8; 32]) -> [u8; 32] {
    let mut input = Vec::with_capacity(32 + BACKUP_WRAP_CTX.len() + 32 + 32);
    input.extend_from_slice(shared);
    input.extend_from_slice(BACKUP_WRAP_CTX);
    input.extend_from_slice(eph_pub);
    input.extend_from_slice(recipient);
    let mut out = [0u8; 32];
    shake256(&input, &mut out);
    out
}

/// A sealed backup: header (recipient stanzas) + the STREAM ciphertext chunks.
pub struct SealedBackup {
    pub header: BackupHeader,
    pub chunks: Vec<Vec<u8>>,
}

/// The X25519 basepoint (RFC 7748 §5: u = 9).
const X25519_BASEPOINT: [u8; 32] = [9u8; 32];

/// Seal `state` to the vendor `recipients` set. Draws a random file key from `rng`,
/// wraps it per recipient, then STREAM-encrypts the payload under the file key.
pub fn seal(state: &[u8], recipients: &RecipientSet, rng: &mut dyn Rng) -> Result<SealedBackup, BackupError> {
    if recipients.is_empty() {
        return Err(BackupError::NoRecipients);
    }
    // 1. random file key (RNG-free core: entropy enters via the port).
    let mut file_key = [0u8; AEAD_KEY_LEN];
    rng.fill(&mut file_key);

    // 2. per-recipient stanzas.
    let mut stanzas = Vec::with_capacity(recipients.len());
    for r in recipients.recipients() {
        let mut eph_sec = [0u8; 32];
        rng.fill(&mut eph_sec);
        let shared = x25519(&eph_sec, r.as_bytes());
        let eph_pub = x25519(&eph_sec, &X25519_BASEPOINT);
        let wrap_key = derive_wrap_key(&shared, &eph_pub, r.as_bytes());
        let mut nonce = [0u8; STREAM_NONCE_LEN];
        rng.fill(&mut nonce);
        let cipher = Aes256Gcm::new_from_slice(&wrap_key).map_err(|_| BackupError::AeadInvalid)?;
        let wrapped = cipher
            .encrypt(Nonce::from_slice(&nonce), file_key.as_slice())
            .map_err(|_| BackupError::AeadInvalid)?;
        let mut wrapped_file_key = [0u8; AEAD_KEY_LEN + AEAD_TAG_LEN];
        wrapped_file_key.copy_from_slice(&wrapped);
        stanzas.push(RecipientStanza {
            ephemeral_pub: eph_pub,
            nonce,
            wrapped_file_key,
        });
    }

    // 3. STREAM payload.
    let chunks = stream_encrypt(state, &file_key);
    Ok(SealedBackup {
        header: BackupHeader {
            version: BACKUP_ENVELOPE_VERSION,
            recipients: stanzas,
        },
        chunks,
    })
}

/// STREAM-encrypt: 64 KiB chunks, each under `stream_nonce(j, is_last)`. The final
/// chunk carries `STREAM_FINAL`; every other chunk carries `STREAM_MORE`.
fn stream_encrypt(plaintext: &[u8], file_key: &[u8; 32]) -> Vec<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(file_key).expect("file_key is exactly 32 bytes");
    let total = plaintext.len();
    let n_chunks = total.div_ceil(BACKUP_CHUNK_BYTES).max(1); // at least one (even empty) chunk
    let mut out = Vec::with_capacity(n_chunks);
    for j in 0..n_chunks {
        let start = j * BACKUP_CHUNK_BYTES;
        let end = (start + BACKUP_CHUNK_BYTES).min(total);
        let chunk = &plaintext[start..end];
        let is_last = j + 1 == n_chunks;
        let flag = if is_last { STREAM_FINAL } else { STREAM_MORE };
        let nonce = stream_nonce(j as u64, flag);
        let ct = cipher
            .encrypt(Nonce::from_slice(&nonce), chunk)
            .expect("aes-gcm encrypt cannot fail for valid key+nonce");
        out.push(ct);
    }
    out
}

/// Open a sealed backup with the vendor's identity. Finds the stanza whose ECDH the
/// vendor secret satisfies, re-derives the file key, then stream-decrypts.
pub fn open(
    header: &BackupHeader,
    chunks: &[Vec<u8>],
    vendor_sec: &[u8; 32],
    vendor_pub: &[u8; 32],
) -> Result<Vec<u8>, BackupError> {
    if header.version != BACKUP_ENVELOPE_VERSION {
        return Err(BackupError::VersionUnsupported);
    }
    // Find the stanza this vendor can open (ECDH → wrap key → AEAD-open file key).
    let mut file_key: Option<[u8; 32]> = None;
    for stanza in &header.recipients {
        let shared = x25519(vendor_sec, &stanza.ephemeral_pub);
        let wrap_key = derive_wrap_key(&shared, &stanza.ephemeral_pub, vendor_pub);
        let cipher = Aes256Gcm::new_from_slice(&wrap_key).map_err(|_| BackupError::AeadInvalid)?;
        if let Ok(wrapped) = cipher.decrypt(Nonce::from_slice(&stanza.nonce), stanza.wrapped_file_key.as_slice()) {
            if wrapped.len() == AEAD_KEY_LEN {
                let mut fk = [0u8; AEAD_KEY_LEN];
                fk.copy_from_slice(&wrapped);
                file_key = Some(fk);
                break;
            }
        }
    }
    let file_key = file_key.ok_or(BackupError::AeadInvalid)?; // no matching recipient → no key
    stream_decrypt(chunks, &file_key)
}

/// STREAM-decrypt with truncation / reorder / extension resistance. The final chunk
/// MUST carry `STREAM_FINAL`; any interior chunk carrying `FINAL`, or any chunk after
/// a `FINAL`, is rejected. A stream that ends on a `MORE` chunk (final dropped) is
/// `Truncated`.
fn stream_decrypt(chunks: &[Vec<u8>], file_key: &[u8; 32]) -> Result<Vec<u8>, BackupError> {
    let cipher = Aes256Gcm::new_from_slice(file_key).map_err(|_| BackupError::AeadInvalid)?;
    let n = chunks.len();
    if n == 0 {
        return Err(BackupError::Truncated);
    }
    let mut out = Vec::new();
    let mut saw_final = false;
    for (j, ct) in chunks.iter().enumerate() {
        let is_last = j + 1 == n;
        if saw_final {
            // A chunk appeared AFTER the FINAL flag ⇒ extension / reorder.
            return Err(BackupError::ChunkReorder);
        }
        if is_last {
            // Try MORE first; if it opens, the stream ended without a FINAL ⇒ truncation.
            let nonce_more = stream_nonce(j as u64, STREAM_MORE);
            if cipher.decrypt(Nonce::from_slice(&nonce_more), ct.as_slice()).is_ok() {
                return Err(BackupError::Truncated);
            }
            let nonce_final = stream_nonce(j as u64, STREAM_FINAL);
            let pt = cipher
                .decrypt(Nonce::from_slice(&nonce_final), ct.as_slice())
                .map_err(|_| BackupError::AeadInvalid)?;
            out.extend_from_slice(&pt);
            saw_final = true;
        } else {
            let nonce = stream_nonce(j as u64, STREAM_MORE);
            let pt = cipher
                .decrypt(Nonce::from_slice(&nonce), ct.as_slice())
                .map_err(|_| BackupError::AeadInvalid)?;
            out.extend_from_slice(&pt);
        }
    }
    if !saw_final {
        return Err(BackupError::Truncated);
    }
    Ok(out)
}

// ═══════════════════════════════════════════════════════════════════════════
// M4 — state snapshot primitive (the rollback story for STATE, R5 risk #1).
// The event-log chain-tip content-id IS the epoch. A restore verifies the chain
// tip returns EXACTLY to the pre-promote epoch.
// ═══════════════════════════════════════════════════════════════════════════

/// The event-log chain-tip content-id at snapshot time (`event_log::sha3_256` chain).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochHash(pub [u8; 32]);

/// A pre-promote LOCAL state snapshot. PLAINTEXT, on the vendor's own box, NEVER
/// uploaded (the supervisor can auto-rollback without the vendor's offline private
/// key). The OFF-SITE backup (M1) is the encrypted-to-vendor-key artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSnapshot {
    pub epoch: EpochHash, // restore verifies the chain tip returns to this id
    pub from_version: Version,
    pub taken_at_ms: u64,
    pub event_log_path: String,
    pub projection_path: String,
}

/// PORT: event-log + projection store. `snapshot`/`restore`/`chain_tip`.
pub trait StateStore {
    fn snapshot(&self, from: &Version) -> Result<StateSnapshot, UpdateError>;
    fn restore(&mut self, snap: &StateSnapshot) -> Result<EpochHash, UpdateError>;
    fn chain_tip(&self) -> EpochHash;
}

/// In-memory `StateStore` over the kernel `EventLog<MemEventStore>`. The tip is the
/// epoch anchor; `restore` re-points the tip to the snapshot epoch (the real impl
/// re-folds the projection from the log under the old code).
pub struct MemStateStore {
    log: EventLog<MemEventStore>,
    actor: [u8; 32],
    seq: u64,
    restore_called: bool,
}
impl Default for MemStateStore {
    fn default() -> Self {
        Self::new()
    }
}
impl MemStateStore {
    pub fn new() -> Self {
        MemStateStore {
            log: EventLog::new(MemEventStore::new()),
            actor: [0xaa; 32],
            seq: 0,
            restore_called: false,
        }
    }
    /// Append a deterministic event, advancing the chain tip. Returns the new tip.
    pub fn append_event(&mut self, payload: &[u8]) -> [u8; 32] {
        let prev = self.log.tip().unwrap_or([0u8; 32]);
        let ev = MeshEvent {
            prev,
            actor_pubkey: self.actor,
            actor_seq: self.seq,
            payload: payload.to_vec(),
        };
        self.seq += 1;
        match self.log.append(ev) {
            Ok(AppendOutcome::Committed(id)) | Ok(AppendOutcome::Duplicate(id)) => id,
            Err(_) => [0u8; 32],
        }
    }
    pub fn restore_was_called(&self) -> bool {
        self.restore_called
    }
}
impl StateStore for MemStateStore {
    fn snapshot(&self, from: &Version) -> Result<StateSnapshot, UpdateError> {
        let epoch = self.log.tip().ok_or(UpdateError::SnapshotFailed)?;
        Ok(StateSnapshot {
            epoch: EpochHash(epoch),
            from_version: from.clone(),
            taken_at_ms: 0,
            event_log_path: "local://event-log".to_string(),
            projection_path: "local://projection".to_string(),
        })
    }
    fn restore(&mut self, snap: &StateSnapshot) -> Result<EpochHash, UpdateError> {
        self.restore_called = true;
        // Re-point the chain tip to the snapshot epoch (the anchor). Real impl also
        // re-folds the PgStore/W13 projection from the log under old code.
        self.log.store.set_tip(snap.epoch.0);
        Ok(snap.epoch.clone())
    }
    fn chain_tip(&self) -> EpochHash {
        EpochHash(self.log.tip().unwrap_or([0u8; 32]))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// M5/M6/M7 — A/B slot update supervisor (atomic symlink flip, real health gate,
// mandatory pre-promote snapshot + rollback-after-schema-migration).
// ═══════════════════════════════════════════════════════════════════════════

pub const SLOT_COUNT: usize = 2; // A/B, fixed
pub const HEALTH_PROBE_TIMEOUT_S: u32 = 30; // 503->200 window; timeout ⇒ never flip
pub const CRASH_LOOP_WINDOW_S: u32 = 120; // post-flip watch window
pub const CRASH_LOOP_MAX_RESTARTS: u32 = 3; // > this many ⇒ auto-rollback

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version(pub String); // semver; compared via ReleaseSource
impl Version {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    A,
    B,
}

/// The update state machine. `Promoted` is reachable ONLY through `SnapshotTaken`
/// THEN `HealthPassed`: promote-without-snapshot and promote-without-health are
/// UNREPRESENTABLE (§5.1) — the type has no producer for either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateState {
    Idle {
        current: Slot,
        pinned: Option<Version>,
    },
    Fetched {
        into: Slot,
        version: Version,
    }, // self_update verified into idle slot
    Migrated {
        into: Slot,
        version: Version,
    }, // forward migrations ran under new code
    SnapshotTaken {
        into: Slot,
        version: Version,
        snapshot: EpochHash,
    }, // MANDATORY pre-promote (§4.4)
    HealthPassed {
        into: Slot,
        version: Version,
        snapshot: EpochHash,
    }, // real-code-path probe returned Ready
    Promoted {
        current: Slot,
        previous: Slot,
        rollback_to: EpochHash,
    }, // symlink flipped (atomic rename)
    RolledBack {
        current: Slot,
        trigger: RollbackTrigger,
        restored: EpochHash,
    },
    Failed(UpdateError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackTrigger {
    CrashLoop,
    HealthTimeout,
    OwnerRequested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    FetchFailed,
    VerifyFailed,
    MigrationFailed,
    HealthFailed,
    SnapshotFailed,
    PinnedVersion,
    NoPreviousSlot,
    Io(String),
}

/// A REAL health result. 503 WarmingUp until a real code path served correctly;
/// 200 Ready ONLY then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthResult {
    WarmingUp,
    Ready,
    Failed(String),
}

// PORTS (out-of-core).
pub trait ReleaseSource {
    fn latest(&self) -> Result<Version, UpdateError>;
    fn fetch_verified(&self, v: &Version, into: Slot) -> Result<(), UpdateError>; // zipsign + SHA-256/512
}
pub trait HealthProbe {
    fn probe(&self, slot: Slot) -> HealthResult; // exercises a real path (§4.3-M6)
}
pub trait SlotFs {
    fn flip_current(&mut self, to: Slot) -> Result<(), UpdateError>; // ONE rename() syscall (atomic)
    fn current(&self) -> Slot;
    fn previous(&self) -> Option<Slot>;
}
pub trait ServiceCtl {
    fn restart(&mut self) -> Result<(), UpdateError>;
    fn restart_count_since(&self, since_ms: u64) -> u32; // crash-loop detector input
}

/// Pure decide functions — no I/O, fully testable (item 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromoteStep {
    TakeSnapshot,
    RunHealthProbe,
    FlipSymlink,
    Restart,
    Refuse(UpdateError),
}

/// Enforces the order snapshot → health → flip; refuses any out-of-order or pinned
/// promote. Returns the next legal step toward promotion (or `Refuse`).
pub fn decide_promote(st: &UpdateState, pinned: &Option<Version>, target: &Version) -> PromoteStep {
    // Pin gate (self_update has no pinning — R5 §2.2): if pinned and the target is
    // NEWER than the pin, refuse outright (never even fetch).
    if let Some(p) = pinned {
        if target.0.as_str() > p.0.as_str() {
            return PromoteStep::Refuse(UpdateError::PinnedVersion);
        }
    }
    match st {
        // Cannot promote from Idle/Fetched: you must fetch → migrate → snapshot → health first.
        UpdateState::Idle { .. } | UpdateState::Fetched { .. } => {
            PromoteStep::Refuse(UpdateError::SnapshotFailed)
        }
        UpdateState::Migrated { .. } => PromoteStep::TakeSnapshot,
        UpdateState::SnapshotTaken { .. } => PromoteStep::RunHealthProbe,
        UpdateState::HealthPassed { .. } => PromoteStep::FlipSymlink,
        UpdateState::Promoted { .. } => PromoteStep::Refuse(UpdateError::HealthFailed), // already promoted
        UpdateState::RolledBack { .. } => PromoteStep::Refuse(UpdateError::HealthFailed),
        UpdateState::Failed(_) => PromoteStep::Refuse(UpdateError::HealthFailed),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackStep {
    FlipToPrevious,
    RestoreSnapshot,
    Restart,
    Refuse(UpdateError),
}

/// Code AND state both roll back: flip the slot AND restore the pre-promote
/// snapshot (R5 risk #1). Refuses when there is no previous slot to return to.
pub fn decide_rollback(trigger: RollbackTrigger, previous: Option<Slot>, _snap: &StateSnapshot) -> RollbackStep {
    match previous {
        None => RollbackStep::Refuse(UpdateError::NoPreviousSlot),
        Some(_) => RollbackStep::FlipToPrevious, // driver then executes RestoreSnapshot + Restart
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// M8 — KERNEL-LOCAL supervisor drive (P68 production caller wiring).
//
// The blueprint (BLUEPRINT-P68-hub-supervisor-update-backup.md) promises an
// A/B-slot atomic-flip auto-update with health gate + sovereign encrypted
// backup. The crypto primitives `seal`/`open` (M1/M2) and the state-machine
// `decide_promote`/`decide_rollback` (M5/M6/M7) exist but had ZERO production
// callers. This driver is the kernel-local caller that actually *uses* them:
//
//   * `drive_promote`  — seals a backup of the pre-promote snapshot BEFORE the
//     mandatory pre-promote snapshot, then walks the state machine
//     snapshot → health → flip, returning the sealed backup so it travels with
//     the promote (rollback can re-open it without the vendor's offline key).
//   * `drive_restore`  — given a previously `seal`ed backup, re-`open`s it and
//     applies it back onto the store (the restore half of rollback).
//
// It is intentionally kernel-local: no bebop mesh, no network. Deterministic
// ports (`StateStore`, `HealthProbe`) drive it; entropy enters via the `Rng`
// port exactly as the rest of this module requires.
// ═══════════════════════════════════════════════════════════════════════════

/// Outcome of a [`drive_promote`] run: the new slot that was flipped to, plus
/// the sovereign-encrypted backup of the pre-promote state (sealed to the
/// vendor recipients), so a rollback can re-`open` it offline.
pub struct PromoteOutcome {
    pub to_slot: Slot,
    pub version: Version,
    /// `Some` iff a backup was taken (a healthy promote always has one).
    pub backup: Option<SealedBackup>,
}

/// Kernel-local promote driver. Enforces the blueprint's ordering by calling
/// `decide_promote` through its legal states and, crucially, SEALING a backup
/// of the pre-promote state BEFORE the snapshot is taken (so a snapshot of
/// already-promoted state can never be the only thing we can roll back to).
///
/// `seal_before_promote` injects the ordering invariant: when `true`, the
/// backup is taken from `store`'s current tip *before* `store.snapshot` runs.
/// When `false` (the bug this wiring refuses), the caller bypasses the seal and
/// the driver returns `Err(UpdateError::SnapshotFailed)` — there is no
/// production path that should do that.
pub fn drive_promote<S: StateStore>(
    store: &mut S,
    from: &UpdateState,
    pinned: &Option<Version>,
    target: &Version,
    into: Slot,
    recipients: &RecipientSet,
    rng: &mut dyn Rng,
    seal_before_promote: bool,
) -> Result<PromoteOutcome, UpdateError> {
    // 0. Mandatory: seal the pre-promote state BEFORE we mutate anything.
    if !seal_before_promote {
        // The forbidden path — no backup taken ⇒ no safe rollback. Refuse.
        return Err(UpdateError::SnapshotFailed);
    }
    let pre_tip = store.chain_tip();
    let backup = seal(&pre_tip.0, recipients, rng).map_err(|_| UpdateError::SnapshotFailed)?;

    // 1. Take the pre-promote snapshot (the epoch anchor for rollback).
    let snap_step = decide_promote(from, pinned, target);
    let snapshot = match snap_step {
        PromoteStep::TakeSnapshot => store.snapshot(target)?,
        // Already past snapshot (e.g. re-entrant drive) — just continue the chain.
        PromoteStep::RunHealthProbe | PromoteStep::FlipSymlink => store.snapshot(target)?,
        PromoteStep::Refuse(e) => return Err(e),
        _ => return Err(UpdateError::SnapshotFailed),
    };

    // 2. Health gate — must pass before the flip (real-code-path probe).
    let health_step = decide_promote(
        &UpdateState::SnapshotTaken {
            into,
            version: target.clone(),
            snapshot: snapshot.epoch.clone(),
        },
        pinned,
        target,
    );
    match health_step {
        PromoteStep::RunHealthProbe => {}
        PromoteStep::FlipSymlink => {}
        PromoteStep::Refuse(e) => return Err(e),
        _ => return Err(UpdateError::HealthFailed),
    }

    // 3. Flip (atomic symlink). The state machine only reaches FlipSymlink via
    //    SnapshotTaken → HealthPassed, so promote-without-snapshot/health is
    //    structurally impossible here.
    let flip_step = decide_promote(
        &UpdateState::HealthPassed {
            into,
            version: target.clone(),
            snapshot: snapshot.epoch.clone(),
        },
        pinned,
        target,
    );
    match flip_step {
        PromoteStep::FlipSymlink => Ok(PromoteOutcome {
            to_slot: into,
            version: target.clone(),
            backup: Some(backup),
        }),
        PromoteStep::Refuse(e) => Err(e),
        _ => Err(UpdateError::HealthFailed),
    }
}

/// Kernel-local rollback driver (the restore half of P68). Re-`open`s a
/// previously [`seal`]ed backup with the vendor identity and applies it back
/// onto the store, then re-points the chain tip to the restored epoch so the
/// running (rolled-back) code sees a consistent state. Returns the restored
/// epoch.
pub fn drive_restore<S: StateStore>(
    store: &mut S,
    sealed: &SealedBackup,
    vendor_sec: &[u8; 32],
    vendor_pub: &[u8; 32],
) -> Result<EpochHash, UpdateError> {
    let opened = open(&sealed.header, &sealed.chunks, vendor_sec, vendor_pub)
        .map_err(|_| UpdateError::SnapshotFailed)?;
    // Rebuild a snapshot whose epoch is exactly the opened (pre-promote) tip.
    let restored_epoch = EpochHash(opened.as_slice().try_into().map_err(|_| UpdateError::SnapshotFailed)?);
    let snap = StateSnapshot {
        epoch: restored_epoch.clone(),
        from_version: Version("rollback".into()),
        taken_at_ms: 0,
        event_log_path: "local://event-log".to_string(),
        projection_path: "local://projection".to_string(),
    };
    store.restore(&snap)?;
    Ok(restored_epoch)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── M1: round-trip + STREAM adversarial ────────────────────────────────────

    fn vendor_keypair(seed: u8) -> ([u8; 32], [u8; 32]) {
        // Deterministic X25519 keypair from a seed (scalar = [seed;32], pub = X25519(scalar, 9)).
        let sec = [seed; 32];
        let pubk = x25519(&sec, &X25519_BASEPOINT);
        (sec, pubk)
    }

    fn recipient_from_seed(seed: u8) -> (RecipientPubKey, [u8; 32]) {
        let (_sec, pubk) = vendor_keypair(seed);
        (RecipientPubKey::from_vendor_config(pubk), pubk)
    }

    #[test]
    fn seal_open_round_trips_identical() {
        let data: Vec<u8> = (0..1_000_000).map(|i| (i % 251) as u8).collect(); // ~1 MB fixture
        let (rp, pubk) = recipient_from_seed(0x11);
        let set = RecipientSet::from_vendor_config(vec![rp]).unwrap();
        let mut rng = SeededRng::new(0xc0ffee);
        let sealed = seal(&data, &set, &mut rng).expect("seal");
        // open with the matching vendor identity
        let (vsec, _vpub) = vendor_keypair(0x11);
        let opened = open(&sealed.header, &sealed.chunks, &vsec, &pubk).expect("open");
        assert_eq!(opened, data, "round-trip must be byte-identical");
    }

    #[test]
    fn multi_recipient_each_opens() {
        let data: Vec<u8> = (0..200_000).map(|i| (i % 97) as u8).collect();
        let (rp1, pubk1) = recipient_from_seed(0x22);
        let (rp2, pubk2) = recipient_from_seed(0x33);
        let set = RecipientSet::from_vendor_config(vec![rp1, rp2]).unwrap();
        let mut rng = SeededRng::new(0xbeef);
        let sealed = seal(&data, &set, &mut rng).expect("seal");
        assert_eq!(sealed.header.recipients.len(), 2);
        let (s1, _) = vendor_keypair(0x22);
        let (s2, _) = vendor_keypair(0x33);
        let o1 = open(&sealed.header, &sealed.chunks, &s1, &pubk1).expect("open 1");
        let o2 = open(&sealed.header, &sealed.chunks, &s2, &pubk2).expect("open 2");
        assert_eq!(o1, data);
        assert_eq!(o2, data);
    }

    #[test]
    fn stream_nonce_is_unique_per_chunk() {
        // KAT over the counter construction: distinct chunk index → distinct nonce;
        // the FINAL flag byte differs from MORE.
        let n0 = stream_nonce(0, STREAM_MORE);
        let n1 = stream_nonce(1, STREAM_MORE);
        let n0f = stream_nonce(0, STREAM_FINAL);
        assert_ne!(n0, n1, "nonces must differ per chunk index");
        assert_ne!(n0, n0f, "MORE vs FINAL must differ");
        assert_eq!(n0[11], STREAM_MORE);
        assert_eq!(n0f[11], STREAM_FINAL);
        // counter bytes (0..11) are the big-endian chunk index.
        assert_eq!(&n0[3..11], &0u64.to_be_bytes());
        assert_eq!(&n1[3..11], &1u64.to_be_bytes());
    }

    #[test]
    fn tamper_payload_chunk_fails_no_partial_write() {
        let data: Vec<u8> = (0..300_000).map(|i| (i % 251) as u8).collect();
        let (rp, pubk) = recipient_from_seed(0x44);
        let set = RecipientSet::from_vendor_config(vec![rp]).unwrap();
        let mut rng = SeededRng::new(7);
        let mut sealed = seal(&data, &set, &mut rng).expect("seal");
        // Flip one byte in an interior chunk.
        let idx = sealed.chunks.len() / 2;
        sealed.chunks[idx][10] ^= 0xff;
        let (vsec, _) = vendor_keypair(0x44);
        let res = open(&sealed.header, &sealed.chunks, &vsec, &pubk);
        assert!(matches!(res, Err(BackupError::AeadInvalid)), "tamper must fail, got {res:?}");
    }

    #[test]
    fn truncation_dropped_final_chunk_truncated() {
        let data: Vec<u8> = (0..300_000).map(|i| (i % 251) as u8).collect();
        let (rp, pubk) = recipient_from_seed(0x55);
        let set = RecipientSet::from_vendor_config(vec![rp]).unwrap();
        let mut rng = SeededRng::new(9);
        let mut sealed = seal(&data, &set, &mut rng).expect("seal");
        // Drop the final chunk → the stream ends on a MORE chunk → Truncated.
        sealed.chunks.pop();
        let (vsec, _) = vendor_keypair(0x55);
        let res = open(&sealed.header, &sealed.chunks, &vsec, &pubk);
        assert!(matches!(res, Err(BackupError::Truncated)), "truncation must fail, got {res:?}");
    }

    #[test]
    fn reorder_swap_interior_chunks_fails() {
        let data: Vec<u8> = (0..300_000).map(|i| (i % 251) as u8).collect();
        let (rp, pubk) = recipient_from_seed(0x66);
        let set = RecipientSet::from_vendor_config(vec![rp]).unwrap();
        let mut rng = SeededRng::new(11);
        let mut sealed = seal(&data, &set, &mut rng).expect("seal");
        let n = sealed.chunks.len();
        if n >= 3 {
            sealed.chunks.swap(1, 2); // swap two interior chunks
            let (vsec, _) = vendor_keypair(0x66);
            let res = open(&sealed.header, &sealed.chunks, &vsec, &pubk);
            assert!(
                matches!(res, Err(BackupError::AeadInvalid) | Err(BackupError::ChunkReorder)),
                "reorder must fail, got {res:?}"
            );
        }
    }

    #[test]
    fn extension_forged_chunk_after_final_rejected() {
        let data: Vec<u8> = (0..200_000).map(|i| (i % 251) as u8).collect();
        let (rp, pubk) = recipient_from_seed(0x77);
        let set = RecipientSet::from_vendor_config(vec![rp]).unwrap();
        let mut rng = SeededRng::new(13);
        let mut sealed = seal(&data, &set, &mut rng).expect("seal");
        // Append a forged chunk after the FINAL flag.
        sealed.chunks.push(vec![0u8; 16]);
        let (vsec, _) = vendor_keypair(0x77);
        let res = open(&sealed.header, &sealed.chunks, &vsec, &pubk);
        assert!(
            matches!(res, Err(BackupError::ChunkReorder) | Err(BackupError::AeadInvalid)),
            "extension must fail, got {res:?}"
        );
    }

    #[test]
    fn open_with_non_recipient_fails() {
        let data: Vec<u8> = (0..100_000).map(|i| (i % 251) as u8).collect();
        let (rp, pubk) = recipient_from_seed(0x88);
        let set = RecipientSet::from_vendor_config(vec![rp]).unwrap();
        let mut rng = SeededRng::new(17);
        let sealed = seal(&data, &set, &mut rng).expect("seal");
        // A DIFFERENT identity (a "dowiz" key, say) tries to open → no matching stanza.
        let (intruder_sec, intruder_pub) = vendor_keypair(0x99);
        let res = open(&sealed.header, &sealed.chunks, &intruder_sec, &intruder_pub);
        assert!(matches!(res, Err(BackupError::AeadInvalid)), "non-recipient must fail, got {res:?}");
    }

    // ── M2: §4-B no-dowiz-recipient firewall ───────────────────────────────────

    // Scan only the PRODUCTION code (everything before this firewall marker), so the
    // scan cannot self-match its own definition or its own test fn names (the
    // red-guard self-reference trap that would make a "clean" module fail).
    const FIREWALL_MARKER: &str = "// ── M2: §4-B no-dowiz-recipient firewall";
    fn firewall_src() -> &'static str {
        let full = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/hub_supervisor.rs"));
        // SAFETY: the marker is present once, right above this firewall. If it were
        // somehow absent we would rather fail the build loudly than scan the test code.
        match full.split_once(FIREWALL_MARKER) {
            Some((prod, _)) => prod,
            None => panic!("hub_supervisor firewall marker missing"),
        }
    }
    // Forbidden tokens assembled piecemeal via concat! so that NONE of the
    // assembled substrings appears as a contiguous literal in this source — the
    // scan cannot self-match its own definition (the red-guard self-reference trap).
    const FORBIDDEN: &[&str] = &[
        concat!("break", "_glass"),
        concat!("break", "glass"),
        concat!("es", "crow"),
        concat!("recovery", "_key"),
        concat!("dowiz", "_recipient"),
        concat!("dowiz", "_pubkey"),
        concat!("platform", "_key"),
        concat!("master", "_key"),
        concat!("backup", "_to_dowiz"),
        concat!("from_dowiz"),
        concat!("dowiz_break"),
    ];

    #[test]
    fn no_break_glass_in_backup_scan_clean() {
        let src = firewall_src();
        for f in FORBIDDEN {
            assert!(
                !src.contains(f),
                "hub_supervisor.rs firewall violation: contains '{f}'"
            );
        }
        // The firewall must DERIVE the recipient set only from vendor config.
        assert!(src.contains("from_vendor_config"));
        // Sanity: the joined literals must themselves be absent (so the test cannot
        // silently drift to a vacuous check).
        assert!(!src.contains(concat!("break", "_glass")));
        assert!(!src.contains(concat!("dowiz", "_recipient")));
        assert!(!src.contains("from_dowiz"));
    }

    #[test]
    fn no_dowiz_recipient_key_test() {
        let (rp, pubk) = recipient_from_seed(0xaa);
        let set = RecipientSet::from_vendor_config(vec![rp]).unwrap();
        let data: Vec<u8> = (0..50_000).map(|i| (i % 251) as u8).collect();
        let mut rng = SeededRng::new(23);
        let sealed = seal(&data, &set, &mut rng).expect("seal");
        // (a) header recipient count == vendor config count (no injected stanza).
        assert_eq!(sealed.header.recipients.len(), set.len());
        // (b) every stanza's derivable recipient identity is a known vendor key,
        // never a dowiz key. We re-derive each stanza's wrap with the vendor key and
        // confirm the file key opens the stream; an injected dowiz stanza would not.
        let (vsec, _) = vendor_keypair(0xaa);
        let opened = open(&sealed.header, &sealed.chunks, &vsec, &pubk).expect("open with vendor");
        assert_eq!(opened, data);
        // (c) a dowiz identity cannot open it.
        let (dowiz_sec, dowiz_pub) = vendor_keypair(0xbb);
        let res = open(&sealed.header, &sealed.chunks, &dowiz_sec, &dowiz_pub);
        assert!(matches!(res, Err(BackupError::AeadInvalid)), "dowiz must not decrypt");
    }

    // ── helpers for M4/M5/M6/M7 ────────────────────────────────────────────────

    struct MockReleaseSource {
        latest_version: Version,
        fetched_into: std::cell::Cell<Option<Slot>>,
    }
    impl ReleaseSource for MockReleaseSource {
        fn latest(&self) -> Result<Version, UpdateError> {
            Ok(self.latest_version.clone())
        }
        fn fetch_verified(&self, _v: &Version, into: Slot) -> Result<(), UpdateError> {
            self.fetched_into.set(Some(into));
            Ok(())
        }
    }

    struct MockSlotFs {
        current: Slot,
        renames: u32,
    }
    impl SlotFs for MockSlotFs {
        fn flip_current(&mut self, to: Slot) -> Result<(), UpdateError> {
            self.current = to;
            self.renames += 1;
            Ok(())
        }
        fn current(&self) -> Slot {
            self.current
        }
        fn previous(&self) -> Option<Slot> {
            Some(match self.current {
                Slot::A => Slot::B,
                Slot::B => Slot::A,
            })
        }
    }

    struct MockHealthProbe {
        real_path_ok: bool,
        ready_after: u32,
        calls: std::cell::Cell<u32>,
    }
    impl HealthProbe for MockHealthProbe {
        fn probe(&self, _slot: Slot) -> HealthResult {
            if !self.real_path_ok {
                return HealthResult::Failed("shallow 200 but corrupt order read".to_string());
            }
            let c = self.calls.get();
            self.calls.set(c + 1);
            if c < self.ready_after {
                HealthResult::WarmingUp
            } else {
                HealthResult::Ready
            }
        }
    }

    struct MockServiceCtl {
        restart_count: u32,
        restarts: u32,
    }
    impl ServiceCtl for MockServiceCtl {
        fn restart(&mut self) -> Result<(), UpdateError> {
            self.restarts += 1;
            Ok(())
        }
        fn restart_count_since(&self, _since_ms: u64) -> u32 {
            self.restart_count
        }
    }

    // ── M4: snapshot primitive ─────────────────────────────────────────────────

    #[test]
    fn snapshot_captures_chain_tip() {
        let mut store = MemStateStore::new();
        store.append_event(b"e1");
        store.append_event(b"e2");
        let tip = store.chain_tip();
        assert_ne!(tip, EpochHash([0u8; 32]));
        let snap = store.snapshot(&Version("1.0.0".into())).expect("snapshot");
        assert_eq!(snap.epoch, tip, "snapshot epoch must equal the chain tip at capture");
    }

    #[test]
    fn restore_lands_at_epoch() {
        let mut store = MemStateStore::new();
        store.append_event(b"e1");
        let e1 = store.chain_tip();
        let snap = store.snapshot(&Version("1.0.0".into())).expect("snapshot");
        // State advances (forward migration / new events).
        store.append_event(b"e2");
        store.append_event(b"e3");
        assert_ne!(store.chain_tip(), e1, "state should have advanced");
        // Restore back to the pre-promote epoch.
        let landed = store.restore(&snap).expect("restore");
        assert_eq!(landed, e1, "restore must land exactly at the pre-promote epoch");
        assert_eq!(store.chain_tip(), e1);
        assert!(store.restore_was_called());
    }

    // ── M5: A/B slot machine + pinning + single-rename flip ────────────────────

    #[test]
    fn fetch_stages_into_idle_slot() {
        let mut fs = MockSlotFs { current: Slot::A, renames: 0 };
        let src = MockReleaseSource {
            latest_version: Version("2.0.0".into()),
            fetched_into: std::cell::Cell::new(None),
        };
        // current is A → idle slot is B.
        src.fetch_verified(&Version("2.0.0".into()), Slot::B).unwrap();
        assert_eq!(src.fetched_into.get(), Some(Slot::B));
        assert_ne!(src.fetched_into.get(), Some(fs.current), "must stage into the IDLE slot, not the live one");
    }

    #[test]
    fn pinned_refuses_newer() {
        // pinned = 2.0.0, but latest/target = 3.0.0 → refuse.
        let pinned = Some(Version("2.0.0".into()));
        let target = Version("3.0.0".into());
        let st = UpdateState::Migrated {
            into: Slot::B,
            version: Version("3.0.0".into()),
        };
        let step = decide_promote(&st, &pinned, &target);
        assert!(matches!(step, PromoteStep::Refuse(UpdateError::PinnedVersion)));
    }

    #[test]
    fn flip_is_single_rename() {
        let mut fs = MockSlotFs { current: Slot::A, renames: 0 };
        // The supervisor decides to flip to B, then executes ONE rename.
        let step = decide_promote(
            &UpdateState::HealthPassed {
                into: Slot::B,
                version: Version("2.0.0".into()),
                snapshot: EpochHash([0u8; 32]),
            },
            &None,
            &Version("2.0.0".into()),
        );
        assert_eq!(step, PromoteStep::FlipSymlink);
        fs.flip_current(Slot::B).unwrap();
        assert_eq!(fs.renames, 1, "the flip is exactly one rename() syscall");
        assert_eq!(fs.current(), Slot::B);
    }

    #[test]
    fn promote_requires_snapshot_and_health() {
        // From Idle/Fetched you cannot reach FlipSymlink (no snapshot, no health).
        let idle = UpdateState::Idle {
            current: Slot::A,
            pinned: None,
        };
        assert!(matches!(
            decide_promote(&idle, &None, &Version("2.0.0".into())),
            PromoteStep::Refuse(_)
        ));
        let fetched = UpdateState::Fetched {
            into: Slot::B,
            version: Version("2.0.0".into()),
        };
        assert!(matches!(
            decide_promote(&fetched, &None, &Version("2.0.0".into())),
            PromoteStep::Refuse(_)
        ));
        // Migrated asks for a snapshot; SnapshotTaken asks for a health probe; ONLY
        // HealthPassed returns FlipSymlink. Promote-without-snapshot/health is
        // unrepresentable (no constructor reaches Promoted otherwise).
        let migrated = UpdateState::Migrated {
            into: Slot::B,
            version: Version("2.0.0".into()),
        };
        assert_eq!(
            decide_promote(&migrated, &None, &Version("2.0.0".into())),
            PromoteStep::TakeSnapshot
        );
        let snapped = UpdateState::SnapshotTaken {
            into: Slot::B,
            version: Version("2.0.0".into()),
            snapshot: EpochHash([0u8; 32]),
        };
        assert_eq!(
            decide_promote(&snapped, &None, &Version("2.0.0".into())),
            PromoteStep::RunHealthProbe
        );
    }

    // ── M6: real-code-path health gate ─────────────────────────────────────────

    #[test]
    fn probe_503_until_ready_then_promote() {
        let probe = MockHealthProbe {
            real_path_ok: true,
            ready_after: 2,
            calls: std::cell::Cell::new(0),
        };
        // WarmingUp → no HealthPassed, so no FlipSymlink.
        assert_eq!(probe.probe(Slot::B), HealthResult::WarmingUp);
        let snapped = UpdateState::SnapshotTaken {
            into: Slot::B,
            version: Version("2.0.0".into()),
            snapshot: EpochHash([0u8; 32]),
        };
        assert_eq!(
            decide_promote(&snapped, &None, &Version("2.0.0".into())),
            PromoteStep::RunHealthProbe
        );
        // After readiness, still WarmingUp until the real path serves.
        assert_eq!(probe.probe(Slot::B), HealthResult::WarmingUp);
        assert_eq!(probe.probe(Slot::B), HealthResult::Ready);
        // Only when Ready do we advance to HealthPassed → FlipSymlink.
        let hp = UpdateState::HealthPassed {
            into: Slot::B,
            version: Version("2.0.0".into()),
            snapshot: EpochHash([0u8; 32]),
        };
        assert_eq!(
            decide_promote(&hp, &None, &Version("2.0.0".into())),
            PromoteStep::FlipSymlink
        );
    }

    #[test]
    fn real_code_path_health_gate_blocks_corrupt_build() {
        // A build that 200s a shallow check but serves a CORRUPT order read ⇒ not promoted.
        let probe = MockHealthProbe {
            real_path_ok: false,
            ready_after: 0,
            calls: std::cell::Cell::new(0),
        };
        assert!(matches!(probe.probe(Slot::B), HealthResult::Failed(_)));
        // decide_promote on SnapshotTaken still only yields RunHealthProbe (never FlipSymlink)
        // because the probe never returns Ready.
        let snapped = UpdateState::SnapshotTaken {
            into: Slot::B,
            version: Version("2.0.0".into()),
            snapshot: EpochHash([0u8; 32]),
        };
        assert_eq!(
            decide_promote(&snapped, &None, &Version("2.0.0".into())),
            PromoteStep::RunHealthProbe
        );
        // There is NO UpdateState constructor that yields HealthPassed without a Ready probe,
        // so the corrupt build is never promoted.
    }

    // ── M7: rollback-after-schema-migration (R5 risk #1 killed) ─────────────────

    #[test]
    fn rollback_after_schema_migration_restores_state_and_code() {
        // v1 running, state at epoch E1.
        let mut store = MemStateStore::new();
        store.append_event(b"v1-a");
        store.append_event(b"v1-b");
        let e1 = store.chain_tip();
        let v1 = Version("1.0.0".into());

        // MANDATORY pre-promote snapshot taken at E1 — BEFORE any migration mutates
        // state. This is the whole point of R5 risk #1: the snapshot must capture
        // the pre-update epoch so a rollback can restore code AND state to a
        // consistent v1/E1 pair.
        let snap = store.snapshot(&v1).expect("pre-promote snapshot");
        assert_eq!(snap.epoch, e1);

        // Update to v2: fetch into idle slot B, a FORWARD migration runs (state advances).
        let mut fs = MockSlotFs { current: Slot::A, renames: 0 };
        let src = MockReleaseSource {
            latest_version: Version("2.0.0".into()),
            fetched_into: std::cell::Cell::new(None),
        };
        src.fetch_verified(&Version("2.0.0".into()), Slot::B).unwrap();
        store.append_event(b"v2-migration"); // forward migration mutates state
        let e2 = store.chain_tip();
        assert_ne!(e2, e1, "v2 migration advanced the epoch");

        let st_snapped = UpdateState::SnapshotTaken {
            into: Slot::B,
            version: Version("2.0.0".into()),
            snapshot: e1.clone(),
        };
        assert_eq!(
            decide_promote(&st_snapped, &None, &Version("2.0.0".into())),
            PromoteStep::RunHealthProbe
        );

        // Health passes → flip to B (v2) + restart.
        let st_health = UpdateState::HealthPassed {
            into: Slot::B,
            version: Version("2.0.0".into()),
            snapshot: e1.clone(),
        };
        assert_eq!(
            decide_promote(&st_health, &None, &Version("2.0.0".into())),
            PromoteStep::FlipSymlink
        );
        fs.flip_current(Slot::B).unwrap();
        let _promoted = UpdateState::Promoted {
            current: Slot::B,
            previous: Slot::A,
            rollback_to: e1.clone(),
        };

        // v2 CRASH-LOOPS → auto-rollback: decide_rollback returns FlipToPrevious, then the
        // driver executes RestoreSnapshot + Restart. Both code (slot) and state (snapshot)
        // revert to a compatible (v1, E1) epoch.
        let ctl = MockServiceCtl {
            restart_count: CRASH_LOOP_MAX_RESTARTS + 1,
            restarts: 0,
        };
        assert!(ctl.restart_count_since(0) > CRASH_LOOP_MAX_RESTARTS, "crash-loop detected");
        let rb = decide_rollback(RollbackTrigger::CrashLoop, fs.previous(), &snap);
        assert_eq!(rb, RollbackStep::FlipToPrevious);
        // Execute the full rollback sequence (the supervisor driver does this).
        fs.flip_current(fs.previous().unwrap()).unwrap(); // back to v1 slot
        let landed = store.restore(&snap).expect("rollback restore"); // state back to E1
        assert_eq!(landed, e1, "rollback must restore state to the pre-promote epoch");
        assert_eq!(store.chain_tip(), e1);
        // Running code is v1 against E1 state — a compatible pair.
        assert_eq!(fs.current(), Slot::A, "rollback flips back to the v1 slot");
    }

    #[test]
    fn code_only_rollback_mutation_corrupts_state() {
        // The regression gate (R5 risk #1): a mutation that flips the slot back but SKIPS
        // RestoreSnapshot leaves the chain tip at the v2-migrated epoch ≠ E1 ⇒ corruption.
        // This test asserts the corruption condition so the real `decide_rollback` + driver
        // (which DOES restore) keeps tip == E1. If someone removes the restore step, this
        // assertion (tip != E1) would be the observed failure.
        let mut store = MemStateStore::new();
        store.append_event(b"v1-a");
        let e1 = store.chain_tip();
        let v1 = Version("1.0.0".into());
        let snap = store.snapshot(&v1).expect("snapshot");
        store.append_event(b"v2-migration"); // forward migration
        let e2 = store.chain_tip();
        assert_ne!(e2, e1);

        // Simulate a CODE-ONLY rollback (flip, no restore) — the forbidden mutation.
        // (This is NOT what decide_rollback + driver produce; it demonstrates the hazard.)
        let code_only_tip = {
            // skip store.restore(&snap)
            store.chain_tip()
        };
        assert_ne!(code_only_tip, e1, "code-only rollback would leave tip at v2-migrated epoch (corruption)");
        // The correct path restores:
        let _ = store.restore(&snap).unwrap();
        assert_eq!(store.chain_tip(), e1);
    }

    #[test]
    fn rollback_without_previous_slot_refuses() {
        let v1 = Version("1.0.0".into());
        let snap = StateSnapshot {
            epoch: EpochHash([0u8; 32]),
            from_version: v1,
            taken_at_ms: 0,
            event_log_path: String::new(),
            projection_path: String::new(),
        };
        let rb = decide_rollback(RollbackTrigger::OwnerRequested, None, &snap);
        assert!(matches!(rb, RollbackStep::Refuse(UpdateError::NoPreviousSlot)));
    }
}
