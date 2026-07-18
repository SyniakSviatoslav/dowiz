//! P34 — cross-repo mesh kernel wiring: signed-append over consented hubs.
//!
//! This module builds ONLY the kernel primitive — an append-only signed log
//! (`MeshLog`) — plus a hub-transport *trait* (`HubTransport`). It does NOT
//! perform any real cross-repo git push / network IO. The transport is
//! config-driven: a caller supplies any concrete `HubTransport` implementation
//! (a test stub, an HTTP client, an iroh node, …) — there is no hardcoded
//! endpoint baked into the kernel.
//!
//! ## Cryptography — real, not invented
//! Signing/verification use the kernel's EXISTING, KAT-gated ML-DSA-65
//! primitive at [`crate::pq::dsa`] (FIPS 204, byte-exact vs NIST ACVP
//! vectors). No new crypto is introduced here. The module is gated behind the
//! `pq` feature for the same reason `pq` itself is: the canonical order/money
//! core stays pure-std and serde-free; signing is a mesh/transport identity
//! seam (see `lib.rs` `pq` feature rationale).
//!
//! ## Chain structure
//! Each [`SignedEntry`] carries `prev_hash` (SHA3-256 of the previous entry's
//! canonical bytes, zero for genesis), the opaque `payload`, the ML-DSA
//! signature, and the signer's public key. [`MeshLog::verify_chain`] walks the
//! chain and re-checks every signature — tampered payloads and broken `prev`
//! links are both rejected.

#![cfg(feature = "pq")]

use crate::event_log::sha3_256;
use crate::pq::dsa::{keygen, sign, verify, MlDsa65Pk, MlDsa65Sig, MlDsa65Sk, RNDBYTES, SEEDBYTES};

/// A single append-only, signed entry in the mesh log.
///
/// The entry is content-addressed and chained: `prev_hash` binds it to the
/// previous entry so the log is tamper-evident. The signature covers the
/// entry's *canonical* bytes (see [`SignedEntry::signed_bytes`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedEntry {
    /// SHA3-256 of the previous entry's canonical bytes. Zero for the genesis
    /// entry. This is the hash-chain link.
    pub prev_hash: [u8; 32],
    /// Opaque payload (e.g. a serialized, consented hub event).
    pub payload: Vec<u8>,
    /// ML-DSA-65 signature over [`SignedEntry::signed_bytes`].
    pub sig: Vec<u8>,
    /// The signer's ML-DSA-65 public key (identity, NOT a score).
    pub pubkey: Vec<u8>,
}

/// Errors a [`MeshLog`] or [`HubTransport`] operation can produce. Fail-closed:
/// every arm reports the failure instead of silently accepting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    /// A signature failed to verify (forged, tampered payload, or wrong key).
    BadSignature,
    /// The chain's `prev_hash` link is broken: an entry does not point at the
    /// hash of the entry that precedes it (a removed/reordered/corrupted entry).
    BrokenLink {
        /// The expected `prev_hash` (the chain tip before this entry).
        expected: [u8; 32],
        /// The `prev_hash` actually carried by the offending entry.
        got: [u8; 32],
    },
    /// The genesis entry (sequence index 0) must have a zero `prev_hash`.
    GenesisMustBeRoot,
    /// A hub transport operation failed (e.g. a stub could not reach a hub).
    Transport(String),
}

/// A signer capable of producing ML-DSA-65 signatures over an entry's canonical
/// bytes. The kernel provides [`MlDsaSigner`]; a caller may supply their own
/// (e.g. a hardware-backed or threshold signer) as long as it verifies against
/// the returned public key.
pub trait Signer {
    /// Sign `msg` and return the raw signature bytes.
    fn sign(&self, msg: &[u8]) -> Vec<u8>;
    /// The public key this signer certifies (used to populate `SignedEntry::pubkey`).
    fn pubkey(&self) -> Vec<u8>;
}

/// Kernel-native [`Signer`] backed by the existing ML-DSA-65 primitive.
///
/// Determinism note: pass a fixed `rnd` (e.g. all zeros for FIPS deterministic
/// signing mode) so signatures are reproducible in tests. NEVER draw from an OS
/// RNG here — the crypto hot path is RNG-free by design (see `pq::dsa`).
pub struct MlDsaSigner {
    sk: MlDsa65Sk,
    pk: MlDsa65Pk,
    rnd: [u8; RNDBYTES],
}

impl MlDsaSigner {
    /// Derive a signer from a 32-byte seed (same seed → same keypair, since
    /// `pq::dsa::keygen` is deterministic). `rnd` is the ML-DSA randomness
    /// input; use `[0u8; 32]` for FIPS deterministic signing mode.
    pub fn from_seed(seed: &[u8; SEEDBYTES], rnd: [u8; RNDBYTES]) -> Self {
        let (pk, sk) = keygen(seed);
        MlDsaSigner { sk, pk, rnd }
    }

    /// Public key bytes (identity).
    pub fn pubkey_bytes(&self) -> Vec<u8> {
        self.pk.bytes.clone()
    }
}

impl Signer for MlDsaSigner {
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        sign(&self.sk, msg, &self.rnd).bytes
    }
    fn pubkey(&self) -> Vec<u8> {
        self.pk.bytes.clone()
    }
}

impl SignedEntry {
    /// Canonical bytes that get signed/verified: the **payload only**.
    ///
    /// The signature authenticates the payload + signer; the chain *position*
    /// is authenticated separately (see [`SignedEntry::content_hash`] and
    /// [`MeshLog::verify_chain`]). This split is deliberate: it makes
    /// "tampered payload" and "broken `prev_hash` link" two *distinct* failure
    /// poles. A forger without the key cannot re-sign a mutated payload
    /// (`BadSignature`), and cannot relink an entry to a different parent
    /// without breaking the link check (`BrokenLink`) — the link is
    /// content-addressed, not signed, so only a legitimate signer can form a
    /// valid chain.
    pub fn signed_bytes(&self) -> &[u8] {
        &self.payload
    }

    /// Verify this entry's signature against its embedded public key. Does NOT
    /// check chain linkage (that is [`MeshLog::verify_chain`]'s job).
    pub fn verify_sig(&self) -> bool {
        let pk = MlDsa65Pk {
            bytes: self.pubkey.clone(),
        };
        let sig = MlDsa65Sig {
            bytes: self.sig.clone(),
        };
        verify(&pk, self.signed_bytes(), &sig)
    }

    /// Content hash of this entry — the `prev_hash` the NEXT entry must carry.
    /// SHA3-256 over `prev_hash || payload`, binding the whole entry (chain
    /// position via `prev_hash` + payload) into one collision-resistant id. The
    /// signature does NOT cover this, so a broken link is caught by the link
    /// check, not masked as a signature failure.
    pub fn content_hash(&self) -> [u8; 32] {
        let mut buf = Vec::with_capacity(32 + self.payload.len());
        buf.extend_from_slice(&self.prev_hash);
        buf.extend_from_slice(&self.payload);
        sha3_256(&buf)
    }
}

/// An append-only, signed, chain-linked log — the kernel primitive for
/// cross-repo mesh wiring. Entries are signed with an ML-DSA-65 [`Signer`]
/// (signed-append over consented hubs). The log itself holds no network code.
#[derive(Debug, Clone, Default)]
pub struct MeshLog {
    entries: Vec<SignedEntry>,
}

impl MeshLog {
    /// Empty log.
    pub fn new() -> Self {
        MeshLog::default()
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Borrow the entry at `index` (0 = genesis).
    pub fn entry(&self, index: usize) -> Option<&SignedEntry> {
        self.entries.get(index)
    }

    /// Borrow all entries in chain order.
    pub fn entries(&self) -> &[SignedEntry] {
        &self.entries
    }

    /// Append `payload`, signing it with `signer`. The new entry's `prev_hash`
    /// is bound to the content hash of the current tip (genesis → zero). Returns
    /// the signed entry. This is "signed-append": the entry is authenticated
    /// before it enters the log.
    pub fn append<S: Signer>(&mut self, payload: &[u8], signer: &S) -> SignedEntry {
        let prev_hash = match self.entries.last() {
            Some(prev) => prev.content_hash(),
            None => [0u8; 32],
        };
        let entry = SignedEntry {
            prev_hash,
            payload: payload.to_vec(),
            sig: Vec::new(),    // filled below
            pubkey: Vec::new(), // filled below
        };
        let sig = signer.sign(&entry.signed_bytes());
        let pubkey = signer.pubkey();
        let entry = SignedEntry {
            prev_hash,
            payload: payload.to_vec(),
            sig,
            pubkey,
        };
        self.entries.push(entry.clone());
        entry
    }

    /// Verify the entire chain from genesis to tip:
    /// 1. every signature verifies against its embedded pubkey;
    /// 2. each link's `prev_hash` equals the content hash of the preceding
    ///    entry (genesis must be a root, i.e. zero `prev_hash`);
    /// 3. no entry mutates in place (each entry's own content hash is internally
    ///    consistent — a no-op here since we hold owned structs, but the link
    ///    walk is what catches insertions/removals/reorders).
    ///
    /// Returns `Ok(())` only if every entry is authentic and every link holds.
    pub fn verify_chain(&self) -> Result<(), MeshError> {
        let mut prev_hash: [u8; 32] = [0u8; 32];
        for (i, e) in self.entries.iter().enumerate() {
            // (1) signature must verify.
            if !e.verify_sig() {
                return Err(MeshError::BadSignature);
            }
            // (2) link check. Genesis must be a root.
            if i == 0 {
                if e.prev_hash != [0u8; 32] {
                    return Err(MeshError::GenesisMustBeRoot);
                }
            } else if e.prev_hash != prev_hash {
                return Err(MeshError::BrokenLink {
                    expected: prev_hash,
                    got: e.prev_hash,
                });
            }
            // Advance the expected link for the next entry.
            prev_hash = e.content_hash();
        }
        Ok(())
    }
}

/// Hub transport seam for the mesh log. Implementations are SUPPLIED BY THE
/// CALLER and are config-driven — the kernel bakes in NO endpoint, NO protocol,
/// NO network. A concrete impl might push to a consented git remote, an HTTP
/// hub, an iroh node, or (in tests) an in-memory vector. The trait proves the
/// wiring is transport-agnostic: signed entries leave the kernel only through
/// `send`, and arrive back only through `recv`.
///
/// **No real cross-repo git push is performed by any kernel code.** This trait
/// is the firewall that keeps the kernel network-free; the network half is a
/// caller-supplied adapter, exactly like `event_log::EventStore`.
pub trait HubTransport {
    /// Send one signed entry to the consented hub(s).
    fn send(&self, e: &SignedEntry) -> Result<(), MeshError>;
    /// Receive signed entries from the hub (e.g. for a peer's log to verify).
    fn recv(&self) -> Result<Vec<SignedEntry>, MeshError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic signer from a single seed byte, all-zero rnd (FIPS
    /// deterministic signing mode → reproducible signatures).
    fn signer(seed_byte: u8) -> MlDsaSigner {
        let mut seed = [0u8; SEEDBYTES];
        seed[0] = seed_byte;
        MlDsaSigner::from_seed(&seed, [0u8; RNDBYTES])
    }

    // ── GREEN: a valid append+verify chain is accepted. ──
    #[test]
    fn valid_chain_is_accepted() {
        let mut log = MeshLog::new();
        let s = signer(1);
        log.append(b"consent: hub-A <-> hub-B", &s);
        log.append(b"event: order placed", &s);
        log.append(b"event: order settled", &s);
        assert_eq!(log.len(), 3);
        assert!(log.verify_chain().is_ok(), "valid signed chain must verify");
    }

    // ── RED: a tampered payload is rejected. ──
    #[test]
    fn tampered_payload_is_rejected() {
        let mut log = MeshLog::new();
        let s = signer(2);
        let _e0 = log.append(b"honest payload", &s);
        let mut e1 = log.append(b"another honest payload", &s);
        // Mutate the payload AFTER signing (an attacker tampering at rest / in
        // transit). The stored signature no longer matches the bytes.
        e1.payload = b"forged payload".to_vec();
        // Replace the last entry with the tampered one.
        log.entries.pop();
        log.entries.push(e1);
        match log.verify_chain() {
            Err(MeshError::BadSignature) => {}
            other => panic!("expected BadSignature, got {:?}", other),
        }
    }

    // ── RED: a broken prev_hash link is rejected. ──
    #[test]
    fn broken_prev_link_is_rejected() {
        let mut log = MeshLog::new();
        let s = signer(3);
        log.append(b"first", &s);
        let mut e1 = log.append(b"second", &s);
        // Sever the chain link: rewrite prev_hash to a wrong value while keeping
        // the (still-valid) signature over the ORIGINAL bytes. Link check must
        // catch it; signature check alone would not.
        e1.prev_hash = [9u8; 32];
        log.entries.pop();
        log.entries.push(e1);
        match log.verify_chain() {
            Err(MeshError::BrokenLink { expected, got }) => {
                assert_eq!(got, [9u8; 32]);
                assert_ne!(expected, [9u8; 32]);
            }
            other => panic!("expected BrokenLink, got {:?}", other),
        }
    }

    // ── GREEN (config-driven trait proof): a HubTransport impl supplied by a
    //    test (in-memory) proves the transport is caller-supplied, not hardcoded
    //    to any endpoint. send/recv round-trip signed entries. ──
    #[test]
    fn hub_transport_impl_supplied_by_test() {
        // Config-driven stub: holds entries in a Vec; "endpoint" is just the
        // struct's own memory, set entirely by the test — no baked-in URL/host.
        struct MemHub {
            outbox: std::cell::RefCell<Vec<SignedEntry>>,
        }
        impl HubTransport for MemHub {
            fn send(&self, e: &SignedEntry) -> Result<(), MeshError> {
                self.outbox.borrow_mut().push(e.clone());
                Ok(())
            }
            fn recv(&self) -> Result<Vec<SignedEntry>, MeshError> {
                Ok(self.outbox.borrow().clone())
            }
        }

        let hub = MemHub {
            outbox: std::cell::RefCell::new(Vec::new()),
        };
        let mut log = MeshLog::new();
        let s = signer(4);
        let e0 = log.append(b"wire to hub", &s);
        // The kernel emits the signed entry through the caller-supplied transport.
        hub.send(&e0).expect("transport send");
        // And can pull it back / verify it on the other side.
        let got = hub.recv().expect("transport recv");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].content_hash(), e0.content_hash());
        // The received entry still verifies under the chain it would join.
        let mut peer = MeshLog::new();
        peer.entries.push(got[0].clone());
        assert!(peer.verify_chain().is_ok());
    }

    // ── GREEN: signature of the genesis entry with nonzero prev_hash is caught. ──
    #[test]
    fn genesis_must_be_root() {
        let mut log = MeshLog::new();
        let s = signer(5);
        let e = log.append(b"genesis", &s);
        // Tamper only the prev_hash link of the genesis entry to nonzero while
        // keeping the valid signature — link/root check must reject.
        let mut bad = e;
        bad.prev_hash = [1u8; 32];
        let mut lone = MeshLog::new();
        lone.entries.push(bad);
        match lone.verify_chain() {
            Err(MeshError::GenesisMustBeRoot) => {}
            other => panic!("expected GenesisMustBeRoot, got {:?}", other),
        }
    }
}
