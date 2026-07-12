# Spacecraft/Lab-Grade Transport for a Reliability-over-Latency Mesh
## Deep-research report — PRIMARY sources only

Scope: custody/store-and-forward substrate for a 3-role decentralized delivery protocol
(bebop/dowiz) with NOSTR/ActivityPub/MCP adapters, no server, local SQLite, existing
from-scratch ML-KEM-768 + ML-DSA-65 (PQ) Rust/WASM kernel. Dropping libp2p-gossipsub
because it is latency-optimized.

Legend: (a) genuine engineering fact · (b) applicable to THIS protocol · (c) over-claimed poetry.

────────────────────────────────────────────────────────
## 0. TRANSCRIPTION CORRECTIONS (read first)
────────────────────────────────────────────────────────
- **RFC 6250 is NOT "TCP-over-DTN."** RFC 6250 (Thaler/IAB, May 2011) is *"Evolution of
  the IP Model"* — an Informational IAB document about layering misconceptions. The real
  TCP convergence layer for DTN is **RFC 9174 "Delay-Tolerant Networking TCP
  Convergence-Layer Protocol Version 4 (TCPCLv4)"** (Standards Track, Jan 2022). Citing
  RFC 6250 as "TCP-over-DTN" is a factual error.
- **`rs-bp` does not exist.** github.com/RensVeenhuis/rs-bp returns HTTP 404 (verified).
  It is not a real Rust BP implementation. The real, active Rust impl is **dtn7/dtn7-rs**.
- **BPv7 custody transfer is demoted.** RFC 9171 Appendix A.2.2 states the spec *"Migrates
  custody transfer to the bundle-in-bundle encapsulation specification [BIBE]"*
  (draft-ietf-dtn-bibect, an IETF Internet-Draft — NOT yet an RFC). So BPv7 retains only a
  *"Request reporting of custody acceptance"* primary-block flag (bit 15, from RFC 5050);
  the actual custody-transfer mechanism is an extension, not core forwarding. Claims that
  "BPv7 has first-class custody transfer" are PARTIALLY true and must be qualified.

────────────────────────────────────────────────────────
## 1. DTN / Bundle Protocol (RFC 9171, RFC 4838, RFC 9174, BPSec)
────────────────────────────────────────────────────────
**Primary facts (verified):**
- RFC 4838 (Cerf et al., IRTF, Apr 2007) defines DTN as an overlay "above the transport
  (or other) layers," using "persistent storage to help combat network interruption,"
  with "hop-by-hop transfer of reliable delivery responsibility and optional end-to-end
  acknowledgement," explicitly targeting "intermittent connectivity, large and/or variable
  delays, and high bit error rates." §3.10 names "Reliability and Custody Transfer." → (a)(b)
- RFC 9171 (Burleigh/Fall/Birrane, IETF Standards Track, Jan 2022) is BPv7: "store-carry-
  forward overlay network" able to "use physical motility for the movement of data" (couriers)
  and "cope with intermittent connectivity, including cases where the sender and receiver are
  not concurrently present." §1. → (a)(b) — this is EXACTLY the courier use case.
- RFC 9174 (TCPCLv4) is the reference convergence layer (BP over TCP), Standards Track. → (a)
- BPSec RFC 9172 (Standards Track, Jan 2022) provides integrity + confidentiality for bundles,
  and explicitly supports **security-at-rest** ("store-carry-forward nature ... may require
  protecting data at rest"). Default contexts in RFC 9173. → (a)(b) directly relevant to PQ.

**Rust impl:** dtn7/dtn7-rs — "Rust implementation of a DTN based on RFC 9171" (verified,
110★, last release v0.21.0 Mar 2024, commits as recent as May 2026, Apache-2.0/MIT). It is
the only real Rust BP core. *Unverified:* whether dtn7-rs implements BIBE custody transfer and
a PQ BPSec context — must be confirmed before adoption.

**Verdict:** GENUINE BEST FIT for the substrate. (a) proven, (b) directly matches courier/
disruption/intermittent mandate. Caveat: native custody is via BIBE (I-D, not RFC); Rust impl
maturity/MIT-vs-PQ gap needs verification. NOT poetry.

────────────────────────────────────────────────────────
## 2. QUIC (RFC 9000)
────────────────────────────────────────────────────────
**Primary facts (verified):** RFC 9000 (Iyengar/Thomson, IETF Standards Track, May 2021) is
"a secure general-purpose transport protocol," "connection-oriented," with "low-latency
connection establishment," "0-RTT," flow-controlled streams, multiplexing, packet
authentication/encryption, and path migration. It "provides the necessary feedback to
implement reliable delivery and congestion control." → (a)

**Applicability:** QUIC REQUIRES both endpoints reachable within a live connection; it is
loss-resilient and 0-RTT but NOT store-and-forward across multi-day disconnections. Its own
design centers on *low latency*. → For intermittently-connected couriers, QUIC **alone does
NOT meet reliability>latency** (it heals loss on a live path, not absence of a path).

**Rust impl:** quinn (quinn-rs/quinn) — verified, 100% Rust, 5.2k★, 183 contributors, very
active (commits Jul 2026), Apache-2.0/MIT. Production-grade as a QUIC library.

**Verdict:** Excellent RELIABLE CONVERGENCE LAYER *under* a DTN overlay (a QUIC-CLA can carry
bundles on connected segments), but wrong as the primary substrate. (a) true, (b) only as
underlay, (c) "QUIC solves disruption tolerance" is poetry. Use QUIC (or TCPCLv4) as the CLA,
not the mesh.

────────────────────────────────────────────────────────
## 3. Eclipse Zenoh
────────────────────────────────────────────────────────
**Primary facts (verified):** eclipse.dev/zenoh states Zenoh "minimize[s] network overhead,
support[s] extremely constrained devices, support[s] devices with low duty-cycle ... provide[s]
a rich set of abstraction for distributing, querying and storing data," and — critically —
"provide extremely low latency and high throughput." It is pub/sub + query + geo-distributed
storages. Project state: **Incubating** at Eclipse; releases up to 1.9.0 (2026). → (a)
Rust impl: eclipse-zenoh/zenoh — verified, **100% Rust**, 3k★, 72 contributors, active (Jul
2026), Apache-2.0/EPL-2.0. The Rust crate is the reference implementation; other languages bind
to it. → (a) real Rust.

**Applicability:** Zenoh's own mission statement optimizes **latency and throughput**, the
opposite pole from the mandated reliability>latency. It has storages ("data at rest") and
liveliness, but is fundamentally a real-time data-flow protocol, not a store-carry-forward
overlay for months-long disruption. No custody-transfer concept. → (b) usable as a carrier/
convergence layer on connected lab segments; NOT a disruption-tolerant custody substrate.

**Verdict:** Real, Rust, strong for cyber-physical/edge/space LANs, but (c) "Zenoh is
disruption-tolerant" is over-claimed; its design centers on latency. (a) engineering true,
(b) partial (underlay/edge only).

────────────────────────────────────────────────────────
## 4. Reticulum (RNS)
────────────────────────────────────────────────────────
**Primary facts (verified, its own manual):** Reticulum is "a cryptography-based networking
stack ... that can continue to operate under adverse conditions, such as extremely low
bandwidth and very high latency." Offers "end-to-end encryption, forward secrecy,
autoconfiguring cryptographically backed multi-hop transport, ... unforgeable packet
acknowledgements." Runs in userland on Python 3; "Reference Implementation ... is the
Reference Implementation ... The Reticulum Protocol is defined entirely and authoritatively by
this reference implementation." Crypto: "Asymmetric X25519 encryption and Ed25519 signatures,"
"AES-256 in CBC mode ... HMAC using SHA256." Store-and-forward exists via the LXMF message
layer. → (a) genuine, real, deployed.

**Disqualifiers for THIS protocol:**
- **No Rust core.** Reference impl is Python 3. Cannot drop into the existing Rust/WASM
  kernel without a full re-implementation. (b) FAILS integration constraint.
- **Not post-quantum.** X25519/Ed25519/AES-256-CBC — classical. Operator mandated PQ
  (ML-KEM/ML-DSA). (b) FAILS crypto mandate.
- **No custody-transfer primitive.** Reliability is link ACKs + unforgeable ACKs + Channel/
  Buffer; no DTN-style custody handoff. (b) weaker for courier accountability.
- **Code-as-spec governance.** Protocol "defined entirely ... by this reference
  implementation" — weak for a spacecraft/lab-grade, auditable mandate. (c-ish risk.

**Verdict:** (a) real engineering, (b) NOT applicable here (no Rust, not PQ, no custody).
Do not adopt as the substrate. Could inspire the adapter/store-and-forward pattern only.

────────────────────────────────────────────────────────
## 5. SpaceWire / SpaceFibre (ESA / STAR-Dundee)
────────────────────────────────────────────────────────
**Primary facts (verified):** ESA/STAR-Dundee — SpaceWire is an on-board spacecraft link/
router standard (nodes + routers, time-codes); SpaceFibre is the higher-rate successor. These
are **hardware link layers**, not Rust libraries, not a mesh protocol. CCSDS (ccsds.org, 28
nations, 1000+ missions) standardizes the Space Telematics Domain — the "space analog" of
terrestrial Internet comms — and explicitly notes ground comms are "commercially based" while
"more specialized protocols [are] employed when crossing into space regions."

**Verdict:** (a) genuine space link standard; (b) relevant ONLY as the physical/link layer
*under* a node in a lab/sat (e.g., a bebop node's radio). It is NOT the mesh substrate and not
a Rust dependency. Note as link-layer context only. Not poetry, just scope-limited.

────────────────────────────────────────────────────────
## 6. COMPARISON & RECOMMENDED STACK
────────────────────────────────────────────────────────
Fit for "reliability > latency, usable in labs/satellites," scored (✔ strong, ~ partial, ✘ no):

| Candidate | Store&Fwd | Custody | Rust core | PQ-ready | Sat/lab fit |
|-----------|-----------|---------|-----------|----------|-------------|
| DTN/BPv7 (RFC 9171) | ✔ native | ~ via BIBE | ✔ dtn7-rs | ~ (BPSec ctx) | ✔ (CCSDS-aligned) |
| QUIC (RFC 9000) | ✘ | ✘ | ✔ quinn | ✘ (TLS cl) | ~ (transport) |
| Zenoh | ~ storage | ✘ | ✔ 100% Rust | ✘ | ~ (edge LAN) |
| Reticulum | ✔ LXMF | ✘ | ✘ Python | ✘ classical | ~ (off-grid) |
| SpaceWire/Fibre | n/a link | n/a | n/a | n/a | ✔ link only |

**RECOMMENDED CONCRETE STACK:**
**DTN/BPv7 as the custody/store-and-forward substrate, with QUIC (or TCPCLv4, RFC 9174) as the
reliable convergence layer on connected segments, running over link layers that may be
SpaceWire/SpaceFibre (sat/lab) or LoRa/ETH (ground).** Rust core = dtn7/dtn7-rs (verify BIBE +
PQ BPSec context). NOSTR/ActivityPub/MCP adapters serialize to opaque bundle payloads; the
3-role delivery logic lives above BP. This is the only option that is (a) standards-proven for
disruption/intermittent, (b) Rust-implementable, (c) custody-capable, and (d) CCSDS-space-aligned.

Alternative if a connected low-latency edge is the only environment: Zenoh-native (Rust, real)
— but it sacrifices custody/disruption-tolerance and is Incubating.

────────────────────────────────────────────────────────
## 7. RED-LINE GUARDRAILS (non-negotiable)
────────────────────────────────────────────────────────
1. **Custody transfer MUST be real, not assumed.** BPv7 core only *requests reporting of
   custody acceptance* (flag bit 15); implement **BIBE (draft-ietf-dtn-bibect)** for actual
   custody handoff between courier hops. Verify dtn7-rs supports it; if not, contribution
   required. Without BIBE, "custody" is poetry.
2. **Replay protection is mandatory in store-and-forward.** Enforce bundle *lifetime* expiry
   and dedupe on (source EID, Creation Timestamp) per RFC 9171; DTN time is in milliseconds.
   BPSec (RFC 9172) integrity blocks prevent tampering at rest. Replay is a first-class risk
   when bundles can sit for days.
3. **PQ envelope at the PROTOCOL layer REGARDLESS of transport.** QUIC TLS, Zenoh, and
   Reticulum are ALL classical (X25519/Ed25519/AES/TLS1.3). The mandated from-scratch
   ML-KEM-768 + ML-DSA-65 MUST wrap the bundle payload (e.g., as a custom BPSec security
   context, RFC 9172/9173 pattern, or an app-layer envelope) so post-quantum holds whether the
   underlay is QUIC, TCPCLv4, Zenoh, or SpaceWire. Do NOT rely on any transport's native crypto
   for PQ. This is the hard constraint the operator set.
4. **No latency-optimized default.** Keep gossipsub dropped; never default to "lowest latency"
   routing. Prioritize durable delivery + custody receipts over propagation speed.
5. **Auditability.** Prefer spec-defined protocols (RFCs) over code-as-spec (Reticulum) for a
   spacecraft/lab-grade mandate; keep a written security policy per RFC 9172 §1.2.

────────────────────────────────────────────────────────
## SOURCES (all PRIMARY)
- RFC 4838 — DTN Architecture (IRTF, Cerf et al.)
- RFC 9171 — Bundle Protocol v7 (IETF, Burleigh/Fall/Birrane)
- RFC 9172 — Bundle Protocol Security / BPSec (IETF, Birrane/McKeever)
- RFC 9174 — DTN TCP Convergence-Layer v4 (IETF, Sipos et al.)
- RFC 9000 — QUIC Transport (IETF, Iyengar/Thomson)
- RFC 6250 — Evolution of the IP Model (IAB) [corrected: NOT TCP-over-DTN]
- eclipse.dev/zenoh — Eclipse Zenoh project page; github.com/eclipse-zenoh/zenoh
- reticulum.network/manual — Reticulum Network Stack Manual (whatis/using)
- ccsds.org / esa.int / star-dundee.com — SpaceWire/SpaceFibre link-layer context
- github.com/dtn7/dtn7-rs, github.com/quinn-rs/quinn — verified Rust impls
