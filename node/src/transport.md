# `node/src` Transport Note — `Bundle` ↔ BPv7 (1:1 mapping sketch)

> **STATUS: SKETCH ONLY.** This file is a *design note* for the later operator-gated
> integration gate **S1** (integrate `dtn7-rs` / `bp7-rs`). It does **NOT** change any
> code. `Bundle` and the PQ envelope in `lib.rs` are unchanged. Read together with
> `docs/transport-research-2026-07-12.md` (decision **P7** reaffirmed).

## Goal (S1, future)

Replace the in-memory `Node::forward` hop with a real **BPv7** daemon (`dtn7-rs`,
RFC 9171) so courier custody/forwarding is handled by a standards-track DTN stack,
while keeping the local `Bundle` shape and the PQ envelope as the payload.

## 1:1 field mapping (local `Bundle` → BPv7)

| Local `Bundle` field (`lib.rs`) | BPv7 structure (RFC 9171) | Notes |
|----------------------------------|---------------------------|-------|
| `source: String` | Primary block **Source EID** (`dtn:` or `ipn:` URI scheme, §4.3.1 / §4.2.5.1) | e.g. `dtn://owner-1/~couriers`. `bp7-rs` supports both `dtn` and `ipn`. |
| `dest: String` | Primary block **Destination EID** | Final recipient; drives BP forwarding. |
| `creation_ts: u64` | Primary block **Creation Timestamp** (DTN time + sequence, §4.2.7) | Used by `accept()` replay check `(source, creation_ts)`. |
| `lifetime: u64` | Primary block **Lifetime** (§4.2 — bundle expires at creation + lifetime) | Drives `accept()` expiry RED gate. |
| `sender_pk: Vec<u8>` | Carried in **payload** (PQ `SignedEnvelope`) — NOT a BP field | Sender's ML-DSA-65 pk; couriers verify envelope against this, not their own key (sender-authenticating custody). |
| `sender_hybrid_pk: Vec<u8>` | Carried in **payload** (PQ envelope / transit blob) | X25519 ‖ ML-KEM-768 hybrid transit key (D4). Intermediate couriers cannot decrypt (no hybrid secret). |
| `payload: Vec<u8>` | **Payload block** (block type 1, §4.3.2) = opaque bytes | A serialized `SignedEnvelope` (PQ-wrapped message) or, for secret bundles, `(HybridCiphertext, keystream_blob)` then sealed. BP treats it as opaque. |

### Block layout produced by S1
```
Primary Block (RFC 9171 §4.3.1)
  ├─ Source EID      <- Bundle.source
  ├─ Destination EID <- Bundle.dest
  ├─ Creation Timestamp <- Bundle.creation_ts (+ per-bundle seq)
  └─ Lifetime        <- Bundle.lifetime
Payload Block (type 1)
  └─ block-type-specific-data <- Bundle.payload  (the PQ SignedEnvelope)
```
Optional BPSec (RFC 9172/9173) blocks may later wrap the payload:
- **BIB-HMAC-SHA2** (RFC 9173 §3) for integrity (PQ envelope already provides ML-DSA auth; BPSec is defense-in-depth / at-rest).
- **BCB-AES-GCM** (RFC 9173 §4) for confidentiality at rest in courier stores (complements the D4 hybrid transit encryption).

## Custody / BIBE mapping (RFC 9171 + draft-ietf-dtn-bibect-05)

- `Node::accept()` (verify envelope, check expiry, check replay, push to `custody`) ≡ **taking custody** of a bundle at a BP node.
- `Node::forward(next_hop)` (drain custody, hand to next `accept`) ≡ **custody transfer** to the next hop.
- **BIBE** tunnels a bundle as the payload of an encapsulating bundle; **BRM** (Bundle Retransmission Method, adapted from BPv6 custody transfer) recovers lost encapsulating bundles. S1 wires `forward()` to emit BIBE PDUs (admin-record type 64443) toward the next BCLA, with BRM signals (type 64444) for retransmission — making the *store* the source of truth, exactly as the lib.rs doc-comment states.

## Convergence layer (P7 bearer)

- Default CLA: **TCPCLv4 over QUIC** (RFC 9174 + RFC 9000) with mandatory TLS 1.3 (RFC 9174 §7.11). `dtn7-rs` already implements TCPCLv4; QUIC is the TLS-secured bearer beneath it.
- Alternative CLAs available in `dtn7-rs`: minimal TCP (mtcp), HTTP, UDP — selectable per deployment (spacecraft bus → UDP/MTCP; terrestrial → QUIC/TCPCLv4).
- Underlay is swappable (QUIC, TCP, or a SpaceWire/SpaceFibre-attached link) **without touching the BPv7 overlay or the PQ envelope** — this is the D3 underlay-independence guarantee.

## What stays invariant across S1
- `Bundle` field set above (the 1:1 map).
- PQ envelope verification against `sender_pk` (ML-DSA-65) — underlay/transport change does NOT affect PQ.
- RED+GREEN gates in `lib.rs` (expiry, replay, tamper, wrong-dest, custody handoff) remain the *reference oracle*; `dtn7-rs` behavior must match them.

## Open items for S1 (operator-gated, not done here)
1. Add `bp7-rs`/`dtn7-rs` as deps; implement `Bundle <-> bp7::Bundle` codec.
2. Replace `Node::forward` internals with `dtnd` CLA calls (BIBE PDU emission + BRM).
3. Keep a headless `Bundle` path (current `lib.rs`) as the test oracle; S1 adds the live-CLA path behind a feature flag.
4. Decide BPSec context: default BIB-HMAC-SHA2 / BCB-AES-GCM (RFC 9173) or PQ-native only.
