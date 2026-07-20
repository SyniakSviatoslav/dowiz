# BLUEPRINT P48-INTAKE — Omnichannel order-intake adapters: Telegram / WhatsApp / SimpleX / Instagram / Facebook / Web, with Linktree as a generated discovery surface (2026-07-20)

> **Status: BLUEPRINT / PLAN — no code written, nothing built.** This document builds out the
> intake lane of **P48** (`docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P48-owner-hub-surface.md`),
> the reserved inbound-intake slot, as the mirror of **P43** outbound send
> (`docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P43-external-integration-ports.md`). It reuses
> both blueprints' vocabulary verbatim — it does not rename anything they already named.
>
> **Product constraint governing every section (operator framing, this session):** dowiz is
> "simple / secure / configurable for the food owners needing a digital delivery service" — a
> small, non-technical restaurant or shop owner in the Albania/EU launch market (DECISIONS.md
> D12). This is **not** a general-purpose omnichannel CRM. Every channel below is judged first
> on: *can such an owner actually obtain and operate it?*
>
> **Honesty up front:** of the four tasks in this batch, this is the most infrastructure-heavy
> remaining gap. Payment's hard part (webhook verification, replay window, normalized trusted
> event) is already built in `kernel/src/ports/payment_provider.rs`. This task's hard parts —
> a public webhook-receiving surface, per-platform signature verification, per-provider
> idempotency, and Meta's deniable review gates — do not exist anywhere in the tree. Repo-wide
> grep for `getUpdates|setWebhook|x-hub-signature|hub.challenge` returns zero hits.

---

## 0. Ground truth — what exists and what does not (verified against the live tree, 2026-07-20)

| # | Fact | Cite | Consequence |
|---|---|---|---|
| 1 | `kernel/src/messenger.rs` is **outbound-only** deep-link string construction — `telegram_link()`, `whatsapp_link()`, `viber_link()`, `normalize_phone()`. "Contact/link construction only — it never sends." No DOM, no network. | `kernel/src/messenger.rs:12,33,39,51` | Reused as-is for the Linktree-style discovery surface (§6, Lane L). Zero inbound role. |
| 2 | `ChannelKind { Telegram, Sms, WhatsApp, SimpleX, Email }` — 5 variants, no Instagram/Facebook/Web/Linktree. `CustomerRef { channel, peer, order_refs }`; "There is NO durable customer identity (P49 deferral)"; erasure scoped by `(ChannelKind, peer)` with crypto-erasure (`register_key`/`redact_peer`/`key_alive`). | `kernel/src/ports/owner_surface.rs:99-114,721-797` | Any new intake channel MUST be erasable through this exact closure (§4, §5.6). |
| 3 | `kernel/src/ports/notification.rs` is outbound-only send fabric (`MessengerProvider { Telegram, WhatsApp, SimpleX }`, `Notifier` fan-out, dead-token eviction, retry taxonomy). No receive path. | `kernel/src/ports/notification.rs` | The send direction is owned; this blueprint adds only the receive direction. |
| 4 | The order FSM is **channel-agnostic**: `Order.channel: Option<String>` is free-form carried metadata; `place_order()`/`place_order_priced()` store and log it but never branch the transition table on it. | `kernel/src/order_machine.rs`, `kernel/src/domain.rs` | Channel normalization is purely an ingestion-layer concern. The kernel needs **no FSM change** for any channel in this document. |
| 5 | `tools/native-spa-server` has **no external-webhook infrastructure**: routes are `/healthz`, `/api/order`, `/api/order/{id}`, `/api/order/{id}/advance`, `/api/agent`, every route gated by dowiz's own internal capability-cert header. | `tools/native-spa-server` | The webhook receive surface is net-new (§5.4). |
| 6 | The proven template for "external signature → internally-trusted normalized event": `fn verify_webhook(&self, raw: &[u8], sig: &WebhookHeaders) -> Result<PaymentEvent, PayError>`, `WebhookHeaders { sig, ts }`, `WEBHOOK_TS_TOLERANCE_S = 300`, plus a per-provider `seen` dedup set inside `verify_webhook_local`. Verify at the adapter edge; the kernel never sees the raw external format. | `kernel/src/ports/payment_provider.rs:222,231,244,712-775` | Reused, not reinvented, as the shape of every channel adapter (§5.3). |
| 7 | The canonical normalization precedent: `engine/src/intent.rs` `RawInput{Pointer,Key,VoicePhrase,Gesture}` → pure `IntentClassifier::classify` → `Intent{...}` — many input shapes, one downstream vocabulary, enforced by a grep-gate. | `engine/src/intent.rs` | The intake layer mirrors this shape: many message formats → one `InboundMessage` → one `place_order*` call. |
| 8 | P48's blueprint already pre-defines the intake vocabulary — planning-stage only, **zero code exists**: `InboundMessage { venue_id, channel, sender, provider_msg_id, text, unix_ms }`, `IntentOutcome { Order(OrderIntent), Ambiguous(Ambiguity), NotAnOrder }`, `OrderIntent` (prices ABSENT by design — `compute_order_total` is the only price authority), pure deterministic `trait IntentParser`. | `BLUEPRINT-P48-owner-hub-surface.md` §2 | This document adopts these types verbatim. It does **not** mint an `IntakeEvent` synonym — one concept, one type. |
| 9 | The recorded three-way boundary: P43 owns channel **transport** both directions ("inbound webhook/sidecar receive surfaces, provider auth, window ledgers"); those receive surfaces "land on P37's HTTP receive surface"; P48 owns the seam AFTER receive — `InboundMessage` → intent → fold — and "adds zero HTTP clients, zero bot-API code, zero webhook parsing". P43's own table: "P43 is outbound-only for messaging; ALL inbound channel intake (orders arriving FROM messengers/social/web-forms) belongs to P48's hub." | `BLUEPRINT-P48-owner-hub-surface.md` §1.2; `BLUEPRINT-P43-external-integration-ports.md:71` | §5's architecture is assigned to lanes accordingly — nothing in this document re-litigates that boundary. |
| 10 | Naming collision guard: `kernel/src/intake.rs` is an **unrelated constraint-solving compiler**. The port referenced in the live comment at `kernel/src/ports/owner_surface.rs:3` ("mirrors `ports/hub_intake.rs` discipline") **does not exist yet**. | live tree | The new kernel port lands at `kernel/src/ports/hub_intake.rs`, never touching `kernel/src/intake.rs`. |
| 11 | Existing Telegram infra (`tools/telemetry/lib.sh` `tg_send`, the `topics` binary) is outbound-only ops alerting to fixed chat/topic ids — fenced off in P43 as "OPS plumbing, not a product channel." | `tools/telemetry/lib.sh` | Not reused, not extended. Product intake is a separate lane. |

---

## 1. Channel verdicts and achievability tiers

Judged strictly against the small-non-technical-owner constraint. Tier assignments carry the
research findings without softening.

| Tier | Channel | Verdict | Why |
|---|---|---|---|
| **0** | **Website / storefront** | Already dowiz's native channel. Zero integration needed. | `storefront.rs`'s `JourneyStep` FSM + `native-spa-server`'s cap-gated `/api/order` is the existing path. Work here is tagging only (§6 Phase 1). |
| **1** | **Telegram** | Realistically self-serve. Build first. | @BotFather `/newbot` is trivial and free forever, no fee, no review. Inbound via `setWebhook` (HTTPS push) or `getUpdates` (long-poll). Webhook auth = `secret_token` echoed in `X-Telegram-Bot-Api-Secret-Token` (constant-string compare). Deep-linking `t.me/<bot>?start=<payload>` (≤64 chars, `A-Za-z0-9_-`, base64url) gives clean per-hub/per-item entry from a QR or button. Inline keyboards + `callback_query` support tappable menus later. dowiz's only job is hosting the webhook receiver. |
| **1.5** | **SimpleX** | dowiz-facilitated, already designed elsewhere — **cross-referenced, not redesigned here.** | `docs/design/MEDIA-COMMS-AGENTIC-AUTONOMY-SYNTHESIS-2026-07-20.md` §4 Leg B: per-order single-use invitation/QR bootstrap, per-contact address, PQ-hybrid ratchet; sidecar (`simplex-chat` CLI over localhost WebSocket, P43 item E-h extended to bidirectional) vs native-ratchet is an open operator decision recorded there (§10.5). Zero vendor account, but a real per-hub sidecar/relay operational cost — dowiz-facilitated, not owner-configured. `ChannelKind::SimpleX` already exists. |
| **2** | **WhatsApp** | dowiz-facilitated only. Not self-serve — do not pretend otherwise to owners. | Only the Cloud API remains (On-Premises client expired Oct 2025). Requires: Meta Business Portfolio + developer app + WhatsApp Business Account + a DEDICATED phone number (not a consumer WhatsApp number) + **Business Verification — document review, days-to-weeks, CAN BE DENIED** + display-name approval. The free consumer WhatsApp Business APP has no API — a different product. Business-initiated messages need pre-approved templates; a customer message opens a 24-hour free-reply service window; marketing/auth messages always charged. The free `wa.me` deep link `messenger.rs` already builds is a legitimate lightweight WhatsApp *presence* needing none of this — but it opens the consumer app; it does **not** ingest orders into dowiz. Both truths are shown to the owner, clearly separated (§6 Phase 3). |
| **3** | **Instagram + Facebook Messaging** | **Deferred.** Out of reach for v1 self-serve; gated on an unmade operator decision (§7-D2). | Shared Meta Graph webhook infra (same `hub.challenge` + `X-Hub-Signature-256` as WhatsApp — the adapter code is shared when this unblocks). The real gate: Standard Access only lets an app message its OWN connected accounts; messaging the public needs Advanced Access = App Review + Business Verification — a genuine multi-week, **deniable** review process. Near-term reality for an owner is manual DM handling (no API) or an approved third-party BSP; neither is "owner flips a switch". |
| — | **Linktree** | **NOT a channel. Explicit design-error guard.** | Linktree is a link-in-bio aggregator: one hosted page of buttons performing outbound redirects only. **No inbound-message API exists — a customer cannot send an order INTO Linktree.** Its buttons deep-link OUT to real channels (a `wa.me` link, a `t.me` link, a storefront URL). It belongs on the discovery/link side — exactly what `messenger.rs` already builds — never the message-ingest side. See §4's enum guard and §6 Lane L. |

---

## 2. What P48-INTAKE is and is not

**Is:** the inbound half of the channel story — per-platform webhook receipt, signature
verification, idempotent dedup, and normalization into the one `InboundMessage` →
`IntentParser` → `place_order*` pipeline, plus generation of the owner's discovery surface
(link/QR page) from the channels that are actually live.

**Is not:**
- Not a CRM, not a unified team inbox product, not chat history sync, not marketing broadcast.
- Not cross-channel identity resolution. The omnichannel-inbox research finding is adopted as a
  law: the same person has a different opaque id per platform; mature tools model each as a
  per-channel handle and never silently infer a merged identity. This matches the existing
  `CustomerRef` design ("channel-address + order-ids is the only handle") and the repo's
  no-scoring/signed-capability trust model. **No merge heuristics, ever.** If explicit customer-
  initiated linking is ever wanted, that is P49's lane, not this one.
- Not a new send path — outbound stays P43's (`ChannelSend`, `notify-adapters`), full stop.
- Not an FSM change — fact 0.4: the kernel's transition table never branches on channel.

**Local-first invariant fit:** receiving inbound webhooks from Telegram/Meta is an edge-adapter
concern of the same already-accepted class (llm-adapters ↔ Ollama, `payment_provider`'s live
webhook, P43's own outbound adapters). The kernel only ever sees a normalized, verified,
`place_order*`-shaped event — never a raw webhook, never a provider SDK type.

---

## 3. Canonical vocabulary — adopted, not invented

From `BLUEPRINT-P48-owner-hub-surface.md` §2, verbatim (planning-stage; this document is the
build plan for it):

- `InboundMessage { venue_id, channel, sender, provider_msg_id, text, unix_ms }` — one
  already-received inbound message, channel-agnostic. `provider_msg_id` is the channel's own
  delivery id — the per-channel dedup key.
- `IntentOutcome { Order(OrderIntent), Ambiguous(Ambiguity), NotAnOrder }` — closed set; typed
  uncertainty, never a silent guess.
- `OrderIntent { venue_id, items, delivery_addr, reply_to }` — **prices absent by design**; the
  menu fold (`compute_order_total`, `kernel/src/domain.rs`) is the only price authority. A
  customer message cannot name a price.
- `trait IntentParser` — pure, deterministic, menu-anchored, total (classifies, never fails).

These land in `kernel/src/ports/hub_intake.rs` (the exact path the live comment at
`owner_surface.rs:3` already names), zero-I/O firewall discipline: no HTTP, no serde, no
provider names. Guard (fact 0.10): this is **not** `kernel/src/intake.rs`, which is an
unrelated constraint-solving compiler.

This mirrors the proven `engine/src/intent.rs` shape — many input formats, one downstream
vocabulary — and the same grep-gate discipline applies: no provider payload type name
(`Update`, `WebhookEntry`, …) may appear outside the adapter crate.

---

## 4. `ChannelKind` extension — the concrete enum ruling

Current (live, `kernel/src/ports/owner_surface.rs:99-105`):
`ChannelKind { Telegram, Sms, WhatsApp, SimpleX, Email }`.

1. **Phase 1 (Telegram) and Phase 1.5 (SimpleX) need zero enum change.** Both variants already
   exist. WhatsApp likewise exists for Phase 3.
2. **`Instagram` and `Facebook` variants are specified here but added only in the same diff
   that lands their intake adapter** (Phase 4, currently deferred). Rationale: `ChannelKind` is
   the owner's *erasure key* — a variant with no producer creates dead erasure-key space and a
   false affordance ("the enum says we support Instagram"). Adding a variant is a reviewed
   kernel-ports diff (the discipline P48's blueprint already states for `ReviewSource`).
3. **Web gets NO `ChannelKind` variant in this blueprint.** The website is dowiz's native path
   with its own identity flow (storefront `JourneyStep` FSM → cap-gated `/api/order`; customer
   identity is P49's deferred lane). `CustomerRef` is channel-shaped-address + order-ids; a web
   session has no messenger-shaped peer address to key erasure by. Web orders carry
   `Order.channel = Some("web")` metadata (fact 0.4 — opaque, never branched on). If P49 later
   creates a durable web-customer handle, whether that handle warrants a `ChannelKind::Web`
   erasure closure is P49's decision — recorded as a hook, not built here (§7-D4).
4. **`Linktree` is NEVER a `ChannelKind` variant — stated as a hard guard.** Linktree is a
   discovery surface dowiz *generates buttons for* (outbound redirects to real channels), never
   an intake source. There is no inbound-message API to adapt; a variant would be
   unfalsifiable dead code and would encode the exact conflation the research flagged as a
   design error. Any future PR adding `ChannelKind::Linktree` is wrong by definition and this
   section is the citation to reject it with.
5. **Canonical channel-string table.** `Order.channel: Option<String>` is free-form; the intake
   layer is the only writer, from one constant table (`"telegram"`, `"whatsapp"`, `"simplex"`,
   `"web"`, `"instagram"`, `"facebook"`) so grep and analytics have one spelling. Not an enum in
   the FSM — the FSM stays channel-agnostic.

---

## 5. Normalization architecture

### 5.1 Crate layout — one shared adapter crate, per-channel modules

**Decision: ONE crate, `intake-adapters/`, with per-channel modules — not a crate per channel.**

- `intake-adapters/src/telegram.rs` — feature `telegram`
- `intake-adapters/src/meta.rs` — shared Meta verification core (GET `hub.challenge` +
  `hub.verify_token` handshake; POST `X-Hub-Signature-256: sha256=<HMAC-SHA256(raw_body,
  app_secret)>`, constant-time compare) — feature-gated per consumer:
  `whatsapp`, and later `instagram` / `facebook` reuse this one module
- `intake-adapters/src/simplex.rs` — sidecar bridge per the media/comms synthesis (feature `simplex`)
- no `web.rs` — the website path never leaves the native surface (§4.3)

Justification (repo precedent, not taste): P43 specs a single `notify-adapters` crate;
`llm-adapters` is one crate with Ollama/vLLM/managed-API backends; `agent-adapters` is one
crate. One crate per *lane* with per-provider feature-gated modules is the established
convention. A crate-per-channel would multiply Cargo.toml/CI/cargo-deny surface (this repo has
no workspace — every crate is a separate build) while buying no isolation the compile firewall
doesn't already provide. The decisive technical point: WhatsApp, Instagram, and Facebook share
one verification scheme — three crates would either duplicate the HMAC code or grow a fourth
shared crate; one `meta.rs` module is strictly simpler.

**Compile firewall (mirrors the P40 agent-lane discipline):** `intake-adapters` does **not**
import `dowiz-kernel` mutation symbols. It produces `InboundMessage` values and hands them to
the intake service (§5.4), which is the only component holding order-placement authority. An
adapter structurally cannot call `place_order`.

Feature discipline: every module off-by-default, header comment stating what it pulls in
(HMAC-SHA256 for `meta.rs` — preferring the kernel's existing hash/crypto surface over a new
external dep; a DECART rationale if any new dep is genuinely unavoidable), and the standard
`cargo tree -e no-dev` verification line proving the default build stays clean.

### 5.2 Per-channel edge verification — the `payment_provider` template, reused

Every adapter implements the same shape as
`payment_provider::verify_webhook(&self, raw: &[u8], sig: &WebhookHeaders) -> Result<PaymentEvent, PayError>`
(fact 0.6), specialized per platform:

```rust
// intake-adapters — shape, per channel module (illustrative signature, not code landed)
fn verify_and_normalize(&self, raw: &[u8], hdrs: &IntakeWebhookHeaders)
    -> Result<Vec<InboundMessage>, IntakeError>;
```

| Channel | Verification at the edge | Dedup key |
|---|---|---|
| Telegram | `X-Telegram-Bot-Api-Secret-Token` header equals the per-hub `secret_token` set at `setWebhook` — constant-string compare | `update_id` |
| WhatsApp / Instagram / Facebook (shared `meta.rs`) | GET `hub.challenge` + `hub.verify_token` handshake at subscription; every POST: `X-Hub-Signature-256` = `sha256=` HMAC-SHA256 over the **raw body** keyed with the app secret, constant-time compare | WhatsApp `messages[].id`; Graph message ids for IG/FB |
| SimpleX | No webhook — sidecar/native ratchet per `MEDIA-COMMS-AGENTIC-AUTONOMY-SYNTHESIS-2026-07-20.md` §4 Leg B; authenticity comes from the ratchet channel itself | per-message id from the sidecar event |

Rules carried from the payment template:
- **Verify before parse.** Signature failure returns a typed reject; the body is never
  deserialized, no kernel-ward call is made, and the HTTP response is a non-retry-inducing 200/401
  per each platform's documented retry semantics (returning errors forever makes providers
  retry forever — at-least-once delivery is documented behavior, not an edge case).
- **Replay window.** Where the provider supplies a timestamp, apply the
  `WEBHOOK_TS_TOLERANCE_S = 300` posture (`payment_provider.rs:244`).
- **Idempotency before processing.** A persisted per-`(channel, provider_msg_id)` seen-set —
  the same discipline as `verify_webhook_local`'s `seen` set — with a bounded retention window
  sized to the provider's documented retry horizon. Duplicate → acknowledge, drop, count. This
  is universal (Chatwoot/Twilio/Respond.io all do it) because every provider retries on
  non-2xx/timeout.

### 5.3 Trust reconciliation — external signature vs internal capability-cert

The question posed: `/api/*` is gated by dowiz's own capability-cert header; webhooks are
externally signed, not capability-cert-signed. Reconciliation, mirroring how
`payment_provider` normalizes to a trusted `PaymentEvent`:

**Two credentials, two questions, never conflated.**
1. The **external signature** (secret-token / HMAC) answers only: *did this platform really
   send this bytes-for-bytes payload?* It authenticates the PLATFORM at the edge.
2. **Order-placement authority** is the intake service's own, separately provisioned,
   **scoped capability cert** — scope: place-order only, and under
   `RedLinePolicy::DenyByDefault` (`kernel/src/ports/agent/scope.rs`) it structurally cannot
   touch ledger/money/auth/migrations.

So yes: after verifying the external signature and normalizing to `InboundMessage` →
`OrderIntent`, the intake service places the order **through the same cap-gated path as any
other client**, presenting its own cert. A webhook adapter never "inherits" trust from the
external signature and never bypasses the internal gate; equivalently, a valid Telegram
signature grants exactly one power — to put a message into the parse pipeline — and nothing
else. If intake runs in-process with the kernel (single-box hub), the same boundary is enforced
by construction instead: `InboundMessage` has a private constructor only the verifying adapter
path can reach, exactly how `PaymentEvent` is only obtainable through `verify_webhook`.

### 5.4 Where the HTTP receive surface lives

The repo's own recorded precedent already leans one way: both P43 and P48 blueprints state
inbound webhook surfaces "land on P37's HTTP receive surface" — P37 being the thin wire shell
that `tools/native-spa-server` implements. **Recommendation: extend `native-spa-server` with a
new, clearly-partitioned route family** — `POST /webhook/{channel}/{hub_id}` (plus the Meta GET
handshake on the same path) — where `{channel}` selects the adapter and `{hub_id}` selects the
per-hub secret material. The `/api/*` capability-cert gate is untouched; `/webhook/*` is the
only route family whose gate is an external-signature check instead.

The honest tradeoff, since this is operator decision D3 (§7), not an assertion:

- **Extend `native-spa-server`:** one deployable, one TLS story, matches the local-first
  single-box hub posture and the P37 precedent; but a webhook-parser bug shares a process with
  the cap-gated order API, and public exposure of the whole binary's attack surface grows.
- **Separate small edge service:** process isolation (parser bug ≠ order-API compromise),
  independently restartable/rate-limitable, could sit on different network exposure; but it is
  a second deployable a small-owner hub must run and keep updated, a second TLS termination,
  and it contradicts the "hub is one box" simplicity constraint — and it *still* needs the
  scoped capability cert of §5.3 to reach `/api/order`, so the trust model is identical either
  way.

Recommendation stands with the P37 precedent (extend), with the route family firewalled at the
router level: `/webhook/*` handlers can only construct `InboundMessage`, never reach order
handlers directly.

Public-HTTPS reality, stated plainly: Telegram `setWebhook` and Meta both require a publicly
reachable HTTPS endpoint. A hub behind NAT needs the already-operational Cloudflare tunnel
pattern (the OpenBebop webhook precedent: `webhook.dowiz.org`) or Telegram's `getUpdates`
long-poll fallback, which needs **no** inbound reachability at all. The Telegram adapter
therefore supports both transports behind the same `verify_and_normalize` seam — long-poll is
the zero-infrastructure degraded mode that keeps reliability-over-latency honest.

### 5.5 The full pipeline, end to end

```
platform POST /webhook/{channel}/{hub_id}
  → [edge] signature verify (constant-time)          — reject ⇒ typed error, zero kernel calls
  → [edge] dedup by (channel, provider_msg_id)       — duplicate ⇒ ack + drop
  → [adapter] parse provider payload → InboundMessage (provider types die here)
  → [P48 port] IntentParser::parse(msg, menu) → IntentOutcome
       Order(intent)   → intake service, holding its scoped cap-cert:
                          place_order* (price from compute_order_total, NEVER from message text)
                          + CustomerRef{channel, peer} linkage + Order.channel tag
       Ambiguous(a)    → hub inbox pane (owner resolves; optional AI-assist per P48 §3.4 is a
                          consumer of Ambiguous, never an IntentParser implementor)
       NotAnOrder      → hub inbox pane
  → confirmation reply rides P43's outbound ChannelSend on the SAME channel (reply_to law)
```

### 5.6 Erasure integration (non-optional)

Every intake channel that stores a peer address plugs into the existing crypto-erasure closure:
`register_key(channel, peer, key)` on first contact, so `CustomerErasureAction` →
`redact_peer`/key destruction works identically for a Telegram chat id, a WhatsApp wa-id, or a
SimpleX per-order address. A channel adapter that cannot be erased through
`(ChannelKind, peer)` does not ship. This is an acceptance criterion in every phase (§6).

---

## 6. Phased build order

Ordering follows the achievability tiers, cheapest-real-value first. Lane L is independent and
can land any time after Phase 1 defines which channels are live.

### Phase 1 — Telegram + Website (self-serve, free, near-zero owner friction)

Scope:
- `kernel/src/ports/hub_intake.rs` lands with the §3 vocabulary (zero-I/O, pure parser).
- `intake-adapters/` crate with `telegram.rs` only: secret-token verify, `update_id` dedup,
  `setWebhook` push + `getUpdates` long-poll behind one seam, `/start <payload>` deep-link
  decode (≤64 chars, base64url hub/item id), payload → `InboundMessage`.
- `native-spa-server`: `/webhook/telegram/{hub_id}` route family (per §5.4 recommendation,
  pending D3).
- Intake service with scoped place-order-only capability cert (§5.3).
- Website: no new intake code — verify the existing storefront → `/api/order` path and add the
  `Order.channel = Some("web")` tag from the §4.5 string table.
- Owner onboarding flow: paste-your-BotFather-token, dowiz sets the webhook + secret for them.

RED→GREEN acceptance (each starts as a failing test):
1. **Happy path:** a captured real Telegram `Update` payload (fixture from a live bot) POSTed to
   the webhook route → exactly one `InboundMessage` → `IntentParser` → exactly one order exists
   in the FSM in `Placed`, priced by `compute_order_total`, `channel == Some("telegram")`,
   `CustomerRef{Telegram, chat_id}` linked.
2. **Idempotency:** the same payload POSTed twice (same `update_id`) → exactly one order; the
   second request acknowledged, counted as duplicate.
3. **Signature tamper:** correct payload, wrong/absent `X-Telegram-Bot-Api-Secret-Token` →
   rejected; assert zero kernel-ward calls occurred (not merely a 4xx).
4. **Price injection:** message text naming a price ("2 pizzas for 1 lek") → order total comes
   from the menu fold, message price provably ignored (exact-integer assertion).
5. **Ambiguity honesty:** unknown item / no quantity → `Ambiguous`, zero `place_order` calls.
6. **Deep link:** `t.me/<bot>?start=<payload>` with a >64-char or invalid-charset payload →
   builder refuses at construction; valid payload round-trips to the right hub/item.
7. **Erasure:** place an order via Telegram, erase `(Telegram, chat_id)` → `redact_peer` output
   redacted, `key_alive == false`, order chain integrity intact.
8. **Channel-agnostic FSM (wire test for fact 0.4):** identical order placed with
   `channel = "telegram"` vs `"web"` produces byte-identical fold results apart from the
   metadata field.
9. **Web tag:** storefront order → `Order.channel == Some("web")`, no other behavior change.

### Phase 1.5 — SimpleX (dowiz-facilitated; designed elsewhere, wired here)

Scope: extend P43 E-h's sidecar (or native ratchet, per the open decision in
`MEDIA-COMMS-AGENTIC-AUTONOMY-SYNTHESIS-2026-07-20.md` §10.5) from outbound-only to
bidirectional; sidecar events → `InboundMessage` through the same pipeline. No webhook, no
public endpoint — this channel's transport is the ratchet.

Acceptance: that document's Leg-B DoD is the acceptance set (single-use invitation — second
redemption fails; channel closed after `Delivered` — post-close send rejected; no durable
customer identity created; KAT-gated primitives only if native), **plus** this document's
pipeline criteria: inbound order round-trip through the sidecar harness → exactly one FSM
order; duplicate sidecar event → one order; erasure over `(SimpleX, per_order_addr)` green.

### Phase 3 — WhatsApp (dowiz-facilitated, Tier 2 — named shepherding, honest costs)

Scope: `intake-adapters/src/meta.rs` (handshake + HMAC verify, constant-time) + WhatsApp
payload mapping (`messages[].id` dedup) → same pipeline; 24-hour service-window ledger for the
reply path (P43's window-ledger lane); template management stays P43/outbound.

**The shepherding, named — this is a real per-owner support-cost commitment, not a switch:**
dowiz staff walk the owner through Meta Business Portfolio creation, WhatsApp Business Account
setup, dedicating a phone number (which is consumed — it cannot remain a consumer WhatsApp
number), submitting **Business Verification (document review, days-to-weeks, and it CAN BE
DENIED — the flow must surface "denied" as a first-class terminal outcome to the owner, not an
error)**, display-name approval, and initial template approval. Per-message template fees are
disclosed to the owner before opt-in. Whether dowiz commits to this concierge lane at all is
operator decision D1.

RED→GREEN acceptance: Phase-1 criteria 1-5/7 transposed (fixture = captured Cloud API webhook;
tamper = bit-flipped `X-Hub-Signature-256`; dedup = replayed `messages[].id`), plus: GET
handshake echoes `hub.challenge` only on `hub.verify_token` match; a webhook replay outside the
timestamp tolerance is rejected; a forged/unverified inbound does NOT open a 24-hour window.

### Phase 4 — Instagram + Facebook (deferred; explicitly gated on D2)

Not scheduled. Blocked on the unmade operator decision: does dowiz pursue Meta App Review +
Advanced Access (multi-week, deniable) at the platform level? If yes, the build cost is small —
`meta.rs` is already shared from Phase 3; the `Instagram`/`Facebook` `ChannelKind` variants land
with the adapters (§4.2). Until then these channels appear on the discovery surface (Lane L) as
plain profile links only, with no claim of order ingestion.

### Lane L — Discovery surface ("Linktree-style" page) — small, cheap, independent

Scope: a generated one-page button set per hub — storefront URL, `t.me/<bot>?start=<hub>` (with
QR), `wa.me` presence link, SimpleX invitation entry point, plain IG/FB profile links — served
by the hub's own web surface. **Pure reuse of `kernel/src/messenger.rs` deep-link builders plus
the Telegram start-param builder from Phase 1. Zero new channel logic, zero inbound code, no
Linktree account, no third-party dependency.** dowiz's role is to GENERATE the button set, and
owners who already have an actual Linktree account can paste these same links into it.

Acceptance: page renders only channels the hub actually has live (a dead button for an
unconfigured channel is the fail condition); every generated link byte-matches the
corresponding builder output; the Telegram button's start payload resolves to the right hub in
the Phase-1 deep-link test; grep-gate proving Lane L introduces no inbound/service code path.

---

## 7. Open operator decisions

| # | Decision | Options and the real cost of each | Blocking |
|---|---|---|---|
| **D1** | Does dowiz shepherd WhatsApp Business Verification for owners? | (a) Concierge lane: dowiz staff time per owner, days-to-weeks latency per onboarding, and dowiz absorbs the "verification denied" support burden. (b) Self-serve/unavailable: WhatsApp remains presence-only (`wa.me` link on Lane L) with an honest "ordering via WhatsApp not available" label. There is no third option — the research is unambiguous that Cloud API onboarding is not self-serve for this owner profile. | Phase 3 |
| **D2** | Does dowiz ever pursue Meta App Review / Advanced Access for Instagram + Facebook messaging? | (a) Pursue: one platform-level multi-week deniable review, after which per-owner onboarding is comparatively light; commits dowiz to Meta platform-policy compliance indefinitely. (b) Defer indefinitely: IG/FB stay discovery-only links on Lane L. | Phase 4 |
| **D3** | Webhook receive surface: extend `native-spa-server` (P37 precedent, one deployable, shared process) or a separate small edge service (process isolation, second deployable)? | Tradeoff spelled out in §5.4; recommendation = extend, per the recorded P37/P43 precedent and the single-box hub constraint. Trust model is identical either way (§5.3). | Phase 1 |
| **D4** | Per-hub owner-owned Telegram bot (owner runs @BotFather, owns the token, dowiz hosts the receiver) vs one dowiz-operated multiplexing bot (deep-link payload selects the hub)? | Owner-owned: sovereignty and blast-radius isolation (one leaked token ≠ all hubs), matches local-first; onboarding = one guided BotFather flow. Multiplexed: zero owner steps, but a single token becomes a platform-wide secret and every hub's traffic transits one bot identity. Recommendation: owner-owned. | Phase 1 |
| **D5** | `ChannelKind::Web` — never, or revisit when P49 creates a durable web-customer handle? | §4.3's recommendation: no variant now; recorded as a P49 hook. Deciding "never" instead closes the hook. | none (documentation only) |
| **D6** | SimpleX transport: `simplex-chat` CLI sidecar vs native PQ-hybrid ratchet. | Already an open decision in `MEDIA-COMMS-AGENTIC-AUTONOMY-SYNTHESIS-2026-07-20.md` §10.5 — cross-referenced, not duplicated; whichever is ruled there binds Phase 1.5 here. | Phase 1.5 |

---

## 8. Anti-scope (closed list)

1. No cross-channel identity merging, heuristic or otherwise (§2).
2. No rating/ranking/reputation of any participant — intake volume per channel is operational
   telemetry, never a score (no-courier-scoring gate discipline extends here).
3. No new outbound send path — P43 owns transmission.
4. No FSM/transition-table change; no new kernel event vocabulary (P34's "no new event
   variants" anti-scope binds — intake maps into the existing order-placement pipeline).
5. No `ChannelKind::Linktree`, ever (§4.4).
6. No provider SDK types outside `intake-adapters` (grep-gated, per the `engine/src/intent.rs`
   precedent).
7. No LLM in the intake trust path — `IntentParser` is pure and deterministic; AI-assist only
   consumes `Ambiguous` outcomes under P41's `AiMode::Off`-works invariant.
8. No touching `kernel/src/intake.rs` (unrelated constraint compiler — naming collision only).
9. No reuse of `tools/telemetry` Telegram plumbing for product intake (fact 0.11).

---

## 9. Honest difficulty statement

Payment's equivalent task inherited a built `verify_webhook` core; this task builds the
receiving side from zero: the first public externally-signed HTTP surface in the tree, two
distinct verification schemes, per-provider idempotency stores, a public-HTTPS/tunnel story for
NAT'd hubs, and — for everything past Tier 1 — review gates owned by Meta that dowiz cannot
accelerate and that can end in denial. Phase 1 is deliberately scoped so that the largest
self-serve win (Telegram + Website + the discovery page) requires none of the deniable gates
and no per-message fees, and every later phase is additive behind the same single
`InboundMessage` seam.

---

*Cross-references: `BLUEPRINT-P48-owner-hub-surface.md` (vocabulary §2, boundaries §1.2, B5),
`BLUEPRINT-P43-external-integration-ports.md` (outbound port, boundary line, window ledger),
`BLUEPRINT-P37-order-http-surface.md` (HTTP receive surface), `kernel/src/ports/payment_provider.rs`
(verification template), `kernel/src/ports/owner_surface.rs` (`ChannelKind`/`CustomerRef`/erasure),
`kernel/src/messenger.rs` (deep-link builders), `engine/src/intent.rs` (normalization precedent),
`MEDIA-COMMS-AGENTIC-AUTONOMY-SYNTHESIS-2026-07-20.md` §4 (SimpleX design), DECISIONS.md D12
(launch market).*
