# Media/Object Layer + In-App Communication + Granular Agent Autonomy — Synthesis Blueprint

**Status: RESEARCH SYNTHESIS / PLAN — no code written, nothing built yet.**
**Date:** 2026-07-20
**Inputs:** Pass 1 (media/file-storage code audit + external ecosystem research), Pass 2 (SimpleX protocol research + dowiz actor-model code audit), operator directives (three messages, including the load-bearing correction on per-layer/per-feature/per-action agent autonomy).
**Sibling documents:** `docs/design/OFFLINE-RESILIENCE-SYNTHESIS-2026-07-20.md` (offline/CvRDT properties inherited here, not re-derived), `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P43-external-integration-ports.md` (item E-h, SimpleX outbound channel — extended, not duplicated, by this document), the voice-hub blueprint (voice *commands*; distinguished from voice *notes* in §7).

---

## 0. Design filter and scope

Every proposal below is checked against one question: **does a small food-business owner (restaurant/shop) running a digital delivery service actually need this?** The operator's framing is *simple, secure, configurable* — not a general-purpose media CMS, not a general-purpose messenger, not enterprise identity management. Features that fail the filter are cut explicitly in §8, with reasons, rather than silently omitted.

What the operator asked for, restated structurally:

1. **File/media storage and management inside dowiz** — take/store/edit/delete/process/deliver media internally and externally, without switching apps — for menu images, inventory, notes, invoices, marketing/content posting, and chat attachments. Native voice/text input and storage.
2. **Internal user communication** — vendor's team (including kitchen personnel) plus courier/client direct communication, with SimpleX as the researched inspiration.
3. **Granular, owner-controlled agent autonomy** (the twice-corrected, then further refined requirement): every path — media, files, communication, inventory, menu, ordering — must be operable by a human OR by an agent, as symmetric alternatives, and the agentic involvement must be adjustable by the vendor owner at **layer**, **feature**, and **action** granularity — fully agentic, human-only, or selected actions only. This is not "AI-assisted human workflow." It is one capability, two interchangeable actors, with the owner deciding per surface which actor runs it.

This document decides architecture and sequencing. It does **not** decide the items listed in §10 (open operator decisions).

---

## 1. Verified starting inventory (what exists, what does not)

All claims below were confirmed by code reading in the two research passes and re-verified against the live tree during this synthesis.

### 1.1 Already built and proven

- **A deterministic, deduplicating, integrity-checked, crash-atomic content-addressed blob store — ~80% of a media backend.** `kernel/src/chunker.rs` (Buzhash content-defined chunker, pure-std, `sha3_256` block ids, locality-preserving under edits) + `kernel/src/backup.rs` (`BlockStore` trait; `MemStore` and `FileBlockStore` with 65536-way sharded fan-out; crash-atomic writes via tmp-file+fsync+rename; content-address integrity re-check on read; `Manifest { blocks, total_len }`; `BackupOrgan` with a byte-identical `backup`/`restore` round-trip; fail-closed errors). Architecturally this is Git's object store / IPFS / Perkeep. Perkeep's "permanode + claims" layer over immutable content-addressed blobs is the closest prior art for the missing metadata layer (§3).
- **A person-agnostic capability substrate.** `kernel/src/capability_cert.rs`: `SelfSignedRoot` (hybrid Ed25519⊕ML-DSA-65), `CertDelegation` binding a `Scope` to *any* subject keypair — never hardcoded to "the owner" — single-hop, `may_delegate=false`, depth==1. `Scope` is a set of `(Resource, Action)` pairs, UCAN-style attenuable (narrow-only via `is_subset_of`).
- **A closed, pinned, fail-closed `Resource`/`Action` model.** `kernel/src/ports/agent/scope.rs`: `Resource` enum (`Route=0x01 … AgentBridge=0x12, Web=0x13`), explicit pinned discriminants, unknown bytes fail decode; `Action` enum including red-line verbs (`SettlementRecorded`, `Authenticate`, `DeploySecret`, `RunMigration`); `RedLinePolicy::DenyByDefault` with the `discriminants_are_pinned` wire-stability test. Provisional bytes `0x1F`/`0x20` (`AdmitAgent`/`InvokeAgent`) are already claimed past bebop2's `0x18` high-water mark and B2's proposed `0x19–0x1E` block.
- **The ToolPort pattern as the single human/agent entry mechanism.** `agent-facade/src/lib.rs`: `ReadOrderStatusTool` and feature-gated `WebFetchTool`, both implementing `ToolPort` (ToolSpec + ToolScope + fail-closed pre-check). The same code path is callable by a human UI adapter or an agent — this is the mechanism the dual-actor requirement rides on; no new mechanism is invented (§6).
- **Signed, attributed, offline-first event replication.** `kernel/src/event_log.rs` (`MeshEvent.actor_pubkey`, explicitly "identity, NOT a score"; per-actor hash chain) + `kernel/src/mesh_replication.rs` (signed G-Set CvRDT, gossip pull, deterministic merge). Chat-as-events inherits these properties for free (§4.2).
- **Content moderation seam.** `kernel/src/moderation.rs` (P74) — event-log-based flagging of abusive user-generated content, no scoring. This is the natural gate for a content-posting pipeline and chat.
- **SimpleX already on the books as an outbound channel.** `BLUEPRINT-P43-external-integration-ports.md` item E-h specs SimpleX via the `simplex-chat` CLI sidecar over localhost WebSocket with per-order one-time invitations — outbound customer notifications only. `ChannelKind::SimpleX` exists in live code (`kernel/src/ports/owner_surface.rs:103`, `kernel/src/ports/notification.rs:60`).

### 1.2 Genuinely missing (the honest work list)

- **Metadata/naming layer over the blob store.** `Manifest` has no filename, MIME, owner, created-at, alt-text, or transcript fields; manifests are not persisted as addressable objects at all — `FileBlockStore::open` creates a `manifests/` directory that nothing writes into. No cross-manifest refcount/GC. Ingest API takes a whole in-memory `&[u8]` (no streaming).
- **Zero media in the catalog.** `kernel/src/catalog.rs` (`PriceableLeaf`, `CatalogNode { label: String, body }`) — menu items are text-only.
- **Zero agent file/media capability.** No `Media`/`Blob`/`Asset` `Resource` discriminant; no media ToolPort.
- **No in-app chat transport.** `kernel/src/messenger.rs` is deep-link *construction* only (t.me/wa.me/viber href builders).
- **No multi-person-per-hub domain concept.** Owner identity is one keypair (`kernel/src/hub_provisioning.rs` `OwnerId([u8;32])`, "== owner-root NodeId bytes"; `ports/owner_surface.rs`: "the owner root is modeled as a classical RefSigner keypair," every mutation is one `OwnerCapIntent`, "no admin database, no dowiz aggregator"). No `Staff`/`Team`/`Employee`/`Role` types anywhere. "Kitchen" appears only as `catalog::kitchen_tickets` (order routing for KDS fan-out, per `VendorId` — `kernel/src/vendor.rs`, `kernel/src/foodcourt.rs`), never as a person. Customers and couriers are account-less (`CustomerRef` = ChannelKind + free-form peer string + order-refs, "NO durable customer identity (P49 deferral)"; a courier's authority is a delegated cert: `Scope::single(Route, Send)`, `may_delegate=false`, depth 1).
- **Upload plumbing.** `tools/native-spa-server` (axum + tower-http + tokio + rustls) caps request bodies at 64 KiB buffered via `to_bytes` — not wired for multipart or large uploads.
- **The autonomy configuration surface** (§6) — the most cross-cutting new work. Nothing like it exists; it must be designed so that it *is* the capability system, not a second policy engine beside it.

---

## 2. Architecture overview — one object layer, one communication capability, two interchangeable actors

```
                         ┌─────────────────────────────────────────────┐
                         │              OWNER ROOT KEYPAIR             │
                         │  mints narrowed CertDelegations to:         │
                         │   staff keypairs · courier keypairs ·       │
                         │   THE HUB AGENT'S keypair (same mechanism)  │
                         └───────────────┬─────────────────────────────┘
                                         │  Scope = {(Resource, Action)} + caveats
              ┌──────────────────────────┼──────────────────────────┐
              ▼                          ▼                          ▼
   HUMAN ADAPTERS              TOOLPORT PRE-CHECK             AGENT LOOP
   (multipart upload UI,       (fail-closed, identical        (agent-facade tools,
    chat UI, menu editor)       for both actors)               bounded plan→act→observe)
              └──────────────────────────┼──────────────────────────┘
                                         ▼
                  ┌──────────────────────┴───────────────────────┐
                  │                KERNEL CORE                    │
                  │  Media/object layer: chunker → BlockStore →   │
                  │    Manifest-as-object → MediaRecord (§3)      │
                  │  Chat: signed events in event_log,            │
                  │    replicated via mesh_replication (§4)       │
                  │  Orders/money: decide/fold FSM + ledger       │
                  │    (unchanged; red-line)                      │
                  └───────────────────────────────────────────────┘
```

Two structural commitments, both derived from what already exists:

1. **One ingestion/mutation core per capability; thin adapters per actor.** The Pass-1 pattern — "one core (bytes → chunker → BlockStore → manifest+metadata record), three thin adapters (human multipart upload, agent ToolPort call, voice/text capture)" — generalizes to every capability in this document. The human UI adapter and the agent both call the *same* ToolPort; neither is primary. This is already how `native-spa-server` routes human and agent POSTs through one capability-framed handler.
2. **Authority is always a delegated capability checked at the same gate.** An agent operating a path end-to-end holds a `CertDelegation` to *its own keypair* with a scope the owner chose — exactly the mechanism a staff member or courier uses. Symmetric authority, never elevated authority. Red-line resources (`Ledger`, `Auth`, `Secret`, `Migration`) remain `RedLinePolicy::DenyByDefault` for every non-owner actor regardless of configuration (§6.3).

---

## 3. The media/object layer (extend `chunker.rs` + `backup.rs`, Perkeep-style)

### 3.1 What gets built

**M-1. Persist manifests as addressable objects.** Serialize `Manifest` deterministically, store it through the same `BlockStore`, address it by its own sha3-256 hash. The `manifests/` directory `FileBlockStore::open` already creates becomes real. A media object's canonical id = its manifest hash.

**M-2. `MediaRecord` — the Perkeep-style metadata claim.** A new kernel type, following Perkeep's permanode+claims split: the *bytes* are immutable and content-addressed; the *metadata* is a mutable-by-append record referencing the manifest hash. Fields (v1, closed set): `manifest_hash`, `media_class` (see §3.2), `filename`, `mime`, `created_at` (event-log ordinal, not wall clock — the core decision path stays clock-free per MANIFESTO C2), `owner_scope_ref` (which delegation created it), `alt_text: Option<String>`, `transcript_ref: Option<manifest_hash>` (sibling text object for voice notes), `variants: Vec<(VariantKind, manifest_hash)>`, `tombstone: bool`. Each `MediaRecord` mutation is a signed event in the existing event log — attributed (`actor_pubkey`), hash-chained, CvRDT-replicated. "Edit" = append a new claim; "delete" = append a tombstone claim (blocks are reclaimed only by GC, §3.4).

**M-3. `Resource::Media` + one new action.** Add `Media` to the closed `Resource` enum and `Retire` to `Action` (tombstone verb; `Append` = upload/claim, `Read` = fetch, `Retire` = tombstone). Discriminant allocation: propose `Media = 0x21`, `Retire = 0x24` (with `0x22`/`0x23` for chat, §4.4) — past the provisional `0x20` — **flagged as pending the same discriminant-allocation ruling** that governs `0x1F`/`0x20`, with the `discriminants_are_pinned` test updated in the same commit. `Media` is deliberately **not** red-line: no money, no auth, no migration semantics.

**M-4. ToolPorts: `MediaWriteTool`, `MediaReadTool`, `MediaRetireTool`.** Follow `ReadOrderStatusTool`/`WebFetchTool` exactly: ToolSpec + ToolScope + fail-closed pre-check against the caller's delegation. These are the *only* mutation entry points — the human upload UI adapter calls `MediaWriteTool` too (§2, commitment 1).

**M-5. HTTP ingest adapter.** In `native-spa-server`: axum's first-class streaming `Multipart` (avoids buffering whole files), lifting the 64 KiB `to_bytes` cap for the media route only. Streaming ingest into the chunker is additive work (current API is `&[u8]`); v1 may buffer photos (a few MB) in memory honestly, with streaming as the M-phase-2 upgrade — stated as such, not hidden.

**M-6. Serving.** Content-addressing gives strong ETags for free (hash = ETag, immutable caching). Pre-generate a **small fixed variant set at ingest** (thumbnail + web-optimized) rather than on-the-fly resize — deterministic, cache-friendly, and avoids a resize service in the request path. tower-http `ServeDir`'s historical non-support of Range requests must be re-verified against the current version before relying on it for audio playback (voice notes need Range).

**M-7. Image processing, feature-gated per repo discipline.** Behind an off-by-default `media-image` feature (header comment stating pulls + default-graph verification per `kernel/Cargo.toml` convention): `fast_image_resize` (SIMD resize/thumbnail, wasm32-capable) + `zune-jpeg`/`zune-png` (Rust PNG decode measured 1.5–1.8x faster than C libpng) or the `image` crate; `ravif` for pure-Rust AVIF encode if AVIF variants are wanted. **Honest weak spot carried forward from Pass 1:** lossy WebP encode still requires libwebp (C) via the `webp` crate, or the unproven `zenwebp` (~30k LOC, new, unaudited) — decision deferred to the operator (§10.4); v1 serves JPEG/PNG/AVIF variants and skips WebP.

**M-8. Catalog wiring.** Add `media: Vec<MediaRecordRef>` (or `image: Option<MediaRecordRef>` — minimum viable) to `PriceableLeaf`/`CatalogNode`. This is the single highest-value user-visible outcome: menu items get photos.

**M-9. Moderation gate.** Any `MediaRecord` created with `media_class` ∈ {Marketing, ChatAttachment} flows through the existing `moderation.rs` flagging path (event-log based, no-scoring). No new moderation machinery.

### 3.2 `MediaClass` — the feature-granularity anchor

`MediaClass` (closed enum, v1): `MenuPhoto`, `Invoice`, `Note`, `Marketing`, `ChatAttachment`, `VoiceNote`. It serves two masters at once: (a) storage policy (retention, variant set — invoices get no thumbnails, voice notes get no resize), and (b) **the feature-level grain of the autonomy configuration** (§6.2) — "agent may manage menu photos but not invoices" is expressed as a class caveat on the delegation, not as more `Resource` discriminants.

### 3.3 What "process" honestly means in v1

Images: decode, resize to the fixed variant set, re-encode. Audio (voice notes): store as recorded, no transcode; transcript is a sibling text object produced by whatever STT the owner has configured (external tool or agent — the kernel stores, it does not transcribe). **Video: store-and-serve-as-is, zero new heavy dependency** — Pass 1 found no mature pure-Rust transcode path (`video-rs` wraps `ffmpeg-next` wraps FFmpeg C); ffmpeg behind a hard feature gate only if a real need materializes (§8).

### 3.4 Refcount/GC

Cross-manifest block refcounting does not exist. v1 policy: **no GC** — tombstoned records stop being served but blocks stay on disk (a small shop's photo corpus is small; disk is not the bottleneck). Refcount+sweep is a later, self-contained phase (Phase M4, §9) with its own DoD; shipping media does not wait for it.

---

## 4. In-app communication (borrow named SimpleX ideas; keep dowiz's attributed-event spine)

### 4.1 The crux, stated plainly

SimpleX's verified core thesis is the **erasure of persistent identifiers**: unidirectional SMP queues, two one-way queues per conversation with different pairwise addresses at each end (plus an optional notifier address), recipient-created queues, single-use out-of-band invitation/QR bootstrap, relays that cannot link sender to recipient because sent and received traffic share no identifier. dowiz's authority model is the deliberate opposite: `MeshEvent.actor_pubkey` is "identity, NOT a score"; the event log is a per-actor hash chain replicated as a signed G-Set CvRDT. dowiz *wants* attribution because money and order authority need a provable signer. **Wholesale adoption of the no-identifier thesis would fight the kernel's foundation.** The synthesis therefore borrows three *named, specific* SimpleX ideas and rejects the thesis:

1. **One-time invitation / QR out-of-band bootstrap** — connect a customer or courier *without a durable account*. This fits dowiz's existing P49 stance exactly (`CustomerRef` already has no durable customer identity, and already echoes SimpleX's per-contact addressing: a different channel-address per contact).
2. **Per-contact addressing instead of a global directory** — no dowiz-wide user directory, ever.
3. **The PQ-hybrid ratchet as the encryption reference for the customer-facing leg** — SimpleX augments each double-ratchet DH step with PQ KEMs run in parallel (Streamlined NTRU Prime sntrup761), arguing PQ *break-in recovery* / post-compromise security, stronger than Signal's PQXDH (which hardens only initial agreement), without sacrificing deniability. dowiz adopts the *design shape* (hybrid classical+PQ per ratchet step — mirroring the caps' existing Ed25519⊕ML-DSA-65 hybrid) with dowiz's own KAT-gated primitives (`kernel/src/pq/`: X25519+ML-KEM-768 hybrid, no classical-only fallback) — never a stubbed primitive. Whether the ratchet is built in-kernel or delegated to the `simplex-chat` CLI sidecar (P43 E-h's mechanism) is an operator decision (§10.5); both paths are honest.

Two Trail of Bits reviews (2022 implementation assessment, 2024 crypto design review) make SimpleX a genuinely audited reference, but the protocol/server (simplexmq) is **Haskell** (clients Kotlin/Swift), AGPLv3 + trademark restrictions — there is no Rust implementation to vendor. The two integration shapes are: architectural borrowing (build the leg on dowiz primitives) or CLI-sidecar driving (P43 E-h, extended to bidirectional).

### 4.2 Three communication legs, two trust regimes

**Leg A — internal staff chat (attributed regime).** A chat message is **another signed event in the existing content-addressed log**: new event payload kind `ChatMsg { channel, body_text | attachment: MediaRecordRef }`, signed by the sender's keypair (owner, staff, or agent), appended to the per-actor hash chain, replicated by `mesh_replication`. This inherits — *without re-derivation* — the entire offline-first story already designed in `docs/design/OFFLINE-RESILIENCE-SYNTHESIS-2026-07-20.md`: a kitchen tablet that loses connectivity keeps appending chat events locally and CvRDT-merges on reconnect, exactly like orders. Text rides the event payload directly (small); voice notes and files are content-addressed blobs referenced by manifest hash from the event (§3). Group semantics: a staff "channel" is just an event filter over the hub's log — the hub's staff all replicate the same log, so SimpleX's O(N)-pairwise-fanout group problem (and its unshipped super-peer answer) does not arise and is not imported (§8).

**Leg B — hub ↔ customer chat (unlinkability-leaning regime).** Per-order, bootstrap via single-use invitation link/QR embedded in the order confirmation (SimpleX idea 1), per-contact address (idea 2), PQ-hybrid-ratchet-encrypted transport (idea 3). No durable customer identity is created — the channel is scoped to the order and closable after fulfillment. Extends P43 E-h from outbound-notification-only to bidirectional; `ChannelKind::SimpleX` already exists for the sidecar path.

**Leg C — courier ↔ customer (masked, ephemeral).** The delivery-platform pattern from prior art (DoorDash/Uber-style masked relay that closes after delivery, so neither party retains the other's durable contact) combined with the disposable-queue idea: hub mints a per-delivery channel joining the courier's cert-holding keypair to the customer's per-order address; the channel closes at `Delivered` (fold event), retaining the hub-side attributed record (the courier is an attributed actor — their authority is already a delegated cert) while the customer stays account-less.

**Honest caveat inherited from the research:** SimpleX itself acknowledges traffic-correlation attacks via timing/IP as mitigated (different-servers-per-direction, Tor), not solved. dowiz makes no stronger claim for legs B/C.

### 4.3 The internal-note leak pitfall — solved structurally, not by a toggle

Every cited prior-art product (Front, Intercom, Missive, Zendesk Agent Workspace, Crisp, Chatwoot) pairs an internal collaboration layer with the customer conversation, and the canonical documented failure is **"internal note leaks to the customer"** — a mis-set reply/note UI toggle over one shared send path. dowiz applies its existing compile-time discipline instead: internal staff chat and customer-facing chat are **separate `Resource` discriminants with separate scopes** (§4.4). A staff member (or agent) holding `(ChatStaff, Send)` but not `(ChatCustomer, Send)` *structurally cannot* produce a customer-visible message — the fail-closed ToolPort pre-check rejects it before any send path exists, the same way the routing enums omit `Ord`/`PartialOrd` to make a quality-router unrepresentable. The leak-test in Phase C1's DoD (§9) makes this falsifiable.

Consistent with the no-scoring invariant (`no-courier-scoring` CI job): chat carries **no participant ratings, no reputation, no read-receipt-derived scoring** of any kind. Trust remains a signed capability.

### 4.4 New discriminants (complete list this document proposes)

| Addition | Kind | Proposed byte | Purpose |
|---|---|---|---|
| `Media` | `Resource` | `0x21` | blobs/manifests/`MediaRecord` claims |
| `ChatStaff` | `Resource` | `0x22` | internal, attributed (Leg A) |
| `ChatCustomer` | `Resource` | `0x23` | customer/courier-facing (Legs B, C) |
| `Retire` | `Action` | `0x24` | tombstone a `MediaRecord` (and close a chat channel) |

All four are flagged **pending the same discriminant-allocation ruling** as `0x1F`/`0x20` (scope.rs header note), land with `discriminants_are_pinned` updated, and none is red-line. This is the entire enum growth — no `Staff` resource is needed (§5.2 explains why), and no per-media-class discriminants (class is a caveat, §6.2).

---

## 5. The minimum viable staff model

### 5.1 What a small food business actually needs

Two to ten people: the owner, kitchen staff, maybe a counter person. Not an org chart. The capability substrate already supports this shape with **zero rework**: "adding staff" = the owner root mints additional narrowed `CertDelegation`s to additional keypairs — the same operation that already provisions a courier ("duty(deliver)" cert = `Scope::single(Route, Send)`).

### 5.2 New domain surface (all of it)

- **`StaffMember`** (kernel domain type): `{ keypair_pubkey, display_name, role_preset, delegation_ref }`. Created/revoked only by an `OwnerCapIntent` — which means staff management is an **authority change**, i.e., `(Auth, Authenticate)`, i.e., **red-line, deny-by-default, owner-root only**. A structural consequence worth stating: *no agent, under any autonomy configuration, can add staff, mint capabilities, or widen a scope* — not by policy, but because the operation is a red-line resource the autonomy system can never grant (§6.3).
- **Role presets = named `Scope` bundles, not an RBAC system.** v1 vocabulary (default proposal; final vocabulary is operator decision §10.2): **Owner** (root, everything), **Kitchen** (`(Order, Read)` + `(Order, Append)` for status advancement on kitchen-relevant transitions, `(ChatStaff, Send|Read)`, `(Media, Read)`), **Counter/Manager** (Kitchen + `(Menu, Append)` + `(Media, Append|Retire)` for menu-class media + `(ChatCustomer, Send|Read)`). A role is a constructor for a `CertDelegation` — nothing more. No permission inheritance trees, no groups-of-roles, no per-object ACLs (§8: enterprise RBAC is cut).
- **Kitchen scope vs owner scope, concretely:** kitchen never holds `Ledger`, `Auth`, `Secret`, `Migration` (red-line — denied even if a scope tried to name them), never `(ChatCustomer, Send)` by default (leak firewall, §4.3), never `(Menu, Append)` (kitchen cooks the menu; it doesn't edit it). The existing `catalog::kitchen_tickets` order-routing concept is untouched — it remains vendor-level routing; `StaffMember` is people-level identity, and the two compose (a kitchen ticket renders on whichever device a Kitchen-scoped keypair is logged into).

That is the whole staff model. It is deliberately small; anything more fails the §0 filter.

---

## 6. Granular agent autonomy — the configuration surface (cross-cutting, first-class)

This section implements the operator's refined requirement: agentic operation adjustable **per layer, per feature, per action**, each independently settable to fully-agentic / selected-actions-only / human-only — everything decided by the vendor owner.

### 6.1 The core design decision: the capability system IS the configuration store

There is no second policy engine. The **hub agent is a keypair** — provisioned exactly like a staff member — and the owner's autonomy configuration *is* the set of `CertDelegation`s the owner root has minted to that keypair. This maps the three granularities onto existing (or minimally extended) machinery:

| Granularity | Operator example | Mechanism |
|---|---|---|
| **Layer** ("media management as a whole") | agent runs all of media | delegation contains all `(Media, *)` pairs |
| **Feature within a layer** ("menu photos yes, invoices no") | agent manages `MenuPhoto`-class media only | **class caveat** on the delegation (§6.2) — the one genuinely new piece |
| **Action within a feature** ("upload yes, delete no") | `(Media, Append)` granted, `(Media, Retire)` withheld | existing `(Resource, Action)` pair granularity, unchanged |

Properties this buys for free, because they are already properties of the capability system:

- **Symmetry.** Human staff and the agent are configured by the identical mechanism (delegations to keypairs). Neither actor is primary; "agent-operated menu editing" and "counter-person menu editing" are the same grant shape to different keys. The ToolPort pre-check is one code path for both.
- **Attenuation-only.** `Scope::is_subset_of` (narrow-only, UCAN-style) means an autonomy configuration can only ever be a subset of the owner's authority — no grant can exceed the root.
- **Fail-closed.** An action outside the agent's delegation is rejected at the ToolPort pre-check before any effect, same as for an under-scoped staff member.
- **Self-escalation is structurally impossible.** Changing the autonomy configuration = minting/revoking delegations = `OwnerCapIntent` = `(Auth, …)` = red-line. An agent can never widen its own scope, because the widening operation lives behind the one gate no autonomy setting can open.

### 6.2 The one genuinely new mechanism: delegation caveats + the pending-intent flow

Two pieces of the requirement exceed what `(Resource, Action)` pairs express today. Both are named honestly as new work:

**(a) Class caveats (feature granularity).** `Scope` today is a set of `(Resource, Action)` pairs; "menu photos but not invoices" needs a finer grain. Proposal: an optional, attenuation-only caveat field on `CertDelegation` — v1 supports exactly one caveat kind, `media_classes: Option<Set<MediaClass>>` (absent = all classes; present = only listed classes), checked in the ToolPort pre-check (fail-closed: a `MediaWriteTool` call whose record's `media_class` is outside the caveat set is rejected). This is deliberately *not* a general predicate language — one closed caveat kind, exhaustively checkable, extendable later only through the same discriminant-discipline process. The alternative (per-class `Resource` discriminants) was considered and rejected: it bloats the closed enum multiplicatively and entangles wire stability with product vocabulary.

**(b) `PendingIntent` (the confirm-first mode).** The requirement's middle setting — "agent may act, but a human confirms first" — needs a staged-mutation primitive. Proposal: a generic kernel envelope, `PendingIntent { proposed_by, tool_call, expires }`, appended as a signed event by an agent holding a **`Propose`-flavored grant** (modeled as the delegation carrying a `confirm_required: bool` caveat rather than a new Action — keeps the Action enum stable). The proposed mutation has **no effect at fold time** until a matching `IntentApproved` event signed by a keypair holding the *real* grant (owner, or suitably-scoped staff) lands; `IntentRejected` or expiry voids it. This is decide/fold-compatible: proposal and approval are both events; the fold applies the effect only on the approval event. One generic envelope serves every capability — there is no per-domain confirmation machinery.

### 6.3 Composition with `RedLinePolicy::DenyByDefault` — stated without qualification

**Owner-granted agent autonomy on any `(Resource, Action)` pair never grants, implies, or relaxes anything on `Ledger`, `Auth`, `Secret`, or `Migration`. Red-line resources are actionable only by a human actor — a hard, non-configurable ceiling, not a permissive default the owner can override toward agent autonomy.** An agent identity's capability cert is structurally incapable of covering a red-line `Resource`/`Action` pair, and the autonomy configuration surface (§6.5) does not even present red-line actions as a configurable option for agent identities — there is no grant path, not merely a default that is off until toggled. This is unaffected by how permissively the owner configures agent autonomy elsewhere: refunds (`SettlementRecorded`), staff/capability changes (`Authenticate`), secrets, and migrations are owner-root-signed (or human-staff-signed, where the owner has delegated red-line-adjacent authority to a *human*) in every configuration this document defines — never to the hub agent's keypair, under any preset, including the most permissive. The agent operating a path end-to-end passes the exact same capability check a human staff member would — symmetric authority, never elevated authority, and never a red-line authority.

### 6.4 Concrete example configuration (falsifiable, not abstract)

A plausible owner configuration for a small restaurant, showing all three modes across features:

| Action | Capability tuple (+ caveat) | Human-only | Agent-assisted (default) | Agent-run |
|---|---|---|---|---|
| Upload + tag menu photo | `(Media, Append)` ∧ class=`MenuPhoto` | no agent grant | grant with `confirm_required` → `PendingIntent`, owner taps approve | full grant — agent ingests, tags, attaches to `PriceableLeaf` |
| Delete menu photo | `(Media, Retire)` ∧ class=`MenuPhoto` | no agent grant | **no agent grant** (deletes stay human in this preset) | grant with `confirm_required` — even "Agent-run" defaults deletes to confirm-first; owner may override to full |
| Store an invoice | `(Media, Append)` ∧ class=`Invoice` | no agent grant | full grant (low-risk, append-only) | full grant |
| Send customer chat message | `(ChatCustomer, Send)` | no agent grant | grant with `confirm_required` — agent drafts, human sends | full grant (order-status Q&A handled end-to-end by agent) |
| Post in staff chat | `(ChatStaff, Send)` | no agent grant | full grant (agent posts prep summaries) | full grant |
| Update menu item / inventory count | `(Menu, Append)` | no agent grant | grant with `confirm_required` | full grant |
| Advance order status | `(Order, Append)` | no agent grant | full grant (FSM forbids illegal transitions regardless of actor — `order_machine.rs` errors, not no-ops) | full grant |
| **Process a refund** | `(Ledger, SettlementRecorded)` | **human only (owner or delegated human staff)** | **human only — not configurable for the agent identity** | **human only — red-line, no grant path exists for an agent keypair** |
| **Add/remove staff, change autonomy config** | `(Auth, Authenticate)` | **owner root only** | **owner root only** | **owner root only — red-line, no grant path exists for an agent keypair** |

Falsifiability of the whole table: each row is a pair of executable tests — (actor holds grant → call succeeds and folds) and (actor lacks grant / caveat excludes / red-line → call rejected at pre-check, no partial effect). Phase A1's DoD (§9) requires exactly these, including a structural test that no delegation constructor can produce an agent-keypair cert naming a red-line pair at all (not just that such a cert would be rejected at use).

### 6.5 Keeping the configuration UX simple (presets over toggles)

The underlying model is fully granular; the owner-facing surface must not be. A small owner sets a posture in one decision, then optionally overrides individual rows:

- **Three presets** (the three columns of §6.4): **Human-only** (zero agent delegations), **Agent-assisted** (agent proposes; human confirms mutations; low-risk appends auto), **Agent-run** (agent operates everything except: red-line always human, deletes default to confirm-first). A preset is nothing but a named generator of a delegation set — selecting one mints/revokes the corresponding certs in a single owner-signed operation. Red-line rows never appear as togglable in any preset's generated table — they are omitted from the agent-configurable surface entirely, not shown-and-disabled.
- **Default posture: Agent-assisted is offered, Human-only is the shipped default.** Conservative by default; autonomy is opt-in per the security framing. No action is ever agent-full without an explicit owner choice (preset selection *is* that choice).
- **Overrides are per-row, discoverable, and few.** The entire toggle surface at maximum granularity is small by construction — the closed `Resource`×`Action` product intersected with what tools exist is a few dozen rows, not hundreds — because the enums are closed. The preset screen shows the §6.4-style table (minus the red-line rows, which render as a fixed, informational "always human" line rather than a toggle) with three-state toggles per remaining row; that table *is* the config UI. Granularity and simplicity are not in tension because granularity lives in the capability system and simplicity lives in the preset generator.
- **Config changes are themselves owner-signed events** — auditable in the same log, replicated the same way, and (per §6.1/§6.3) unreachable by the agent, including for red-line rows which the agent cannot touch even indirectly by requesting a config change (a config-change proposal is itself gated by `(Auth, Authenticate)`).

---

## 7. Voice: notes vs commands (a distinction, recorded so future work doesn't conflate them)

- A **voice note in chat** is a *stored attachment*: audio blob → chunker → BlockStore → `MediaRecord` (class `VoiceNote`) + optional sibling transcript object, referenced by manifest hash from a `ChatMsg` event. It is entirely this document's scope (§3, §4).
- A **voice command to the hub** ("read me today's orders," "86 the salmon") is the *Phase-4 STT → agent-loop pipeline* from the earlier voice-hub blueprint: transcribed speech becomes an agent tool invocation, which then flows through the §6 autonomy configuration like any other agent action.
- Shared substrate, different pipelines: both may store audio via the same media layer, but a note terminates in storage while a command terminates in a ToolPort call. The voice-hub blueprint owns the command pipeline; this document owns the storage substrate it will reuse. Neither blocks the other.

---

## 8. Explicit cuts and deferrals (the §0 filter, applied)

| Cut/deferred | Reason |
|---|---|
| **Video transcoding** | No mature pure-Rust path (`video-rs`→`ffmpeg-next`→FFmpeg C). A small food business needs menu photos and voice notes, not a transcode farm. v1 = store-and-serve-as-is, zero new heavy dep; ffmpeg behind a hard feature gate only on demonstrated need. |
| **SimpleX group super-peer architecture** | Unshipped upstream; and dowiz's Leg-A groups are event-log filters over an already-replicated log, so the O(N) pairwise-fanout problem it solves never arises here. |
| **Enterprise RBAC** (role hierarchies, per-object ACLs, permission inheritance) | A 2–10 person shop needs 3 role presets = 3 scope bundles. Anything more is scope creep against the operator's framing. |
| **Wholesale no-identifier identity model** | Conflicts structurally with `MeshEvent.actor_pubkey` attribution that money/order authority requires (§4.1). Borrowed ideas only. |
| **Global user directory / cross-hub contact discovery** | Violates per-contact addressing (borrowed SimpleX idea 2) and P49; nobody asked for it. |
| **Lossy WebP encode (v1)** | Only paths are a C dep (libwebp via `webp`) or an unaudited ~30k-LOC fork (`zenwebp`). JPEG/PNG/AVIF variants cover the need; decision escalated (§10.4), not smuggled in. |
| **On-the-fly image resizing service** | Fixed variant set at ingest is deterministic and cache-friendly; a resize-per-request path adds a hot dependency for no v1 user benefit. |
| **General caveat/predicate language on delegations** | One closed caveat kind (`media_classes`, §6.2a) is exhaustively checkable; a predicate DSL is unbounded verification surface. |
| **Chat message editing/deletion semantics beyond tombstones** | Append-only log + tombstone display suppression is sufficient and honest; retroactive mutation fights the hash chain. |
| **Read receipts, typing indicators, presence** | Not needed to coordinate a kitchen; presence leaks metadata on the customer leg; and anything aggregatable into a responsiveness *score* brushes the no-scoring invariant. |

---

## 9. Phased build plan (falsifiable DoD per phase; RED→GREEN discipline throughout)

Ordering rationale: the blob store is proven, so the metadata layer (M1) unblocks everything user-visible; chat (C-phases) depends on M1 only for attachments; the autonomy surface (A1) is orthogonal machinery but is sequenced after the first real ToolPorts exist so its tests bind to real tools. Each phase lands with a RED test proving the gap existed, per repo culture ("verified, not claimed").

**Phase M1 — Manifest persistence + `MediaRecord` (kernel only).**
Work: M-1, M-2 (§3.1); `MediaClass` enum; record claims as signed events.
DoD: (1) RED→GREEN: test showing `manifests/` was write-never before, now round-trips manifest-by-hash; (2) store→tombstone→restore property test: bytes identical, tombstoned record excluded from listing, blocks still present; (3) `MediaRecord` claim events replicate and CvRDT-merge under the existing `mesh_replication` test harness; (4) default build stays serde-free/pure-std (`cargo tree -e no-dev | grep -c serde` == 0).

**Phase M2 — `Resource::Media` + ToolPorts + HTTP ingest (the dual-actor spine).**
Work: M-3, M-4, M-5; discriminant additions with `discriminants_are_pinned` updated; `native-spa-server` streaming multipart route calling `MediaWriteTool`.
DoD: (1) same-path proof: one test drives `MediaWriteTool` via the agent call shape, another via the HTTP adapter, asserting both produce byte-identical events through one core; (2) fail-closed: call without `(Media, Append)` grant rejected pre-effect; class-caveat violation rejected pre-effect; (3) RED→GREEN on the 64 KiB cap: multi-MB upload fails before, succeeds after, on the media route only; (4) red-line unaffected: a scope naming `(Ledger, *)` still denied at admission.

**Phase M3 — Variants, serving, catalog images.**
Work: M-6, M-7, M-8; `media-image` feature gate with default-graph verification; hash-as-ETag serving; `PriceableLeaf` media ref.
DoD: (1) ingest of a JPEG deterministically yields the fixed variant set (same input → same variant hashes, twice); (2) ETag equals content hash; conditional GET returns 304; (3) Range-request behavior verified or worked around for audio (the tower-http `ServeDir` question resolved with a test, not an assumption); (4) menu render test: a `PriceableLeaf` with an image ref serves the thumbnail; (5) default build (feature off) compiles with zero of the image crates in the graph.

**Phase C1 — Internal staff chat + staff model (Leg A).**
Work: §5 (`StaffMember`, role presets), `ChatStaff` resource, `ChatMsg` event kind, chat ToolPorts, attachment refs.
DoD: (1) two staff keypairs exchange messages through the event log; offline append + reconnect merge passes under the existing CvRDT harness (inheritance from `OFFLINE-RESILIENCE-SYNTHESIS-2026-07-20.md` demonstrated, not re-derived); (2) **the leak test**: an actor holding `(ChatStaff, Send)` but not `(ChatCustomer, Send)` cannot produce a customer-visible message by any call path — RED first (before the resource split, a single send path would leak), GREEN after; (3) staff mint/revoke requires owner root; an agent attempt is denied as red-line; (4) voice-note message: blob + transcript ref round-trip through a `ChatMsg`.

**Phase C2 — Customer/courier legs (B, C).**
Work: single-use invitation bootstrap; per-order channel lifecycle (open at confirmation, close at `Delivered` fold); encryption per the §4.1/§10.5 decision (sidecar per P43 E-h, or native ratchet); `ChatCustomer` resource; courier masked channel.
DoD: (1) invitation is single-use: second redemption fails; (2) channel closed after `Delivered`: post-close send rejected; (3) no durable customer identity created (assert no new identity record exists after channel close — P49 stance preserved); (4) if native ratchet: KAT-gated primitives only, no classical-only fallback path representable; if sidecar: bidirectional round-trip through `simplex-chat` CLI over localhost WebSocket in the integration harness; (5) moderation flag path fires on a flagged attachment class.

**Phase A1 — Autonomy configuration surface (presets, caveats, `PendingIntent`).**
Work: §6.2 caveat check in ToolPort pre-check; `PendingIntent`/`IntentApproved`/`IntentRejected` events + fold semantics; preset generators; owner-signed config-change events.
DoD: (1) the full §6.4 table executed as tests — every row, both directions (granted→succeeds, withheld/red-line→rejected pre-effect); (2) confirm-first: agent proposal has zero fold effect until approval; expiry voids; approval by an under-scoped key rejected; (3) self-escalation impossibility: agent attempt to mint/modify a delegation denied as red-line (RED→GREEN against a hypothetical unguarded path); (4) **red-line non-configurability**: a structural test that no delegation constructor available to the autonomy config surface can produce an agent-keypair cert naming `Ledger`/`Auth`/`Secret`/`Migration`, i.e. the omission is enforced in the type/constructor layer, not just checked at use; (5) preset switch is one owner-signed operation whose resulting delegation set exactly matches the preset's declared table; (6) `Scope::is_subset_of` property: no generated preset ever exceeds owner root scope.

**Phase M4 (decoupled, later) — refcount + GC sweep.**
DoD: refcount property tests (shared block survives one manifest's tombstone; sweep reclaims only zero-ref blocks; sweep is crash-safe under the existing tmp+fsync+rename discipline).

Sequencing vs sibling threads: M1–M3 and C1 are independent lanes after M1 lands; C2 waits on the §10.5 operator decision; A1 waits on M2+C1 (needs real tools to bind to); nothing here blocks or is blocked by the voice-hub blueprint (§7) beyond sharing the M1 substrate.

---

## 10. Open decisions for the operator (this document does not decide these)

1. **Relay topology for legs B/C.** Self-host an SMP relay per hub (verified feasible: single Haskell binary, in-memory, no deps, 1 GB VPS — but adds a Haskell operational component per hub), route through SimpleX's public relay infrastructure (zero ops, third-party dependency), or make it per-hub operator-configurable (most consistent with hub sovereignty, most surface to test). Recommendation withheld; the P43 E-h sidecar assumption (public relays) is the path of least resistance but the weakest sovereignty story.
2. **Default staff-role vocabulary.** §5.2 proposes Owner/Kitchen/Counter-Manager as the v1 preset trio. Confirm, rename, or extend (e.g., is a distinct "Courier-dispatcher" preset needed at launch, or does Counter cover it?).
3. **Media storage: local-disk-only vs off-hub backup vs CDN delivery.** `FileBlockStore` on hub disk is v1. `kernel/src/backup.rs`'s design intent already contemplates off-hub backup — should hub media be included in that stream (encrypted, content-addressed dedup makes it cheap), and is public menu-image delivery ever CDN-fronted (interacts with the sovereignty stance and with hash-ETag caching, which makes any dumb HTTP cache sufficient)?
4. **WebP position.** Accept the libwebp C dependency behind a feature gate, adopt/audit `zenwebp`, or skip WebP indefinitely (JPEG/PNG/AVIF variants). §8 defers; a browser-support-driven case may eventually force the question.
5. **Customer-leg encryption: native ratchet vs sidecar.** Build a PQ-hybrid double ratchet on dowiz's KAT-gated primitives (larger, audited-by-us crypto surface; fully sovereign; real work) vs drive the audited `simplex-chat` CLI as a per-hub sidecar (P43 E-h mechanism; AGPLv3 + trademark constraints are compatible with sidecar *use*, but it adds a non-Rust runtime component). This gates Phase C2.
6. **Discriminant allocation ruling.** Bytes `0x21`–`0x24` (§4.4) join `0x1F`/`0x20` under the pending discriminant-allocation ruling referenced in the scope.rs header. Needs the same sign-off before C1/M2 land.
7. **Retention defaults.** Tombstone-only-no-GC (v1, §3.4) is proposed; confirm, and set a default retention posture for closed customer chat channels (delete-on-close vs retain-hub-side-N-days) — a privacy/records tradeoff this document should not set unilaterally.
8. **Shipped autonomy default.** §6.5 proposes Human-only as the shipped default with Agent-assisted offered at onboarding. If the product thesis is "agentic delivery OS," the operator may prefer Agent-assisted as default-on; that is a positioning call, not an architecture call.

---

## 11. Summary of what is genuinely new vs reused

| Surface | Reused (proven) | Genuinely new |
|---|---|---|
| Blob storage | `chunker.rs` + `backup.rs` in full | manifest persistence; `MediaRecord` metadata claims; streaming ingest; (later) refcount/GC |
| Capability/authority | `Scope`, `CertDelegation`, `RedLinePolicy`, ToolPort pattern, red-line CI gates | `Media`/`ChatStaff`/`ChatCustomer`/`Retire` discriminants; one caveat kind; `PendingIntent` flow |
| Communication | event log + `mesh_replication` CvRDT (Leg A transport in full); `moderation.rs`; `ChannelKind::SimpleX`; P43 E-h sidecar spec | `ChatMsg` event kind; invitation bootstrap; per-order channel lifecycle; customer-leg encryption (per §10.5) |
| People | delegation machinery unchanged | `StaffMember` + 3 role presets |
| Autonomy | the capability system *as* the config store | preset generators; caveat check; confirm-first fold semantics; the §6.4 test matrix; the structural (not just checked-at-use) exclusion of red-line from the agent-configurable surface |

The largest single block of genuinely new work is not any one feature — it is the §6 autonomy surface, because it must be wired through *every* ToolPort this document introduces and proven symmetric (human/agent) and red-line-inert at each one. That cost is accepted deliberately: it is the operator's stated core requirement, and building it as capability-native (rather than a bolt-on policy layer) is what keeps it from ever being bypassable.
