# Transport Research: Spacecraft / Lab-Grade Mesh for the PQ Delivery Core

**Date:** 2026-07-12
**Branch:** `wt/p7-transport` (worktree `dowiz-pq-wt-p7-transport`, clean at `5a8ea3c4`)
**Scope:** RESEARCH ONLY. No code integration. No changes to `node/src/lib.rs` crypto or `Bundle` struct. The later operator-gated integration of `dtn7-rs` is a separate sequential gate **S1** and is explicitly out of scope here (per task constraints).
**Objective:** Compare spacecraft/lab-grade transport options for a decentralized, disruption-tolerant PQ delivery mesh, and reaffirm decision **P7**: **DTN / Bundle Protocol v7 (RFC 9171) + QUIC / TCPCLv4 (RFC 9174) + BIBE custody handoff**, with **libp2p NOT chosen**.

---

## 1. Context & Decision Anchors (from `node/src/lib.rs`)

The existing mesh node already encodes the binding decisions in its doc-comment:

- **C11** — *reliability > latency*: a courier node must tolerate intermittent links.
- **D3** — *DTN / BPv7, RFC 9171*: a bundle is **accepted into custody**, stored, and forwarded only when the next hop is reachable.
- **BIBE** (`draft-ietf-dtn-bibect`) — Custody transfer: if I ACK custody and then lose the bundle, that is a protocol fault, so the **store is the source of truth**.
- Production transport = **`dtn7-rs`** (real BPv7 daemon); its bundle structure maps 1:1 onto the local `Bundle` (source EID, creation timestamp, lifetime, payload).
- PQ envelope (`dowiz_kernel::pq::envelope`) is the **payload**, verified on receipt against the *sender's* public key. **PQ holds regardless of underlay** — QUIC, TCPCLv4, or SpaceWire (D3).

This doc's job is to *reaffirm* P7 against the alternatives with primary sources, not to change code.

---

## 2. Candidates Evaluated

| # | Option | Primary classification | Relevant standards / sources |
|---|--------|------------------------|------------------------------|
| A | **DTN / Bundle Protocol v7** | Store-carry-forward overlay | RFC 9171 (BPv7), RFC 9174 (TCPCLv4/QUIC), RFC 9172 (BPSec), RFC 9173 (default contexts), draft-ietf-dtn-bibect-05 (BIBE) |
| B | **QUIC / TCPCLv4** | Reliable transport / convergence layer under BP | RFC 9000 (QUIC), RFC 9174 (TCPCLv4) |
| C | **Zenoh** | Pub/sub + query + storage middleware (Eclipse) | eclipse.dev/zenoh, github.com/eclipse-zenoh/zenoh |
| D | **Reticulum (rns)** | Crypto-based sovereign mesh (L2/L3) | reticulum.network, github.com/markqvist/Reticulum |
| E | **SpaceWire / SpaceFibre** | On-board spacecraft hardware bus (reference only) | CCSDS (ccsds.org), ESA SpaceFibre, STAR-Dundee |

> **Note on QUIC placement:** QUIC (RFC 9000) is not a competitor to BP — it is the *bearer* for TCPCLv4 (RFC 9174), which is a BP convergence layer. P7 therefore composes them: **BPv7 (application-layer overlay) running over QUIC/TCPCLv4 (reliable convergence layer)**. This is exactly the layering RFC 9174 §1 / Figure 1 prescribes ("when BP is using TCP as its bearer with the TCPCL as its convergence layer, both BP and the TCPCL reside at the application layer").

---

## 3. Per-Candidate Assessment

### A. DTN / Bundle Protocol v7 (RFC 9171) — ✅ CHOSEN (P7 core)

- **Latency profile:** *Disruption-tolerant by design.* BP is a store-carry-forward overlay (RFC 9171 §1): it "copes with intermittent connectivity, including cases where the sender and receiver are not concurrently present." Latency is **bounded by reachability, not by RTT** — bundles wait in custody stores until a contact exists. This is the correct trade for C11 (reliability > latency).
- **Disruption tolerance:** Native. "Ability to take advantage of scheduled, predicted, and opportunistic connectivity... in addition to continuous connectivity" (RFC 9171 §1). Lifetime/expiry, replay-dedupe, and custody are first-class (mirrored by the local `Bundle`: `creation_ts`, `lifetime`, `seen` set).
- **Mesh fit:** Excellent for a *decentralized* mesh. DTN is explicitly a "store-carry-forward overlay network" (RFC 9171) with late binding of overlay EIDs to underlying addresses and per-node routing/forwarding left to the implementation. `dtn7-rs` ships epidemic/flooding/spray-and-wait/static routing.
- **Rust maturity:** **Mature and standards-track aligned.** `dtn7-rs` — *"Rust implementation of a disruption-tolerant networking (DTN) daemon for the Bundle Protocol version 7 - RFC9171"* — also implements **TCPCLv4 (RFC 9174)**, minimal TCP CL, HTTP CL, UDP CL, IPND discovery, and WebSocket/REST interfaces. BPv7 encode/decode is split into the separate `bp7-rs` crate (clean reuse target for S1). License: Apache-2.0 / MIT. Repo: https://github.com/dtn7/dtn7-rs — ~462 commits, 110 stars, 29 forks, active (last commit 2026-05-27, mdns discovery #79).
- **BIBE custody handoff mapping:** Direct. BIBE (draft-ietf-dtn-bibect-05) is a BP *convergence-layer adapter* that tunnels bundles as the payload of encapsulating bundles, with a Bundle Retransmission Method (BRM) adapted from BPv6 custody transfer. Semantics match the local `Node::accept` → `forward` flow: accept into custody, store, hand to next hop, and the *store* is the source of truth. The local tests (`green_custody_handoff_forward_works`: A→B→C custody transfer) are a 1:1 rehearsal of the BIBE/BRM custody lifecycle.

### B. QUIC / TCPCLv4 (RFC 9000 + RFC 9174) — ✅ CHOSEN (P7 bearer)

- **Latency profile:** Low. RFC 9000 provides "low-latency connection establishment" (0/1-RTT), multiplexed streams, and connection migration. As the bearer under TCPCLv4 it adds TLS 1.3 transport security and per-transfer acknowledgments (XFER_ACK) for reliable bundle delivery (RFC 9174 §5.2).
- **Disruption tolerance:** QUIC alone assumes a live path; it is *not* store-and-forward. Its value here is as the **reliable, TLS-secured convergence layer** beneath BP — RFC 9174 §1.1 scopes TCPCL to "transporting bundles between adjacent entities." QUIC's path migration (RFC 9000 §9) and 0-RTT help survive NAT rebinding and intermittent links at the *link* layer, while BP handles *network-layer* disruption.
- **Mesh fit:** As a convergence layer, QUIC/TCPCLv4 is link-scoped (adjacent entities only, RFC 9174 §1.1). It does not provide the overlay routing/mesh; BP does. They are complementary, not alternatives — hence P7 binds them.
- **Rust maturity:** Strong. `quinn` (pure-Rust QUIC, RFC 9000) is widely used; `dtn7-rs` already depends on a TLS/QUIC-capable stack for TCPCLv4. "Mandatory-to-Implement TLS" (RFC 9174 §7.11) gives us PQ-upgradeable transport security (future: hybrid/ML-KEM TLS key exchange).
- **BIBE custody handoff mapping:** N/A at the bundle layer — QUIC provides the *reliable byte pipe* that BIBE/BP rides on. BIBE custody signals (BRM) are carried end-to-end as bundle payloads; QUIC guarantees the encapsulating bundle's bytes arrive intact between adjacent CLAs.

### C. Zenoh (Eclipse) — ❌ Considered, not chosen

- **Latency profile:** Extremely low latency / high throughput is its stated design goal ("extremely low latency and high throughput," eclipse.dev/zenoh). Pub/sub + query + geo-distributed storage blend.
- **Disruption tolerance:** **Weak for stressed/DTN environments.** Zenoh targets IoT/robotic/SDV networks with *continuous or low-duty-cycle* connectivity. It has no store-carry-forward, no custody, no lifetime/expiry, no opportunistic-contact model. It presumes a reachable broker/peer at publish time.
- **Mesh fit:** Good for *real-time* data distribution, poor for *intermittent* courier mesh. Its routing is geared to low-latency dissemination, not to "sender and receiver not concurrently present" (RFC 9171 §1).
- **Rust maturity:** **Very mature** — github.com/eclipse-zenoh/zenoh, 100% Rust, ~4,753 commits, 3k stars, 329 forks, 51 releases, v1.9.0 (2026-04). Best-in-class Rust networking stack.
- **BIBE custody handoff mapping:** No native custody concept. Could be bolted on at the app layer but that re-implements BP's hard parts. Not a fit for C11.

### D. Reticulum / rns — ❌ Considered, not chosen

- **Latency profile:** Tolerates "very high latency and extremely low bandwidth" (reticulum.network). Designed for adverse conditions — a closer *philosophical* cousin to DTN than Zenoh.
- **Disruption tolerance:** Strong at the link/network edge: no source addresses, self-sovereign portable addresses, always-encrypted (ephemeral keys, forward secrecy), unencrypted packets dropped by default. Works over LoRa, packet radio, TCP, UDP.
- **Mesh fit:** Strong *decentralized* story ("networks without kill-switches"). However it is a **fixed application-layer protocol with its own addressing, routing, and LXMF message format** — not an RFC standards-track overlay and not BP-compatible. Adopting it would *replace* the BPv7 model, breaking D3 and the 1:1 `Bundle`↔BPv7 mapping.
- **Rust maturity:** **None / not a fit.** Reticulum is a **pure-Python** reference implementation (manual states "Pure-Python Reticulum"). No first-class Rust crate; integrating it into a Rust PQ core means FFI or reimplementation. This alone disqualifies it for our Rust PQ delivery core.
- **BIBE custody handoff mapping:** No BIBE/BP custody model; its reliability is link-level (acknowledged links) not custody-transfer. Would require re-deriving the custody semantics the local `Node` already validates.

### E. SpaceWire / SpaceFibre — ℹ️ Hardware reference only (NOT a software transport)

- **Latency profile:** On-board *hardware* bus. SpaceFibre (ESA/CCSDS successor to SpaceWire) reaches **up to 6.25 Gbps per lane** (ESA: "15 times higher data rates per lane ... up to 6.25 Gbps"; UKspace: "6.25 Gbit/s in current flight technology, electrical or fibre"). SpaceWire is ~100–400 Mbps.
- **Disruption tolerance:** N/A as a *mesh transport* — it is a deterministic, synchronous on-board link inside a single spacecraft / between adjacent boxes. No store-carry-forward, no EID routing, no custody.
- **Mesh fit:** None at the delivery-mesh layer. It is the *physical/logical link* inside a node, exactly analogous to how a UART/ETH PHY sits under a CLA. **It is a valid *underlay* beneath a BP convergence layer running on flight hardware**, not an alternative to BP.
- **Rust maturity:** N/A (hardware/VHDL + vendor SDKs, e.g. STAR-Dundee). Not consumable as a Rust networking crate.
- **BIBE custody handoff mapping:** N/A. Cited only to (a) show the *deepest* disruption-tolerant pedigree BP descends from (CCSDS/space agencies) and (b) confirm BP runs *on top of* such busses — reinforcing that the underlay is swappable while the BPv7 overlay (P7) stays fixed.

---

## 4. Comparison Matrix

| Dimension | DTN/BPv7 (A) | QUIC/TCPCLv4 (B) | Zenoh (C) | Reticulum (D) | SpaceWire/Fibre (E) |
|-----------|--------------|------------------|-----------|---------------|---------------------|
| **Disruption tolerance** | Native (store-carry-forward) | Link-level only | Weak (needs live path) | Strong (edge) | None (on-board link) |
| **Latency model** | Reachability-bound (C11 ✓) | Low RTT, 0/1-RTT | Very low | High-latency OK | Gbps hardware |
| **Mesh / decentralized** | ✅ overlay, per-node routing | Link-scoped | Real-time distrib. | ✅ sovereign mesh | — (single node) |
| **Custody / BIBE** | ✅ native (RFC 9171 + draft-ietf-dtn-bibect) | Reliable pipe for BP | ❌ app-layer only | ❌ link-ack only | ❌ |
| **Rust maturity** | ✅ dtn7-rs / bp7-rs (RFC-aligned) | ✅ quinn + dtn7-rs | ✅ 100% Rust, mature | ❌ pure Python | N/A (HW) |
| **Standards-track** | ✅ IETF RFC 9171/9174/9172/9173 | ✅ IETF RFC 9000/9174 | Eclipse project | Custom (markqvist) | CCSDS (HW) |
| **PQ-underlay-independence** | ✅ envelope = payload | ✅ TLS-upgradeable | partial | yes (but Python) | yes (PHY) |
| **Fit for PQ delivery core** | **CHOSEN (P7)** | **CHOSEN (P7 bearer)** | Rejected | Rejected (no Rust) | Reference underlay |

---

## 5. Why libp2p is NOT chosen

- **libp2p** is a peer-to-peer *networking stack* (transport + muxing + peer routing + pubsub), not a disruption-tolerant overlay. It assumes peers are reachable on a transport (TCP/QUIC/WebRTC) and provides content routing (Kademlia) and gossipsub — but **no custody, no store-carry-forward, no lifetime/expiry, no BIBE**. For C11 (reliability > latency) and D3 (BPv7), libp2p would force us to *re-implement* DTN semantics on top of a stack that actively optimizes for low-latency liveness.
- Its QUIC transport is solid, but QUIC there is a *substitute* for BP, not a *bearer* for it. P7 instead uses QUIC **as the convergence layer under BP** (RFC 9174), keeping the BPv7 overlay (and its custody/BIBE model) as the source of truth.
- Adopting libp2p would also break the **1:1 `Bundle` ↔ BPv7 mapping** that the local `Node` already validates and that S1's `dtn7-rs` integration depends on. **Decision: libp2p explicitly NOT chosen.**

---

## 6. Conclusion — P7 Reaffirmed

**P7 = DTN/BPv7 (RFC 9171) + QUIC/TCPCLv4 (RFC 9174) + BIBE custody handoff (draft-ietf-dtn-bibect-05) is reaffirmed as the transport architecture for the decentralized PQ delivery mesh.**

1. **BPv7 (RFC 9171)** is the only candidate with *native* disruption tolerance, custody, and lifetime/expiry — the exact semantics the local `Node` already proves (RED+GREEN gates). It is the overlay that makes the mesh decentralized and C11-compliant.
2. **QUIC/TCPCLv4 (RFC 9000 + RFC 9174)** is the chosen *reliable, TLS-secured convergence layer* beneath BP — not a competitor to it. BP's store-carry-forward handles network-layer disruption; QUIC handles link-layer reliability and 0/1-RTT.
3. **BIBE (draft-ietf-dtn-bibect-05)** provides the custody-handoff mechanism (BRM retransmission, adapted from BPv6 custody transfer) that maps 1:1 onto `Node::accept → forward`. The store remains the source of truth.
4. **Rust maturity is settled:** `dtn7-rs` (https://github.com/dtn7/dtn7-rs) + `bp7-rs` implement RFC 9171/9174 in Rust, dual Apache-2.0/MIT — the integration target for gate **S1** (operator-gated, later, out of scope here).
5. **libp2p is NOT chosen** — it lacks custody/store-carry-forward and would break the `Bundle`↔BPv7 1:1 mapping.
6. **Zenoh** (excellent Rust, but real-time/IoT, no custody), **Reticulum** (decentralized but pure-Python, non-RFC, no Rust), and **SpaceWire/SpaceFibre** (on-board hardware underlay, not a mesh transport) are explicitly *not* the mesh transport. SpaceWire/Fibre remains a valid *physical underlay* beneath a BP convergence layer.

**PQ is underlay-independent (D3):** the ML-DSA-65 envelope is the bundle *payload* and is verified against the sender's key regardless of whether the byte pipe is QUIC, TCPCLv4, or a spacecraft bus. P7 does not weaken post-quantum guarantees.

---

## 7. Primary Sources Cited

- RFC 9171 — Bundle Protocol Version 7. https://www.rfc-editor.org/rfc/rfc9171.txt
- RFC 9172 — Bundle Protocol Security (BPSec). https://www.rfc-editor.org/rfc/rfc9172.txt
- RFC 9173 — Default Security Contexts for BPSec (BIB-HMAC-SHA2, BCB-AES-GCM). https://www.rfc-editor.org/rfc/rfc9173.txt
- RFC 9174 — DTN TCP Convergence-Layer Protocol Version 4 (TCPCLv4, QUIC/TCP bearer). https://www.rfc-editor.org/rfc/rfc9174.txt
- RFC 9000 — QUIC: A UDP-Based Multiplexed and Secure Transport. https://www.rfc-editor.org/rfc/rfc9000.txt
- draft-ietf-dtn-bibect-05 — Bundle-in-Bundle Encapsulation (BIBE), IETF DTN WG (expired 2025-09-14, work-in-progress). https://datatracker.ietf.org/doc/draft-ietf-dtn-bibect/ · https://www.ietf.org/archive/id/draft-ietf-dtn-bibect-05.txt
- dtn7-rs (Rust DTN daemon, RFC 9171/9174). https://github.com/dtn7/dtn7-rs
- bp7-rs (BPv7 codec, used by dtn7-rs). https://github.com/dtn7/bp7-rs
- Eclipse Zenoh. https://eclipse.dev/zenoh/ · https://github.com/eclipse-zenoh/zenoh
- Reticulum Network. https://reticulum.network/ · https://github.com/markqvist/Reticulum
- CCSDS (space data systems standards, SpaceWire/SpaceFibre pedigree). https://www.ccsds.org/about/
- ESA SpaceFibre (up to 6.25 Gbps/lane). https://www.esa.int/Enabling_Support/Space_Engineering_Technology/Onboard_Data_Processing/SpaceFibre

---

## 8. Next Gate Pointer (S1 — NOT executed here)

The operator-gated, sequential gate **S1** is: integrate `dtn7-rs`/`bp7-rs` as the real BPv7 daemon, mapping the local `Bundle` 1:1 onto BPv7 primary + payload blocks (see `node/src/transport.md`). This research doc is the *prerequisite finding*; S1 requires explicit operator approval and is intentionally **not** performed in this branch.
