//! dowiz autonomous node — DTN-style store-and-forward mesh.
//!
//! Per MANIFESTO C11 (reliability > latency) + D3 (DTN/BPv7, RFC 9171): a courier node
//! must tolerate intermittent links. A bundle is *accepted into custody*, stored, and
//! forwarded only when the next hop is reachable. Custody (BIBE, draft-ietf-dtn-bibect)
//! means: if I ACK custody and then lose the bundle, that is a protocol fault — so the
//! store is the source of truth.
//!
//! This crate proves the custody/lifetime/dedupe/replay semantics headlessly and
//! falsifiably. The production transport is `dtn7-rs` (real BPv7 daemon); its bundle
//! structure maps 1:1 onto [`Bundle`] below (source EID, creation timestamp, lifetime,
//! payload). The PQ envelope ([`dowiz_kernel::pq::envelope`]) is the payload and is
//! verified on receipt against the *sender's* public key — PQ holds regardless of
//! whether the underlay is QUIC, TCPCLv4, or SpaceWire (D3).

use dowiz_kernel::pq::envelope::{new_identity, open, seal, SignedEnvelope, ENTROPY_LEN};
use dowiz_kernel::pq::hybrid::{hybrid_decaps, hybrid_encaps, hybrid_keygen, HybridCiphertext, HybridKeypair};
use std::collections::HashSet;

/// Autonomous role state machines (P4): owner/merchant, courier, customer.
pub mod roles;

/// A custody-transfer bundle (BPv7-shaped, minimal).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Bundle {
    /// Source endpoint ID, e.g. "dtn://owner-1/~couriers".
    pub source: String,
    /// Destination endpoint ID.
    pub dest: String,
    /// Sender's ML-DSA-65 public key — custodians verify the envelope against this,
    /// not their own key (intermediate custody is sender-authenticating, not receiver).
    pub sender_pk: Vec<u8>,
    /// Sender's X25519+ML-KEM-768 hybrid public key (for confidential transit per D4).
    pub sender_hybrid_pk: Vec<u8>,
    /// Creation timestamp (seconds since epoch) — RFC 9171 creation timestamp.
    pub creation_ts: u64,
    /// Lifetime (seconds) — RFC 9171 bundle lifetime; expires at creation_ts+lifetime.
    pub lifetime: u64,
    /// Opaque payload: a serialized [`SignedEnvelope`] (the PQ-wrapped message).
    pub payload: Vec<u8>,
}

/// A node: holds a custody store + a dedupe set + its own PQ signing + hybrid transit keys.
pub struct Node {
    pub eid: String,
    pub now: u64, // ponytail: injected clock — avoids SystemTime in tests; swap for real clock in prod.
    custody: Vec<Bundle>,
    seen: HashSet<(String, u64)>, // (source, creation_ts) for replay detection.
    sk: Vec<u8>,
    pk: Vec<u8>,
    /// Hybrid transit keypair (X25519 + ML-KEM-768, both mandatory per D4). Used to
    /// encrypt bundle payloads so intermediate couriers cannot read them.
    hybrid: HybridKeypair,
}

/// Split a `hybrid_pk()` blob (x_pk ‖ kem_pk) back into its two public components.
fn split_hybrid_pk(blob: &[u8]) -> ([u8; 32], Vec<u8>) {
    let mut x = [0u8; 32];
    x.copy_from_slice(&blob[..32]);
    (x, blob[32..].to_vec())
}

/// Derive a deterministic sub-seed for the hybrid legs from the node seed + label.
fn hash_seed(seed: &[u8; 32], label: u8) -> [u8; 32] {
    let mut buf = [0u8; 33];
    buf[..32].copy_from_slice(seed);
    buf[32] = label;
    // reuse envelope hash (SHAKE256) — keeps deps minimal
    let mut out = [0u8; 32];
    dowiz_kernel::pq::keccak::shake256(&buf, &mut out);
    out
}

/// XOR `data` with a SHAKE256 keystream keyed by `key` (a length-preserving stream cipher).
/// Used to encrypt the confidential payload under the hybrid shared secret. ponytail: not
/// an AEAD (no MAC) — the ML-DSA envelope already provides integrity/auth; this only hides
/// plaintext from intermediate couriers. Upgrade trigger: real funds in transit → AES-256-GCM.
fn xor_stream(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let mut stream = Vec::with_capacity(data.len());
    let mut block = [0u8; 32];
    let mut counter: u64 = 0;
    let mut produced = 0;
    while produced < data.len() {
        let mut src = key.to_vec();
        src.extend_from_slice(&counter.to_le_bytes());
        dowiz_kernel::pq::keccak::shake256(&src, &mut block);
        for b in block {
            if produced >= data.len() {
                break;
            }
            stream.push(data[produced] ^ b);
            produced += 1;
        }
        counter += 1;
    }
    stream
}

impl Node {
    /// Create a node with its own ML-DSA-65 identity + hybrid transit keypair,
    /// seeded deterministically for tests.
    pub fn new(eid: &str, seed: &[u8; ENTROPY_LEN], now: u64) -> Self {
        let (pk, sk) = new_identity(seed);
        // Derive two independent seeds for the hybrid legs from the node seed.
        let x_seed = hash_seed(seed, 1);
        let kem_seed = hash_seed(seed, 2);
        let hybrid = hybrid_keygen(&x_seed, &kem_seed);
        Node {
            eid: eid.to_string(),
            now,
            custody: Vec::new(),
            seen: HashSet::new(),
            sk,
            pk,
            hybrid,
        }
    }

    /// Public hybrid transit key (X25519 pk ‖ ML-KEM pk) for peers to encrypt toward us.
    pub fn hybrid_pk(&self) -> Vec<u8> {
        let mut v = self.hybrid.x_pk.to_vec();
        v.extend_from_slice(&self.hybrid.kem_pk);
        v
    }

    /// Wrap a raw message into a PQ-signed envelope, then into a custody bundle.
    /// Returns the bundle ready to hand to the next hop.
    pub fn make_bundle(&self, dest: &str, msg: &[u8], creation_ts: u64, lifetime: u64) -> Bundle {
        let rnd = [0u8; ENTROPY_LEN];
        let env = seal(msg, &self.sk, &rnd);
        let payload = serde_json::to_vec(&env).expect("envelope serializes");
        Bundle {
            source: self.eid.clone(),
            dest: dest.to_string(),
            sender_pk: self.pk.clone(),
            sender_hybrid_pk: self.hybrid_pk(),
            creation_ts,
            lifetime,
            payload,
        }
    }

    /// Confidential variant (D4): encrypt `msg` under the recipient's hybrid transit key
    /// (X25519 + ML-KEM-768, both required — no classical-only fallback), then seal the
    /// result in a signed envelope so couriers carry opaque, sender-authenticated bytes.
    /// `dest_hybrid_pk` is the recipient's `hybrid_pk()` output (x_pk ‖ kem_pk).
    /// `m` is ML-KEM encaps entropy; `eph_seed` is the ephemeral X25519 scalar seed.
    #[allow(clippy::too_many_arguments)]
    pub fn make_secret_bundle(
        &self,
        dest: &str,
        dest_hybrid_pk: &[u8],
        msg: &[u8],
        m: &[u8; 32],
        eph_seed: &[u8; 32],
        creation_ts: u64,
        lifetime: u64,
    ) -> Bundle {
        let (peer_x_pk, peer_kem_pk) = split_hybrid_pk(dest_hybrid_pk);
        let peer = HybridKeypair {
            x_pk: peer_x_pk,
            x_sk: [0u8; 32],
            kem_pk: peer_kem_pk,
            kem_sk: Vec::new(),
        };
        // Establish the hybrid shared secret (both legs required by hybrid.rs RED gate).
        let (ct, ss) = hybrid_encaps(&peer, m, eph_seed);
        // Encrypt the message under ss via a SHAKE256 keystream (zero extra deps). The
        // ciphertext and the sealed plaintext blob travel together, serde-framed so the
        // variable-length ML-KEM ct (1536 B) is parsed correctly.
        let blob = xor_stream(msg, &ss);
        let transit = serde_json::to_vec(&(ct, blob)).expect("transit serializes");
        let rnd = [0u8; ENTROPY_LEN];
        let env = seal(&transit, &self.sk, &rnd);
        let payload = serde_json::to_vec(&env).expect("envelope serializes");
        Bundle {
            source: self.eid.clone(),
            dest: dest.to_string(),
            sender_pk: self.pk.clone(),
            sender_hybrid_pk: self.hybrid_pk(),
            creation_ts,
            lifetime,
            payload,
        }
    }

    /// Deliver a confidential bundle (D4): only a node whose EID matches `dest` AND whose
    /// hybrid secret key matches the recipient can recover the plaintext. Decapsulates the
    /// hybrid ciphertext (X25519 + ML-KEM-768, both required) and verifies our ML-DSA envelope.
    /// A courier that is not the final recipient cannot decrypt (no hybrid secret key).
    pub fn deliver_secret(&self, b: &Bundle) -> Result<Vec<u8>, &'static str> {
        if b.dest != self.eid {
            return Err("not-addressed-to-me");
        }
        let env: SignedEnvelope = serde_json::from_slice(&b.payload).map_err(|_| "bad-envelope")?;
        if open(&env, &b.sender_pk).is_err() {
            return Err("tampered-or-unsigned");
        }
        // transit = JSON(ct, blob)
        let (ct, blob): (HybridCiphertext, Vec<u8>) =
            serde_json::from_slice(&env.payload).map_err(|_| "bad-transit")?;
        let ss = hybrid_decaps(&self.hybrid, &ct).map_err(|_| "hybrid-decaps-failed")?;
        Ok(xor_stream(&blob, &ss))
    }

    /// Accept a bundle into custody. A courier stores whatever it is handed (it is not
    /// the final destination — DTN store-and-forward). Returns Ok(()) if accepted,
    /// Err(reason) if rejected.
    /// Rejection cases (RED gates): expired (lifetime), replay (duplicate source+ts),
    /// tampered/unsigned envelope (verified against sender's key).
    pub fn accept(&mut self, b: Bundle) -> Result<(), &'static str> {
        if self.now > b.creation_ts + b.lifetime {
            return Err("expired");
        }
        if self.seen.contains(&(b.source.clone(), b.creation_ts)) {
            return Err("replay");
        }
        // Verify the PQ envelope against the SENDER's public key before taking custody.
        let env: SignedEnvelope = serde_json::from_slice(&b.payload).map_err(|_| "bad-envelope")?;
        if open(&env, &b.sender_pk).is_err() {
            return Err("tampered-or-unsigned");
        }
        self.seen.insert((b.source.clone(), b.creation_ts));
        self.custody.push(b);
        Ok(())
    }

    /// Forward all custody bundles to the next hop (hands custody on). In production this
    /// is the DTN forwarder; here we pass them to the caller's node.
    pub fn forward(&mut self, next_hop: &mut Node) -> usize {
        let pending: Vec<Bundle> = self.custody.drain(..).collect();
        let mut handed = 0;
        for b in pending {
            if next_hop.accept(b).is_ok() {
                handed += 1;
            }
        }
        handed
    }

    /// Deliver: only a bundle whose `dest` matches this node's EID is opened and the
    /// plaintext returned. A courier that is not the final destination cannot deliver.
    pub fn deliver(&self, b: &Bundle) -> Result<Vec<u8>, &'static str> {
        if b.dest != self.eid {
            return Err("not-addressed-to-me");
        }
        let env: SignedEnvelope = serde_json::from_slice(&b.payload).map_err(|_| "bad-envelope")?;
        open(&env, &b.sender_pk).map_err(|_| "tampered-or-unsigned")
    }

    pub fn custody_len(&self) -> usize {
        self.custody.len()
    }

    /// Read-only snapshot of the current custody store. Lets a role inspect the
    /// bundles it holds (e.g. the customer reading the order it was handed)
    /// without mutating custody. Does not change crypto/money semantics.
    pub fn custody_snapshot(&self) -> Vec<Bundle> {
        self.custody.clone()
    }
}

// ── RED+GREEN tests: every gate fails if the logic breaks ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SEED_A: [u8; 32] = [1u8; 32];
    const SEED_B: [u8; 32] = [2u8; 32];
    const SEED_C: [u8; 32] = [3u8; 32];

    #[test]
    fn green_valid_bundle_accepted_into_custody() {
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);
        let bundle = a.make_bundle("dtn://b", b"hello courier", 1000, 3600);
        assert!(b.accept(bundle).is_ok());
        assert_eq!(b.custody_len(), 1);
    }

    #[test]
    fn red_expired_bundle_rejected() {
        let a = Node::new("dtn://a", &SEED_A, 5000); // now far past lifetime.
        let mut b = Node::new("dtn://b", &SEED_B, 5000);
        let bundle = a.make_bundle("dtn://b", b"late", 1000, 3600); // expires at 4600.
        assert_eq!(b.accept(bundle), Err("expired"));
        assert_eq!(b.custody_len(), 0);
    }

    #[test]
    fn red_replay_bundle_rejected() {
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);
        let bundle = a.make_bundle("dtn://b", b"dup", 1000, 3600);
        assert!(b.accept(bundle.clone()).is_ok());
        // Second delivery of same (source, creation_ts) = replay.
        assert_eq!(b.accept(bundle), Err("replay"));
    }

    #[test]
    fn red_wrong_dest_rejected_at_delivery() {
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);
        let bundle = a.make_bundle("dtn://c", b"misrouted", 1000, 3600);
        // B accepts into custody (courier) but cannot DELIVER a bundle not for it.
        assert!(b.accept(bundle.clone()).is_ok());
        assert_eq!(b.deliver(&bundle), Err("not-addressed-to-me"));
    }

    #[test]
    fn red_tampered_envelope_rejected() {
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);
        let mut bundle = a.make_bundle("dtn://b", b"legit", 1000, 3600);
        // Tamper the envelope at the struct level (flip a sig byte), then re-serialize,
        // so JSON stays valid but ML-DSA verification fails.
        let mut env: SignedEnvelope = serde_json::from_slice(&bundle.payload).unwrap();
        if !env.sig.is_empty() {
            env.sig[0] ^= 0xFF;
        }
        bundle.payload = serde_json::to_vec(&env).unwrap();
        assert_eq!(b.accept(bundle), Err("tampered-or-unsigned"));
        assert_eq!(b.custody_len(), 0);
    }

    #[test]
    fn green_custody_handoff_forward_works() {
        // A -> B -> C: B accepts into custody, then forwards to C (custody transfers).
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);
        let mut c = Node::new("dtn://c", &SEED_C, 1000);
        let bundle = a.make_bundle("dtn://c", b"relay", 1000, 3600);
        // B accepts (B is a courier holding custody for C).
        assert!(b.accept(bundle).is_ok());
        assert_eq!(b.custody_len(), 1);
        // B forwards to C.
        let handed = b.forward(&mut c);
        assert_eq!(handed, 1);
        assert_eq!(b.custody_len(), 0);
        assert_eq!(c.custody_len(), 1);
    }

    #[test]
    fn green_secret_bundle_roundtrip_only_recipient_decrypts() {
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);
        let c = Node::new("dtn://c", &SEED_C, 1000);
        let msg = b"dispatch: 3 pizzas to grid 7";
        let m = [7u8; 32];
        let eph = [8u8; 32];
        let bundle = a.make_secret_bundle("dtn://b", &b.hybrid_pk(), msg, &m, &eph, 1000, 3600);
        assert!(b.accept(bundle.clone()).is_ok());
        // B (the recipient) decrypts the real plaintext.
        let plain = b.deliver_secret(&bundle).expect("recipient decrypts");
        assert_eq!(plain, msg);
        // C (not the recipient, no hybrid secret key) cannot decrypt to the real plaintext.
        let wrong = c.deliver_secret(&bundle);
        assert!(wrong.is_err() || wrong.unwrap() != msg, "non-recipient must not read plaintext");
    }
}
