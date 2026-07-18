# OPUS-R4 — Order-Flow, Courier-Matching & Notification-Adapter Research

**Date:** 2026-07-18
**Scope:** Wave-0 buildable design research feeding the Tier-3 blueprint for dowiz/DeliveryOS.
**Method:** Six parallel web-research lanes (real, current sources — 2025/2026 crate freshness verified
on crates.io / docs.rs / GitHub) reconciled against the live kernel source and the operator's
2026-07-18 roadmap decisions.
**Binding context (not re-litigated here):** `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`
§16.3, §16.5, §16.14, §16.22, §16.23/§16.47, §16.49, §16.52, §16.53, §16.60, §17.6.

Rust-native only (no Node/TS, ever). Every external service is a **port with swappable adapters**
(the standing `IP-*` integrations-ports convention). Nothing central to dowiz stores order state,
tokens, or customer data.

---

## 0. Codebase ground-truth (what already exists, verified this pass)

Two of the six questions are already answered *in code* and only needed external confirmation, not
design:

- **HRW courier matcher — implemented.** `/root/bebop-repo/bebop2/proto-cap/src/matcher.rs`.
  `hrw_weight(order_id, courier_pubkey)` = FNV-1a over the 40-byte `order_id.le ‖ pubkey`;
  `assign()` sorts weight-DESC, tie-breaks pubkey-ASC, truncates to `max`, returns the ranked
  candidate list. Top entry = primary, remainder = deterministic fallbacks. `CourierKey` is a bare
  32-byte Ed25519 key — the `Courier` struct **structurally cannot** carry a score/rating/rank
  field, and CI gate `scripts/ci-no-courier-scoring.sh` locks that. `primary_for()` +
  requeue-never-drop invariant already coded and tested (`r_mesh05_*`).
- **Unified order state machine — implemented.** `/root/dowiz/kernel/src/order_machine.rs`.
  `OrderStatus` is one enum; the legal-transition table already reads
  `Ready => [InDelivery, PickedUp, Refunding]`, i.e. delivery (`Ready→InDelivery→Delivered`, with
  dispatch) and pickup/dine-in (`Ready→PickedUp`, no dispatch) are **already one machine that
  branches at `Ready`**. §16.60's "pickup is the same flow minus the dispatch step" is not a new
  design — it is the existing code.

The research below confirms both against industry practice and surfaces the real edge cases; the
remaining four questions (push, SMS/email, offline-draft, Signal-style transfer) are genuine
new-build design.

---

## 1. Web Push / APNs / FCM — native Rust, hub-owned tokens (§16.22)

**Requirement:** each hub pushes to APNs/FCM/Web-Push endpoints **directly** and stores its own
subscription tokens locally. No dowiz-central token store, no third-party push relay.

### 1.1 There is no unified Rust crate — integrate three channels behind one port

No server-side Rust crate abstracts Web-Push + APNs + FCM together. (`tauri-plugin-notification` is
*client-side receive*, not hub-side send — not applicable.) This is fine and expected: it maps onto
the existing port convention.

```
trait PushSender { async fn send(&self, token: &Token, payload: &Payload) -> Result<PushOutcome, PushError>; }
```
with three adapters:

| Channel | Crate | Latest / freshness | Verdict |
|---|---|---|---|
| **Web Push** (RFC 8030 + VAPID 8292 + aes128gcm 8291) | [`web-push`](https://crates.io/crates/web-push) (pimeys/rust-web-push) | **0.11.0**, 2025-02-22, active | **Default pick.** Native VAPID signing + RFC 8188 payload encryption (delegates crypto to Mozilla's `ece`). Pluggable transport (`isahc` default, `hyper` feature). |
| Web Push, pure-Rust-crypto alt | [`web-push-native`](https://github.com/leotaku/web-push-native) | needs manual freshness check | Best fit for a "no C deps" stance (no OpenSSL/`ece`); smaller/less battle-tested. Fallback, not default. |
| **APNs** (token-based ES256 JWT, HTTP/2) | [`a2`](https://crates.io/crates/a2) (now WalletConnect/reown) | **0.10.0**, 2026-06-08, active | **Recommended.** Native async on `h2`/`hyper`; `.p8` token auth auto-renews the ES256 JWT; "millions/day" in production; warns against per-request HTTP/2 connections. |
| **FCM** (HTTP v1 — legacy API dead since Jun 2024) | [`fcm_v1`](https://docs.rs/fcm_v1) (sanath-2024) | **0.3.0**, 2026-05-30, fresh but low adoption | Purpose-built HTTP-v1, OAuth2 via `yup-oauth2`. Type-safe single payload spanning Android/iOS/WebPush. |
| FCM, full-schema alt | [`google-fcm1`](https://docs.rs/google-fcm1) (Byron/google-apis-rs) | 7.0.0+20251212 | Official schema but "looking for a new maintainer," heavy generated boilerplate. Only if full API surface is needed. |

**Do not** use anything targeting FCM's legacy API (`fcm`/`gcm` crates) — Google shut it down June
2024. FCM v1 and APNs token-auth are both plain-enough protocols (bearer-JWT + HTTP/2 POST) that a
hand-rolled adapter over `h2` + `p256`/`ring` (APNs) or `reqwest` + `yup-oauth2` (FCM) is a
reasonable DIY escape hatch if the dependency trees are unwelcome.

### 1.2 Production gotchas (all confirmed)

- **Dead-token handling is uniform:** Web-Push `410 Gone`, APNs `410 BadDeviceToken`, FCM v1
  `UNREGISTERED` → **delete the stored token, never retry.** The `PushOutcome`/`PushError` enum must
  distinguish "retryable transient" from "token is dead, evict it."
- **One persistent HTTP/2 client per hub process** for APNs (and ideally FCM) — reconnecting
  per-send risks Apple flagging DoS. `a2`/`h2`/`hyper` all multiplex.
- **Token lifetimes:** APNs ES256 JWT must rotate ≤60 min (`a2` automates this); FCM OAuth2 access
  tokens expire hourly (cache/refresh via `yup-oauth2`/`gcp_auth`, don't reacquire per message).
- **VAPID keypair:** generate the hub's EC P-256 keypair **once**, persist the private key in the
  hub's local secret store, reuse across all browser subscriptions. This is per-hub identity —
  consistent with §16.22's "each hub owns its own tokens."

Sources: [web-push](https://crates.io/crates/web-push) ·
[a2](https://github.com/WalletConnect/a2) · [fcm_v1](https://github.com/sanath-2024/fcm_v1) ·
RFC 8030/8291/8292.

---

## 2. SMS + Email — mandatory Wave-0 fallback, provider-agnostic (§16.52)

**Requirement:** SMS/email order-status updates for customers with no app / push disabled / on web.
Multi-market, no vendor lock-in → provider-agnostic port + thin swappable adapters.

### 2.1 SMS — hand-roll REST behind a port; skip the stale crates

- **No maintained official Twilio Rust SDK.** The community [`twilio`](https://crates.io/crates/twilio)
  crate is v1.1.0, last published **2024-03-25** — too stale to hard-depend on. Twilio's own
  [Rust tutorial](https://www.twilio.com/en-us/blog/developers/tutorials/send-sms-rust-30-seconds)
  confirms the real pattern: `reqwest` + form body + `.basic_auth(sid, token)` POST to the Messages
  endpoint. ~20 lines; no dependency risk. **Do this.**
- **Multi-market alternatives with clean REST APIs** the same thin adapter can target: **Plivo**
  (deliberately Twilio-shaped API), **Vonage**, **MessageBird/Bird**, **Africa's Talking**, and
  **AWS SNS** (which *does* have an actively-maintained official SDK, `aws-sdk-sns`).
  ([provider comparison](https://knock.app/blog/the-top-sms-providers-for-developers))
- **Self-hosted Android-gateway** (the roadmap already references `httpSMS`): prefer
  **[SMSGate](https://sms-gate.app/)** (Apache-2.0, single-command "Private Mode" deploy, REST/webhooks,
  no external registration) over [httpSMS](https://docs.httpsms.com/) — httpSMS still requires
  Firebase + SMTP + Cloudflare Turnstile to self-host, heavier than it looks. Either is a legitimate
  adapter behind the same port for zero-marginal-cost markets.
- **Provider-agnostic trait — prior art exists but is immature:**
  [`smskit`/`sms-core`](https://github.com/ciresnave/smskit) defines exactly the shape wanted
  (`SmsClient: async send(SendRequest)`, per-provider adapter crates, runtime registry with fallback
  chaining) — but it's a solo project (v0.3.0, ~2 stars). **Use it as a design reference; build the
  `SmsPort` trait in-house**, one thin adapter per provider.

### 2.2 Email — managed API by default, SMTP-direct as opt-in

- **SMTP transport is production-ready as a library:** [`lettre`](https://crates.io/crates/lettre)
  **0.11.22**, last published **2026-05-14**, async (`tokio1`), `rustls` TLS, `dkim` signing feature.
  The *crate* is solid; **deliverability is the risk, not the library.**
- **Deliverability decides the default.** Gmail began **hard-rejecting** non-SPF/DKIM/DMARC mail at
  the SMTP level in **Nov 2025**; Microsoft followed
  ([PowerDMARC, self-hosting email in 2026](https://powerdmarc.com/self-hosting-email/)). Even with
  perfect SPF/DKIM/DMARC/PTR, a **fresh IP has no reputation** and lands in spam until warmed — which
  a newly-provisioned per-tenant Hetzner hub cannot do on day one.
- **Therefore: managed transactional-email API is the shipped default; SMTP-direct via `lettre` is an
  explicit operator opt-in.** API adapters with real Rust support:
  [`aws-sdk-sesv2`](https://crates.io/crates/aws-sdk-sesv2) (v1.124.0, 2026-07-08, official AWS team) ·
  [`resend-rs`](https://crates.io/crates/resend-rs) (v0.28.0, 2026-07-05, official). Postmark /
  Mailgun / SendGrid have no maintained Rust SDK but expose simple JSON/form REST APIs suited to the
  same thin-`reqwest`-adapter pattern.

### 2.3 Recommended notification-adapter architecture (§16.22 + §16.52 unified)

One trait per channel, each hub config-selecting a thin adapter, **all fronted by a single hub-side
`Notifier`** that fans one order-status event out across whatever channels the customer has
registered:

```
trait PushPort  { async fn send(&self, sub: &PushSub,  msg: &StatusMsg) -> Result<(), NotifyErr>; }
trait SmsPort   { async fn send(&self, e164: &str,     msg: &StatusMsg) -> Result<(), NotifyErr>; }
trait EmailPort { async fn send(&self, addr: &str,     msg: &StatusMsg) -> Result<(), NotifyErr>; }

// Adapters (config-selected per hub):
//   PushPort  -> WebPushAdapter(web-push) | ApnsAdapter(a2) | FcmAdapter(fcm_v1)
//   SmsPort   -> TwilioAdapter | PlivoAdapter | VonageAdapter | SnsAdapter | SmsGateAdapter
//   EmailPort -> SesAdapter | ResendAdapter | MailgunAdapter | (SmtpAdapter via lettre, opt-in)
```

- **Defaults:** push (all three sub-channels) + a managed-API email adapter on; SMS on but
  provider-selected by market. SMTP-direct and self-hosted-Android-gateway adapters are opt-in,
  flagged "degraded/reputation-risk."
- **The `Notifier` is the fan-out point:** push is best-effort primary; SMS/email is the mandatory
  fallback (§16.52) — *not* gated on push having failed, because a web/no-app customer never had a
  push channel at all. Channel selection is driven by which contact methods the customer's
  **data-wallet** (§16.23, on-device) chose to hand the hub at checkout, so the hub stores the minimum
  contact info the customer consented to, nothing central.
- **Token/contact storage is per-hub-local** (§16.22 for push tokens; the same for phone/email),
  never dowiz-central.

---

## 3. Offline-draft persistence — hold locally, restore on reconnect (§16.52, §16.14, §16.54)

**Requirement:** on network drop mid-checkout, hold the in-progress cart + wallet-filled fields as a
**local draft**, restore automatically on reconnect, fire payment only when back online. Single
device, single user, single hub — **no multi-writer conflict resolution needed.**

### 3.1 CRDTs are NOT warranted here — this is the load-bearing verdict

[`automerge`](https://crates.io/crates/automerge) (0.7.4, healthy) and
[`yrs`](https://crates.io/crates/yrs) (0.27.2, healthy, 1.75M downloads) are excellent libraries —
but their *entire* value is reconciling **concurrent edits from multiple writers** without a
coordinator. A checkout draft is one device, one user, one order: **"last write" and "only write" are
the same event.** Pulling in a CRDT buys op-history, tombstones, and merge machinery that never fire —
pure liability (binary doc format, bigger dependency, harder debugging). **A plain versioned JSON blob
with last-write-wins is strictly correct here.** CRDTs would only earn their keep if the *same* draft
could be edited concurrently from two devices/tabs and needed automatic reconciliation — explicitly
not this case, and explicitly not something §16.23's on-device wallet requires (device transfer is
one-shot handoff, §6, not live sync).

This matches the standing "simplest correct mechanism" discipline: the CRDT arc elsewhere in the repo
is for *multi-writer* mesh state, not for a single-user checkout draft.

### 3.2 Mechanism — two runtimes, same shape

**Tauri 2.x client:**
- Store the draft with [`tauri-plugin-store`](https://crates.io/crates/tauri-plugin-store) **2.4.3**
  (official JSON KV). **Call `store.save()` explicitly on each meaningful field edit** — do *not*
  trust the 100 ms debounce / graceful-exit autosave, since the exact scenario being defended against
  is a crash/kill mid-checkout. (Reach for `tauri-plugin-sql`/`rusqlite`/`redb` only if the app
  already has a relational local store for other data; don't stand up a second store for one blob.
  `sled` is stalled — avoid.)
- **Outbox loop in native Rust:** `tokio::time::interval` + backoff checking connectivity, flush the
  queued draft/payment via `reqwest` on reconnect. Simpler than the web case — no cross-browser API
  asymmetry.

**Web client (Rust/wasm):**
- Store the draft record in **IndexedDB** via [`idb`](https://lib.rs/crates/idb) or
  [`indexed_db_futures`](https://docs.rs/indexed_db_futures) (note: `gloo-storage` has **no**
  IndexedDB — only localStorage/sessionStorage; the RFCs never landed). Call
  `navigator.storage.persist()` to resist eviction. OPFS is faster for large binaries but is a
  file-stream API and overkill for a small JSON draft.
- **Background Sync is Chromium-only** (Firefox disabled, Safari never implemented), so the reliable
  path is an `online`-event + retry-with-backoff outbox — treat Background Sync as an optional
  fast-path, the `online` listener as the mandatory one.
  ([MDN offline guide](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Offline_and_background_operation))

### 3.3 Idempotency — the actual sharp edge on reconnect

The dangerous case is not "draft not submitted"; it's "payment request maybe went out before the
socket dropped." Handle it exactly as Stripe prescribes
([idempotent requests](https://docs.stripe.com/api/idempotent_requests)):

1. Generate an **idempotency key client-side at draft-creation time**, store it *with* the draft,
   **never regenerate** on resubmit.
2. Give the draft a mini state machine: `Draft` → `PaymentInflight` (set **optimistically the instant
   the payment request is sent**, before any response).
3. On reconnect, **branch on that state**: `Draft` → normal local resume; `PaymentInflight` → **query
   order/payment status by idempotency key first**, never blind-replay — the request may have
   succeeded server-side while the client never saw the ack. This is what prevents a restored draft
   from double-charging.

This dovetails with §16.49 (payment fires client-side, hub sees only a token/confirmation) and
§16.14 (honest client-side status, zero dowiz-central state): the draft and its idempotency key live
on the customer's device; the hub is queried, never a dowiz server.

---

## 4. HRW / rendezvous hashing — production confirmation & edge cases (§16.3, §16.26, §17.6)

The mechanism is settled (see §0). External confirmation and the real dispatch edge cases:

### 4.1 It is a boring, production-grade choice — confirmed

Invented by **Thaler & Ravishankar** (Michigan CSE-TR-316-96, 1996 → *IEEE/ACM ToN* 6(1), Feb 1998,
["A Name-Based Mapping Scheme for Rendezvous"](https://www.microsoft.com/en-us/research/wp-content/uploads/2017/02/HRW98.pdf)).
Core guarantee ([Wikipedia](https://en.wikipedia.org/wiki/Rendezvous_hashing)): when a node fails,
**only the ~1/n keys mapped to it remap, and no keys move between survivors** — which is *exactly* why
the coded "next-ranked candidate on the sorted list" is the correct failover: HRW re-derives precisely
that on removal. Consistent hashing is provably a *special case* of HRW, so this is the more general
mechanism, not a downgrade. Production users: GitHub's **GLB Director** (rendezvous ordering in its
forwarding table, ["only 1/n connections reset" on table
change](https://github.blog/engineering/infrastructure/glb-director-open-source-load-balancer/)),
**Ceph CRUSH** (rendezvous-style RUSH placement), Envoy `ring_hash`/Maglev, Apache Ignite, Twitter
EventBus. For small candidate sets (one venue's couriers) HRW's O(n) lookup is trivial and avoids the
ring's virtual-node tuning.

### 4.2 Real-time dispatch edge cases (and how the current design handles each)

- **Courier goes offline mid-assignment:** advance to the next-ranked candidate on the already-sorted
  list — no rehash of survivors (1/n guarantee). Already how `assign()`'s ranked list works. ✅
- **Timeout-then-reassign:** HRW does **not** define timeout policy — "how long to wait for an accept
  before advancing rank" is an **orchestration-layer** decision layered on top of the deterministic
  list. This is a genuine gap in the current code (matcher returns the order; *nothing yet drives the
  accept-timeout advance*). Flagged for Tier-3.
- **Candidate set changes between attempts (courier joins/leaves):** only orders whose current top
  choice was the joining/leaving courier remap; all other order→courier rankings are stable. Literal
  1/n property. ✅
- **Concentration / "thundering herd":** HRW does **not** inherently prevent a single hot order from
  attracting many couriers polling it, nor a hot-key problem — each key independently picks its own
  primary, so many *distinct* orders balance statistically, but one popular order deterministically
  lands on one primary. Same limitation as consistent hashing. Mitigation is orchestration (only the
  ranked primary is *offered* first; fallbacks are contacted on decline/timeout), not the hash.
- **Ties:** the coded pubkey-ASC secondary sort is standard practice (true ties are hash-collision-rare
  but the deterministic tiebreak keeps the order total & reproducible across nodes). ✅

### 4.3 Two honest notes for the blueprint (not blockers)

- **Weighted HRW (capacity, *not* merit) is available if ever needed.** Formula
  `score = -wᵢ / ln(hᵢ(x))` with `hᵢ(x)` the key-hash normalized to (0,1] and `wᵢ` a capacity weight
  ([Randorithms](https://randorithms.com/2020/12/26/rendezvous-hashing.html),
  [IETF draft-ietf-bess-weighted-hrw-00](https://www.ietf.org/archive/id/draft-ietf-bess-weighted-hrw-00.html),
  [US9571570B1](https://patents.google.com/patent/US9571570B1/en)). This is a *throughput/capacity*
  dial (a courier running two vehicles gets proportionally more assignments), **mathematically
  orthogonal to merit/reputation scoring** — it never compares couriers on performance. It stays
  deterministic from `(order_id, courier_pubkey)`. If capacity weighting is ever wanted it is the
  correct primitive, but it is a *distinct axis* and must not be added silently — it changes the
  `Courier` type's data and would need an explicit no-scoring-boundary review.
- **FNV-1a is adequate here.** It has good avalanche on *opaque* inputs (order_id, 32-byte pubkey) but
  clusters on *sequential* inputs; xxHash/SipHash have measurably better uniformity on structured
  data. Since order IDs and pubkeys are effectively opaque, FNV-1a's weakness is inapplicable. Only if
  order IDs ever became sequential-with-shared-prefix, or couriers became adversaries who can grind
  pubkeys to bias their rank, would SipHash be the safer swap. Note, don't act.

---

## 5. Unified pickup + delivery order machine — confirmed best practice (§16.5, §16.60)

The existing single-machine-with-`Ready`-branch (§0) is **directly consistent with every major
commerce platform surveyed. No platform duplicates whole state machines per fulfillment type.**

### 5.1 Prior art (the recurring shape: type discriminator + one core machine + type-gated dispatch)

- **Square Orders API — textbook match.** A `Fulfillment` object carries a `type` discriminator
  (`PICKUP | SHIPMENT | DELIVERY | IN_STORE`) and **one shared `state` enum**
  (`PROPOSED → RESERVED → PREPARED → COMPLETED`, + `CANCELED/FAILED`) used uniformly. Type-specific
  data lives in side objects (`pickup_details`, `delivery_details`) — the `RESERVED` transition just
  sets `accepted_at` for pickup vs `in_progress_at` for shipment. Same states, different metadata.
  [docs](https://developer.squareup.com/docs/orders-api/fulfillments)
- **Toast (restaurant POS):** a "dining option behavior" field (`TAKE_OUT | DINE_IN | DELIVERY`) on
  the order, not a separate order type. [docs](https://doc.toasttab.com/doc/devguide/apiOrderTypeDetails.html)
- **Saleor click-and-collect:** same Order/Checkout entities as shipped delivery; pickup locations are
  `Warehouse`s; `checkoutDeliveryMethodUpdate` swaps the method. "Pickup is a variant of the shipping
  address assignment, not a different order type." [docs](https://docs.saleor.io/recipes/click-and-collect)
- **Shopify:** one order can carry multiple delivery methods; `DeliveryMethod` is polymorphic
  (`shipping | local | pickupPoint | pickup`) via the Fulfillment Orders API.
  [docs](https://shopify.dev/docs/apps/build/orders-fulfillment/order-routing-apps/build-local-pickup-options-function)
- **Vendure / Solidus / Shopware:** one *order* state machine; `Fulfillment`/`shipment`/`payment` are
  **subordinate, decoupled sub-machines** that feed order transitions — never one machine per
  fulfillment type.
  [Vendure](https://docs.vendure.io/current/core/core-concepts/orders) ·
  [Solidus](https://legacy-guides.solidus.io/developers/orders/order-state-machine) ·
  [Shopware](https://developer.shopware.com/docs/guides/plugins/plugins/checkout/order/using-the-state-machine.html)
- **Microsoft Dynamics 365 BOPIS** reference arch: the same OMS lifecycle is reused for pickup, routed
  to a different fulfillment path — "picked up" is a completion state parallel to "shipped/delivered."
  [docs](https://learn.microsoft.com/en-us/dynamics365/guidance/reference-architectures/supply-chain-commerce-bopis-architecture)

### 5.2 The principle, stated precisely

**Core order state (payment → prep → ready) is orthogonal to fulfillment/dispatch state, which only
exists for types that need it.** Systems keep ONE machine by making dispatch a **type-gated
sub-stage/side-object**, not a parallel top-level machine — exactly `Ready → InDelivery → Delivered`
(delivery) vs `Ready → PickedUp` (pickup/dine-in). Dine-in is absorbed as another `type` variant
(table/server metadata instead of address; same completion states), matching Toast/Square.

### 5.3 Recommended Wave-0 design (small delta on existing code)

The current enum is correct. The one addition needed for Tier-3 is an explicit **`fulfillment_type`
discriminator on the order** (`Delivery | Pickup | DineIn`) so that:
1. The `Ready`-branch is *gated by type* (a `Pickup` order is not offered to the HRW matcher at all —
   §16.60's "same flow minus the dispatch step" becomes a compile-time/validated skip, not a runtime
   accident), and
2. Type-specific data (delivery address + courier vs pickup time vs table number) lives in a
   **side-struct per type**, not by widening the shared state enum — the Square/Toast pattern.

Anti-pattern to avoid (confirmed by the survey): folding payment + fulfillment + dispatch concerns
into one flat status enum until it becomes unmanageable. The escape hatch the industry uses is
**decomposition into cooperating sub-machines** (Shopware/Solidus), not per-type duplication. The
kernel already keeps `Refunding/CompensatedRefund` (money) in the same enum today — if that enum keeps
growing, the Tier-3 blueprint should consider splitting a `PaymentState` sub-machine out, but that is
a *later* refactor, not a Wave-0 blocker.

**§16.49 no-courier-available:** the order is accepted and *queues* (never rejected). In the machine
this is a `Delivery`-type order sitting at `Ready` (or `Confirmed`) with an empty HRW candidate set —
`assign()` over an empty set returns empty, the order simply waits, and re-runs deterministically when
a courier appears (the requeue-never-drop invariant, already coded). No new state needed; honest
client-side "waiting for courier" status (§16.14).

---

## 6. Signal-style QR device-linking for the data-wallet transfer (§16.23, §16.47)

**Requirement:** move the on-device data-wallet (name/address/payment-method token) to a new device
via a **one-shot, self-custody QR transfer** — no central server, loss is the user's responsibility.

### 6.1 What Signal actually does (crypto, from libsignal source — not marketing)

Signal separates two things: **Sesame** (ongoing multi-device *session* management) and the
**provisioning flow** (introducing a brand-new device), implemented in `ProvisioningCipher`. The QR
is the linking bootstrap. From
[libsignal-service-java `ProvisioningCipher.encrypt()`](https://raw.githubusercontent.com/signalapp/libsignal-service-java/master/java/src/main/java/org/whispersystems/signalservice/internal/crypto/ProvisioningCipher.java)
(now Rust in `signalapp/libsignal`; mirror in
[`libsignal-service-rs`](https://whisperfish.github.io/libsignal-service-rs/src/libsignal_service/provisioning/mod.rs.html)):

1. **New device** generates an ephemeral **X25519** keypair, encodes its pubkey (+ a temporary
   server mailbox address) into the QR URI: `sgnl://linkdevice?uuid=<addr>&pub_key=<b64 pubkey>`
   (valid ~1–2 min). ([signal-cli wiki](https://github.com/AsamK/signal-cli/wiki/Linking-other-devices-(Provisioning)))
2. **Primary** (which scanned it) generates its *own* ephemeral X25519 keypair.
3. **ECDH:** `shared = X25519(primary_ephemeral_priv, new_device_ephemeral_pub)`.
4. **HKDF-SHA256** (salt empty, `info = "TextSecure Provisioning Message"`, 64 bytes) → split into
   `encKey ‖ macKey` (32 each).
5. **AES-CBC/PKCS5** (random IV) encrypts the serialized payload protobuf: `ct = IV ‖ AES-CBC(encKey, msg)`.
6. **HMAC-SHA256** over `version(0x01) ‖ ct` with `macKey`.
7. `body = version ‖ ct ‖ mac`, wrapped as `ProvisionEnvelope{ publicKey: primary_ephemeral_pub, body }`,
   server-relayed (opaque) to the new device's mailbox.
8. New device does the mirror ECDH, re-derives keys, verifies MAC, decrypts.

The QR is the **out-of-band authenticated channel**: it is generated/displayed by one device and read
by the other's *camera*, never through the server — so the relaying server cannot substitute keys
undetected. MITM would require intercepting the *visual* QR (physical proximity).

**Newer history-sync** flow adds a 256-bit AES key inside the provisioning message to decrypt a
separately-fetched encrypted archive
([Signal blog](https://signal.org/blog/a-synchronized-start-for-linked-devices/)) — the provisioning
channel itself stays a thin key-bootstrap.

### 6.2 What maps vs. what to drop for a pure device-to-device wallet transfer

The data-wallet case is closer to Signal's **device-TRANSFER** (move to new phone, direct local
connection, old device deactivated) than to **device-LINK** (both coexist under Sesame).

**Applies directly:**
- Ephemeral **X25519 ECDH** + **HKDF-SHA256**-derived symmetric key.
- QR-carried ephemeral pubkey as the authenticated bootstrap.
- Version-tagged encrypted envelope.

**Drop:**
- The **server-relayed provisioning mailbox** — there is no central dowiz server (§16.14). Use a
  **direct local transport** (same-LAN, BLE) or, for zero-infra, re-encode the primary's response as a
  **second (animated) QR** the new device scans. This is the biggest simplification and the one that
  keeps §16.14/§16.47 honest.
- **Sesame** ongoing multi-device bookkeeping — not needed for a one-shot transfer.
- The **server-issued `provisioningCode`** authorization token — there is no central authority to
  issue it; **physical QR proximity *is* the authorization**.
- Prefer **AES-GCM** (single AEAD) over Signal's AES-CBC + separate HMAC — simpler, fewer footguns,
  same security goal.

### 6.3 Rust building blocks

All RustCrypto, catalogued at [cryptography.rs](https://cryptography.rs/):
[`x25519-dalek`](https://crates.io/crates/x25519-dalek) (ECDH), [`hkdf`](https://crates.io/crates/hkdf)
+ [`sha2`](https://crates.io/crates/sha2), [`aes-gcm`](https://crates.io/crates/aes-gcm) or
[`chacha20poly1305`](https://crates.io/crates/chacha20poly1305). `signalapp/libsignal` is itself Rust
but is distributed via its own repo/FFI (not a general crates.io lib) — **vendor the primitive crates
it depends on, don't depend on libsignal wholesale.**

### 6.4 The security lesson that matters as much as the crypto

In 2025, Russia-aligned actors phished victims into scanning **attacker-controlled** linking QR codes
disguised as group invites, silently hijacking accounts; Signal's fix was an **explicit user
confirmation step** before a new device is granted access
([Google Cloud TIG](https://cloud.google.com/blog/topics/threat-intelligence/russia-targeting-signal-messenger/)).
**Implication for the wallet transfer:** the crypto is necessary but not sufficient — the UX must make
the **scan direction and a user-visible confirmation** explicit ("You are about to copy your saved
details to a NEW device — confirm?"), or the same social-engineering vector applies. Bake the
confirmation step into the Tier-3 UX spec, not just the crypto.

---

## 7. Consolidated Wave-0 recommendations

1. **Notification adapter (push + SMS + email):** one hub-side `Notifier` fanning an order-status
   event across three ports (`PushPort`/`SmsPort`/`EmailPort`), each with config-selected thin
   adapters. Push = `web-push` + `a2` + `fcm_v1`. SMS = hand-rolled REST per provider (Twilio / Plivo
   / Vonage / SNS / SMSGate) behind an in-house `SmsPort`. Email = managed API default
   (`aws-sdk-sesv2` / `resend-rs`), `lettre` SMTP opt-in only. SMS/email is a *mandatory* fallback
   fired by channel-availability, not gated on push failure. All tokens/contacts stored per-hub-local
   (§16.22), never dowiz-central. Uniform dead-token eviction on 410/UNREGISTERED.

2. **Offline-draft persistence:** a **versioned JSON blob, last-write-wins** — *no CRDT* (single
   writer). Tauri: `tauri-plugin-store` with **explicit `save()` per edit** + a native `tokio` outbox
   loop. Web/wasm: **IndexedDB** (`idb`/`indexed_db_futures`) + `navigator.storage.persist()` +
   `online`-event outbox (Background Sync only as an optional fast-path). **Client-side idempotency
   key created at draft time + a `Draft`/`PaymentInflight` state**, and on reconnect **query by
   idempotency key before any replay** to prevent double-charge (§16.49/§16.52).

3. **Unified order machine:** keep the existing single `OrderStatus` enum; add an explicit
   **`fulfillment_type` discriminator** (`Delivery | Pickup | DineIn`) that **type-gates the
   `Ready`-branch** (pickup/dine-in never enter HRW dispatch) and moves type-specific data into
   per-type side-structs (Square/Toast pattern). No-courier-available = a `Delivery` order waiting at
   `Ready` with an empty candidate set (requeue-never-drop, already coded). This is a small,
   industry-validated delta on existing code, not a rewrite.

4. **Courier matching:** unchanged. HRW as coded is a confirmed production-grade, boring-in-a-good-way
   choice. The *only* real gap is orchestration-layer: an **accept-timeout → advance-to-next-fallback
   driver** does not yet exist above the matcher (the matcher returns the ranked list; nothing drives
   the timeout advance). That belongs in the Tier-3 dispatch orchestrator.

---

## 8. Riskiest open unknowns for the Tier-3 blueprint

Ranked by how likely each is to force a redesign if left unresolved:

1. **Dispatch orchestration above HRW (highest).** The matcher is pure and settled, but *nothing yet*
   drives: accept-timeout → advance rank, courier-decline handling, the offered-to-one-primary vs
   fan-out-on-decline policy, and re-poll cadence for a queued no-courier order (§16.49). This is real
   stateful coordination logic on the hub with no central coordinator — the hardest un-designed piece.
   It must be designed without reintroducing courier scoring and without a dowiz-central queue.

2. **Wallet-transfer transport with no server (high).** §6's crypto is clear, but the *transport* for
   the primary's encrypted response back to the new device — animated-QR vs BLE vs same-LAN — is
   unresolved and materially affects UX and threat model. Animated-QR is the only zero-infra option
   but is bandwidth-limited (payment-method token + address is small, so likely fine — needs a size
   budget check). Plus the mandatory anti-phishing confirmation-step UX (§6.4).

3. **Web-Push on iOS Safari (high, external constraint).** iOS only supports Web Push for
   **home-screen-installed PWAs** and the behavior/limits shift across iOS versions. For a
   web-first-then-install product (§16.8) this means push simply *cannot* reach many iOS web users —
   which is exactly why SMS/email fallback is mandatory, but the coverage matrix (which customers get
   which channel on which platform) needs to be enumerated so "mandatory fallback" is provably
   sufficient, not assumed.

4. **Email deliverability for fresh per-tenant hubs (medium-high).** Self-hosted SMTP from a new
   Hetzner IP will land in spam (§2.2); the managed-API default mitigates this, but the *shared vs
   per-hub sender identity/domain* question (does dowiz provide a default sending domain? does each
   hub bring its own? DMARC alignment either way) is unresolved and affects both deliverability and
   the §16.14 no-central-dependency stance.

5. **Idempotency contract with the payment provider (medium).** §3.3's client-side idempotency key
   only prevents double-charge if the chosen payment providers (§16.13 multi-provider adapter)
   *honor* an idempotency key and expose a "query payment by key" path. Not all do identically —
   the payment-adapter port must standardize this, or the reconnect-safety guarantee is provider-
   dependent.

6. **Weighted-HRW pressure (low, watch-item).** If real venues ask for capacity-proportional
   assignment (a courier with more capacity), the WRH formula (§4.3) is ready — but it changes the
   `Courier` type and must pass an explicit no-scoring-boundary review before anyone reaches for it.
   Named so it isn't later mistaken for reputation scoring.

---

### Source index (primary)

Push: [web-push](https://crates.io/crates/web-push) ·
[a2](https://github.com/WalletConnect/a2) · [fcm_v1](https://docs.rs/fcm_v1).
SMS/email: [Twilio Rust](https://www.twilio.com/en-us/blog/developers/tutorials/send-sms-rust-30-seconds) ·
[SMSGate](https://sms-gate.app/) · [smskit](https://github.com/ciresnave/smskit) ·
[lettre](https://crates.io/crates/lettre) · [resend-rs](https://crates.io/crates/resend-rs) ·
[aws-sdk-sesv2](https://crates.io/crates/aws-sdk-sesv2) ·
[PowerDMARC self-hosting 2026](https://powerdmarc.com/self-hosting-email/).
Local-first: [tauri-plugin-store](https://crates.io/crates/tauri-plugin-store) ·
[idb](https://lib.rs/crates/idb) · [automerge](https://crates.io/crates/automerge) ·
[yrs](https://crates.io/crates/yrs) · [Stripe idempotency](https://docs.stripe.com/api/idempotent_requests) ·
[MDN offline/background](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Offline_and_background_operation).
HRW: [Thaler & Ravishankar 1998 (PDF)](https://www.microsoft.com/en-us/research/wp-content/uploads/2017/02/HRW98.pdf) ·
[Wikipedia](https://en.wikipedia.org/wiki/Rendezvous_hashing) ·
[Randorithms WRH](https://randorithms.com/2020/12/26/rendezvous-hashing.html) ·
[GitHub GLB](https://github.blog/engineering/infrastructure/glb-director-open-source-load-balancer/).
Unified order: [Square fulfillments](https://developer.squareup.com/docs/orders-api/fulfillments) ·
[Saleor click-and-collect](https://docs.saleor.io/recipes/click-and-collect) ·
[Toast order types](https://doc.toasttab.com/doc/devguide/apiOrderTypeDetails.html) ·
[Vendure orders](https://docs.vendure.io/current/core/core-concepts/orders).
Signal: [Sesame spec](https://signal.org/docs/specifications/sesame/) ·
[libsignal ProvisioningCipher](https://raw.githubusercontent.com/signalapp/libsignal-service-java/master/java/src/main/java/org/whispersystems/signalservice/internal/crypto/ProvisioningCipher.java) ·
[signal-cli provisioning wiki](https://github.com/AsamK/signal-cli/wiki/Linking-other-devices-(Provisioning)) ·
[Signal linked-devices blog](https://signal.org/blog/a-synchronized-start-for-linked-devices/) ·
[Google TIG QR-phishing](https://cloud.google.com/blog/topics/threat-intelligence/russia-targeting-signal-messenger/).

*Codebase ground-truth verified this pass: `bebop2/proto-cap/src/matcher.rs`,
`kernel/src/order_machine.rs`.*
