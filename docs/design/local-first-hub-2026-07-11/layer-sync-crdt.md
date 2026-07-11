# Layer SYNC/CRDT — the commutative-data subset beside the single-writer core

> **Layer blueprint, 2026-07-11 (late evening).** Detailed design for the SYNC/CRDT layer: the
> eventually-consistent, commutative-data subset (menu structure, presence, telemetry, chat) that
> merges freely across the venue's own devices — sitting BESIDE, and explicitly fenced from, the
> single-writer money/order core (`kernel::decide`, the sequencer of Phase R in
> `05-protocol-tech-completion-blueprint.md`). Zero code written; both repos read-only; this file
> is the only artifact created.
>
> Labels: **VERIFIED** (primary source fetched/checked this session — `file:line` or URL),
> **VERIFIED-in-repo-doc** (carried from a sibling lens that verified it, cited),
> **UNVERIFIED** (secondary/inference, flagged), **DESIGN-JUDGMENT** (my call, reasoned from
> verified facts).
>
> Standing decisions honored (binding, not re-litigated): local-first; **COD**; **NO courier
> scoring** (presence ≠ scoring — enforced structurally below); anonymity (E2EE per stream,
> crypto-shred, minimal retention); multichannel / no dedicated app (the customer is a one-shot
> keyless browser and **never participates in any CRDT**); **crypto HYBRID-ONLY** (Ed25519
> audited half mandatory, ML-DSA additive); storefront sovereignty (customers only ever see what
> the vendor signed); integer money `Lek(i64)`.
>
> Ground truth read this session: `B-data-sync.md` (the commutative-vs-consensus partition, §2),
> `02-local-first-architecture.md` (vendor node = single-writer sequencer; CRDT lane confined to
> menus/GPS/chat, §3.4), `C-runtime-transport-identity.md` (iroh transport + capability identity),
> `05-protocol-tech-completion-blueprint.md` (Phases W/S/R/A this layer composes with);
> dowiz schema: `packages/db/migrations/1780310072731_menu.ts` (products.price `integer CHECK
> (price >= 0)`), `1780338982010_menu_modifiers.ts` (modifiers.price_delta integer),
> `1780338982018_menu_versions_table.ts` + `1780338982020/21` (menu_versions bump trigger);
> domain: `rebuild/crates/domain/src/kernel/pricing.rs` (integer-only pricing, f64 banned),
> `money.rs` (`Lek(i64)`, checked arithmetic).

---

## 0. The one-paragraph position

**Only four data families are CRDT-merged — menu STRUCTURE, presence, telemetry, chat — and all
merging happens inside one trust domain (the venue's own enrolled devices), never across trust
domains.** Menu **prices** (and every other value `compose_total`/`decide` consults) are ruled
OUT of the CRDT lane: they travel the vendor-signed single-writer command path, exactly like
orders. The engine is a **hand-rolled delta-state LWW-map + OR-set (~small, borsh-canonical,
per-delta signed)** — not Automerge/Yjs/Loro, all of which are alive in 2026 but solve a richer
problem (collaborative text) than this subset has, and whose opaque binary changesets defeat the
one property this layer cannot lose: **a typed, signable, field-fenced delta in which a
price-touching mutation is a match-arm rejection, not a policy hope.** Anti-entropy is pairwise
delta-interval exchange (Almeida/Shoker/Baquero) hub-and-spoke over the vendor node — the only
always-on device — riding Phase W frames over iroh/WSS; iroh-gossip is the later multi-venue
rung, not the venue-local mechanism. Everything is bounded: causal contexts instead of
tombstones, TTL'd presence, aggregate-then-discard telemetry, and vendor-signed menu snapshots
that double as CRDT compaction points.

---

## 1. The partition, made precise — and the price hazard ruling

### 1.1 The invariant (the fence, stated once)

> **Nothing that affects money, order acceptance, or dispatch assignment is ever CRDT-merged.**
>
> Operationally: (a) `kernel::decide`, `bebop-settle`, and `dispatch.rs` never read a value that
> arrived by merge — they read only the vendor node's own projection, which the node updates
> exclusively from its own signed single-writer streams; (b) the `sync-crdt` crate is not a
> dependency of `domain`, `bebop-settle`, or the dispatch module — **CI-gated by a grep/Cargo.lock
> assertion that can go RED** (Phase Y0); (c) a CRDT delta that names a fenced field is rejected
> at decode time, before any merge function runs.

This is the CALM-theorem boundary from `B-data-sync.md` §2.1 (coordination-free iff monotone —
VERIFIED-in-repo-doc, VLDB'23) turned into a compile-time + CI fence rather than prose.

### 1.2 The entity partition (dowiz tables, exact)

Class key — **CRDT** = merge freely inside the venue trust domain · **SW** = vendor-node-signed
single-writer stream (Phase R sequencer) · **fence** = consulted by money/order/dispatch code,
therefore SW by invariant 1.1.

| Family | dowiz entities (from `packages/db/migrations`) | Class | Merge rule / authority |
|---|---|---|---|
| **Menu structure** | `categories(name, sort_order)`, `products(name, description, image_url, sort_order, is_available)`, `modifier_groups(name, min_select, max_select, required)`, `modifiers(name, sort_order, available)`, `product_modifier_groups(sort_order)`, `product_media`, `product_translations`, `category_translations`, `content_i18n`, `menu_schedules`, `location_themes`/`theme_versions` (branding), `sales_channels` (labels/order only) | **CRDT** | LWW-map per entity keyed by HLC (§3.3), actors = the vendor's enrolled devices only; entity add/remove = OR-set with causal context. Merged **on the node**, then re-published as a **signed snapshot** (§1.4) |
| **Menu money** | `products.price` (`integer CHECK (price >= 0)` — VERIFIED `1780310072731_menu.ts:25`), `modifiers.price_delta` (VERIFIED `1780338982010:20`), `delivery_tiers` (fee schedule → `resolve_delivery_fee`), promotions/discount definitions, tax/commerce config (`price_includes_tax`, rate — feeds `apply_tax`, VERIFIED `kernel/pricing.rs:61-96`), `exchange_rates` cache | **fence → SW** | `Command::SetPrice`-class commands → `decide` on the vendor node → signed `MenuPriceSet` event in the venue config stream. **Never in a CRDT delta** (Y1 RED case) |
| **Presence** | `courier_positions`, `ops_heartbeat`, `location_alerts` (open/closed/busy) | **CRDT** | LWW-register per (device, kind), HLC-ordered, **TTL'd** (§3.4); courier position authored only by that courier's key. **Presence ≠ scoring** — see §1.5 |
| **Telemetry** | `analytics_events`, `analytics_cwv`, `funnel_events`, `delivery_trace`, `order_sensor_events`, per-venue `velocity_events` | **CRDT** | Grow-only append set (G-Set), any enrolled writer, eventual; **aggregate-then-discard** compaction (§3.4). Per-venue only — cross-venue aggregation stays dead (B-lens class E) |
| **Chat** | `order_messages` | **CRDT** | Per-order causal append-only log; each participant signs own messages; merge = union, displayed in (HLC, author-id) order; bounded by order lifecycle + crypto-shred (anonymity ruling, doc 03/04) |
| **Orders / dispatch / money** | `orders*`, `order_items*`, `order_status_history`, `courier_assignments`, `courier_dispatch_queue`, `courier_shifts`, `reservations`, `payments*`, `courier_cash_ledger`, `settlement*`, `money_breakdown`, `idempotency_keys` | **fence → SW** | Unchanged from B-lens §2.3 classes B/C — the Phase R sequencer + Phase X co-signed settlement. **This layer adds nothing to them and takes nothing from them.** |

**Two deliberate sharpenings of the B-lens §2.3 table** (which listed `delivery_tiers` and
`promotions` under class A "menu/catalog"): both are consulted by the money composition
(`resolve_delivery_fee`, the `− discount_total` seam — VERIFIED `kernel/pricing.rs` module doc),
so under invariant 1.1 they move to the fenced single-writer stream. Menu *availability*
(`is_available`, stop-list) stays CRDT: it is consulted at accept time, but only via the node's
own projection, so a late-merging availability edit produces **staleness** (an order the vendor
manually rejects), never an invariant violation — the same failure mode the current
centralized system already has between edit and cache-bust.

### 1.3 The price hazard, examined and ruled

Why price is not "just another LWW field," even though the money path re-prices server-side:

1. **The money path is already safe by construction** — the customer cart intent carries no
   prices, and `decide` prices in-transaction from the node's projection
   (`02-local-first-architecture.md` §4.6, VERIFIED-in-repo-doc). A merged price would not let a
   customer *set* a price.
2. **But an LWW merge can silently resurrect a stale price.** Two vendor devices offline; device
   A raised the price Tuesday, device B's clock is skewed 2h fast and touches the same row
   Wednesday with Monday's price still cached → LWW picks B → the venue sells at the old price
   with **no signature, no audit event, no intent**. That is a real-money loss with no
   accountable act — precisely what the signed event log exists to prevent.
3. **Auditability is a sovereignty requirement.** The vendor must be able to prove which price
   was in force when order N was accepted (dispute evidence, fiscal reporting). An LWW register
   has no "in force" — it has "whatever merged." A signed `MenuPriceSet{product_id, price:
   Lek(i64), effective_from}` event in a totally-ordered stream does.

**RULING (DESIGN-JUDGMENT, the position this layer takes): the menu is split at the field level,
inside a two-layer design.**

- **Layer 1 — the published menu is ALWAYS a vendor-node-signed snapshot** (§1.4). Customers,
  couriers, and `decide` itself only ever consume signed snapshots. This holds regardless of how
  editing happens, and is the storefront-sovereignty guarantee: no relay, peer, or library
  artifact can alter what the venue appears to sell.
- **Layer 2 — editing.** Non-price structure (the CRDT column of §1.2) merges freely among the
  vendor's own enrolled devices, so the owner's phone and the kitchen tablet can both edit
  offline and converge. Price-class fields are **excluded from the delta vocabulary itself**: the
  `MenuDelta` enum has no variant that can carry them, and a forged/hand-crafted delta naming a
  fenced field fails decode → rejected → RED-tested (Phase Y1). Price changes take
  `Command::SetPrice` → node `decide` → signed event → node re-publishes a new signed snapshot.

The alternative ruled out — "whole menu single-writer, no menu CRDT at all" — is the correct
**MVP simplification** (02 §7 risk 7 already pins MVP menus single-writer, VERIFIED-in-repo-doc)
and Phase Y3 is accordingly last and optional-until-needed. But as the *layer design* it is
inferior: multi-device menu editing is the one real multi-writer need a venue has (owner at home,
manager in the kitchen), and forcing every sort-order tweak through the sequencer couples menu
UX latency to the money core for no invariant gain. The split keeps the sequencer's write load =
money-relevant facts only.

### 1.4 The signed menu snapshot (what replaces `menu_versions`)

Today: `menu_versions(location_id, version bigint)` bumped by a SECURITY DEFINER trigger on
every menu-table write (VERIFIED `1780338982018/20/21`) — a cache-bust integer. Its local-first
successor:

```
MenuSnapshot {
  venue_id,
  version: u64,              // monotone, node-assigned
  content_hash: [u8;32],     // H(canonical bytes of the full published menu doc)
  merged_upto: HLC,          // CRDT frontier this snapshot folded in (the compaction point, §3.4)
  price_head: (seq, hash),   // head of the venue config stream whose prices this snapshot embeds
  sig[]                      // hybrid: Ed25519 mandatory, ML-DSA additive (Phase A rule)
}
```

Published as an event in the venue's own stream; served to storefront/browser with the full doc;
verified client-side by the WASM kernel before render. The snapshot **binds structure-merge state
and price-stream head together**, so a snapshot can never mix "new structure, stale prices"
undetectably. `upsert_menu_version`/the trigger die; re-publication after each applied edit batch
replaces them.

### 1.5 Presence ≠ scoring (the NO-courier-scoring red line, made structural)

- Courier position is an **operational LWW register with a TTL**, not a history: retention =
  active assignment + a short dispute window (default 72h, venue-configurable downward), then
  dropped/crypto-shredded with the order's PII envelope (doc 04 ruling).
- **No module may compute per-courier aggregates from presence or telemetry** (rating, speed
  score, acceptance rate). Enforced the same way Phase X enforces no-reputation: a CI test
  asserts the sync/presence crates contain no courier-keyed aggregation and `dispatch.rs` has no
  presence-history input (it may read *current* position for offer routing only — that is
  dispatch's legitimate operational need, not scoring).
- Customer-visible courier position during an active delivery is **coarsened** (rounded grid /
  ETA band) per the anonymity rulings; raw fixes stay inside the venue trust domain, E2EE.

---

## 2. Engine choice

### 2.1 The 2026 field, verified this session

| Engine | 2026 state (checked 2026-07-11) | Rust story | Verdict here |
|---|---|---|---|
| **Automerge 3** | Alive and funded: Automerge 3.0 released 2025, memory cut >10× ("Moby Dick: 700 MB in v2 → 1.3 MB in v3"; a 17-hour-load doc → 9 s) — **VERIFIED** [automerge.org/blog/automerge-3](https://automerge.org/blog/automerge-3/). Rust crate `automerge` 0.10.0, updated **2026-06-05**, 411K downloads — **VERIFIED** crates.io API. Maintainer full-time at Ink & Switch (UNVERIFIED, repo README via search). WASM ≈320 KB (VERIFIED-in-repo-doc, 02 §3.3). Beelay/Keyhive sync+auth incoming (VERIFIED-in-repo-doc, B-lens §3.1) | First-class (it IS a Rust core) | **The escape hatch, not the engine** (§2.2). Adopt per-document only if rich-text menu descriptions ever matter |
| **Yjs / yrs** | Alive: `yrs` 0.27.2 updated **2026-06-12**, 1.94M downloads — **VERIFIED** crates.io API; y-crdt tracks Yjs binary compatibility (VERIFIED repo README via search). JS Yjs ~920K weekly downloads (UNVERIFIED secondary, B-lens §3.1) | Port of a JS-first design; ecosystem (providers, awareness) is JS-shaped | Rejected: same objection as Automerge (opaque changesets) plus a JS-centric ecosystem the stack doesn't share |
| **Loro** | Alive: `loro` 1.13.6 updated **2026-06-21**, 315K downloads — **VERIFIED** crates.io API; youngest ecosystem (B-lens §3.1) | Rust-native | Rejected for MVP: same objection, smallest track record; its own docs warn off CRDTs for invariant data (VERIFIED-in-repo-doc, B-lens §2.1) |
| **cr-sqlite / vlcn** | **Dormant, re-verified at the primary**: latest release v0.16.3 published **2024-01-17** — **VERIFIED** GitHub API this session (`releases/latest.published_at = 2024-01-17T17:42:28Z`), ~30 months stale. (A secondary page-summary this session misread it as 2025-01 — the API date is authoritative.) | C extension + Rust | **Rejected, reaffirmed** (B-lens §1.1). Do not build on it; the CRR *idea* survives in our schema-shaped deltas |
| **Hand-rolled delta-state LWW-map + OR-set** | The literature is settled and stable: delta-state CRDTs, delta-interval anti-entropy, causal-context (dot) semantics — Almeida, Shoker, Baquero — **VERIFIED** [arXiv:1603.01529](https://arxiv.org/pdf/1603.01529) | ~300–500 LoC in our own crate, borsh-canonical, no new deps in core | **CHOSEN** (§2.2) |

### 2.2 Decision: hand-rolled delta-state LWW/OR-Set (`sync-crdt` crate) — DESIGN-JUDGMENT

Four reasons, in order of weight:

1. **The delta must be a typed, signed, field-fenced protocol object.** Every CRDT delta on this
   wire is (a) signed by an enrolled device key under the Phase-A hybrid rule, (b) scoped by a
   capability (`venue:<id>:menu-edit`, `venue:<id>:presence`), and (c) **structurally unable to
   name a fenced field** (§1.3). Automerge/Yjs/Loro changesets are opaque library-internal binary
   formats: you can sign the blob, but attributing and *filtering* per field means parsing the
   library's change encoding — fragile, version-coupled, and un-RED-testable at the type level. A
   hand-rolled `MenuDelta`/`PresenceDelta` enum makes the fence a match arm.
2. **The problem is small after the price ruling.** What remains commutative is: LWW on scalar
   fields, add/remove of keyed entities (OR-set), grow-only logs. No collaborative text, no list
   splicing, no rich-tree moves — the only workloads that justify a full CRDT engine's
   complexity. Loro's and Automerge's own guidance agrees CRDTs earn their keep on text/JSON
   trees, not on LWW scalars.
3. **One canonical-bytes world.** Phase W standardizes borsh-canonical signed frames
   (`05-protocol...` Phase W, VERIFIED-in-repo-doc). A library engine imports a second
   serialization + storage format that must be wrapped, hashed, and versioned separately. The
   hand-rolled deltas ARE Phase-W frames (two new frame types, §3.2) — no second world.
4. **Dependency posture.** `sync-crdt` is a shell-side crate (never inside `domain`/wasm32-gated
   core); still, zero new transitive trees beat three. All three libraries are alive (verified
   above) — this is not a maintenance rejection; it is a fit rejection. cr-sqlite alone is
   rejected on maintenance.

**The escape hatch, named now so it never becomes a rewrite:** if menu descriptions ever need
collaborative rich text, embed **one Automerge 3 document per description field as an opaque CRDT
register value** inside the existing LWW-map (the map's value type is already `bytes`; Automerge's
merge becomes that register's join). Automerge 3 is the designated library because it is the
liveliest Rust-core option (verified above) and its Beelay/Keyhive direction matches the
capability model. This composes without touching the fence: descriptions are structure, never
money.

**What the customer runs: nothing.** The one-shot browser verifies a `MenuSnapshot` signature and
renders it; it holds no CRDT state, authors no deltas, carries no keys (multichannel/no-app
ruling). CRDT participation is exactly: vendor node + vendor's enrolled edit devices + courier
PWA (presence/chat lanes only).

---

## 3. Anti-entropy gossip over the transport

### 3.1 Topology: pairwise delta-interval anti-entropy, hub-and-spoke — not venue-local gossip

The venue trust domain is 2–15 devices (owner phone, kitchen tablet, warm-spare box, couriers on
shift), of which **exactly one is always-on** — the vendor node (C-lens §1.3/§1.4: phones are
intermittent, push-woken; VERIFIED-in-repo-doc). Epidemic gossip protocols buy convergence when
there are many peers and no hub; our physics already provides a hub. So:

- **Every device syncs pairwise with the vendor node** (and opportunistically with any directly
  reachable peer on LAN — the algorithm is symmetric, the topology is just who's reachable).
- The exchange is **delta-interval anti-entropy** (Almeida/Shoker/Baquero, VERIFIED
  [arXiv:1603.01529](https://arxiv.org/pdf/1603.01529)): each replica keeps, per peer, the ack
  frontier; on contact it ships the join of deltas the peer hasn't acked (a *delta-interval*);
  the paper's causal-delta-merging condition is what makes this deliver causal consistency
  without shipping full state. Joins are idempotent, commutative, associative — **conflict-free
  by construction**; duplication and reordering are absorbed by the join, so the transport needs
  no ordering or exactly-once guarantees.
- **iroh-gossip is deliberately NOT used venue-locally.** It is alive and healthy (0.101.0,
  updated 2026-06-15 — VERIFIED crates.io; HyParView membership + PlumTree epidemic broadcast
  trees, 32-byte topic ids, IO-free `proto` state machine — VERIFIED
  [docs.rs/iroh-gossip](https://docs.rs/iroh-gossip/latest/iroh_gossip/)), and it is the named
  mechanism for the **later multi-venue rung** (cross-venue presence/blocklist feeds, mesh
  discovery — 02 §6's "relay generalizes into rendezvous/gossip", iroh-gossip
  browser-compatibility VERIFIED-in-repo-doc 02 §4.2). Pulling swarm membership machinery into a
  5-device star today is complexity without a payer. The seam is kept: `DeltaTransport` is a
  trait; `iroh-gossip` becomes a second impl when venues federate.

### 3.2 Wire integration (rides Phase W, adds two frame types)

Extend the Phase-W `frame_type` enum: **`CRDT_DELTA`** and **`CRDT_ACK`** (`SYNC_REQ`/`SYNC_BATCH`
stay order-stream-only — different semantics: total order vs join). Frame body (borsh-canonical,
signed header‖body like every Phase-W frame, hybrid signature rule from Phase A):

```
CrdtDelta {
  lane: u8,                        // 1=menu-structure 2=presence 3=telemetry 4=chat
  venue_id,
  author: device_id,               // must match an enrolled key with the lane's capability
  dots: [(actor, from_seq, to_seq)],  // the delta-interval this frame covers
  payload: Vec<LaneDelta>,         // typed per lane — MenuDelta | PresenceDelta | ...
}
CrdtAck { lane, venue_id, frontier: VersionVector }
```

- `content_hash = SHA-256(body)` dedups replayed frames for free (same rule as the ledger and
  mesh content-addressing — VERIFIED-in-repo-doc B-lens §2.2).
- **Admission = capability present; per-frame check = signature + capability `exp` + lane match**
  — the ADR-0013 tri-state guard semantics carried over (C-lens §3.3, VERIFIED-in-repo-doc):
  unverifiable ⇒ withhold, never merge-then-check. An unknown actor's delta is dropped *before*
  it can inflate any causal context (this is also the state-growth bound, §3.4).
- Customer leg: none. Deltas never transit the customer WSS channel; the storefront gets
  snapshots (§1.4).

### 3.3 Clocks: HLC for LWW, dots for membership

- LWW ordering key = **Hybrid Logical Clock** `(physical_ms, logical, actor_id)` — monotone,
  causality-preserving, 64-bit + actor tiebreak (Kulkarni/Demirbas et al., "Logical Physical
  Clocks", VERIFIED [cse.buffalo.edu/tech-reports/2014-04.pdf](https://cse.buffalo.edu/tech-reports/2014-04.pdf)).
  Deterministic total order ⇒ two replicas that have seen the same deltas hold byte-identical
  state (the Y1 proof hinges on this).
- **Bounded skew acceptance**: a delta whose HLC physical component is > W ahead of the
  receiver's clock (W ≈ 5 min) is rejected, bounding the "clock-skewed device wins forever"
  failure — consistent with C-lens §4.3's bounded-acceptance rule. Note this exact hazard is why
  prices are fenced (§1.3): for structure fields a skew-won LWW is a cosmetic annoyance the next
  edit fixes; for money it would be a silent loss.
- OR-set element identity = **dots** `(actor, seq)` with a per-lane **causal context**, per the
  delta-CRDT literature: remove = drop the dot from the entry set while the causal context
  remembers it — **no per-element tombstones**; the causal context compresses to a version
  vector + small dot-cloud (VERIFIED [arXiv:1603.01529](https://arxiv.org/pdf/1603.01529) §"causal
  context"; the tombstone-free property is the reason to use dots at all).

### 3.4 Bounded state growth (the honesty section)

| Lane | Growth vector | Bound |
|---|---|---|
| Menu structure | causal context per actor; dead entity dots | Actor set = enrolled devices only (capability-gated, revocation removes the actor); **compaction = the signed MenuSnapshot** (§1.4): `merged_upto` is a frontier — deltas ≤ frontier are discarded on every device; a device returning from beyond the horizon does state-reset-to-snapshot + replays only newer deltas. Snapshot cadence: every applied edit batch (natural) with a size/age floor |
| Presence | one register per (device, kind) — O(roster) | TTL expiry is a *local* rule, not a merge: an expired register is dropped everywhere without coordination (expiry-by-timeout is monotone). Position history: retention window then crypto-shred (§1.5) |
| Telemetry | unbounded append by nature | **Aggregate-then-discard**: the node folds raw events into windowed counters (per-venue, never per-courier) and truncates raw G-Set segments past N days; segments are content-addressed so truncation is a frontier drop, same mechanism as menu compaction |
| Chat | append-only per order | Bounded by the order lifecycle: the log is sealed at order close + dispute window, then crypto-shredded with the order's PII envelope (doc 04 ruling). No cross-order chat state exists |

Revocation note (carried from B-lens §1.3 honestly): a revoked device stops being *accepted* as
fast as the `CAP_REVOKE` frame propagates; deltas it authored before revocation remain merged
(they were legitimate when authored). Ejecting an actor from the causal context is a
snapshot-compaction event, not a merge event.

---

## 4. Phases, with VbM (entry precondition · modules · RED cases · effort · dependencies)

Effort unit: focused sessions (≈ half-day), matching `05-protocol...` conventions. All phases are
**shell/library work — nothing here touches `rebuild/crates/domain`** (the wasm32-gated core
never learns CRDTs exist).

### Phase Y0 — The fence (partition as code, before any CRDT code exists)

- **Entry precondition:** none. Can land today; it hardens the current state (in which the
  correct number of CRDT merges is zero).
- **Modules:** no new crate. (a) `docs/spec/PARTITION-v0.md` — the §1.2 table as the normative
  registry, each entity → class; (b) CI gates: `sync-crdt` (once it exists) absent from the
  dependency graph of `domain`/`bebop-settle`/dispatch (Cargo metadata assertion); grep-gate for
  fenced field names (`price`, `price_delta`, `fee`, `discount`, `rate_micro`) appearing in any
  `*Delta` type; the no-courier-aggregation gate of §1.5.
- **VbM proof:** the gates run green on the current tree. **RED case:** a fixture branch adds
  `sync_crdt` to `domain`'s Cargo.toml → CI RED; a fixture `MenuDelta::SetPrice` variant → grep
  gate RED. (A gate that has never fired is unproven — ship the fixture REDs in the test suite.)
- **Effort:** 1 session. **Depends on:** nothing. **Feeds:** all Y phases; referenced by W/R.

### Phase Y1 — `sync-crdt` crate (the engine: HLC + LWW-map + OR-set + delta-intervals)

- **Entry precondition:** Phase W's `Canonical` encode trait and frame-type registry drafted
  (the crate is pure otherwise; it can develop against `transport-mem`).
- **Modules (new crate `sync-crdt`, shell-side, no_std not required):**
  - `hlc.rs` — HLC generate/receive/compare, skew-window check (§3.3).
  - `dots.rs` — dot, version vector, causal context (compressed VV + dot cloud).
  - `lww.rs` — LWW-register + LWW-map keyed by `(HLC, actor)`; join.
  - `orset.rs` — add-wins OR-set over dots, causal-context removes (tombstone-free).
  - `delta.rs` — `LaneDelta` typed enums per lane; **the fenced-field property lives here as the
    absence of variants**; decode rejects unknown lanes/fields fail-closed.
  - `antientropy.rs` — per-peer ack frontiers, delta-interval assembly, `CrdtDelta`/`CrdtAck`
    handling; `DeltaTransport` trait (impls: `mem` now; iroh/WSS in Y2; iroh-gossip later).
- **VbM proof:** property-based convergence oracle — generate N random concurrent edit histories
  across k simulated actors, deliver with random order/duplication/partition, assert all replicas
  reach one byte-identical state hash; plus the determinism oracle (same delta set, any
  permutation ⇒ same hash). **RED cases:** (a) *two offline menu edits converge
  deterministically* — and the falsifier: a mutation-tested build in which the LWW tiebreak is
  flipped to actor-order-dependent MUST fail the permutation oracle (proves the oracle can
  detect non-commutativity); (b) a hand-crafted delta naming a fenced field (crafted at the
  byte level, since the type can't express it) ⇒ decode error, state unchanged; (c) a delta
  with HLC 10 min in the future ⇒ rejected; (d) an unsigned or PQ-only-signed delta ⇒ rejected
  (hybrid rule, same RED as Phase A).
- **Effort:** 3–5 sessions. **Depends on:** Y0, W (trait only). **Feeds:** Y2, Y3.

### Phase Y2 — Presence, telemetry, chat lanes (the low-risk lanes ship first)

- **Entry precondition:** Y1 green; Phase R node runtime exists at least as the staging binary
  (the node hosts the hub-side of anti-entropy in `node/sync.rs`, extended); Phase A
  capabilities issue `venue:<id>:presence` etc.
- **Modules:** `node/sync_crdt.rs` (hub anti-entropy endpoint, lane routing, TTL sweeper,
  telemetry compactor, retention/crypto-shred hooks); courier PWA: WASM bindings for
  `sync-crdt` lanes 2/4 (presence out, chat in/out); customer WSS: **read-only coarse position
  + chat for the active order only**, delivered as node-signed messages, not deltas (the
  customer is not a CRDT actor).
- **VbM proof:** kill-the-network drill — courier device offline 10 min while sending positions
  and two chat messages; on reconnect, node and a second device converge to identical presence
  registers and an identical chat log order. **RED cases:** (a) a position row older than the
  retention window found on the node's disk after order close + window ⇒ retention sweep is
  broken (scan assertion that can fire); (b) the §1.5 no-scoring gate: a fixture that adds a
  courier-keyed `avg_speed` fold ⇒ CI RED; (c) a presence delta signed by a key without the
  presence capability ⇒ withheld, not merged; (d) chat: a replayed duplicate message frame ⇒
  exactly-once in the folded log (content-hash dedup proven by count).
- **Effort:** 3–4 sessions. **Depends on:** Y1, R (staging), A. **Feeds:** the courier-PWA
  product surface; Y3 (shares the lane plumbing).

### Phase Y3 — The menu lane (structure-CRDT + vendor-signed price path + signed snapshots)

- **Entry precondition:** G11 GREEN posture respected — menus are already single-writer in the
  MVP (02 §7 risk 7); this phase turns ON multi-device structure editing, so it waits until
  (a) the venue config stream (prices) exists in Phase R's vocabulary and (b) a real venue has
  asked for second-device editing OR the owner-app roadmap needs it. Frozen `MenuSnapshot`
  schema (§1.4) is the entry artifact.
- **Modules:** `node/menu_lane.rs` — applies structure joins to the node's menu projection,
  runs `Command::SetPrice`-class money edits through `decide` into the venue config stream,
  publishes `MenuSnapshot` (binding `merged_upto` + `price_head`), serves snapshots to
  storefront; edit-device WASM/UI bindings for `MenuDelta` authoring; snapshot-verify in the
  storefront render path (customer side — verify before render, refuse on bad signature).
- **VbM proof:** the full two-device drill — device A renames a product + re-sorts a category
  offline; device B edits the description + toggles availability offline; both reconnect; node
  merges, republishes; both devices and a fresh storefront fetch show the identical snapshot
  (one content_hash). **RED cases:** (a) **a price change is NEVER silently CRDT-merged** — the
  drill's falsifier: device A attempts a price edit while offline; the edit MUST be queued as a
  `Command::SetPrice` (not a delta), MUST NOT appear in any merged state until the node's
  `decide` has emitted the signed event, and a byte-crafted price-bearing delta injected at the
  node ⇒ rejected with the menu projection provably unchanged (hash-compare before/after);
  (b) a `MenuSnapshot` with structure newer than `price_head` re-signed by a non-node key ⇒
  storefront refuses to render (sovereignty RED); (c) snapshot compaction: a device offline past
  the horizon converges via reset+replay to the same hash (and a corrupted snapshot ⇒ refuses,
  falls back to last-good).
- **Effort:** 4–6 sessions. **Depends on:** Y1, Y2 plumbing, R (venue config stream), A.
  **Feeds:** owner multi-device UX; the Automerge-3 escape hatch slots here later if ever needed.

**Total: ~11–16 sessions**, all shell-side, none blocking the W→S→R→X spine; Y0 can land
immediately and Y1 can run in parallel with Phase S/A as a pure-library lane (same
"library-vs-production-traffic" split as `05-protocol...` §3.2).

---

## 5. Sources

**Local (VERIFIED reads this session):**
`B-data-sync.md` §§1–3 (partition, CALM anchor, cr-sqlite/Automerge liveness prior);
`02-local-first-architecture.md` §§3.4–3.5, 4.2, 6, 7 (single-writer ruling, CRDT lane, iroh, MVP menu single-writer);
`C-runtime-transport-identity.md` §§1.3–1.4, 2, 3.3, 4.3 (device intermittency, iroh recommendation, per-frame capability guard, bounded clock acceptance);
`05-protocol-tech-completion-blueprint.md` Phases W/S/R/A (frames, canonical bytes, hybrid rule, sequencer);
`packages/db/migrations/1780310072731_menu.ts`, `1780338982010_menu_modifiers.ts`, `1780338982018/20/21` (menu schema, integer prices, menu_versions trigger);
`rebuild/crates/domain/src/kernel/pricing.rs`, `money.rs` (integer-only money composition, `Lek(i64)`).

**Web (fetched/checked 2026-07-11):**
[Automerge 3.0 announcement](https://automerge.org/blog/automerge-3/) (VERIFIED: released 2025; >10× memory cut; Moby Dick 700 MB→1.3 MB) ·
crates.io API (VERIFIED, all four): `automerge` 0.10.0 (2026-06-05), `yrs` 0.27.2 (2026-06-12), `loro` 1.13.6 (2026-06-21), `iroh-gossip` 0.101.0 (2026-06-15) ·
[cr-sqlite releases — GitHub API](https://api.github.com/repos/vlcn-io/cr-sqlite/releases/latest) (VERIFIED: v0.16.3 published 2024-01-17; dormant ~30 months) ·
[iroh-gossip docs](https://docs.rs/iroh-gossip/latest/iroh_gossip/) (VERIFIED: HyParView + PlumTree epidemic broadcast trees, 32-byte topics, IO-free proto core) ·
[Almeida, Shoker, Baquero — Delta State Replicated Data Types, arXiv:1603.01529](https://arxiv.org/pdf/1603.01529) (VERIFIED: delta-intervals, causal-delta-merging ⇒ causal consistency, causal contexts replacing tombstones) ·
[Kulkarni, Demirbas et al. — Logical Physical Clocks (HLC)](https://cse.buffalo.edu/tech-reports/2014-04.pdf) (VERIFIED: HLC construction; monotone, 64-bit, causality-preserving) ·
[y-crdt repo](https://github.com/y-crdt/y-crdt) (VERIFIED via search: Rust port, Yjs binary compatibility; 0.27.2 released days before this session) ·
Yjs weekly-download figure and Automerge maintainer staffing: UNVERIFIED (secondary sources), flagged inline.

*Produced 2026-07-11. Read-only session; this file is the only artifact created.*
