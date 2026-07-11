# 02 — Local-First Reference Architecture: topology + reachability physics

> **Lens 2 of 4** of the local-first-hub design program (2026-07-11). Scope: pick the **network
> topology** for turning dowiz's two half-hubs into ONE local-first decentralized hub, and design
> the **reference architecture** that survives the hard physics — a one-shot mobile-web customer
> reaching a vendor node that sits behind cellular/CGNAT.
>
> **Read-only session.** The ONLY file created is this one. Both repos left exactly as found
> (`bebop2/core` uncommitted WIP untouched; `dowiz/rebuild` untouched).
>
> **Evidence labels used throughout:**
> - **VERIFIED** — I read the primary source directly this session (repo file `path:line`, or a web
>   page a research lane fetched and quoted from its own domain / GitHub / spec).
> - **UNVERIFIED** — from a search snippet, a secondary source, or model memory; not confirmed against
>   a fetched primary.
> - **DESIGN JUDGMENT** — my synthesis/inference, not a citable fact; falsifiable by building it.
>
> **Sibling lenses** (reconcile against these; some not yet present at write time): the transition
> ladder + risk verdict is `D-transition-blueprint.md` (present, VERIFIED read of its §0–1.1); the
> vision-reconcile, data-sync-detail, and economics lenses are referenced where they own a decision
> this lens defers to them.
>
> **Primary sources read this session (repo):**
> `docs/research/2026-07-11-hub-architecture-review.md` (§0–5), `…/2026-07-11-full-project-audit-dowiz-bebop.md`,
> `docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v2-2026-07-11.md`,
> `docs/design/sovereign-core-mvp/DECISIONS.md` (D1–D7), `MANIFESTO.md` (§6),
> `rebuild/crates/domain/src/kernel.rs` (Envelope/Event/decide), `…/order_status.rs` (10-status machine),
> `docs/design/rebuild-plan/07-channel-hub-adoption.md` (cart-token spec), `docs/connection-budget.md`,
> `bebop-repo/bebop2/{README,ARCHITECTURE}.md`, `bebop-repo/docs/design/delivery-protocol/PROTOCOL-CENTRALIZATION-MAP.md`,
> `bebop-repo/docs/design/fable-protocol-2026-07-11/{F1,F3}.md`, `dowiz/web3-logistics-postmortem.md`,
> `dowiz/platform-vs-protocol-logistics.md`.
>
> **Web research** performed for this lens (four parallel lanes, ~140 searches/fetches total): browser↔node
> reachability physics; local-first sync-engine landscape; vendor-node packaging; Albania domain + delivery
> precedents. Their sourced claims are carried inline with labels.

---

## 0. Executive answer (the position, up front)

**Topology: (c) HYBRID — a per-vendor sovereign node + a dumb stateless reachability relay.** Not
(a) pure sovereign-node-only (it cannot be *reached* by a transient customer behind mobile NAT), not
(b) full P2P mesh (the one-shot browser customer physically cannot be a peer). The hybrid is the only
topology that satisfies all three operator constraints simultaneously:

| Operator constraint (verbatim intent) | How the hybrid satisfies it |
|---|---|
| "all order processing maximally LOCAL on the participants' own devices" | The **deterministic kernel runs on the vendor's device** and is the single writer/sequencer for that venue's orders. Customer & courier devices run the *same* kernel (WASM) for optimistic local decisions, and hold their own signed event slices. Processing is on the participants' devices. |
| "no single server that processes everything" | There is **no application server**. The one always-on box in the middle is a **dumb relay that processes nothing** — TLS terminates on the vendor node; the relay forwards ciphertext by SNI and cannot read, price, sequence, or decide anything. |
| "a dumb relay that processes nothing may be acceptable" | This is exactly, and only, what the middle box is. The industry-converged term for it is a **rendezvous/relay** (iroh relays, Earthstar replica servers, `@localfirst/relay`, DXOS signaling, Tailscale Funnel all land here — Lane-reachability, VERIFIED). |

**The one hard physical fact that forces this** (Lane-reachability, VERIFIED from Tailscale + libp2p +
WebKit primary sources): a one-shot mobile-web customer **cannot** open a peer-to-peer connection to a
vendor node behind cellular CGNAT. A browser page cannot open a raw socket; it can only make a *secure
context* connection (HTTPS / WSS with a **CA-valid** cert / WebRTC with STUN-TURN); Safari does not
implement `serverCertificateHashes` so libp2p-style self-signed WebTransport/WebRTC-direct is
Chromium+Firefox-only and in any case needs a **public IP on the server side** that a CGNAT'd node does
not have; and NAT hole-punching tops out at ~70–80% even between two cooperating apps. Therefore the
customer leg **must** traverse a publicly-addressable relay. The design's job is to make that relay
*dumb* — a reachability facilitator, never a processor.

**"Local decentralized" is therefore defined honestly as:** local-first done right does not delete
servers, it **relocates the kernel to devices and demotes every remaining server to a door** (relay,
push gateway, static host, backup blob store). This is the same rule the repo already trusts — the
EXPANSION-PLAN's "doors carry, the kernel decides" (hub review §1.1, VERIFIED) — carried to its
conclusion. It matches the sibling `D-transition-blueprint.md` §0 framing (VERIFIED read).

**Money integrity survives decentralization because payment is Cash-on-Delivery.** COD is the
architecture's superpower, not a limitation: cash settles physically at the door (atomic, offline,
double-spend-proof by physics), so **the distributed system never moves money — it only keeps a
verifiable double-entry record of obligations.** That collapses the hardest distributed-money problem
(offline double-spend, unsolved without secure hardware — Lane-sync, VERIFIED) into ordinary
signed-event bookkeeping with one sequencer per order. The vendor node is a per-venue
TigerBeetle-in-miniature (Lane-sync, VERIFIED).

Everything below scores the alternatives, then specifies the reference architecture, then confronts the
physics in detail.

---

## 1. The three parties and their irreducible asymmetry

The whole topology falls out of one observation: **the three parties are not peers.** Any design that
treats them symmetrically (topology b) fights physics and the trust model at once.

| Party | Device | Install? | Network | Identity | Authority role | Lifetime |
|---|---|---|---|---|---|---|
| **Customer** | one-shot mobile browser (Android Chrome dominant, iOS Safari minority) | **NO — will not install anything** | cellular / café Wi-Fi, **CGNAT, IPv4-only** | verified phone (OTP) — no durable keypair | **none** — proposes an order, confirms receipt | seconds–minutes (a single order) |
| **Vendor** | their own device (mini-PC / tablet / phone), runs 12h/day | **YES — owns the node** | shop Wi-Fi/fixed-wireless, often CGNAT, flaky | vault keypair (self-certifying) | **AUTHORITY** — sequences & decides all of their orders | persistent (the venue) |
| **Courier** | phone, vendor-**employed** | **YES — employee, can sideload an APK** | cellular, CGNAT, screen locked | vault keypair (self-certifying) | proposes lifecycle events; countersigns custody | per-shift |

Three consequences that bound the topology:

1. **The customer can never be a durable peer or hold a signing key** → the customer leg is a
   thin request/response over a reachable, CA-trusted TLS endpoint, *not* a P2P session. (Lane-reachability,
   VERIFIED: transient anonymous browser tab cannot be a durable peer; every system that admits browser
   visitors stands up a small always-on rendezvous node.)
2. **The vendor is naturally authoritative** — they own the menu, the order, the customer relationship,
   and they employ the courier. There is exactly **one accountable party per order.** This is precisely
   the property ONDC's food-delivery collapse shows you must preserve: ONDC split accountability across
   strangers and lost ~35% of orders when subsidies withdrew (Lane-domain, UNVERIFIED but well-sourced;
   `web3-logistics-postmortem.md` echoes it, VERIFIED). Vendor-authority is not a compromise of
   decentralization — it is the trust anchor that makes it work.
3. **Courier and vendor CAN be real peers** (both are installed apps that can hole-punch) — so the
   vendor↔courier leg *can* be P2P (iroh) with relay fallback, while the customer↔vendor leg cannot.
   The topology is asymmetric by necessity.

---

## 2. Topology options, scored

Scoring key: ✅ satisfies · ⚠️ partial / conditional · ❌ fails. Weighted for a 3-party COD money flow
with a transient customer and a non-technical vendor. Each score cites the load-bearing evidence.

### 2.1 The candidates

- **(a) Per-vendor sovereign node only** — the vendor's device is the hub; couriers and customers
  connect *directly* to it. "Local decentralized" = no central SaaS. **No relay.**
- **(b) Full P2P mesh** — every participant (customer, vendor, courier) is a peer; orders propagate
  peer-to-peer; no privileged node.
- **(c) Hybrid** — per-vendor sovereign node (the authority + kernel) **+ a dumb stateless relay** for
  reachability of the transient customer and NAT-fallback for couriers. The relay processes nothing.

### 2.2 Scorecard

| Criterion (weight) | (a) Sovereign-only | (b) Full P2P mesh | (c) Hybrid node+dumb-relay |
|---|---|---|---|
| **Transient customer can reach the node from cellular NAT** (critical) | ❌ A CGNAT'd node has no inbound path; DDNS+port-forward is dead under CGNAT (Lane-reachability, VERIFIED Tailscale). Works only if the vendor happens to have a public IPv4 + opens a port — false for most Albanian mobile/fixed-wireless links. | ❌ Browser cannot be a mesh peer (no raw socket; Safari no certhash; hole-punch needs public IP + ~70–80% best case). (Lane-reachability, VERIFIED libp2p/WebKit) | ✅ Customer dials a **CA-valid public relay** over plain HTTPS/WSS; relay forwards ciphertext to the node. The only path physics allows. |
| **"No single server that processes everything"** (red-line) | ✅ trivially (no server at all) | ✅ trivially | ✅ relay is dumb (SNI/TCP passthrough; TLS terminates on the vendor node — relay sees only SNI + timing). (Lane-reachability, VERIFIED Tailscale Funnel "relays do not decrypt"; WireGuard+nginx `stream`) |
| **Money integrity / global invariants** (red-line: integer money + state machine) | ✅ single writer on the node | ❌ **mesh cannot hold a global invariant** — two racing debits both commit locally; CRDTs give convergence, never invariant preservation (Lane-sync, VERIFIED Kleppmann OOPSLA'17 + 2024 invariants paper). Money in a mesh is unsound. | ✅ single writer = the vendor node's `kernel::decide`; relay never touches money. TigerBeetle-shape (Lane-sync, VERIFIED). |
| **Courier reachability (locked phone)** | ⚠️ courier app can hole-punch to node ~70–80%; fails behind double-NAT; no wake path | ⚠️ same hole-punch ceiling; plus no authority to dispatch from | ✅ iroh (hole-punch **+ relay fallback**) for the data path; **push (FCM/ntfy) via a dumb gateway** to wake a locked phone. (Lane-reachability + Lane-packaging, VERIFIED iroh 1.0 relay fallback; Web Push needs vendor push service) |
| **Non-technical vendor operability** (weight: high — restaurateur in Durrës) | ⚠️ they must expose a port / run a tunnel themselves | ❌ mesh membership, peer discovery, key management on every device — untenable for one-shot customers | ✅ node ships as an appliance; relay is a €4/mo box or a free Funnel — set once, forget. Benchmark: as simple as a certified fiscal box (~€84/yr easyPos). (Lane-domain, VERIFIED) |
| **Offline / partition resilience** | ✅ node is self-contained; but *unreachable* offline (no relay) | ⚠️ mesh degrades but customer can't join anyway | ✅ node keeps deciding offline; queued intents drain when relay reconnects; mirrors Albania's own 48h fiscal-offline law (Lane-domain, VERIFIED) |
| **Confidentiality / dumb-relay trust model** | ✅ (nothing in the middle) | ✅ (E2E by construction) | ✅ **if** the relay is passthrough (own VPS/Funnel). ❌ if Cloudflare Tunnel (terminates TLS, inspects plaintext — reject for the dumb-relay requirement). (Lane-reachability, VERIFIED CF community) |
| **Cost at 1 venue** | €0 infra, but needs a public IP (often unavailable) | €0 but needs hosted signaling/STUN/TURN anyway (DXOS runs hosted signaling — Lane-reachability, VERIFIED) — so "no server" is illusory | **~€4.15/mo** Hetzner CX23 (20 TB incl.) or **€0** Tailscale Funnel. Order traffic ≈1.5 GB/mo at 1,000 orders/day — a rounding error. (Lane-reachability, VERIFIED Hetzner + my traffic calc) |
| **Path to "one node among many" endgame** (courier liquidity pooling, open matcher) | ⚠️ isolated nodes, no rendezvous to federate | ✅ mesh is the endgame — but unreachable today | ✅ the relay generalizes into a rendezvous/gossip layer later; iroh-gossip is browser-compatible and 1.0-stable (Lane-reachability, VERIFIED) — evolves without a rewrite |
| **Prior-art validation** | CoopCycle = one central instance per co-op (not this) | No production mesh food-delivery exists; every "decentralized delivery" token is dead/zombie (`web3-logistics-postmortem.md`, VERIFIED) | Ditto × Chick-fil-A proves **restaurant order flow on cloud-optional local sync at national scale** — closest living validation (Lane-domain + Lane-sync, VERIFIED qsrmagazine/accesswire) |

### 2.3 Verdict

**(c) Hybrid wins decisively; (b) is physically impossible for the customer leg and unsound for money;
(a) is unreachable in the target market.**

The subtle point the operator's framing invites — "(a) per-vendor sovereign node as *local
decentralized* because there's no central SaaS" — is **almost right and worth preserving**: the vendor
node genuinely *is* the hub and the authority. What (a) misses is that a hub nobody can reach is not a
hub. Adding a **dumb relay does not re-introduce a central processor** — it re-introduces a *doorway*,
and the whole discipline of this architecture is that doorways carry, they never decide (hub review
§1.1, VERIFIED). So the honest statement is: **(c) is (a) made reachable, with the reachability layer
held to the same "carries, never decides" law as every other door in the system.**

This also matches what the repo's own doctrine already concluded independently: the UNIFIED blueprint
v2 §2 pins "the owner hub is ALWAYS the vendor's sovereign control point; the network only ever adds
courier liquidity and cross-node trust — it NEVER takes custody of the vendor's money or data"
(VERIFIED). The relay is the first, minimal instance of "the network."

---

## 3. Reference architecture

### 3.1 The picture

```
                    ┌───────────────────────────────────────────────────────────────┐
                    │                 IRREDUCIBLE INFRA FLOOR                          │
                    │   (dumb doors — process NOTHING about orders/money)             │
                    │                                                                 │
   one-shot         │   ┌────────────┐   ┌──────────────┐   ┌───────────────────┐    │
   customer  ──HTTPS/WSS─▶│  RELAY     │   │ PUSH GATEWAY │   │ STATIC HOST + LE   │    │
   (browser, ──(CA-valid  │ SNI/TCP    │   │ FCM / Apple  │   │ storefront HTML,   │    │
    no install)   cert)   │ passthrough│   │ / ntfy       │   │ WASM kernel bundle │    │
                    │      │ (ciphertext│   │ (encrypted   │   │ (cacheable)        │    │
                    │      │  only)     │   │  payloads)   │   └───────────────────┘    │
                    │      └─────┬──────┘   └──────┬───────┘                            │
                    └────────────┼─────────────────┼────────────────────────────────────┘
                                 │ ciphertext       │ wake-only nudge
                                 ▼                  ▼
        ┌────────────────────────────────────────────────────────────────┐
        │  VENDOR SOVEREIGN NODE   (the AUTHORITY — the only processor)    │
        │                                                                  │
        │   dowiz-core kernel (native)   Command → decide → Vec<Event>     │
        │       single writer / sequencer for THIS venue's orders          │
        │   SQLite: append-only event log  +  projections  (WAL, FULL)     │
        │   bebop2 codec: canonical bytes + per-event signature slot        │
        │   fiscalization queue (→ Albania CIS, 48h-offline-tolerant)       │
        │   Litestream → S3/MinIO backup (dumb blob store)                 │
        └───────▲───────────────────────────────────────▲─────────────────┘
                │ iroh (QUIC hole-punch                   │ same kernel (WASM),
                │       + relay fallback)                 │ optimistic + rebase
                │                                         │
        ┌───────┴────────┐                        (customer browser also runs the
        │  COURIER PHONE  │                         WASM kernel read-only to render
        │  PWA + thin     │                         status; holds only its order slice
        │  native wrapper │                         via the track-token it was given)
        │  (vendor-employed│
        │   → can install) │
        │  vault keypair   │
        └─────────────────┘
```

### 3.2 What runs where (the "maximally local" decomposition)

| Concern | Vendor node | Courier device | Customer browser | Relay / gateway |
|---|---|---|---|---|
| `kernel::decide` (authority) | **YES — sole writer** | no (proposes commands) | no (proposes 1 intent) | **never** |
| `fold`/`replay` (read model) | YES (SQLite projection) | YES (WASM, own slice) | YES (WASM, render status) | never |
| Event log (append-only) | **YES — the log of record** | own signed slice | order slice only | never |
| Money math (`Lek(i64)`, pricing) | **YES** | no | no | **never** |
| Signing | node key (sequenced heads) | courier key (lifecycle + custody) | none (phone/OTP identity) | never |
| Order-status state machine | **YES (enforced in `decide`)** | mirrors, optimistic | mirrors, read-only | never |
| Fiscalization → CIS | **YES (only here)** | no | no | no |
| Reachability / NAT traversal | terminates TLS; iroh endpoint | iroh + push | plain TLS client | **YES — its only job** |
| Wake a locked device | — | receives push | — | **push gateway only** |

Everything that is *processing* is on the vendor's device. Everything in the middle is a door.

### 3.3 The kernel (L0) — one Rust core, native + WASM everywhere

The kernel already exists and is the correct shape (VERIFIED reads this session):

- **`decide: (&OrderState, Command) -> Result<Vec<Event>, DomainError>`** — the ONE business-mutation
  door (`rebuild/crates/domain/src/kernel.rs:297`, VERIFIED). Pure, total, side-effect-free; composes
  state-machine → actor gate → courier-strand guard → pricing corridors.
- **10-status state machine** with an exhaustive transition relation and terminal set
  (`order_status.rs:19–92`, VERIFIED: Pending/Confirmed/Preparing/Ready/InDelivery/Delivered/
  Rejected/Cancelled/Scheduled/PickedUp).
- **Integer money** `Lek(i64)`, no `From<f64>`, checked arithmetic; float banned by clippy + the wasm32
  build gate (VERIFIED via DECISIONS + kernel doc).
- **`Envelope { seq, at, cause, event }`** (`kernel.rs:214`, VERIFIED) — the log row; `decide` returns
  bare `Event`s, the shell wraps each into an Envelope at the persistence boundary. This is **already
  the signed-event seam**: DECISIONS D2 mandates "every mutating event carries content-hash
  (`request_hash`) + a signature slot" (VERIFIED). The slot is NULL for the MVP and becomes real
  per-actor signatures as the FIRST decentralization step (UNIFIED v2 gap 4, VERIFIED).

**Portability (DESIGN JUDGMENT, backed by proven precedent):** compile the same crate to (a) native on
the vendor node and (b) `wasm32` for the courier PWA and the customer page. This is the Automerge
pattern — "one Rust core, exposed via WASM/FFI everywhere" — mature in production; Automerge's WASM
bundle is ≈320 KB and this kernel is domain logic, not a CRDT engine, so a `wasm-opt -Oz` build
realistically lands **50–200 KB** (Lane-packaging, VERIFIED pattern; size = engineering estimate,
UNVERIFIED). Serve the one bundle from the static host; the service worker caches it. The customer page
runs `fold`/`replay` **read-only** to render status; the courier runs it to make optimistic decisions
that the node confirms or rebases.

> **Blocker to note (VERIFIED-in-doc):** the Rust checkout currently **bypasses `kernel::decide`** —
> no `Command::PlaceOrder` is constructed in the api crate; pricing is called directly (hub review §3
> finding 1; UNIFIED v2 gap 6). The local-first architecture is *predicated* on the kernel being the
> sole door. Closing this bypass is Phase 1 of `D-transition-blueprint.md` and a hard precondition for
> everything here. Until it closes, "the node decides" is aspirational.

### 3.4 The vendor node = single-writer sequencer (why not CRDTs for orders)

The literature and industry are unanimous for invariant-heavy domains (Lane-sync, VERIFIED):

- **CRDTs cannot maintain a global invariant** (money conservation, non-negative balance, legal state
  transitions) — invariants aren't locally decidable; enforcing them requires synchronization
  (Kleppmann, "Verifying Strong Eventual Consistency," OOPSLA'17; 2024 "Consistent Local-First
  Software" paper uses the exact "prevent negative balance" example). VERIFIED.
- **When an authority exists, "CRDTs and OT are merely optimizations over server reconciliation"**
  (Matthew Weidner, "Architectures for Central Server Collaboration," 2024). VERIFIED-fetched. The
  vendor node *is* the authority.
- The productized shape is **Replicache/Zero mutation-replay**: clients run named mutators optimistically;
  the authority replays them authoritatively; client state rebases on the authoritative outcome. Zero 1.0
  shipped June 2026 and is explicitly server-authoritative. VERIFIED.
- The event-sourced flavor is **LiveStore** ("sync the log, not the state"; SQLite is a pure fold of an
  ordered per-store eventlog; the sync backend is a dumb ordered relay — a **vendor node can BE the sync
  backend**). LiveStore 0.4.0, Apache-2.0. VERIFIED. **We do not adopt LiveStore** (TypeScript, pre-1.0)
  — our Rust kernel replaces it — but its model is the exact reference (DESIGN JUDGMENT).

**Therefore:** the vendor node is the single writer for its venue's order aggregates. Customer and
courier submit **signed commands / proposed events**; the node's `decide` accepts or rejects and
**sequences** them into the log; other parties rebase on the authoritative sequenced heads. This is
architecturally a per-venue **TigerBeetle** (hash-chained append-only log + deterministic state machine
+ fixed debit/credit schema, invariants checked before an event is emitted). VERIFIED as the canonical
invariant-heavy prior art.

**CRDTs still have a lane** — but only for genuinely convergent, invariant-free data: menu drafts /
availability / stop-list, courier GPS trail, chat. Use **Loro or Automerge 3** (both healthy, Rust/WASM)
there; **never** for money or order status (Lane-sync, VERIFIED). This split is exactly
`D-transition-blueprint.md`'s Phase 3 (device-authoritative menus) vs Phase 4 (money core) ordering.

### 3.5 SQLite as event store + projection (the local store)

- **Schema:** append-only `events(aggregate_id, seq, event_blob, sig, content_hash, prev_hash)` with
  `UNIQUE(aggregate_id, seq)`; projections are rebuildable folds. A hand-rolled table beats the thin
  Rust ES crates (`cqrs-es`/`sqlite-es` exist but the ecosystem is thin; the deterministic kernel needs
  almost nothing from a framework). Lane-sync, VERIFIED.
- **Durability:** WAL mode with **`synchronous=FULL` on the event-append transaction** (money-bearing);
  `NORMAL` may lose the last committed transaction on power-cut — not corruption, but unacceptable for an
  order log. Projections can rebuild, so `NORMAL` is fine there. Lane-sync + Lane-packaging, VERIFIED
  (sqlite.org WAL/howtocorrupt + agwa.name durability essay).
- **Storage is the real enemy:** SD cards lie about flush and corrupt on brownout — prefer an **SSD**
  (N100 mini-PC) or industrial SD + read-only overlayfs root + separate data partition. Lane-packaging,
  VERIFIED (adafruit read-only-pi + linuxblog).
- **Backup:** **Litestream v0.5.14** (July 2026, actively maintained; v0.5.9 added follow-mode for a
  warm spare) → any S3-compatible target (self-hosted MinIO or Backblaze B2). The backup target is a
  **dumb blob store** — zero processing. Lane-packaging, VERIFIED (fly.io + GitHub releases).
- **Browser SQLite** (for the courier PWA's local read model): official `sqlite3` WASM `opfs-sahpool`
  VFS works Safari 16.4+ with no special headers (single-connection); or `wa-sqlite` OPFSCoopSyncVFS for
  multi-tab. Safari remains the weak platform (OPFS handles close on backgrounding). Lane-packaging,
  VERIFIED (sqlite.org/wasm + powersync blog). **DESIGN JUDGMENT:** keep the customer page keyless and
  storage-light (in-memory fold of its own order slice); reserve OPFS SQLite for the installed courier
  PWA.

### 3.6 The bebop2 protocol layer — canonical bytes + signature slot

"bebop2 protocol" in this program means the operator's **own** zero-dependency core in
`/root/bebop-repo/bebop2/` (not the 6over3 `bebop` wire format on crates.io) — a hand-written
fixed-layout codec producing content-addressed canonical bytes, plus hybrid PQ signatures (VERIFIED read
of `bebop2/{README,ARCHITECTURE}.md`). Its role here:

- **Canonical bytes for signing.** Every `Envelope` must serialize to *one* byte string so a signature is
  verifiable by any party. Options ranked (Lane-packaging, VERIFIED): **borsh** ("canonical and
  deterministic… meant for security-critical projects" — its raison d'être is hashing/signing) > postcard
  (deterministic for a fixed schema but serde map-ordering can betray you — ban unordered maps) > CBOR
  needs a determinism profile (CDE/dCBOR) = extra moving parts > **the 6over3 `bebop` format has no
  canonical-bytes contract — do NOT use it for signatures** (transport only). **DESIGN JUDGMENT:** the
  bebop2 *hand-written fixed-layout codec* is aligned with borsh's discipline (fixed layout, no
  reflection, no alloc at hot path — VERIFIED ARCHITECTURE.md); adopt it for the signed envelope, with
  borsh as the fallback if bebop2's codec is not ready. This keeps the "zero-dependency, deterministic"
  mandate.
- **Signatures — tier the crypto honestly.** bebop2's hybrid **ML-DSA-65 ⊕ Ed25519** is the right
  *destination*, but it is **not FIPS-interoperable, has KyberSlash-class timing leaks, and its wasm32
  empty-import gate currently FAILS (~94 errors)** (UNIFIED v2 §4, VERIFIED-in-doc). **Therefore
  (DESIGN JUDGMENT, carrying G09's tiering):** until bebop2 is constant-time + ACVP-interop + externally
  audited, use **host-crate Ed25519** for anything guarding value/identity, and use bebop2 primitives
  only at **Tier 1** (non-adversarial integrity, e.g. content hashes). The signature *slot* and the
  self-certifying identity *scheme* (`id = H(pq_pub ‖ classical_pub)`) go in now (cheap seam, DECISIONS
  D2); the bebop2 *implementation* graduates into the slot only when audited. `D-transition-blueprint.md`
  Phases 3–5 gate this identically — reconcile there.

### 3.7 Identity floor

- **Vendor & courier:** self-certifying vault keypair; id = hash of public key(s). No central issuer to
  phone home to (`vault.rs`/`pod.rs` self-cert is the strongest, most honest part of the protocol —
  F3 §0, VERIFIED). Courier is vendor-employed, so **enrollment is a local, in-person trust event**
  (owner mints the courier's key into the venue's roster) — no global KYC oracle needed, sidestepping
  DANGER #4 (identity root-of-trust) entirely for the MVP.
- **Customer:** identity = **verified phone number via OTP**, plus the existing opaque **track-token**
  (`apps/api/src/routes/customer/track.ts`, VERIFIED: opaque `?t=` → customer JWT exchange). The phone is
  the only durable customer identity and the vendor's dispute recourse channel. OTP order-confirmation is
  the proven 25–40% fake-order/RTO killer in COD markets and the natural fit for a one-shot visitor
  (Lane-domain, UNVERIFIED but multiply-sourced). WhatsApp/Viber saturation in Albania makes WhatsApp OTP
  higher-delivery than SMS (Lane-domain, UNVERIFIED).

### 3.8 Money as double-entry over signed events (the COD superpower)

Because no digital money moves, the system maintains a **verifiable record of obligations**, not a value
transfer (Lane-sync, VERIFIED as the COD "cash-in-transit" reconciliation model):

```
order placed      : customer  →  vendor    debt = order total        (Priced event, node-signed)
cash collected    : courier holds vendor's cash at the door          (CashCollected, courier-signed)
delivered         : custody handoff complete                         (Delivered, courier-signed;
                                                                       customer OTP / countersign)
settlement        : courier  →  vendor    remittance clears the debt (SettlementReceived, node-signed)
```

- **One sequencer per order** (the vendor node) enforces conservation before emitting any event —
  money errors become *impossible states*, not reconciliation bugs (TigerBeetle discipline, VERIFIED).
- **Counter-signature at each custody hand-off** replaces escrow. No escrow, no on-chain settlement, no
  offline-digital-money problem (which is unsolvable without secure hardware — Lane-sync, VERIFIED CBDC
  literature). This is already how the live system models it: `payment.method: 'cash'` literal +
  deliver-v2 `payment_outcome` enum + append-only `courier_cash_ledger` 'hold' rows (hub review §4.3,
  VERIFIED). The local-first design **signs** those same events and lets the courier/customer countersign.
- **`paid_partial` is structurally unrepresentable**, `paid_full` requires `cash_amount === total` else
  422 (hub review §4.3, VERIFIED) — the state machine already refuses the money-integrity violations.
- **Dispute = "what happened," not "double-spend."** The signed, counter-acknowledged event chain is the
  evidence; the vendor↔courier **employment relationship is the arbiter** (one accountable party per
  order). PoD is *contestable*, never signature-as-ground-truth (F3 §5 weakest-link admission, VERIFIED);
  design for contestability. The legally-mandated **NIVF/QR fiscal receipt is a free,
  government-verifiable proof-of-sale** to chain the PoD to (Lane-domain, VERIFIED).

### 3.9 Fiscalization — the one legally-unavoidable central endpoint, kept on the node

Albania's Law 87/2019 ("fiskalizimi") requires **every** cash sale to be reported in real time to the
tax authority's Central Information System (CIS), which returns a **NIVF** code (~2 s) that must print on
the receipt (Lane-domain, VERIFIED dddinvoices + sherbimekontabiliteti). This is the **only** mandatory
central server in the whole design — but the law is **offline-first by construction**: the issuer's
software signs an **NSLF** locally (works offline), and offline invoices upload within **48 hours** of
reconnection (Lane-domain, VERIFIED). Architecture rules (DESIGN JUDGMENT):

1. Fiscalization lives on the **vendor node only** — never on the relay, never on any shared server.
2. The hub is **not** the fiscal system of record. Either integrate a certified package
   (easyPos-class, ~€84/yr) or leave fiscalization to the restaurant's existing certified software and
   treat the order log as *non-fiscal* operational data (Lane-domain, VERIFIED). Print **"order tickets,"
   not legal receipts** (the certified fiscal device issues the legal receipt) to avoid the 50,000-lekë
   per-receipt fine (Lane-packaging/domain, VERIFIED/UNVERIFIED).
3. The fiscal queue **mirrors the event log's offline discipline exactly** — queue NIVF requests, drain
   on reconnect within 48h. The node's own local-first design *is* the legally-prescribed offline
   procedure. Never let relay/hub availability gate invoice issuance.

---

## 4. The hard physics — customer browser ⇆ CGNAT'd vendor node

This is the section the whole topology turns on. The operator asked to "address the hard physics" and
"enumerate real options." All claims here are from the reachability research lane (fetched primaries).

### 4.1 What a mobile browser can and cannot do (VERIFIED)

- **Cannot** open a raw TCP/UDP socket. The Direct Sockets API exists only for Isolated Web Apps
  (effectively ChromeOS) — never for a QR-scanned website. (chromestatus/developer.chrome.com)
- **Can** make secure-context connections only: HTTPS `fetch`, **WSS with a CA-valid cert**
  (self-signed fails on mobile with **no override**), **WebTransport** (Chrome/Edge/Firefox; Safari
  ≥26.4 but **without** `serverCertificateHashes`), and **WebRTC DataChannel** with STUN/TURN (all
  browsers; the one API designed for P2P).
- **The self-signed-cert P2P dream is dead on mobile:** WebKit stated (Dec 2024) it does **not** intend
  to implement `serverCertificateHashes` — so libp2p-style WebTransport/WebRTC-direct is
  Chromium+Firefox-only, and even there needs a **public IP on the server**, which a CGNAT'd node lacks.
  (w3c/webtransport#623; libp2p browser-connectivity docs.)

### 4.2 iroh (Rust, n0-computer) — 2026 state (VERIFIED)

- **iroh v1.0.0 shipped June 15, 2026** ("Dial keys, not IPs"); v1.0.2 July 6, 2026. Wire-protocol +
  API stability guaranteed across minor versions and languages. ~200M endpoints created in 30 days on
  public relays. (github releases + iroh.computer/blog/v1)
- **Architecture:** relays are stateless facilitators; connections start relayed, **upgrade to direct
  QUIC** via hole-punching; **"relay servers do not have access to the data — it's E2E-encrypted."**
  (docs.iroh.computer/concepts/relays) — this is the "dumb relay" property, native.
- **Browser: iroh runs in WASM, but ALL browser connections go through a relay over WebSocket**
  ("browsers can't send UDP"); direct browser paths (WebTransport certhash / WebRTC) are *future*, not
  implemented. Of the protocol crates, **iroh-gossip is browser-compatible**. (docs.iroh.computer/
  deployment/wasm-browser-support)
- **Self-hosting:** open-source `iroh-relay` crate + CLI, TOML config with allowlist/denylist or
  **bearer-token** access control, Let's Encrypt built in. n0's public relays are dev/test only ("no
  uptime guarantees"); hosted n0des Free $0 / Pro $19/mo. (crates.io/iroh-relay; iroh.computer/pricing)
- **Production users:** Delta Chat, Fedimint, Paycode payment terminals. (awesome-iroh)
- **Hole-punch success:** n0 publishes **no official number**; secondary "~90%" claims are UNVERIFIED;
  large-scale DCUtR measurement puts conditional success at **70% ± 7.1%** (arXiv 2604.12484, UNVERIFIED
  snippet).

**Verdict on iroh (DESIGN JUDGMENT):** iroh is the **right transport for vendor↔courier and (later)
vendor↔vendor** — both are apps, hole-punch + relay fallback is exactly its design, self-hosted
`iroh-relay` with bearer tokens keeps the relay dumb and owned. iroh is **not** the customer-leg answer:
in a browser it is relay-only over WebSocket anyway, and pulling the iroh WASM (compile-your-own, no NPM)
into a one-shot page is heavier than a plain WSS request for zero benefit. **Customer leg = plain
HTTPS/WSS; courier/vendor mesh = iroh.**

### 4.3 libp2p — 2026 state (VERIFIED)

- **WebTransport:** certhash valid ≤14 days, js-libp2p rotates 2 overlapping (~28 days). **Safari 26.4
  (Mar 24, 2026) shipped WebTransport** — but **not `serverCertificateHashes`**, so Safari can't dial
  self-signed libp2p endpoints. rust-libp2p has a browser client but **no production server listener**.
- **WebRTC-direct:** browser→**public-IP** server with certhash, no signaling — but **fails when the
  server is behind NAT** (needs a dialable IP). rust-libp2p `libp2p-webrtc` is **0.9.0-alpha.1, still
  alpha, UDP-only** in 2026.
- **Circuit Relay v2 + DCUtR:** a browser *can* dial through a relay (relay carries WebRTC SDP
  signaling); hole-punch ~**80%** (libp2p's own figure) / 70% (measured). Relay must be publicly
  reachable; js/Node make poor relays (go-libp2p recommended).
- **Maintenance:** js-libp2p **3.3.5 (Jul 2026), biweekly** — healthy. **rust-libp2p 0.56.0 (Jun 2025)**
  — a year without a stable release; slow cadence.

**Verdict on libp2p (DESIGN JUDGMENT):** heavier and less browser-friendly than iroh for our exact
shape; Safari's certhash refusal + rust-libp2p's slow cadence + alpha WebRTC server make it a poor fit.
Not chosen. iroh covers the same P2P need with a cleaner Rust story and native dumb-relay.

### 4.4 Tunnels / reachability for the customer leg — the decision (VERIFIED)

The customer dials a **publicly-addressable, CA-valid** endpoint that forwards to the CGNAT'd node. The
requirement is that this endpoint be a **dumb relay that never sees plaintext.**

| Option | Dumb (no TLS termination)? | Cost | Verdict |
|---|---|---|---|
| **Self-hosted WireGuard→VPS + nginx `stream` SNI-passthrough** (or **rathole** / **frp** TCP mode) | ✅ **TLS terminates on the vendor node**; VPS forwards ciphertext by SNI | **€4.15/mo** Hetzner CX23 (20 TB) | **PRIMARY.** Canonical dumb relay; you own it; passthrough. frp v0.69.1 (Jun 2026) very active; rathole (Rust, ~500 KiB, Noise-encrypted control) works but last release Oct 2023. |
| **Tailscale Funnel** | ✅ "Funnel relays do not decrypt"; TLS terminates on your device | **€0** | **FALLBACK (zero-VPS).** But `*.ts.net` domain, ports **443/8443/10000 only**, unpublished bandwidth caps, third-party dependency. Good for pilot; owns your DNS name. |
| **Cloudflare Tunnel** | ❌ **terminates TLS at CF edge, inspects plaintext/WAF** | €0 | **REJECT** for the dumb-relay red-line. (Fine only if plaintext-at-edge is acceptable — it is not here.) |
| **ngrok free** | partial | €0–8/mo | **REJECT** — interstitial warning page kills one-shot customer conversion. |
| **DDNS + port-forward** | n/a | €0 | **DEAD under CGNAT** — no inbound mapping exists; you can't reconfigure the carrier's NAT. |
| **IPv6 direct** | n/a | €0 | **Not a primary path.** Albania ~39% user IPv6 (Vodafone 15%, ONE 14%); both ends must have it + you still need a CA cert. Opportunistic only. |

**Cert story (DESIGN JUDGMENT):** the vendor node obtains a CA-valid cert via **Let's Encrypt DNS-01**
(works behind NAT — no inbound needed) for a subdomain the platform delegates
(`<venue>.order.example.al`), or uses the Funnel-provided `ts.net` cert in the fallback. TLS terminates
on the node; the relay never holds the key. This is the single most important line of the trust model:
**customer→vendor confidentiality rests on TLS terminating on the vendor's device.**

**Bandwidth reality (VERIFIED calc):** an order payload is 2–10 KB; 1,000 orders/day ≈ **1.5 GB/month** —
~0.008% of a €4 VPS's 20 TB allowance. The relay cost is the fixed €4/mo box, not traffic. "No single
server that processes everything" costs €4/month.

### 4.5 Waking a locked courier phone (VERIFIED)

The dispatch-notification problem is separate from the data path and is the current system's single
largest product gap (couriers get **zero** out-of-app notification — hub review §4.6, VERIFIED).

- **Web Push** works Android-Chrome-PWA (rides FCM; subject to Doze/OEM killers — fine for "new order,"
  not guaranteed-latency) and iOS **only** after Add-to-Home-Screen (16.4+). It **always** routes via the
  browser vendor's push service (Chrome→FCM, Safari→Apple) — an **unavoidable centralization** for
  notifications, but payloads are E2E-encrypted (RFC 8291), so Google/Apple see metadata, not order
  contents. (Lane-reachability, VERIFIED.)
- **Because couriers are vendor-employed and CAN install**, the reliable answer is a **thin native
  wrapper** (Tauri-mobile or a small Kotlin shell) with a real FCM token + battery-optimization
  exemption, wrapping the PWA — native-grade delivery on Android. Or **ntfy/UnifiedPush** self-hosted
  (couriers install the distributor). Both keep order logic off the transport (the push is a wake-only
  "open your app" nudge; the deep-link opens the local/tunneled node). (Lane-packaging/reachability,
  VERIFIED.)
- **Do NOT design customer flows around push** — iOS needs A2HS and the customer tab is one-shot. Customer
  status = in-page WebSocket (via the relay) + SMS/WhatsApp confirmation. (Lane-reachability, VERIFIED.)
- **Background geolocation is impossible for a PWA in 2026** (Geolocation runs only foreground; Wake
  Lock is the fragile workaround) — so live courier GPS requires the app foregrounded during a run, or
  the native wrapper. (Lane-reachability, VERIFIED.)
- **Telegram Bot API** is a legitimate zero-infra courier *notifier*: `getUpdates` long-polling is
  **outbound from the vendor node** (no inbound port — perfect behind CGNAT), free, ~1 msg/s/chat. It
  becomes a notification-transport SPOF, but **zero order logic transits it** if messages are just "open
  your PWA" nudges. Acceptable as a dumb notification door; note the dependency. (Lane-packaging,
  VERIFIED — and the owner bot already has `/open`, hub review §4.6.)

### 4.6 The customer order flow, end-to-end (DESIGN JUDGMENT, physics-checked)

```
1. Customer scans QR / taps link  →  GET https://<venue>.order.example.al/s/:slug?ch=qr
   (plain HTTPS through the dumb relay → TLS terminates on the vendor node; static storefront + WASM
    kernel bundle may also be served from the static host / CDN and cached)
2. Browser renders menu (server-priced from the node's SQLite), builds cart client-side.
3. Customer submits a signed-shaped ORDER INTENT — the existing cart-token doctrine:
   { slug, items:[{product_id, qty, note?}], channel, iat, exp≤15m, nonce }  — NO prices, NO totals.
   (07-channel-hub-adoption.md §3, VERIFIED: single-use nonce, server re-prices every line.)
   POST over WSS/HTTPS → relay (ciphertext passthrough) → vendor node.
4. Vendor node: verify OTP (phone) → kernel::decide(PlaceOrder{cart}) → server-prices in-transaction →
   emits [Priced, StatusChanged→Pending] → appends signed Envelopes to the SQLite log → mints the
   opaque track-token.
5. Node returns track-token; customer page holds ONLY its own order slice, runs fold() (WASM) read-only
   to render live status over the WSS room. On reconnect it rebases on the node's sequenced heads.
6. Owner accepts (node or Telegram) → courier dispatched (iroh + push) → deliver-v2 cash-as-proof at the
   door (courier signs CashCollected/Delivered; customer OTP/countersign) → node emits SettlementReceived.
```

Every processing step is on the vendor node. The relay moved ciphertext. The customer never held a key,
never installed anything, and reached a CGNAT'd node from cellular — the physics hold.

---

## 5. The irreducible infra floor (what "no central server" honestly costs)

After full local-first, the surviving infrastructure — none of it a *processor* — is:

| Component | What it is | Processes orders/money? | Cost | Owned/dumb? |
|---|---|---|---|---|
| **Reachability relay** | SNI/TCP passthrough (WireGuard+nginx / rathole / frp) or Tailscale Funnel | **NO** (ciphertext only) | €4.15/mo or €0 | own VPS, or Funnel (3rd-party but E2E) |
| **Push gateway** | FCM / Apple / ntfy — wake a locked device | **NO** (encrypted wake nudge) | €0 | 3rd-party (unavoidable for push); or self-host ntfy |
| **Static host + TLS (LE)** | storefront HTML + WASM kernel bundle; DNS-01 certs | **NO** | €0–few | CDN or the relay |
| **Backup blob store** | Litestream target (MinIO / B2) | **NO** | ~€0–5 | own MinIO or dumb S3 |
| **Fiscal CIS** (Albania) | government tax endpoint | reports invoices (legally mandated; node-only, 48h-offline-tolerant) | included in fiscal sw | external, unavoidable, offline-first |

**This is the honest meaning of "no single server that processes everything":** every box above is a
door or a legally-mandated external endpoint; the *only* thing that decides, prices, sequences, or holds
the money log is the vendor's own device. That is "maximally local" made real.

---

## 6. Endgame: from one node to "one node among many" (gated, not built)

The hybrid topology is not a dead-end compromise — it is the first rung of the decentralized protocol,
reachable without a rewrite (consistent with DECISIONS D2/D6 "seams now, machinery later," VERIFIED, and
UNIFIED v2 §2, VERIFIED):

- **The relay generalizes into a rendezvous/gossip layer.** iroh-gossip is browser-compatible and
  1.0-stable (Lane-reachability, VERIFIED); the same dumb relay that reaches one node can later route
  discovery among many nodes.
- **The network adds courier liquidity + cross-node trust, never custody.** A vendor short on couriers
  can borrow from a pooled matcher; the matcher is a **pure replicable function** (any node runs it,
  results signed & verifiable — `matcher.rs` is test-proven, F3 §0 VERIFIED), never a hosted server
  (DANGER #1). Ownership stays per-vendor; only dispatch liquidity pools (UNIFIED v2 §2, VERIFIED).
- **The five re-centralization traps are named and mapped** (`PROTOCOL-CENTRALIZATION-MAP.md`, VERIFIED):
  matcher/sequencer (#1), SDK/access layer (#2), settlement oracle (#3), identity root (#4), liquidity
  (#5). The single genuine one today is **#2 the access layer** — which is *this lens's relay + client*.
  The mitigation is doctrinal and must be honored from day one: **the relay is a thin dumb pipe and the
  client contract is open**, so no one is ever hostage to one door (ship a reference alt-client to prove
  it — F3 DANGER #2, VERIFIED).

**Hard gate (carry, do not re-litigate):** every serious source in both repos warns that decentralization
theater kills projects — token-first logistics plays are all dead/zombie (`web3-logistics-postmortem.md`,
VERIFIED); ONDC lost 35% of food orders when it split accountability (Lane-domain). The network layer is
**hard-gated behind a validated MVP** (DECISIONS D6; `D-transition-blueprint.md` Phases 4–5, VERIFIED).
The success metric is not architectural elegance — it is **one real order from a non-operator customer on
a claimed venue** (hub review §0 / D-blueprint Phase 0, VERIFIED). Build the hybrid; earn the mesh.

---

## 7. Risks, open questions, and what would falsify this design

| # | Risk / open question | Severity | Note / who owns it |
|---|---|---|---|
| 1 | **Kernel bypass**: Rust checkout doesn't go through `decide` today (no `Command::PlaceOrder`). | **Blocking** | The whole "node decides" premise fails until closed. Phase 1 of `D-transition-blueprint.md`. VERIFIED-in-doc. |
| 2 | **bebop2 crypto not ready**: non-FIPS-interop, timing leaks, wasm32 gate fails (~94 errors), Ed25519 perf hang. | High | Use host-crate Ed25519 + Tier-1 bebop2 until audited. Do NOT let bebop2 solely guard money/identity. G09 tiering. VERIFIED-in-doc. |
| 3 | **Non-technical vendor runs a node + relay**. | High | Ship an appliance (N100 mini-PC preferred: SSD durability, €130–160; Pi 5 4GB ~€85 + read-only overlayfs + A/B; Android phone as fallback). Auto-update via Minisign-signed static feed (Tauri-updater pattern — dumb static host). Lane-packaging, VERIFIED. UX benchmark: a €84/yr fiscal box. |
| 4 | **iOS is a second-class citizen** on every axis (Web Push needs A2HS, no certhash, OPFS fragile, WebTransport no certhash). | Medium | Customers on iOS use plain WSS + SMS/WhatsApp confirm (works). Couriers on iOS are the weak case — prefer Android couriers, or native wrapper. Lane-reachability/packaging, VERIFIED. |
| 5 | **Relay is a reachability SPOF** (if it's down, new customer orders can't arrive). | Medium | Cheap to run 2 relays / add Funnel as backup; node keeps operating offline for in-flight orders. Not a *processing* SPOF (no data at risk). DESIGN JUDGMENT. |
| 6 | **Push centralization** (FCM/Apple) is unavoidable for waking locked phones. | Low–Med | Accept it as a dumb encrypted nudge; self-host ntfy/UnifiedPush for employees to reduce it. Telegram long-poll is a CGNAT-friendly alternative. Lane-reachability, VERIFIED. |
| 7 | **Concurrent multi-writer edits** of the same entity (e.g. two devices editing a menu) need CRDT merge. | Low (MVP) | Menus are single-writer (owner) in MVP; introduce Loro/Automerge only for genuinely convergent data, never money/status. DECISIONS D2 defers CRDT. VERIFIED. |
| 8 | **Fiscal CIS outages** (e-Albania has had multi-day outages). | Low (legally handled) | The 48h offline grace covers it; queue-and-drain mirrors the law. Node-only. Lane-domain, VERIFIED/UNVERIFIED. |
| 9 | **Cellular IPv6 could one day enable direct dial** (39% AL, rising). | Opportunity | Treat any direct-dial success as an optimization over the relay, never the required path. Lane-reachability, UNVERIFIED. |

**Falsification (Verified-by-Math discipline).** This design is falsified if any of the following RED
cases fire in a build: (a) an order write path exists outside `kernel::decide` (grep-gate); (b) a
`fold(events)` on the SQLite log ≠ the projection state (byte-compare oracle); (c) the relay can read
order plaintext (packet capture shows cleartext = TLS is terminating in the wrong place); (d) a
relay-injected event without a valid signature is accepted by the node (forge drill); (e) the node
cannot complete an in-flight order lifecycle with the relay killed (kill-the-relay drill); (f) money
totals on the device ≠ the pricing oracle (no-money-invented drill). These map 1:1 to the RED cases in
`D-transition-blueprint.md` Phases 1–5 — reconcile there for sequencing.

---

## 8. One-paragraph handoff

Adopt topology **(c): per-vendor sovereign node + dumb stateless relay.** The vendor's device runs the
deterministic Rust kernel (`decide`/`fold`/`replay`, `Lek(i64)`, 10-status machine) as the **single
writer** for its venue's orders over an append-only SQLite event log with bebop2 canonical-bytes +
signature-slot envelopes; the same kernel compiles to WASM for the courier PWA and the customer page
(optimistic, rebased on the node's sequenced heads). The one always-on box in the middle is a **dumb
SNI-passthrough relay** (own €4/mo VPS, or free Tailscale Funnel) that terminates no TLS and processes
nothing — the only way a one-shot mobile-web customer behind cellular CGNAT can reach the node, since a
browser cannot peer and hole-punching cannot reach a CGNAT'd server. Couriers get iroh (hole-punch +
relay fallback) plus a dumb push gateway to wake locked phones. **COD makes money integrity tractable**
— the system never moves money, only keeps double-entry books of obligations settled by counter-signed
custody hand-offs, with the vendor↔courier employment relationship as arbiter and the legally-mandated
NIVF fiscal receipt (node-only, 48h-offline-tolerant) as free proof-of-sale. This is "local
decentralized" honestly defined: the kernel lives on devices; every surviving server is demoted to a
door. Everything decentralization-network-shaped (open matcher, courier pooling, PQ signatures, mesh
sync) is a seam baked in now and hard-gated behind one real order from one real venue.
