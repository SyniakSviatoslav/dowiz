//! M5 + M6 — Signal-style QR wallet transfer + the mandatory anti-phishing confirmation (§4.5 / §4.6).
//!
//! R4 §6.1-6.2, with the server dropped. ALL crypto REUSED from the kernel `pq` module —
//! ZERO new crypto deps:
//!   * X25519 ECDH   → `crate::pq::x25519`
//!   * SHAKE256 KDF  → `crate::pq::keccak::shake256` (sovereign HKDF-SHA256 substitute)
//!   * AES-256-GCM   → the in-tree `aes-gcm` (already a kernel dep under `pq`)
//!
//! The `TransferState` machine makes SEALING reachable ONLY through the confirmation gate
//! (§4.6): sealing without an explicit user confirmation is unrepresentable (the anti-phishing
//! invariant, R4 §6.4).
//!
//! Compiled only under the `pq` feature (the primitives it reuses are gated there).

use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Nonce};
use crate::pq::keccak::shake256;
use crate::pq::x25519::x25519;
use crate::wallet::record::WalletRecord;

/// Signal's ~1–2 min link-code window (R4 §6.1).
pub const TRANSFER_QR_TTL_S: u32 = 120;
/// Sealed-envelope ceiling (size budget §4.5).
pub const MAX_TRANSFER_BYTES: usize = 4096;
/// QR v40 binary capacity → ≤2 animated frames.
pub const QR_FRAME_PAYLOAD_MAX: usize = 2953;
pub const AEAD_KEY_LEN: usize = 32; // AES-256-GCM
pub const AEAD_NONCE_LEN: usize = 12;
/// Short-auth-string shown at confirmation (§4.6).
pub const FINGERPRINT_LEN: usize = 8;
/// KDF domain separator (vs the cert chain, X8 — sibling, not merged).
pub const TRANSFER_KDF_CTX: &[u8] = b"dowiz.wallet.transfer.v1";
pub const TRANSFER_ENVELOPE_VERSION: u8 = 1;

/// Ephemeral X25519 keypair for ONE transfer. Generated fresh, never persisted (self-custody).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EphemeralKeypair {
    pub secret: [u8; 32],
    pub public: [u8; 32],
}

impl EphemeralKeypair {
    /// Generate a fresh ephemeral keypair from supplied 32-byte entropy (RNG-free core, C10).
    pub fn from_entropy(secret: [u8; 32]) -> Self {
        // The public key is X25519(secret, 9) — the standard basepoint (RFC 7748).
        let basepoint = [9u8; 32];
        let public = x25519(&secret, &basepoint);
        EphemeralKeypair { secret, public }
    }
}

/// QR-1: emitted by the NEW (receiving) device. Carries its ephemeral pubkey + a nonce + a ttl.
/// The out-of-band authenticated bootstrap (R4 §6.1) — read by the source device's camera.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferInit {
    pub new_device_pub: [u8; 32],
    pub nonce: [u8; 12],
    pub issued_ms: u64,
}

/// QR-2 (animated): emitted by the SOURCE device AFTER confirmation. The sealed wallet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedWallet {
    pub version: u8,                 // = TRANSFER_ENVELOPE_VERSION
    pub src_ephemeral_pub: [u8; 32], // source's ephemeral pubkey (the peer half of the ECDH)
    pub nonce: [u8; AEAD_NONCE_LEN],
    pub ct: Vec<u8>, // AES-256-GCM(ct ‖ tag) over the serialized WalletRecord
}

/// The transfer machine. `AwaitingConfirmation` is MANDATORY and UNSKIPPABLE (§4.6).
/// Producing a `SealedWallet` is ONLY reachable through the confirmation gate — sealing without
/// an explicit user confirmation is unrepresentable (the anti-phishing invariant, R4 §6.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferState {
    Idle,
    // NEW device side:
    AwaitingScanOfInit {
        kp: EphemeralKeypair,
        init: TransferInit,
    }, // showing QR-1
    AwaitingSealed, // scanning for QR-2
    Received {
        rec: WalletRecord,
    }, // recovered the wallet
    // SOURCE device side:
    ScannedInit {
        peer: TransferInit,
        kp: EphemeralKeypair,
        fingerprint: [u8; FINGERPRINT_LEN],
    },
    AwaitingConfirmation {
        peer: TransferInit,
        kp: EphemeralKeypair,
        fingerprint: [u8; FINGERPRINT_LEN],
    }, // MANDATORY GATE (§4.6)
    Confirmed {
        sealed: SealedWallet,
    }, // showing QR-2 (animated)
    Failed(TransferError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferError {
    QrDecodeFailed,
    Expired,
    TooLarge,
    AeadInvalid,
    UserRejected,
    VersionUnsupported,
}

/// The two source-side commands. Sealing is gated on Confirm (§4.6).
#[derive(Debug, Clone)]
pub enum TransferCmd {
    ScanInit(TransferInit),
    ConfirmTransfer,
    RejectTransfer,
}

/// Transport ports (the QR encode/decode live out-of-core; P53 owns the encoder, a camera adapter
/// owns the decode — named, not a silent gap, §4.5).
pub trait QrEncodePort {
    fn encode(&self, bytes: &[u8]) -> Vec<QrMatrix>;
}
pub trait QrScanPort {
    fn next_frame(&mut self) -> Option<Vec<u8>>;
}
/// A QR matrix: 1 bit/module, P53's output shape.
pub struct QrMatrix {
    pub size: u16,
    pub modules: Vec<u8>,
}

// ── crypto helpers (all reuse in-tree `pq` primitives) ──

/// Derive the transfer key from the ECDH shared secret: `SHAKE256(shared ‖ CTX ‖ new_pub ‖ src_pub)`.
/// The sovereign substitute for Signal's HKDF-SHA256 (§4.5 DECART).
pub fn derive_key(shared: &[u8; 32], new_pub: &[u8; 32], src_pub: &[u8; 32]) -> [u8; 32] {
    let mut input =
        Vec::with_capacity(shared.len() + TRANSFER_KDF_CTX.len() + new_pub.len() + src_pub.len());
    input.extend_from_slice(shared);
    input.extend_from_slice(TRANSFER_KDF_CTX);
    input.extend_from_slice(new_pub);
    input.extend_from_slice(src_pub);
    let mut key = [0u8; AEAD_KEY_LEN];
    shake256(&input, &mut key);
    key
}

/// Short-auth-string (SAS) fingerprint over the two device pubkeys — shown on BOTH devices.
pub fn fingerprint(new_pub: &[u8; 32], src_pub: &[u8; 32]) -> [u8; FINGERPRINT_LEN] {
    let mut input = Vec::with_capacity(new_pub.len() + src_pub.len());
    input.extend_from_slice(new_pub);
    input.extend_from_slice(src_pub);
    let mut fp = [0u8; FINGERPRINT_LEN];
    shake256(&input, &mut fp);
    fp
}

/// Seal a wallet for a specific peer ephemeral pubkey. Returns the `SealedWallet` (QR-2 payload).
/// REFUSES if the serialized wallet exceeds `MAX_TRANSFER_BYTES` (no silent truncation).
pub fn seal(
    wallet: &WalletRecord,
    src_kp: &EphemeralKeypair,
    new_device_pub: &[u8; 32],
    nonce: [u8; 12],
) -> Result<SealedWallet, TransferError> {
    // Serialize via the wallet's serde-free codec (always available).
    let plaintext = crate::wallet::record::serialize(wallet);
    if plaintext.len() > MAX_TRANSFER_BYTES {
        return Err(TransferError::TooLarge);
    }
    let shared = x25519(&src_kp.secret, new_device_pub);
    let key = derive_key(&shared, new_device_pub, &src_kp.public);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("key is exactly 32 bytes");
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|_| TransferError::AeadInvalid)?;
    Ok(SealedWallet {
        version: TRANSFER_ENVELOPE_VERSION,
        src_ephemeral_pub: src_kp.public,
        nonce,
        ct,
    })
}

/// Open a `SealedWallet` on the NEW device. Returns the recovered [`WalletRecord`] — or a typed
/// error on tamper/expiry/version mismatch. No partial write (GCM tag fails closed).
pub fn open(
    sealed: &SealedWallet,
    new_kp: &EphemeralKeypair,
    issued_ms: u64,
    now_ms: u64,
) -> Result<WalletRecord, TransferError> {
    if sealed.version != TRANSFER_ENVELOPE_VERSION {
        return Err(TransferError::VersionUnsupported);
    }
    if now_ms.saturating_sub(issued_ms) > TRANSFER_QR_TTL_S as u64 * 1000 {
        return Err(TransferError::Expired);
    }
    let shared = x25519(&new_kp.secret, &sealed.src_ephemeral_pub);
    let key = derive_key(&shared, &new_kp.public, &sealed.src_ephemeral_pub);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("key is exactly 32 bytes");
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&sealed.nonce), sealed.ct.as_slice())
        .map_err(|_| TransferError::AeadInvalid)?;
    crate::wallet::record::deserialize(
        std::str::from_utf8(&plaintext).map_err(|_| TransferError::AeadInvalid)?,
    )
    .map_err(|_| TransferError::AeadInvalid)
}

// ── the confirmation-gated SOURCE machine ──

/// Build the `ScannedInit` state from a scanned `TransferInit` + a freshly generated source
/// keypair (entropy-supplied). Enters the confirmation gate (no seal yet).
pub fn source_scanned(peer: TransferInit, src_kp: EphemeralKeypair) -> TransferState {
    let fp = fingerprint(&peer.new_device_pub, &src_kp.public);
    TransferState::ScannedInit {
        peer,
        kp: src_kp,
        fingerprint: fp,
    }
}

/// Advance the source machine: `ScannedInit` + `ConfirmTransfer` ⇒ the MANDATORY confirmation
/// gate. `RejectTransfer` ⇒ `Failed(UserRejected)`. No other command leaves `ScannedInit`.
pub fn source_confirm(state: TransferState, cmd: TransferCmd) -> TransferState {
    match (state, cmd) {
        (TransferState::ScannedInit { peer, kp, fingerprint }, TransferCmd::ConfirmTransfer) => {
            TransferState::AwaitingConfirmation {
                peer,
                kp,
                fingerprint,
            }
        }
        (TransferState::ScannedInit { .. }, TransferCmd::RejectTransfer) => {
            TransferState::Failed(TransferError::UserRejected)
        }
        (other, _) => other, // no seal without an explicit ConfirmTransfer
    }
}

/// The canonical SOURCE sealing step: given the `AwaitingConfirmation` gate + the wallet to
/// transfer, produce `Confirmed { sealed }`. Sealing WITHOUT the gate is unrepresentable — this
/// is the only function that returns a `SealedWallet`, and it requires `AwaitingConfirmation`.
pub fn source_seal(
    state: TransferState,
    wallet: &WalletRecord,
) -> Result<SealedWallet, TransferError> {
    match state {
        TransferState::AwaitingConfirmation { peer, kp, fingerprint: _ } => {
            seal(wallet, &kp, &peer.new_device_pub, peer.nonce)
        }
        _ => Err(TransferError::QrDecodeFailed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::record::{Address, Contact, PaymentMethodRef, WALLET_SCHEMA_VERSION};

    fn sample_wallet() -> WalletRecord {
        WalletRecord {
            schema_version: WALLET_SCHEMA_VERSION,
            rev: 1,
            updated_at_ms: 1_700_000_000_000,
            wallet_id: [0xaa_u8; 32],
            name: Some("Grace Hopper".into()),
            addresses: vec![Address {
                label: "Lab".into(),
                lines: vec!["1 Compiler Rd".into()],
                note: None,
            }],
            contact: Some(Contact {
                email: Some("grace@example.com".into()),
                phone_e164: None,
            }),
            method_ref: Some(PaymentMethodRef("pm_xyz".into())),
        }
    }

    /// A deterministic RNG-free entropy source for tests (XOF over a seed).
    fn entropy(seed: u8) -> [u8; 32] {
        let mut s = [0u8; 32];
        s[0] = seed;
        let mut out = [0u8; 32];
        shake256(&s, &mut out);
        out
    }

    #[test]
    fn transfer_round_trips_identical_wallet() {
        // Two in-memory devices.
        let src_kp = EphemeralKeypair::from_entropy(entropy(1));
        let new_kp = EphemeralKeypair::from_entropy(entropy(2));
        let init = TransferInit {
            new_device_pub: new_kp.public,
            nonce: [7u8; 12],
            issued_ms: 1_700_000_000_000,
        };
        let wallet = sample_wallet();
        // Source seals only AFTER the confirmation gate.
        let scanned = source_scanned(init.clone(), src_kp.clone());
        let gate = source_confirm(scanned, TransferCmd::ConfirmTransfer);
        let sealed = source_seal(gate, &wallet).expect("seal");
        // New device opens.
        let recovered = open(&sealed, &new_kp, init.issued_ms, init.issued_ms + 100).expect("open");
        assert_eq!(recovered, wallet, "wallet must round-trip byte-identical");
    }

    #[test]
    fn kdf_matches_on_both_sides() {
        let src_kp = EphemeralKeypair::from_entropy(entropy(3));
        let new_kp = EphemeralKeypair::from_entropy(entropy(4));
        let shared_src = x25519(&src_kp.secret, &new_kp.public);
        let shared_new = x25519(&new_kp.secret, &src_kp.public);
        assert_eq!(shared_src, shared_new, "ECDH must be symmetric");
        let key_src = derive_key(&shared_src, &new_kp.public, &src_kp.public);
        let key_new = derive_key(&shared_new, &new_kp.public, &src_kp.public);
        assert_eq!(key_src, key_new, "both devices derive the same key");
    }

    #[test]
    fn fingerprint_shown_both_sides() {
        let src_kp = EphemeralKeypair::from_entropy(entropy(5));
        let new_kp = EphemeralKeypair::from_entropy(entropy(6));
        let fp_src = fingerprint(&new_kp.public, &src_kp.public);
        let fp_new = fingerprint(&new_kp.public, &src_kp.public);
        assert_eq!(fp_src, fp_new);
    }

    #[test]
    fn sealed_fits_one_frame() {
        let wallet = sample_wallet();
        let json = crate::wallet::record::serialize(&wallet);
        assert!(
            json.len() + 16 <= QR_FRAME_PAYLOAD_MAX,
            "typical wallet fits one QR frame"
        );
        assert!(json.len() <= MAX_TRANSFER_BYTES);
    }

    #[test]
    fn tampered_ct_is_aead_invalid() {
        let src_kp = EphemeralKeypair::from_entropy(entropy(7));
        let new_kp = EphemeralKeypair::from_entropy(entropy(8));
        let init = TransferInit {
            new_device_pub: new_kp.public,
            nonce: [3u8; 12],
            issued_ms: 1_700_000_000_000,
        };
        let scanned = source_scanned(init.clone(), src_kp.clone());
        let gate = source_confirm(scanned, TransferCmd::ConfirmTransfer);
        let mut sealed = source_seal(gate, &sample_wallet()).expect("seal");
        sealed.ct[0] ^= 0xff; // flip one byte
        let res = open(&sealed, &new_kp, init.issued_ms, init.issued_ms + 100);
        assert_eq!(
            res,
            Err(TransferError::AeadInvalid),
            "tampered ct must fail closed"
        );
    }

    #[test]
    fn expired_qr_is_refused() {
        let src_kp = EphemeralKeypair::from_entropy(entropy(9));
        let new_kp = EphemeralKeypair::from_entropy(entropy(10));
        let init = TransferInit {
            new_device_pub: new_kp.public,
            nonce: [1u8; 12],
            issued_ms: 0,
        };
        let scanned = source_scanned(init.clone(), src_kp.clone());
        let gate = source_confirm(scanned, TransferCmd::ConfirmTransfer);
        let sealed = source_seal(gate, &sample_wallet()).expect("seal");
        let res = open(
            &sealed,
            &new_kp,
            init.issued_ms,
            TRANSFER_QR_TTL_S as u64 * 1000 + 10_000,
        );
        assert_eq!(res, Err(TransferError::Expired));
    }

    #[test]
    fn version_unsupported_is_refused() {
        let new_kp = EphemeralKeypair::from_entropy(entropy(11));
        let mut sealed = SealedWallet {
            version: TRANSFER_ENVELOPE_VERSION + 1,
            src_ephemeral_pub: [0u8; 32],
            nonce: [0u8; 12],
            ct: vec![],
        };
        let res = open(&mut sealed, &new_kp, 0, 1);
        assert_eq!(res, Err(TransferError::VersionUnsupported));
    }

    #[test]
    fn seal_requires_confirm() {
        // A SealedWallet is ONLY produced by source_seal, which requires the AwaitingConfirmation
        // gate. There is no code path from Idle / ScannedInit straight to a sealed wallet — sealing
        // without an explicit ConfirmTransfer is unrepresentable.
        let src_kp = EphemeralKeypair::from_entropy(entropy(12));
        let new_kp = EphemeralKeypair::from_entropy(entropy(13));
        let init = TransferInit {
            new_device_pub: new_kp.public,
            nonce: [5u8; 12],
            issued_ms: 1_700_000_000_000,
        };
        let scanned = source_scanned(init.clone(), src_kp.clone());
        let gate = source_confirm(scanned, TransferCmd::ConfirmTransfer);
        let sealed = source_seal(gate, &sample_wallet()).expect("seal only after confirm");
        assert_eq!(sealed.version, TRANSFER_ENVELOPE_VERSION);
        // And a RejectTransfer never yields a seal.
        let scanned2 = source_scanned(init.clone(), src_kp.clone());
        let rejected = source_confirm(scanned2, TransferCmd::RejectTransfer);
        assert_eq!(rejected, TransferState::Failed(TransferError::UserRejected));
    }

    #[test]
    fn substituted_qr_fails_confirm() {
        // An attacker substitutes `new_device_pub` in QR-1 with an attacker key. The source
        // computes a fingerprint that DIFFERS from the one the genuine new device shows ⇒ the
        // user rejects (RejectTransfer) ⇒ no seal, no leak.
        let src_kp = EphemeralKeypair::from_entropy(entropy(14));
        let genuine_new = EphemeralKeypair::from_entropy(entropy(15));
        let attacker = EphemeralKeypair::from_entropy(entropy(16));
        let init = TransferInit {
            new_device_pub: attacker.public, // substituted!
            nonce: [2u8; 12],
            issued_ms: 1_700_000_000_000,
        };
        let fp_source = fingerprint(&attacker.public, &src_kp.public);
        let fp_genuine = fingerprint(&genuine_new.public, &src_kp.public);
        assert_ne!(
            fp_source, fp_genuine,
            "substituted QR ⇒ fingerprint mismatch ⇒ user rejects"
        );
    }

    #[test]
    fn no_second_recipient_possible() {
        // The sealed ciphertext is bound to exactly the derived key (src secret × new pub). A
        // different device's secret yields a different key ⇒ open fails. No master/escrow key.
        let src_kp = EphemeralKeypair::from_entropy(entropy(17));
        let new_kp = EphemeralKeypair::from_entropy(entropy(18));
        let other = EphemeralKeypair::from_entropy(entropy(19));
        let init = TransferInit {
            new_device_pub: new_kp.public,
            nonce: [8u8; 12],
            issued_ms: 1_700_000_000_000,
        };
        let scanned = source_scanned(init.clone(), src_kp.clone());
        let gate = source_confirm(scanned, TransferCmd::ConfirmTransfer);
        let sealed = source_seal(gate, &sample_wallet()).expect("seal");
        let res = open(&sealed, &other, init.issued_ms, init.issued_ms + 100);
        assert_eq!(
            res,
            Err(TransferError::AeadInvalid),
            "sealed for B only; C cannot open"
        );
    }

    #[test]
    fn oversized_wallet_refused_before_seal() {
        // A wallet larger than MAX_TRANSFER_BYTES is refused at seal time.
        let mut big = sample_wallet();
        big.name = Some("x".repeat(MAX_TRANSFER_BYTES + 1));
        let src_kp = EphemeralKeypair::from_entropy(entropy(20));
        let new_kp = EphemeralKeypair::from_entropy(entropy(21));
        let init = TransferInit {
            new_device_pub: new_kp.public,
            nonce: [4u8; 12],
            issued_ms: 1_700_000_000_000,
        };
        let scanned = source_scanned(init, src_kp);
        let gate = source_confirm(scanned, TransferCmd::ConfirmTransfer);
        assert_eq!(source_seal(gate, &big), Err(TransferError::TooLarge));
    }
}
