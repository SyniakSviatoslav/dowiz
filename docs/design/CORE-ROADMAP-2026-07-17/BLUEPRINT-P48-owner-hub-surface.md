# BLUEPRINT P48 — Owner Hub Surface: omnichannel order intake, adaptive notifications, reviews ingestion (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §8 — every point
> addressed, none skipped). This file PROMOTES P48 out of
> `BLUEPRINT-P47-P50-gap-closing-phases.md` §3 into its own standalone blueprint: the operator's
> 2026-07-18 rulings (recorded there and in
> `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §11's P48 entry) re-centered P48 from
> "admin CRUD surface" to a HUB architecture, and a same-day follow-up directive (verbatim in
> §1.1) grew the scope past what a shared four-phase document can carry. The shared doc's §3
> text is preserved there with a dated redirect; everything it resolved is carried here
> unchanged — nothing is re-litigated. Structure/depth template:
> `BLUEPRINT-P34-mesh-kernel-wiring.md` / `BLUEPRINT-P43-external-integration-ports.md`.
>
> **Already RESOLVED, carried forward (not reopened):**
> - **Rendering: WebGPU, NO DOM exemption** (operator ruling 2026-07-18, recorded in the shared
>   doc §3.2 and master roadmap §11 P48 note): "логіка для інтерфейсу така ж сама як і всюди —
>   це продовження рендер бекенду через фізику." §10.3 invariant 4 holds uniformly; FE-15's
>   a11y mirror stays the only DOM survivor; P38a is an unconditional dependency; hard parts
>   (MSDF text for data-dense panes) land on P38a's critical path, not on a DOM fallback.
> - **Role = multi-channel intake hub** ("тут власне уся суть, що замовити може будь-хто і з
>   різних входів") — every intake channel maps into the SAME
>   `DeliveryEvent::OrderPlaced(OrderPlacedPayload)` pipeline. Wave-0 candidates already named:
>   social-DM intake + web-form intake (shared doc §3.4-B5) — extended in §3.1 now that P43
>   has researched channel adapters (Telegram/WhatsApp/SMS/SimpleX).
> - **Agentic support** ties to P40's tool loop, advisory-only at Wave 0 (shared doc §3.4-B6),
>   under P41's three-mode invariant: every flow works at `AiMode::Off`.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against `/root/dowiz` `main` (clean) and `/root/bebop-repo` `main`. Web
rows researched this pass (marked), not recalled.

| # | Claim | Fresh cite (this pass) | Status |
|---|---|---|---|
| 1 | The one order-intake vocabulary: `DeliveryEvent::OrderPlaced(OrderPlacedPayload)`; payload is `{order_id: u64, amount_i64: i64, src, dst}` — "Money is i64, never float" in the payload's own doc | `bebop-repo/bebop2/proto-cap/src/event_dict.rs:278-282` (enum + variant), `:104-112` (payload) | verified — the pipeline every intake channel must map into (B5's RED check) |
| 2 | KernelFacade seam: `pub fn submit_intent(&self, frame: &SignedFrame) -> Result<Vec<Event>, Reject>`; "The facade contains no decide/money/fold logic; it only verifies then delegates" | `bebop-repo/bebop2/proto-cap/src/facade.rs:123` (+ doc `:121-122`) | verified — intake folds through this seam, never around it |
| 3 | Roster machinery: `pub struct RevocationSet` — append-only invalidate set, "deliberately no unrevoke" | `bebop-repo/bebop2/proto-cap/src/revocation.rs:47-51` | verified — carried B3 reuses it, no new machinery |
| 4 | Content-addressed local-first event log: module doc "commits signed intents to a content-addressed event-log locally, running the kernel decide/fold Law before any network IO. The event-id is a hash of (prev, actor_pubkey, actor_seq, payload) so replays of the same content are idempotent (no TTL dedup — a duplicate is a structural no-op)"; `MeshEvent` `:134`, `event_id()` `:148`, `EventStore` trait `:182`, `EventLog<S>` `:282`, `append` `:302`, `commit_after_decide` `:366`, `verify_chain` `:475` | `kernel/src/event_log.rs:1-23` + listed lines | verified — §1.5's central design principle rides this, builds nothing new |
| 5 | Order-total organ intake pricing must use: `compute_order_total` (overflow-safe, `checked_add`, money invariant in the signature's doc); `place_order` `:156` | `kernel/src/domain.rs:127-131,154-158` | verified — H1's price authority; the customer's message text is never one |
| 6 | Messenger code is non-sending link construction only: `telegram_link()` `:33`, `whatsapp_link()` (wa.me click-to-chat, optional prefilled message) `:39` | `kernel/src/messenger.rs:31-41` | verified — the wa.me prefilled-message builder is load-bearing for H1's WhatsApp window-opening flow (§3.1) |
| 7 | No intake/notify/review port exists: `kernel/src/ports/` = `{llm.rs, agent/, mod.rs}` | live `ls`, this pass | verified — P43's `notify.rs` and this blueprint's modules are both still planning-stage; sequencing in §9 |
| 8 | Hub sovereignty anchors: **M5** "Every HUB = autonomous HYDRA… intra-hub = hub's own business"; **M7** "No single point of failure"; **M8** "Local-only metrics/logging… NEVER exfiltrated (no surveillance)" | `docs/design/ARCHITECTURE.md:14,16,17` | verified — §1.5's "distributed" is these three lines, applied |
| 9 | P43's live channel map (READ-ONLY reference — a parallel task may be editing that file; it is cited here, never edited): `pub enum Channel { Telegram, Sms, WhatsApp, SimpleX, Email }` + `Recipient`/`Notification`/`SendReceipt`/`SendError` + `ChannelSend` trait; SimpleX added 2026-07-18 as E-h, "ADDITIONAL, never a replacement"; inbound webhook surfaces (WhatsApp window ledger, httpSMS delivery events) land on P37's HTTP receive surface; WhatsApp honest cost model: business-initiated templates billed from first send, customer-initiated 24h service window free (1,000 conversations/month free tier), free-form messages inside an open window free | `BLUEPRINT-P43-external-integration-ports.md` §2 (types), §1.1 E-h, §3.4, §0 rows (read this pass) | verified — P48 consumes this vocabulary; transport is P43's, full stop |
| 10 | P40 tool loop (consumed, not owned): `ToolPort` trait + `AgentLoop` bounded plan→act→observe executor, compilation-firewalled; P41 mode law: `AiMode` default **Off**, fail-closed; mode 1 requires zero AI for every product flow | `BLUEPRINT-P40-agent-loop-tool-wiring.md` §2/§3.1/§3.4; `BLUEPRINT-P41-three-mode-ai-operation.md` C-b (`AiMode`, default Off) | verified — H1's parser is deterministic-first; AI assist is optional/advisory by construction |
| 11 | Operator ruling text + prior P48 resolution live in two places this file supersedes as BLUEPRINT (not as record): shared doc §3 (role re-centering, B1–B6, adversarials 1–3) and master roadmap §11 P48 entry (`:1230-1294`) whose Blueprint link this pass repoints here | `BLUEPRINT-P47-P50-gap-closing-phases.md` §3; `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:1230-1294` | verified — carried items listed in §3.0 |
| 12 | **Google Business Profile API** (web-verified this pass, developers.google.com/my-business): **free of charge — no per-call billing, no billing SKUs**; the constraint is access, not money: every new Google Cloud project starts at **quota 0** and must submit an "Application For Basic API Access" (verified GBP active 60+ days, business website, legitimate use case; approval reportedly 2–4 weeks); default quota after approval **300 QPM** (~5 QPS) with a QPD dimension; increases evaluated against real usage patterns. Reviews surface (v4, fully active in 2026): `GET mybusiness.googleapis.com/v4/accounts/{a}/locations/{l}/reviews` (`reviews.list`) returns **ALL of the venue's own reviews**, paginated (`pageSize`/`pageToken`; ~50/page); owner reply `PUT …/reviews/{id}/reply`; delete reply `DELETE …/reviews/{id}/reply` | web research 2026-07-18 (developers.google.com/my-business/content/limits + /content/review-data; corroborating 2026 integration guides) | verified — §3.3's Wave-0 mechanism; the free-tier claim is CONFIRMED but the approval gate is real and external |
| 13 | **Google Places API is the WRONG tool for own-reviews and is rejected with numbers** (web-verified this pass, developers.google.com/maps): since March 2025 the $200/month credit is replaced by per-SKU free tiers; Place Details SKUs: Essentials $5.00/1k (10k free calls/mo), Pro $17.00/1k (5k free/mo), Enterprise $20.00/1k (1k free/mo), **Enterprise + Atmosphere $25.00/1k (1k free/mo) — the `reviews` field bills at this tier**; AND the `reviews` field returns **at most 5 reviews, no pagination** (limitation standing since 2015) — it structurally cannot enumerate a venue's review history at any price | web research 2026-07-18 (developers.google.com/maps/billing-and-pricing/pricing + /documentation/places/web-service/place-details; 5-review cap corroborated by multiple current sources) | verified — the DECART in §3.3 reasons from these numbers |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope & role — what P48 owns, what it consumes, what it must never touch

### 1.1 The operator's own words (2026-07-18 follow-up directive — the scope this file adds)

> "безкоштовні частини варто добавити офіційний WhatsApp Cloud API вже здебільшого
> безкоштовний... можливість постингу, контроль замовлень, і власне самих замовлень з
> месенджерів та налаштування системи сповіщень по кожному з месенджерів адаптивно (один чи
> декілька на вибір клієнта)... найголовніше розподілена уніфікована система що синхронізує
> усе разом у хабі, включно з відгуками з google maps"

Decomposed into four concrete additions, each a section below:

1. **Two-way messenger order flow** (§3.1) — a customer PLACES and MANAGES an order by
   chatting with the venue's bot on any live channel, not just receives notifications.
   The WhatsApp reading is honest per row 9's verified cost model: "вже здебільшого
   безкоштовний" holds exactly for this flow — the customer initiates, which opens the free
   24h service window, so the whole order dialogue rides the free path; business-initiated
   template sends stay billed and stay opt-in (P43 §3.4's `acknowledged_paid` design, reused).
2. **Adaptive per-customer notification channels** (§3.2) — one OR several channels, the
   customer's choice, from whatever is live in P43's channel map.
3. **Reviews ingestion, Google Maps first** (§3.3) — the venue's own reviews flow INTO the
   hub, read-only, generalized as a port so future sources fit the same shape.
4. **The unifying thesis** (§1.5) — "розподілена уніфікована система що синхронізує усе
   разом у хабі" is the existing event-log-first pattern, argued, not asserted.

("можливість постингу" — outbound social POSTING is P22's `SocialPoster` lane and stays
there; the hub gives it a pane, not a second implementation. Boundary row in §1.3.)

### 1.2 P48's single sentence

The venue owner's working surface IS a hub: every inbound thing — an order from any channel,
a customer message, a review — becomes a signed event in the venue's own content-addressed
event log, every pane is a fold-derived projection of that log rendered through the P38a
physics pipeline, and every owner action (menu edit, roster grant, review reply) is an intent
through the same capability-cert law as everything else.

### 1.3 Boundary map (every neighbor's claim honored; nothing scope-duplicated)

| Neighbor | Owns (cited) | P48's side of the line |
|---|---|---|
| **P43** (integration ports) | The channel TRANSPORT, both directions: `ChannelSend` outbound adapters (Telegram/WhatsApp/SMS/SimpleX), inbound webhook/sidecar receive surfaces, provider auth, window ledgers, spool/bucket discipline (row 9). **P43's blueprint is referenced read-only this pass — never edited here** | The seam AFTER receive and BEFORE send: an already-received `InboundMessage` → intent → fold (§3.1); an order state change → `Notification` values handed to P43's port (§3.2). P48 adds zero HTTP clients, zero bot-API code, zero webhook parsing |
| **P49** (customer identity) | STORING the customer-side records: identity default, `NotificationBinding`, and — per this pass — the `NotifyPreference` record's persistence and lifecycle (dies-with-order vs outlives is P49's Wave-0 call) | The intake/routing UI that lets a customer SET/CHANGE the preference (from the hub surface or from inside the chat itself) and the hub-side fan-out routing that reads it (§3.2). Three-way split stated once: **P49 stores, P48 sets/routes, P43 transmits** |
| **P40/P41** (agent loop / modes) | `ToolPort`/`AgentLoop` and the mode law (row 10) | Optional, advisory consumers only: triage assist and ambiguous-intent assist (§3.1/§3.4), all gated so `AiMode::Off` loses convenience, never function |
| **P34** (kernel wiring) | The event vocabulary (row 1) and the facade seam (row 2) — P34's anti-scope "no new event variants" binds here verbatim | P48 consumes both; intake maps INTO `OrderPlaced`, never mints a parallel representation (B5's RED check, kept) |
| **P37/P38a** (wire / render) | HTTP surface + auth; the WebGPU pipelines | Hub panes are P38a-rendered projections; hub auth is P37's owner-scoped capability cert (carried B4: no password path) |
| **P22** (social posting) | Outbound feed/campaign posting, consent-ledger-gated | A hub pane invoking P22's lane; no second poster |
| **P47/P50** | Settlement rails; compliance audit + first-order gate | P48 B1 (managed menu) remains a named prerequisite of P50's §5.3 checklist |

### 1.4 Anti-scope (each a review-rejectable smell; the last is a red-line)

1. **No channel transport code** — P43's, both directions (§1.3 row 1).
2. **No new wire event variants** — P34's anti-scope binds; intake maps into the existing
   vocabulary (row 1) or refuses typed.
3. **No password/admin-auth path, no admin framework, no analytics dashboards** — carried
   verbatim from the shared doc §3.6.
4. **No DOM fallback "just for the hard parts"** — the rendering ruling closed that door;
   hard parts land on P38a's critical path (carried).
5. **No AI in the load-bearing path** — the templated parser is the flow; the tool loop is
   assist (P41 mode-1 invariant, row 10).
6. **No central dowiz-operated aggregation service** — §1.5's argument; each venue's hub owns
   its own log (M5/M7/M8).
7. **No customer CRM/profile machinery** — P49's anti-scope holds; the preference record is
   P49's to store.
8. **🔴 REVIEWS-INTEGRITY RED-LINE (treat with the same seriousness as the money/auth
   red-lines):** dowiz NEVER writes, solicits, incentivizes, gates, moderates, or manipulates
   reviews, and NEVER responds to a public review without an explicit owner action. Ingestion
   is read-only; replies are owner-authored and owner-sent (§3.3's provenance type makes a
   reply without an owner-cert signature unrepresentable). An agent MAY draft a reply
   (advisory, mode-gated); SENDING is a human act, always. No scraping around the official
   API, no fake-review tooling, ever — this is a reputational trust boundary, not a
   nice-to-have. Test-integrity red-line list extended accordingly (§5 ledger row c).

### 1.5 The central design principle — "distributed unified sync" IS the event log (argued, not asserted)

The operator's "найголовніше" clause could be built two ways. The wrong way is a hub
dashboard with its own aggregation store: pull orders from channels into a table, pull
reviews into another, join for display. The right way — and the one this blueprint binds —
is: **every inbound thing becomes a signed event in the SAME content-addressed local-first
log the kernel already runs (row 4), and the hub is nothing but folds over it.** Four
arguments, each falsifiable:

1. **One Law, no second truth.** Orders already ARE events (`OrderPlaced` → decide → fold,
   rows 1–2). An aggregator table beside the log is a second source of truth that can drift —
   the exact defect class P34's ground truth caught live twice (its §0 rows 12/18: a mirror
   table drifting from the Law, undetected for months because nothing gated the pair).
   Carried B2 already mandates "no shadow state in the surface crate"; a bolted-on aggregator
   is shadow state with a nicer name. Falsifier: any hub-surface store that is not a
   fold-derived projection fails B2's review-gate check.
2. **Cross-channel dedup comes free.** Channels redeliver (webhook retries, sidecar
   reconnects, poll overlaps). The log's event-id is a content hash — "a duplicate is a
   structural no-op, not a timeout" (row 4, `event_log.rs:6-7`). A separate aggregator would
   have to rebuild exactly this idempotency machinery per source; the log already is it.
   Falsifier: H1-adversarial-ii and H3-adversarial-ii (replayed inbound message, re-polled
   review page) must produce zero duplicate events with zero new dedup code.
3. **"Distributed" means per-hub, and the canon already rules it.** M5: intra-hub is the
   hub's own business. M7: no single point of failure. M8: no exfiltration, no surveillance
   (row 8). A central dowiz-operated aggregation service would violate all three at once — a
   SPOF that pools every venue's customer PII and review streams on infrastructure the venue
   doesn't control. Instead: each venue's hub owns its own event log; multi-device sync for
   one venue (owner's phone + shop terminal) rides the existing signed-log sync machinery
   (P34B's MerkleLog lane) — nothing new is built, and no cross-venue anything exists.
   Falsifier: grep for any P48 component whose availability depends on a dowiz-operated
   endpoint (Google's API for reviews is a SOURCE, not an aggregator — its outage degrades
   one pane, §3.3-adversarial-iv).
4. **The audit trail is the feature.** For a real venue, "what happened, in order, across
   every channel" is an operational need (dispute with a customer, a missed order, a
   review-reply record). An append-only signed log gives this for free (`verify_chain`,
   row 4); a dashboard aggregator gives a mutable table with no provenance.

**Rejected alternative (DECART, one line):** bolt-on analytics/aggregator layer — rejected
for drift (arg 1), duplicated dedup (arg 2), SPOF+surveillance posture (arg 3), and no
audit provenance (arg 4). Analytics READS the log like every other projection (P22's
`ChannelLedger` already models this).

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

P43's `ports/notify.rs` vocabulary (`Channel`, `Recipient`, `Notification`, `SendReceipt`,
`SendError`, `ChannelSend` — row 9) is REUSED, not redeclared. Sequencing consequence, stated
honestly: these modules land AFTER P43's notify.rs exists (both are planning-stage today,
row 7); the types below reference it as the one channel vocabulary (correspondence: one
concept, one type).

```rust
// ── kernel/src/ports/hub_intake.rs — NEW module (H1/H2). Same zero-I/O firewall
// discipline as ports/llm.rs: no HTTP, no serde, no provider names. ─────────────

/// One already-received inbound message, channel-agnostic. PRODUCED by P43's
/// receive surfaces (webhook/sidecar adapters), CONSUMED here. `provider_msg_id`
/// is the channel's own delivery id — the per-channel dedup key (§3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundMessage {
    pub venue_id: String,
    pub channel: Channel,            // P43's enum — the one channel vocabulary
    pub sender: String,              // channel-shaped address (chat id / wa id / …)
    pub provider_msg_id: String,
    pub text: String,
    pub unix_ms: u64,
}

/// What the parser decided. Closed set; every arm is an observable outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentOutcome {
    Order(OrderIntent),
    Ambiguous(Ambiguity),   // typed uncertainty — NEVER a silent guess (§3.1 adv. v)
    NotAnOrder,             // greeting/question/etc — routed to the hub inbox pane
}

/// A parsed order-intent. Prices are ABSENT by design: the menu fold is the only
/// price authority (compute_order_total, §0 row 5) — a customer message cannot
/// name a price (money red-line, §3.1 adversarial iv).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderIntent {
    pub venue_id: String,
    pub items: Vec<IntentItem>,      // non-empty by constructor
    pub delivery_addr: String,       // free-text, carried opaque to the order
    pub reply_to: Recipient,         // SAME channel the message arrived on (law, §3.1)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentItem { pub menu_item_id: String, pub quantity: u32 } // qty ≥ 1

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ambiguity { pub reason: AmbiguityReason, pub candidates: Vec<String> }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbiguityReason { UnknownItem, MultipleMatches, NoQuantity, NoAddress }

/// Deterministic, menu-anchored parser. Total function — cannot fail, only
/// classify. Implementations are pure (text + menu view in, outcome out);
/// the AI-assist path is a SEPARATE optional consumer of Ambiguous outcomes
/// (§3.4), never an implementor of this trait.
pub trait IntentParser {
    fn parse(&self, msg: &InboundMessage, menu: &MenuView) -> IntentOutcome;
}

/// Customer's chosen notification channels: ONE OR SEVERAL. Non-empty is a
/// constructor invariant (an empty set is unrepresentable; "no notifications"
/// is expressed by not creating a preference — default = intake channel).
/// STORED by P49 (its lane); SET/ROUTED here; TRANSMITTED by P43.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyPreference { channels: Vec<Recipient> /* private; ctor refuses empty */ }

// ── kernel/src/ports/review_ingest.rs — NEW module (H3). Zero-I/O, read-only. ──

/// Closed source set. Wave 0 = GoogleMaps ONLY. Future sources (TripAdvisor-class,
/// Facebook-class) are named as extension points and NOT built — adding a variant
/// is a reviewed kernel-ports diff that must satisfy the same read-only shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewSource { GoogleMaps }

/// One review as ingested — the venue's OWN review data flowing IN. Immutable
/// once evented; an upstream edit produces a NEW event version (append-only law).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRecord {
    pub source: ReviewSource,
    pub source_review_id: String,    // upstream id — the idempotency anchor
    pub rating_stars: u8,            // 1..=5 (GBP star enum; integer, no float)
    pub text: String,
    pub author_display: String,      // as the source presents it; never enriched
    pub created_unix_ms: u64,
    pub updated_unix_ms: u64,
    pub owner_reply: Option<String>, // upstream reply state, mirrored read-only
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewIngestError {
    NotApproved,        // GBP quota 0 — access application not yet granted (§0 row 12)
    QuotaExhausted,     // 300 QPM / QPD ceiling hit — back off, typed
    SourceDown,
    ParseError(String),
}

/// READ-ONLY ingestion port. There is deliberately NO write/post/moderate
/// method on this trait — manipulation is unrepresentable at the port.
pub trait ReviewIngest {
    fn fetch_page(&self, venue_source_ref: &str, cursor: Option<String>)
        -> Result<(Vec<ReviewRecord>, Option<String>), ReviewIngestError>;
}

/// The ONE sanctioned write-back: an owner-authored reply. `owner_sig` is the
/// owner-cert signature over (source_review_id ‖ body) — a reply without a live
/// owner capability signature is unrepresentable (red-line §1.4-8 as a type).
#[derive(Debug, Clone)]
pub struct OwnerReplyAction {
    pub source: ReviewSource,
    pub source_review_id: String,
    pub body: String,
    pub owner_sig: Vec<u8>,
}
pub trait ReviewReply {
    fn send_reply(&self, action: &OwnerReplyAction) -> Result<(), ReviewIngestError>;
}

// ── Constants (hub-adapters lane) ───────────────────────────────────────────────
/// GBP default quota after access approval (web-verified §0 row 12).
pub const GBP_DEFAULT_QPM: u32 = 300;
/// Review poll cadence per venue: hourly delta pull. 24 calls/day/venue —
/// 3 orders of magnitude under the quota; reviews are not a hot path.
pub const REVIEW_POLL_INTERVAL_S: u64 = 3_600;
/// GBP reviews.list page size (§0 row 12).
pub const REVIEW_PAGE_SIZE: u32 = 50;
/// Intake sanity caps — refuse-typed above, never truncate silently.
pub const INTENT_MAX_ITEMS: usize = 32;
pub const INTENT_MAX_TEXT_BYTES: usize = 4_096;
```

**Rejected alternatives (DECART, one line each):** *a per-channel intent parser* — rejected:
the parser is menu-anchored text classification, channel-independent by construction; one
impl, N channels (correspondence). *`f32` rating* — rejected: GBP ratings are a star enum;
integer `u8` matches the repo's no-float-in-domain-data law. *A `ReviewPost`/moderation
method on `ReviewIngest`* — rejected as red-line §1.4-8: the port is read-only so the
violation is unrepresentable, not merely reviewed-against. *Storing `NotifyPreference` in a
P48 table* — rejected: P49 owns customer-side records; P48 storing it would fork identity
storage (§1.3 row 2).

---

## 3. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 3.0 Carried items (resolved in the shared doc, restated as the base — NOT re-litigated)

| Carried | Content (shared doc §3.4/§3.5, verbatim in spirit) |
|---|---|
| B1 | Menu edit → subsequently placed order's fold carries the change |
| B2 | Live order list = read-only projection; review-gate proves no shadow state |
| B3 | Roster grant/revoke via existing `RevocationSet`; revoked courier's next mutating request rejected |
| B4 | Negative test: no password-based admin login path; owner auth = capability cert |
| B5 | ≥2 non-native Wave-0 intake channels, BOTH mapping into `OrderPlaced` — an intake channel minting its own order representation is the fail condition |
| B6 | Agentic-triage design note tied to P40, advisory, non-gating |
| Adv 1–3 | Scope-escalation reject; revoked-owner mid-session reject; edit-vs-in-flight-order race → orders fold against menu state at placement time (no retroactive price change — property test) |

**B5 Wave-0 channel list, extended this pass** (now that P43 has researched adapters —
row 9): (i) **Telegram DM** — first, works-today, zero vendor cost; (ii) **web-form** —
the no-messenger floor; (iii) **WhatsApp** — the customer-initiated flow rides the free
service window end-to-end (§1.1 point 1): the storefront's `wa.me` prefilled link (row 6)
or a cold inbound DM opens the window, the whole order dialogue is free-form inside it;
(iv) **SimpleX** — the privacy channel, sequenced after Telegram proves the intake seam
(mirrors P43 §3.4b's own sequencing). Channel-specific transport stays P43's; this list only
names which of ITS adapters the intake seam exercises at Wave 0.

### 3.1 H1 — two-way messenger order flow (place & manage an order by chatting)

**The flow, concretely (spec):**

```
customer DM ("2x маргарита, доставка на Шевченка 12")
  → P43 receive surface (webhook/sidecar; signature-verified fail-closed — P43's gate)
  → InboundMessage{channel, sender, provider_msg_id, text}
  → IntentParser::parse(msg, menu_fold_view)                       [pure, no AI]
      Order(intent)      → priced by compute_order_total over the menu fold
                         → signed frame → KernelFacade::submit_intent (§0 row 2)
                         → DeliveryEvent::OrderPlaced fold → event log append
                         → confirmation Notification (order id, items, total, ETA)
                           sent back via P43 ChannelSend on the SAME channel
      Ambiguous(reason)  → templated clarification reply on the same channel
                           (menu-anchored candidate list; no AI required)
                         → optional P40 assist drafts a better clarification (§3.4)
      NotAnOrder         → hub inbox pane (owner sees it; no order machinery touched)
```

**Manage, not just place:** follow-up messages from the same sender thread onto the open
order: "скасуйте" → a `StatusChanged`-mapped cancel intent through the same facade (legal
per the kernel's own transition law, refused typed otherwise); "де замовлення?" → a status
reply rendered from the fold. Same parser, same seam — management intents are a second
closed template family, not a second pipeline.

**The parser is deterministic-first (the no-AI invariant, load-bearing):** menu-anchored
matching — quantity tokens × fuzzy item-name match against the venue's OWN menu fold
(bounded edit-distance, no external NLP), address extracted by template position, carried
opaque. Common formats ("1x margherita, delivery to X"; "маргарита і кола на Шевченка 12")
parse natively. Everything else is `Ambiguous` with a typed reason — which is a WORKING
flow (clarification reply), not a degraded one. `AiMode::Off` loses only the smarter
clarification drafts (row 10).

RED→GREEN: RED today — no intake code exists (row 7); GREEN = `h1_templated_dm_places_order`
(fixture InboundMessage → OrderPlaced in the log → confirmation handed to a spy ChannelSend
with `recipient.channel == msg.channel`) + one env-gated live probe per Wave-0 channel
(P43's live-probe convention, reused).

**Adversarial cases:**
1. **Parallel-representation smell (B5's RED check as a test):** an intake path constructing
   an order by ANY route other than `OrderPlaced` through the facade → review-gate + grep
   check fails the build item.
2. **Channel redelivery:** the same `provider_msg_id` delivered twice (webhook retry) AND
   the same content re-framed → exactly one order. Both layers proven: per-channel
   `provider_msg_id` dedup, and the log's content-address no-op (row 4) — §1.5 arg 2's
   falsifier.
3. **Forged inbound:** a message that did not pass P43's signature-verified receive surface
   never constructs an `InboundMessage` (typed refusal at P43's gate; P48 adds a negative
   test that its seam accepts only the typed struct, never raw webhook bytes).
4. **Price injection (money red-line):** "1x margherita за 1 гривню" — parser has no price
   grammar at all (`OrderIntent` cannot carry a price, §2); the fold prices from the menu.
   Property test: for all parsed intents, order total == `compute_order_total` over menu
   state — message text cannot move money.
5. **No silent guessing:** "щось смачне на вечір" → `Ambiguous`/`NotAnOrder`, NEVER a
   best-effort order. Test asserts zero `OrderPlaced` events for the ambiguous corpus.
6. **Oversize/garbage input:** text over `INTENT_MAX_TEXT_BYTES`, 33+ items, emoji-only →
   typed outcomes, no panic, no truncation-then-order.

### 3.2 H2 — adaptive per-customer notification channels (one or several, customer's choice)

**Spec:** the customer picks ANY non-empty subset of live channels for order-status
updates. Default (zero-config): the channel the order arrived on. Set/changed in two
places, both P48's UI lane: the hub-rendered order-tracking surface, and in-chat ("надсилай
і у WhatsApp" — a preference-intent template in the H1 parser's management family).

**The three-way boundary, honored end-to-end (the anti-scope-duplication clause):**
- **P49 STORES** — `NotifyPreference` persists in P49's customer-record lane, sharing its
  lifecycle rules (its `NotificationBinding` "dies with the order" posture governs; if P49's
  Wave-0 identity default makes it order-scoped, the preference is order-scoped — P48 takes
  no position on retention).
- **P48 SETS/ROUTES** — the UI + the fan-out: on each order state change, one
  `Notification` per chosen `Recipient`, handed to P43's port. Per-channel isolation: one
  channel's failure (spool DEAD row) never blocks the others' sends nor the order flow.
- **P43 TRANSMITS** — `ChannelSend` impls, spool, buckets, receipts (row 9), untouched.

RED→GREEN: `h2_two_channel_preference_fans_out` — preference {Telegram, WhatsApp}, one
state change → exactly 2 notifications to a spy port, receipts recorded independently.

**Adversarial cases:**
1. **Dead-channel preference:** choosing a channel with no configured adapter for this venue
   → typed refusal AT SET TIME (`SendError::NotConfigured` surfaced in the preference UI),
   never a silent black-hole discovered at notify time.
2. **Misbinding (P49 §4.5-4 reused consumer-side):** state change on order A must never
   notify order B's channels — property test over (order, preference) pairs.
3. **Empty set:** unrepresentable by constructor (§2); the test is a compile-shape check +
   a ctor refusal test.
4. **Mid-order change:** preference updated between two state changes → the next
   notification honors the NEW set exactly (fold-derived read, no cached routing).
5. **Fan-out amplification:** a malicious/buggy loop toggling preference must not multiply
   sends — per-order-per-state-change idempotency: one notification per (order, transition,
   channel), enforced at the routing fold, tested.

### 3.3 H3 — reviews ingestion port, Google Maps Wave-0 (read-only; owner-gated replies)

**DECART — mechanism (decided here, from §0 rows 12–13's verified numbers):**

| Criterion | **Google Business Profile API — CHOSEN** | Google Places API — rejected for this job |
|---|---|---|
| Cost | **Free — no per-call billing exists for this API** | `reviews` field = Enterprise + Atmosphere SKU: **$25.00/1k calls after 1,000 free calls/month** |
| Coverage | **ALL of the venue's own reviews**, paginated (`REVIEW_PAGE_SIZE`) | **Hard cap: 5 reviews, no pagination** — cannot enumerate history at any price |
| Fit | Owner-authorized access to the venue's OWN data (OAuth as the owner) — exactly the trust shape of a hub | Third-party discovery API — wrong tool: built for showing places to strangers, not for a business reading itself |
| Reply leg | `updateReply`/`deleteReply` exist — the owner-gated response path (§1.4-8) | None |
| The real gate | **Access approval, not money:** project starts at quota 0; Basic API Access application (verified GBP 60+ days, website; ~2–4 weeks); then 300 QPM default | Billing account from call one |

Verdict: GBP API. The honest caveat is operational, not financial — the approval is an
external gate per venue-platform, named in the DoD exactly like P43's Meta-verification gate
(never faked, never self-certified).

**Design:** `GbpReviewIngest` (hub-adapters lane, `ureq` per the thrice-cited spec)
implements `ReviewIngest::fetch_page` against `reviews.list`; a per-venue poll (cadence
`REVIEW_POLL_INTERVAL_S` — 24 calls/day/venue, ~3 orders of magnitude under `GBP_DEFAULT_QPM`)
pulls the delta; **each `ReviewRecord` becomes a signed event in the venue's own log** —
content-addressing makes every re-poll idempotent (zero new dedup code, §1.5 arg 2). The
hub's reviews pane is a fold over those events, same as every other pane. An upstream edit
or deletion appends a new version event; nothing is mutated in place (append-only law,
audit-preserving).

**Owner reply (the ONE write-back):** the owner authors a reply in the hub → the action is
signed with the owner cert (`OwnerReplyAction.owner_sig`, §2) → `ReviewReply::send_reply` →
GBP `updateReply`. The reply is itself an event in the log (who replied, when, to what).
An agent MAY draft (P40, advisory, `AiMode`-gated); the SEND requires the owner-signed
action — unrepresentable otherwise (§1.4-8's red-line as a type).

**Generalization without speculation:** the port shape (closed `ReviewSource`, cursor-paged
read-only fetch, source-id idempotency anchor, owner-gated reply trait) is the pattern any
future source must fit. TripAdvisor-class / Facebook-class sources are NAMED as extension
points and not built, researched, or vendor-evaluated — one variant lands when a real venue
asks, as its own reviewed diff.

RED→GREEN: `h3_fixture_page_folds_to_events` (fixture reviews.list JSON page → N events →
pane fold shows N) + `h3_repoll_is_noop` (same fixture again → 0 new events) + the live
probe gated on a real venue's GBP API approval (external gate, recorded, not faked).

**Adversarial cases:**
1. **Unapproved project (quota 0):** `NotApproved` typed → reviews pane renders empty-with-
   reason; order flow, menu, roster all unaffected (isolation proven by running the full H1
   suite with the ingest adapter black-holed).
2. **Re-poll/overlap replay:** pages re-fetched with overlapping cursors → zero duplicate
   events (content-address; §1.5 arg 2's falsifier).
3. **Reply without owner provenance:** an `OwnerReplyAction` with a missing/invalid/
   non-owner-scope signature → refused typed BEFORE any HTTP (spy asserts zero calls) —
   the red-line's negative test, permanent.
4. **Source outage / quota exhaustion mid-page:** typed error, partial page discarded
   (no half-ingested pagination state), next poll resumes from the last committed cursor —
   crash-consistency by construction (cursor is itself fold-derived).
5. **Hostile review content:** review text is untrusted input rendered in the hub — carried
   opaque, never interpreted (no template expansion, no markdown execution in the P38a text
   path); an injection-shaped fixture (`=SUM(...)`, script tags, RTL overrides) round-trips
   as inert text.
6. **Upstream edit racing a reply:** review edited upstream between fetch and owner reply →
   the reply targets `source_review_id` (stable upstream id), and the next poll's new
   version event shows the post-edit state — no lost update, sequence visible in the log.

### 3.4 H4 — agentic support (carried B6, made concrete; advisory, never load-bearing)

The P40 tool loop gets two read-mostly hub tools (joining P42's catalog per its growth
rule, not via loop edits): `read_hub_inbox` (unhandled `Ambiguous`/`NotAnOrder` messages +
new reviews) and `draft_reply` (a clarification or review-reply DRAFT — text that lands in
the owner's compose box, never on the wire). Triage assist = the agent proposing order
priorities/answers across channels from fold-derived state. All of it: `AiMode::Off` ⇒
absent, everything still works (row 10); no tool has a send/mutate capability — the send
seams (H1 confirmations are automatic-templated; review replies are owner-signed) are
structurally out of the agent's reach. RED check: grep proves no agent-lane crate imports
`ChannelSend` or `ReviewReply`.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

- **Message text cannot move money:** no price grammar exists in the parser's type surface
  (`OrderIntent` has no amount field, §2); totals come only from `compute_order_total` over
  the menu fold (overflow-safe, row 5). Reachability of a text-priced order requires a type
  that does not exist.
- **Review manipulation unrepresentable at the port:** `ReviewIngest` has no write method;
  the one write-back type requires an owner-cert signature; agent tools have no transmit
  capability (§3.4). Three independent structural gates, each grep/type-checkable.
- **Duplicate orders/reviews structurally no-ops:** content-addressed event ids (row 4) —
  the same idempotency argument P34 §4.1 makes for double-fold, inherited not rebuilt.
- **PII surface, named:** `InboundMessage.sender` + preference `Recipient`s ARE personal
  data — per-order, per-venue, in the venue's OWN log only (M8: never exfiltrated; no
  cross-venue store exists to leak). Review author names are already-public source data,
  mirrored verbatim, never enriched or joined against customer identity (a join type does
  not exist).
- **Authority:** every owner action (menu, roster, reply) is capability-cert-scoped
  (carried B3/B4 + adversarials 1–2); customer intents authenticate as channel-verified
  senders only — they can place/manage THEIR order (P49's identity lane), never touch hub
  administration (scope-escalation adversarial, carried).

### 4.2 Schemas & scaling axes (item 8)

- **Inbound messages:** ~10× order volume (chatter, clarifications) — venue-day volume in
  the low thousands worst-case; parse is pure in-memory work (§6 budget). Axis break: none
  before multi-venue nodes; per-venue keying is already the ledger convention.
- **Notification fan-out:** orders/day × state changes (~6) × chosen channels (1–5, closed
  enum bounds it) — hundreds/day/venue, riding P43's spool budgets unchanged.
- **Reviews:** venue lifetime 10²–10⁴ records; delta polls are page-sized; log growth is
  linear and joins the existing event-log compaction story (P34B's stated break point at
  ~10⁶ leaves — reviews add noise-level volume to it).
- **Quota math (the external axis):** 24 polls/day/venue vs 300 QPM — one hub node serves
  hundreds of venues before quota planning matters; the point at which it changes is named
  (multi-venue SaaS-style nodes), and the answer is per-venue OAuth projects, not a shared
  quota pool.

### 4.3 Isolation (11), mesh (12), rollback (13), living memory (15)

- **Isolation:** intake, notification routing, and review ingest are three independent
  consumers/producers of the log; each degrades alone (typed errors, DEAD rows, empty-pane-
  with-reason) and none can block the order fold — the H3-adversarial-1 cross-suite run is
  the proof. The hub surface crate depends on kernel + facade only; adapters live in the
  edge lane (P43's crate discipline mirrored).
- **Mesh (12):** everything is node-local: adapter I/O at the edge, log appends local-first
  (row 4: Law before any network IO). Multi-device sync of one venue's hub = P34B's signed-
  log sync lane, payload = the same DeliveryEvent frames already budgeted (~5–6 KB, P34
  §4.2) plus review events of comparable size. No new transport, no new gossip payload
  class.
- **Rollback (13, vocabulary used precisely):** **Self-Termination** — typed refusals
  everywhere (quota, ambiguity, provenance), bounded polls, caps (§2 consts); an unsafe
  state (text-priced order, unsigned reply) is unrepresentable, not supervised.
  **Snapshot-Re-entry** — the hub is folds over an append-only log: any pane rebuilds from
  replay; an interrupted ingest resumes from the committed cursor. **Self-Healing is NOT
  claimed** — no redundancy math here.
- **Living memory (15):** the log IS the temporal store — reviews and messages are
  time-ordered, content-addressed history; recall by content not location
  (`internal-retrieval-living-memory-arc-2026-07-14`'s principle, inherited via row 4's
  machinery, nothing new).

### 4.4 Linux-discipline verdicts (item 9)

**ALREADY-EQUIVALENT:** one vocabulary/one Law for intake (reusing `OrderPlaced` + facade —
"one implementation of one concept"); ports-at-the-edge with a zero-I/O kernel module pair.
**REINFORCES:** fail-closed typed outcomes for every external surface (quota, ambiguity,
forged inbound); the no-shadow-state review gate (B2) extended to every hub pane.
**EXTENDS:** the fail-closed doctrine to *reputational* state — the reviews red-line
(§1.4-8) is enforced as port shape + signature provenance, the same way P43 extended it to
economic state (its §4.4). **GAP (named, deferred):** in-chat preference-setting natural
language ("надсилай і у WhatsApp") is itself a template family that will grow — growth is
bounded by the closed `AmbiguityReason`/template registry, but a template-sprawl review
trigger is set at 20 templates (tracked in §5's ledger row d).

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

Carried B1–B6 remain binding exactly as recorded (shared doc §3.4; master roadmap §11 P48
DoD 1–7 — DoD-1 already ✅ RESOLVED). New rows:

| Item | RED (fails before) | GREEN (passes after) | Named test / check (permanent, item 17) |
|---|---|---|---|
| H1 two-way flow | no intake code exists (row 7) | fixture DM → `OrderPlaced` in the log + same-channel confirmation; management intents (cancel/status) fold legally | `h1_templated_dm_places_order`, `h1_cancel_intent_folds`, `h1_same_channel_confirmation`, adversarials §3.1-2/4/5/6 |
| H2 adaptive channels | no preference concept anywhere | 2-channel preference fans out exactly 2 receipts; set-time dead-channel refusal | `h2_two_channel_preference_fans_out`, `h2_dead_channel_refused_at_set`, `h2_misbinding_property`, `h2_transition_idempotent_fanout` |
| H3 reviews ingest | no review concept anywhere | fixture page → N events, re-poll → 0; unsigned reply refused with zero HTTP; live probe recorded as externally gated (GBP approval) | `h3_fixture_page_folds_to_events`, `h3_repoll_is_noop`, `h3_unsigned_reply_refused`, `h3_source_outage_isolated`, `h3_hostile_text_inert` |
| H4 agent assist | — (advisory) | draft-only tools in the P42 catalog; grep proves no agent-lane import of `ChannelSend`/`ReviewReply`; all green at `AiMode::Off` | `h4_agent_cannot_transmit` (grep+compile check), mode-off suite run |
| §1.5 no-aggregator | — | review-gate: every hub pane reads a fold; no hub-lane store outside the event log | `hub_no_shadow_store` review-gate check (B2 extended) |

Ledger obligations (`docs/regressions/REGRESSION-LEDGER.md`, red→green per its ratchet
rule): (a) "message text cannot price an order — guardrail: H1 §3.1-adv-4 property test";
(b) "duplicate inbound/review delivery is a structural no-op — guardrail: `h1`/`h3` replay
tests"; (c) "review reply requires owner-cert provenance — guardrail:
`h3_unsigned_reply_refused`" (the reviews-integrity red-line's permanent tooth; the
test-integrity red-line list gains a reviews clause); (d) the §4.4 GAP row
(template-sprawl trigger at 20) as tracked-open.

---

## 6. Benchmark plan (item 10) — modest by design; nothing here is hot next to network I/O

1. `hub/intent_parse` — parse a realistic message corpus against a 100-item menu, **budget
   ≤ 1 ms/message** (pure string+menu matching at memory speed; the number exists so a
   future "smarter" parser can't silently go quadratic).
2. `hub/fanout_enqueue` — one state change → N-channel notification construction+enqueue,
   **budget ≤ 1 ms + P43's own spool budget** (P43 §6's ≤ 1 ms row inherited, not re-owned).
3. `hub/review_page_fold` — 50-record fixture page → events → pane fold, **budget ≤ 10 ms**
   (hash+append dominated; documents the non-network cost so GBP latency is never blamed on
   the fold).

Numbers into the established `BENCH_HISTORY.md` convention; telemetry = the log itself plus
existing spool/ledger read surfaces (no third channel — item 19).

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §11 P48 (the charter; its Blueprint
link repointed here this pass) + §10.3 invariants 3/4/5 · `BLUEPRINT-P47-P50-gap-closing-
phases.md` §3 (the promoted-from section — original rulings preserved there; carried items
§3.0) · `BLUEPRINT-P34-mesh-kernel-wiring.md` (facade/vocabulary consumption; drift-class
precedent cited in §1.5 arg 1; structure template) · `BLUEPRINT-P43-external-integration-
ports.md` (channel transport + types, READ-ONLY reference this pass — deliberately not
edited, a parallel task owns it today) · `BLUEPRINT-P40-agent-loop-tool-wiring.md` +
`BLUEPRINT-P41-three-mode-ai-operation.md` (assist + mode law) · `BLUEPRINT-P42-mcp-agent-
skills.md` (tool-catalog growth rule for §3.4's tools) · `BLUEPRINT-SOCIAL-AUTO-POSTING-
2026-07-17.md` (P22 posting boundary) · `docs/design/ARCHITECTURE.md` M5/M7/M8 (§1.5 arg 3)
· `kernel/src/{event_log.rs, domain.rs, messenger.rs}` (§0 rows 4–6) ·
`docs/design/BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` (D3 device-bound model, not forked) ·
web research 2026-07-18: Google Business Profile API limits + review-data pages, Google
Maps Platform pricing page, Place Details (New) field/SKU reference (§0 rows 12–13 carry
the design-relevant facts inline so this doc stands without re-fetching). Memory files:
`integration-ports-reactive-arc-2026-07-13` (ports doctrine) · `test-integrity-rules-
2026-06-27` (money/PII red-lines; extended by §5 row c) · `never-bypass-human-gates-
2026-06-29` (owner-gated replies are a human gate by design) · `verified-by-math-2026-07-07`
(falsifiability bar) · `internal-retrieval-living-memory-arc-2026-07-14` (§4.3) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (style discipline applied) ·
`rust-native-bare-metal-decision-2026-07-14` (DECART one-liners in §2/§3.3).

**Hermetic principles honored (item 20, explicit):** **P2 CORRESPONDENCE** — one intake
vocabulary (`OrderPlaced`) for every channel, one log for every inbound kind, one channel
enum shared with P43; the hub adds projections, never parallel truths. **P6
CAUSE-AND-EFFECT** — every pane state is a deterministic fold of signed events; replay
reproduces it exactly (§4.3). **P7 GENDER (no self-certification)** — transmission receipts
come from the far side (P43's law); review data comes from the source of record; owner
replies are certified by the owner's own signature, never by the surface claiming intent.
(Other principles not load-bearing here; not claimed decoratively.)

---

## 8. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 13 rows, live-verified this pass; web rows marked researched-this-pass with the numbers inline |
| 2 DoD | §5 — carried B1–B6 binding + 5 new RED→GREEN rows with named tests |
| 3 spec/event-driven TDD | §2 types first; §3 per-item RED tests; assertions on event sequences in the log (H1/H3) |
| 4 predefined types/consts | §2 — full port vocabulary + quota/cadence/caps constants, DECART-rejected alternatives |
| 5 adversarial tests | §3.1 (6), §3.2 (5), §3.3 (6), §3.4 (1) + carried Adv 1–3 — incl. replay, forgery, price-injection, hostile text |
| 6 hazard-safety as math | §4.1 — unrepresentability arguments (no price type, no write method, signature provenance, content-address idempotency) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 — messages, fan-out, reviews, quota; break points named with owners |
| 9 Linux discipline | §4.4 — four verdicts incl. one honest GAP with a tracking row |
| 10 benchmarks+telemetry | §6 — three budgets; log+spool as the telemetry surfaces |
| 11 isolation/bulkhead | §4.3 — three independent lanes, cross-suite isolation proof (H3-adv-1) |
| 12 mesh awareness | §4.3 — node-local; multi-device sync deferred to P34B's lane with payload budget cited |
| 13 rollback/self-heal vocabulary | §4.3 — Self-Termination + Snapshot-Re-entry claimed with mechanisms; Self-Healing explicitly not |
| 14 error-propagation gates | closed enums (§2), review-gate `hub_no_shadow_store`, grep gates (H4, B5's RED check) |
| 15 living memory | §4.3 — the log as temporal store; nothing new built |
| 16 tensor/spectral + eqc | N/A-honest: no closed-form math organ on this path; no decorative claim |
| 17 regression ledger | §5 — four rows incl. the red-line tooth (c) and the open GAP (d) |
| 18 agent-executable instructions | §9 |
| 19 reuse-first | §1.5 (the whole thesis is reuse of the log), §2 (P43 vocabulary reused), §3.3 (GBP over building anything), rejected alternatives throughout |
| 20 Hermetic citations | §7 (folded, explicit per principle) |

---

## 9. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Repo: `/root/dowiz`. **Gate check first:** all build items require P37 (wire+auth), P38a
(render), and P43's `ports/notify.rs` + at least the Telegram adapter to EXIST (row 7: none
do today — this is a planning file until those land). Do not start H-items before the B5
Wave-0 channels have a live receive surface (P43's lane).

1. **T1 (ports).** Create `kernel/src/ports/hub_intake.rs` + `review_ingest.rs` (§2
   verbatim); register in `ports/mod.rs`. NO new kernel deps (`git diff kernel/Cargo.toml`
   → empty). Acceptance: `cd kernel && cargo test --lib` green.
2. **T2 (H1 parser).** Implement the deterministic `IntentParser` (menu-anchored; templates
   for place/cancel/status/preference families) + the H1 test set incl. all six §3.1
   adversarials. The parser crate imports NO llm/agent code (grep gate). Acceptance: parse
   corpus green; `h1` adversarials green; bench §6-1 recorded.
3. **T3 (H1 seam).** Wire `InboundMessage` → parse → facade `submit_intent` → log append →
   confirmation via P43's `ChannelSend` (spy in tests; env-gated live probes per Wave-0
   channel). The confirmation's `recipient.channel` MUST equal the inbound channel unless an
   H2 preference explicitly overrides. Acceptance: `h1_templated_dm_places_order` +
   replay/forgery adversarials green.
4. **T4 (H2).** Preference set/change UI intents (hub pane + in-chat template) routed to
   P49's store (if P49's storage isn't landed yet, STOP — do not create a P48-side store;
   that is the §1.3 boundary, flag it instead) + the fan-out fold + all five §3.2
   adversarials. Acceptance: `h2_*` green.
5. **T5 (H3 ingest).** `GbpReviewIngest` in the hub-adapters lane (`ureq`; OAuth config
   per-venue; poll at `REVIEW_POLL_INTERVAL_S`), reviews→events fold, pane projection.
   Fixture tests first; the live probe is gated on a real venue's GBP Basic API Access
   approval — record the gate's status honestly, never fake the probe. Acceptance:
   `h3_fixture_page_folds_to_events` + `h3_repoll_is_noop` + adversarials 4–6 green.
6. **T6 (H3 reply).** `OwnerReplyAction` signing (owner cert) + `ReviewReply` →
   `updateReply`; the reply becomes a log event. The negative test
   `h3_unsigned_reply_refused` (zero HTTP on bad provenance) is the red-line tooth — it
   ships in the SAME commit as the send path, never later. Acceptance: both green.
7. **T7 (H4).** `read_hub_inbox` + `draft_reply` tools via P42's catalog growth rule;
   `h4_agent_cannot_transmit` grep/compile gate; full suite run at `AiMode::Off`.
   Acceptance: mode-off suite green with tools absent.
8. **T8 (close-out).** Carried B1–B6 verified against their original wording (shared doc
   §3.4); `hub_no_shadow_store` review-gate run; §5's four ledger rows appended; benches
   recorded. Do not mark P48 done if any adversarial was weakened, `#[ignore]`d, or
   inverted.

**Stop-and-flag conditions (do not improvise past these):** (i) any edit to
`BLUEPRINT-P43-external-integration-ports.md` or any channel-transport code (P43's lane,
possibly under parallel edit); (ii) any new wire event variant (P34 anti-scope); (iii) any
price/amount field appearing in parser/intent types; (iv) any review write/moderate/
auto-respond capability, or a reply path without owner-cert provenance (red-line §1.4-8);
(v) a P48-side customer-preference store (P49's lane); (vi) any DOM surface (the ruling is
final); (vii) a central aggregation endpoint of any kind (§1.5).
