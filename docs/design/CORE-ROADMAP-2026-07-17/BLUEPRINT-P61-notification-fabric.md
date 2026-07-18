# BLUEPRINT P61 — Notification fabric: `Notifier` fan-out over `PushPort`/`SmsPort`/`EmailPort`, hub-local tokens, proven coverage matrix (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §11). Wave **W1**,
> foundations. Scope source: `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 W1 row **P61**, the
> cross-cutting finding **X10** (notification coverage matrix — the most load-bearing constraint
> in this blueprint), and §4-F (the confirm-by-default managed-email-domain engineering decision).
> Research substrate: `docs/research/OPUS-R4-ORDERFLOW-COURIER-NOTIFICATIONS-2026-07-18.md`
> §1–§2 (push/SMS/email crate freshness, provider recommendations — all verified on
> crates.io/docs.rs this pass). Structural template + rigor precedent:
> `BLUEPRINT-P51-open-map-routing.md`; sibling order surface: `BLUEPRINT-P37-order-http-surface.md`.
>
> **One sentence:** P61 is the thin hub-side layer that turns a committed `OrderStatus` transition
> into a fan-out across whatever contact transports the customer's data-wallet handed *this* hub at
> checkout — push (web-push/a2/fcm_v1), SMS (hand-rolled REST, provider-per-market), and email
> (managed API by default, SMTP opt-in only) — with **every token/contact stored hub-local, never
> dowiz-central (§16.22)**, and with a **proven** guarantee that every customer who placed an order
> has at least one working status channel (X10), because on iOS Safari web the push channel simply
> does not exist.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. Ground truth is non-discussible; everything
below builds on this table only. The single most load-bearing finding: **the send path is already
scaffolded as a fail-closed order→channel router with an `#[ignore]`d "real send" test — P61 is the
concrete `Notifier` that un-ignores it, it does not invent the routing.**

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| Unified order state machine EXISTS: `OrderStatus` is one 12-variant enum (`Pending…Ready…InDelivery/Delivered`, `PickedUp`, `Refunding`, `CompensatedRefund`) | `kernel/src/order_machine.rs:8-25` | **VERIFIED — P61 consumes these transitions, never re-derives them** |
| The `decide/fold` Law: `assert_transition(from,to)` validates one transition; `fold_transitions` folds a sequence, stopping at first invalid | `kernel/src/order_machine.rs:139`, `:156` | VERIFIED — the transition is the notification trigger |
| Delivery vs pickup already branch at `Ready`: `Ready => [InDelivery, PickedUp, Refunding]` | `kernel/src/order_machine.rs:84` | VERIFIED — one machine, notification-agnostic to fulfillment type |
| `is_terminal()` (Delivered/PickedUp/Rejected/Cancelled/CompensatedRefund) — the states that release a binding | `kernel/src/order_machine.rs:64-73` | VERIFIED — drives `ChannelRegistry` unbind |
| Committed-transition seam: `commit_after_decide(ev, decide)` runs `decide` then persists; dedup on content-id (`event_id()` = SHA3 over `prev‖pubkey‖seq‖payload`) is the **idempotency key** | `kernel/src/event_log.rs:366`, `:148`, `MeshEvent` `:134` | **VERIFIED — the notify hook fires on a committed (non-duplicate) transition; idempotency is free** |
| **Fail-closed order→channel router ALREADY EXISTS** (P49): `NotificationRouter` maps `order_id → channel_ref`; `deliver()` returns the bound channel or `None` (no default, never a cross-order leak) | `kernel/src/ports/customer.rs:249`, `:273` | **VERIFIED — P61 is the send behind `channel_ref`** |
| `NotificationBinding { order_id, channel_ref }` — `channel_ref` is a **reference**, "never holds a contact address or second transport (anti-scope)" | `kernel/src/ports/customer.rs:241`, doc `:237-239` | VERIFIED — the opaque handle bridging anonymous grant → hub-local contacts |
| The `#[ignore]`d **honest-RED** test P61 must turn GREEN: `b2_real_notification_reaches_bound_channel` — "depends on P43 DoD-2 send path (not yet built)" | `kernel/src/ports/customer.rs:650-661` | **VERIFIED — a named DoD un-ignore target (§7)** |
| Cross-order-leak guard is a live property test: order *i*'s state change routes ONLY to channel *i*, never *j≠i* | `kernel/src/ports/customer.rs:586-599` | VERIFIED — P61 preserves this invariant through fan-out |
| Customer identity grant holds **NO email/SMS/contact channel** — contacts are deliberately NOT in the grant | `kernel/src/ports/customer.rs:8` (header anti-scope) | VERIFIED — contacts live hub-local in P61's registry, keyed by `channel_ref`, so the grant stays anonymous |
| Port convention = **compile firewall**: trait module has ZERO network/HTTP/JSON/serde; concrete adapters live in a sibling crate (`llm-adapters`), config-selected, never a kernel recompile | `kernel/src/ports/mod.rs:1-3`; `kernel/src/ports/llm.rs:3-11`, trait `:368` | VERIFIED — P61's three ports mirror `LlmBackend`'s exact shape (sync trait, fail-closed error enum) |
| `TokenBucket` (degrade-closed on poison; `try_acquire` returns false ⇒ caller degrades) exists for retry/send rate-limiting | `kernel/src/token_bucket.rs:26`, `:74` | VERIFIED — P61 reuses, does not re-invent, retry throttling |
| `ports` module registered | `kernel/src/lib.rs:186` | VERIFIED — P61 adds one submodule here |
| **No notification adapter exists yet**: `kernel/src/ports/` = `{customer.rs, llm.rs, mcp.rs, payment.rs, payment_capability.rs, tool.rs, agent/}` — no `notification`/`notify` module; no `notify-adapters` crate at repo root | live `ls`, this pass | VERIFIED — M1/M6 are a genuine build |
| P43 reserves the transactional order-status lane and names a generic `ChannelSend`/`notify.rs` seam + messenger adapters (Telegram/WhatsApp/SimpleX/httpSMS) — **not yet built** | `BLUEPRINT-P43-external-integration-ports.md` §0 (grep "no notify module"), §1.2, E-b/E-c | VERIFIED — reconciled in §2.1 (P61 owns push/SMS/email; P43 keeps messengers) |

---

## 1. Research verdicts (from R4 §1–§2; every crate freshness re-checked this pass) — the port + adapter choices

The mechanism decisions below are **not** re-litigated; they are R4's findings, cited so this
blueprint is executable with zero prior session context (standard item 18). The load-bearing shape:
**there is no unified Rust crate for Web-Push + APNs + FCM, and that is correct** — it maps onto the
existing port convention, three thin adapters behind one trait each (R4 §1.1).

### 1.1 Push — three channels behind `PushPort`, legacy FCM is dead

| Channel | Crate | Freshness (verified) | Verdict |
|---|---|---|---|
| **Web Push** (RFC 8030 + VAPID 8292 + aes128gcm 8291) | [`web-push`](https://crates.io/crates/web-push) | **0.11.0**, 2025-02-22, active | **Default.** Native VAPID signing + RFC 8188 payload encryption. |
| **APNs** (token-based ES256 JWT over HTTP/2) | [`a2`](https://crates.io/crates/a2) (WalletConnect/reown) | **0.10.0**, 2026-06-08, active | **Recommended.** `.p8` token auth auto-renews the ES256 JWT; production "millions/day"; warns against per-request HTTP/2 connections. |
| **FCM** (HTTP v1) | [`fcm_v1`](https://docs.rs/fcm_v1) | **0.3.0**, 2026-05-30, fresh | **Use.** OAuth2 via `yup-oauth2`; type-safe single payload spanning Android/iOS/WebPush. |
| ~~FCM legacy~~ | `fcm`/`gcm` | — | **FORBIDDEN.** Google shut the legacy API down **June 2024** (R4 §1.1). A diff pulling `fcm`/`gcm` is a scope + correctness violation regardless of test state. |

**Dead-token handling is uniform and mandatory (R4 §1.2):** Web-Push `410 Gone`, APNs
`410 BadDeviceToken`, FCM v1 `UNREGISTERED` ⇒ **delete the stored token, never retry.** The
`NotifyError` taxonomy (§3) must make "retryable transient" and "token is dead, evict it"
*structurally distinct* — this is the item-14 error-propagation gate (§6.4).

**Token-lifetime gotchas (R4 §1.2):** one persistent HTTP/2 client per hub process (APNs; reconnecting
per-send risks Apple flagging DoS); APNs ES256 JWT rotates ≤60 min (`a2` automates); FCM OAuth2 access
tokens expire hourly (cache/refresh, don't reacquire per message); the hub's VAPID EC P-256 keypair is
generated **once** and persisted in the hub's local secret store — per-hub identity, consistent with
§16.22.

### 1.2 SMS — hand-roll REST behind `SmsPort`, provider-per-market

- **The `twilio` crate is stale** — v1.1.0, last published **2024-03-25** (R4 §2.1). Do not hard-depend.
  Twilio's own Rust tutorial confirms the real pattern is `reqwest` + form body + `.basic_auth(sid,token)`
  POST — ~20 lines, no dependency risk. **Hand-roll it behind the port.**
- **Multi-market (§16.20) ⇒ provider-per-market**, all targetable by the same thin adapter shape:
  **Plivo** (deliberately Twilio-shaped), **Vonage**, **MessageBird/Bird**, **Africa's Talking**, and
  **AWS SNS** (which *does* ship an official `aws-sdk-sns`).
- **Self-hosted Android gateway:** prefer **[SMSGate](https://sms-gate.app/)** (Apache-2.0, single-command
  "Private Mode", REST/webhooks, no external registration) over **httpSMS** (still needs Firebase + SMTP +
  Cloudflare Turnstile to self-host — heavier than it looks) — **R4 §2.1's explicit correction.** This
  supersedes P43's earlier httpSMS-first framing for the SMS transactional lane (§2.1 reconciliation).
- `smskit`/`sms-core` is prior art for the trait shape but immature (solo, ~2★) — **design reference,
  build `SmsPort` in-house.**

### 1.3 Email — managed API by DEFAULT, `lettre` SMTP is OPT-IN ONLY (this is the footgun, not a preference)

- `lettre` **0.11.22** (2026-05-14, async/`tokio1`, `rustls`, `dkim`) is a solid *crate* — **deliverability
  is the risk, not the library** (R4 §2.2).
- **Deliverability decides the default:** Gmail began **hard-rejecting** non-SPF/DKIM/DMARC mail at the
  SMTP level in **Nov 2025**; Microsoft followed. Even with perfect SPF/DKIM/DMARC/PTR, **a fresh IP has
  no reputation and lands in spam until warmed** — which a newly-provisioned per-tenant Hetzner hub cannot
  do on day one. **SMTP-direct-from-hub is a real footgun for a fresh Hetzner IP, not a naive default.**
- **Therefore the shipped default is a managed transactional-email API; `lettre` SMTP-direct is an explicit,
  flagged (`degraded/reputation-risk`) operator opt-in.** Managed adapters with real Rust support:
  [`aws-sdk-sesv2`](https://crates.io/crates/aws-sdk-sesv2) (v1.124.0, 2026-07-08, official) ·
  [`resend-rs`](https://crates.io/crates/resend-rs) (v0.28.0, 2026-07-05, official). Postmark/Mailgun/SendGrid
  have no maintained Rust SDK but expose plain JSON/form REST suited to the same thin-`reqwest` adapter.

### 1.4 The coverage constraint (X10, R4 §8.3) — the reason this blueprint exists

**iOS Safari web push works ONLY for home-screen-installed PWAs.** A web-first customer on iOS — the
§16.8 zero-friction "try without installing" path — therefore **cannot be reached by push at all.**
This is not a nice-to-have redundancy: §16.52's mandatory SMS/email fallback is *the thing that makes the
product honest here*. §5 proves coverage; it does not assume it.

---

## 2. Scope — what P61 owns vs deliberately does NOT (standard item 19 + anti-scope)

**P61 owns (build items §4):**

| Item | Content |
|---|---|
| M1 | `kernel/src/ports/notification.rs`: `PushPort`/`SmsPort`/`EmailPort` traits + value types + the `NotifyError` retryable-vs-dead taxonomy (compile firewall: no net/HTTP/serde) |
| M2 | `ChannelRegistry` — hub-local `channel_ref → ChannelSet` store (§16.22); bind at checkout, unbind on terminal; **never dowiz-central** |
| M3 | `Reachability` classifier + `channel_coverage()` — **the X10 coverage invariant, proven** (§5) |
| M4 | `Notifier::notify()` fan-out: reachability-classify → send per port → collect `FanoutOutcome`; dead-token eviction; **SMS/email fired by availability, NOT gated on push failure (§16.52)** |
| M5 | Order-machine hook: committed `OrderStatus` transition → `StatusMsg` → `Notifier::notify(channel_ref)`; un-ignores `customer.rs:650`'s `b2` test |
| M6 | `notify-adapters` crate (repo root, mirrors `llm-adapters`): push (web-push/a2/fcm_v1), SMS (hand-rolled REST per provider + SMSGate), email (sesv2/resend default + lettre opt-in) |
| M7 | `EmailSenderIdentity` + §4-F managed-default sending domain with a **real day-one vendor-domain opt-out** |
| M8 | Credential lifecycle: VAPID keypair persisted once/hub, APNs JWT ≤3300 s rotation, FCM OAuth token caching, one persistent HTTP/2 client per hub |

**P61 explicitly does NOT own:**

- **NOT the order state machine.** P61 *observes* transitions (`order_machine.rs`, `event_log.rs`); it
  never adds a state, edge, or `fulfillment_type` discriminator — those are the dispatch/order blueprints'.
- **NOT a dowiz-central token/contact store — hard §16.22 red-line, not a preference.** Every push
  subscription, phone number, and email address is hub-local. A diff that introduces any central dowiz
  token/contact table is a scope violation regardless of test state.
- **NOT customer identity / re-identification.** P49 (`customer.rs`) owns `OrderTrackingGrant` and the
  `channel_ref` handle. P61 is only the *send* behind that handle. The grant stays anonymous; P61's
  registry is keyed by the opaque `channel_ref`, never by a durable customer identity.
- **NOT messenger transactional channels.** Telegram / WhatsApp / SimpleX stay **P43** (§2.1). P61's
  fan-out may *target* a registered P43 messenger transport (delegating to P43's `ChannelSend`), and the
  coverage matrix (§5) counts it as reachable, but P61 builds no messenger adapter.
- **NOT marketing/campaign sends.** Recipient lists, consent-ledger campaigns, and owner-authored content
  are **P22** (`SocialPoster`/`ChannelAdapter`). P61 fires only on an *order event*, only to the *one*
  customer who placed *that* order — the reserved transactional lane (P43 §1.2 row 6).
- **NOT the offline-draft / idempotency contract on the customer device.** That is P66 (§16.23 wallet +
  drafts). P61 consumes the contact set the wallet hands the hub at checkout; it does not build the wallet.
- **NOT payment or PII beyond the minimum contact set the customer consented to.** `StatusMsg` carries the
  order handle, status, and a short human string — never card data (no such type is reachable here), never
  the cart, never a profile.

### 2.1 Reconciliation with P43 and P49 (honest overlap note — the P51 §2 pattern)

`customer.rs`'s `NotificationBinding` doc and the `b2` ignored test both say the send behind a `channel_ref`
belongs to "P43 DoD-2" — written before this synthesis. The 2026-07-18 synthesis (§5 W1) re-scopes the
**push/SMS/email notification fabric specifically to P61**, and R4 §2 (which P61 is built against) supersedes
P43's httpSMS-first SMS framing with the provider-per-market + SMSGate design. Clean, non-overlapping split:

- **P61 owns** the three typed ports (`PushPort`/`SmsPort`/`EmailPort`), the `Notifier` fan-out, the hub-local
  `ChannelRegistry`, the coverage matrix, and dead-token eviction — i.e. the concrete send that
  `NotificationRouter.deliver()` (`customer.rs:273`) and the `b2` test (`customer.rs:650`) defer to. **P61
  un-ignores `b2` (§7).**
- **P43 keeps** the messenger/social transactional channels (Telegram/WhatsApp/SimpleX) and its generic
  `ChannelSend` seam. A registered messenger transport is an *additional* entry in the same `ChannelSet`;
  the `Notifier` delegates to P43's `ChannelSend` for it and **does not re-implement it**.
- **File-collision guard (W1 is zero-collision):** P61 owns the new filename `kernel/src/ports/notification.rs`
  (P43's generic seam, if built, is `notify.rs` — distinct file). No shared hot file between the two blueprints.

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

All in `kernel/src/ports/notification.rs` unless noted. **Compile firewall: this module imports no HTTP,
no serde, no tokio** (mirrors `llm.rs:3-11`). Trait signatures are **sync** (as `LlmBackend` is); the
`notify-adapters` crate owns all async I/O and blocks/drives it on the hub runtime. This keeps the port
WASM-safe and deterministically testable with fake adapters.

```rust
// ── Transport value types (opaque, hub-local; never dowiz-central) ─────────────
pub enum PushKind {
    WebPush { p256dh: Vec<u8>, auth: Vec<u8> },   // RFC 8291 subscription keys
    Apns    { device_token: Vec<u8> },            // APNs device token
    Fcm     { registration_token: String },       // FCM v1 registration token
}
pub struct PushSub { pub kind: PushKind, pub endpoint: String }   // endpoint = web-push URL / "" for native
pub struct E164(pub String);                                     // validated E.164 phone
pub struct EmailAddr(pub String);                                // validated address
pub struct MessengerRef { pub provider: MessengerProvider, pub handle: String } // delegated to P43

// ── The message P61 derives from an order transition (minimum PII) ─────────────
pub struct StatusMsg {
    pub order_handle: String,     // the channel_ref / order id — the routing key, NOT a customer identity
    pub status: OrderStatus,      // reuse kernel enum; order_machine.rs:8
    pub title: String,            // short human string, no cart/PII
    pub body: String,
    pub deep_link: Option<String>,// opens the tracking view (P49 grant), never an account
}

// ── Reachability / platform context — the X10 substrate ────────────────────────
pub enum PlatformContext {        // captured at checkout from the placing client
    IosSafariWeb,                 // ★ the gap: web push UNAVAILABLE (needs home-screen PWA)
    IosPwaInstalled,              // web push available (iOS 16.4+ declarative)
    AndroidWeb, DesktopWeb,       // web push available (VAPID)
    NativeIos,                    // APNs available
    NativeAndroid,                // FCM available
    NativeDesktop,                // web push / OS notification available
    PushDenied,                   // user denied/blocked push on any platform
}
pub enum TransportKind { WebPush, Apns, Fcm, Sms, Email, Messenger }
pub enum UnreachableReason { IosSafariWebPushRequiresPwa, PushDenied, NoTransportRegistered }
pub enum Reachability { Reachable, Unreachable(UnreachableReason) }

// ── The hub-local registered contact set for ONE channel_ref (§16.22) ──────────
pub struct ChannelSet {
    pub platform: PlatformContext,
    pub push: Vec<PushSub>,
    pub sms: Option<E164>,
    pub email: Option<EmailAddr>,
    pub messenger: Vec<MessengerRef>,   // delegated to P43
}
pub struct ChannelRegistry { /* HashMap<String /*channel_ref*/, ChannelSet> — hub-local only */ }

// ── Ports (mirror LlmBackend: sync, fail-closed, never a mock success) ─────────
pub trait PushPort  { fn send(&self, sub: &PushSub,   msg: &StatusMsg) -> Result<Receipt, NotifyError>; }
pub trait SmsPort   { fn send(&self, to: &E164,       msg: &StatusMsg) -> Result<Receipt, NotifyError>; }
pub trait EmailPort { fn send(&self, to: &EmailAddr,  msg: &StatusMsg) -> Result<Receipt, NotifyError>;
                      fn sender_identity(&self) -> &EmailSenderIdentity; }        // §4-F opt-out is observable
pub struct Receipt { pub provider_id: String, pub at_tick: u64 }

// ── Error taxonomy — the item-14 gate: transient ≠ dead (R4 §1.2) ──────────────
pub enum NotifyError {
    Transient(String),            // 5xx / timeout — RETRY with backoff, DO NOT evict
    DeadToken(DeadReason),        // 410 / UNREGISTERED — EVICT, NEVER retry
    Unreachable(UnreachableReason),
    Config(String),               // missing key / bad sender identity — fail-closed
    Rejected(String),             // provider hard-rejected content (bad number/address)
}
pub enum DeadReason { WebPush410Gone, Apns410BadDeviceToken, FcmUnregistered }

// ── §4-F email sender identity ────────────────────────────────────────────────
pub enum EmailSenderIdentity {
    ManagedDefault { subdomain: String },              // dowiz-run, DMARC-aligned — the DEFAULT
    VendorDomain   { domain: String, dkim_selector: String }, // the opt-out — real from day one
}

// ── The fan-out result — every transport's fate is recorded (no silent drop) ───
pub struct FanoutOutcome {
    pub reached: Vec<TransportKind>,
    pub evicted: Vec<(TransportKind, DeadReason)>,
    pub skipped_unreachable: Vec<(TransportKind, UnreachableReason)>,
    pub transient_failures: Vec<(TransportKind, String)>,
}
pub struct NoReachableChannel;    // the coverage error — a checkout precondition failure

pub struct Notifier<'a> { /* &dyn PushPort, &dyn SmsPort, &dyn EmailPort, &mut ChannelRegistry */ }
```

**Constants:**

| Const | Value | Source |
|---|---|---|
| `APNS_JWT_MAX_AGE_S` | `3300` (rotate under 60 min) | R4 §1.2 |
| `FCM_OAUTH_REFRESH_MARGIN_S` | `300` (refresh before hourly expiry) | R4 §1.2 |
| `VAPID_CURVE` | P-256, generated once/hub, persisted in local secret store | R4 §1.2 |
| `PUSH_RETRY_BACKOFF_MS` | `[500, 2_000, 8_000]` (3 tries, then give up — transient only) | new; throttled by `TokenBucket` |
| `DEAD_TOKEN_RETRIES` | `0` (evict immediately) | R4 §1.2 |
| `MANAGED_EMAIL_DOMAIN_ROOT` | `"hub.dowiz.email"` (per-hub subdomain under the dowiz-run root) | §4-F |
| `MAX_STATUSMSG_BODY_BYTES` | `1024` (SMS-segment-aware; PII floor) | new |

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard items 3, 5)

Each item is spec-first, then a test that is **RED before** and **GREEN after** (item 2), including at
least one test **designed to break the invariant** (item 5). Tests live in `kernel/src/ports/notification.rs`
`#[cfg(test)]` (unit/fan-out, fake adapters, no network) and in `notify-adapters` behind a network feature.

### 4.1 M1 — the three ports + `NotifyError` taxonomy (compile firewall)

Spec: three sync traits + value types; `cargo tree -p dowiz-kernel` shows **no HTTP client** in the port
module after implementation (the `llm.rs:6` firewall discipline).
- **RED:** `dead_token_is_not_transient` — a `NotifyError::DeadToken(Apns410BadDeviceToken)` must never
  match the retry arm; a `Transient` must never trigger eviction. Fails to compile/assert until the taxonomy
  distinguishes them.
- **Adversarial:** `no_mock_success` — a fake `PushPort` that returns `Ok` without "sending" is a *test
  double the test controls*, but a production adapter returning a fabricated `Receipt` on a `410` is caught
  by mapping `410 → DeadToken` (§4.6), asserted in M6.

### 4.2 M2 — `ChannelRegistry` (hub-local, §16.22)

Spec: `bind(channel_ref, ChannelSet)` at checkout; `unbind(channel_ref)` when the order is terminal
(`order_machine.rs:64` `is_terminal`); `get(channel_ref)`. **No serialization to any dowiz endpoint.**
- **RED:** `registry_is_hub_local_only` — the registry type exposes no network/central-store method; a
  grep-gate (§6.4) asserts no `dowiz.` host string in the module. `bind_then_terminal_unbinds` — after a
  transition to `Delivered`, `get` returns `None`.
- **Adversarial:** `cross_order_isolation` — bind orders A and B to distinct `ChannelSet`s; a `StatusMsg`
  for A must resolve only A's set (reuses the `customer.rs:586-599` property; P61 must not weaken it).

### 4.3 M3 — `Reachability` + `channel_coverage()` (THE X10 proof — see §5)

Spec: `reachability(kind, platform) -> Reachability` (pure); `channel_coverage(&ChannelSet) -> Result<(), NoReachableChannel>`
returns `Ok` iff the set contains ≥1 transport whose reachability is `Reachable` in that set's platform.
- **RED (the load-bearing one):** `ios_safari_web_push_only_has_no_coverage` — a `ChannelSet { platform:
  IosSafariWeb, push: [one web-push sub], sms: None, email: None }` ⇒ `Err(NoReachableChannel)`. This is
  RED before the classifier exists (function absent) and GREEN after (the gap is *caught*, never assumed).
- **GREEN pair:** `ios_safari_web_with_sms_has_coverage` — same set + `sms: Some(..)` ⇒ `Ok`.
- **Adversarial:** `push_denied_any_platform_needs_fallback`; `empty_channelset_fails_closed`
  (`NoTransportRegistered`); `native_ios_push_alone_is_reachable` (APNs on `NativeIos` ⇒ `Ok`, so the gate
  never over-blocks the installed-app path).

### 4.4 M4 — `Notifier::notify()` fan-out

Spec: `notify(channel_ref, &StatusMsg) -> FanoutOutcome`. For each registered transport: classify
reachability; if `Unreachable`, record in `skipped_unreachable` and continue; else `send`; on
`DeadToken`, evict from the `ChannelRegistry` and record in `evicted`; on `Transient`, retry per
`PUSH_RETRY_BACKOFF_MS` throttled by `TokenBucket`, then record `transient_failures`. **SMS and email are
attempted by availability, not gated on push having failed (§16.52)** — a web/no-app customer never had a
push channel to fail.
- **RED:** `sms_fires_even_when_push_unreachable` — `IosSafariWeb` set with a web-push sub + an SMS number:
  outcome has `WebPush` in `skipped_unreachable` **and** `Sms` in `reached`. (This is the honest-product
  assertion; it is RED until fan-out treats SMS as parallel-mandatory, not push-conditional.)
- **Adversarial:** `dead_token_evicted_mid_batch` — three push subs, the middle one returns `410`: exactly
  that one is evicted, the other two still `reached`, registry now holds two subs. `transient_never_evicts`
  — a `503` must appear in `transient_failures`, never in `evicted` (the token survives). `zero_reachable_is_no_send`
  — a bound order whose entire set is unreachable produces an empty `reached` and **no default channel**
  (mirrors `customer.rs:603-607` fail-closed).

### 4.5 M5 — order-machine hook (committed transition → notify)

Spec: a `notify_on_transition(&EventLog, channel_ref, from, to)` observer invoked *after*
`commit_after_decide` succeeds with a non-duplicate outcome (`event_log.rs:366`): it maps `(to)` to a
`StatusMsg` (title/body per status) and calls `Notifier::notify`. Idempotency is inherited — a duplicate
event (same content-id, `event_log.rs:148`) never re-commits, so it never re-notifies.
- **RED:** un-ignore `kernel/src/ports/customer.rs:650` `b2_real_notification_reaches_bound_channel` — bind
  order A → channel_ref `CH-B2`; on A's terminal transition, the `Notifier` delivers EXACTLY to `CH-B2`'s
  set and to no other. The `#[ignore = "…P43 DoD-2 send path (not yet built)"]` attribute is removed; the
  test goes GREEN against the fake adapters.
- **Adversarial:** `duplicate_transition_notifies_once` — replaying the same `MeshEvent` yields
  `AppendOutcome::Duplicate` and zero additional fan-out. `notify_only_on_meaningful_transition` — a
  scaffold/illegal transition (rejected by `assert_transition`, `order_machine.rs:139`) never fires a notify.

### 4.6 M6 — `notify-adapters` crate (repo root; mirrors `llm-adapters`)

Spec: the concrete adapters, all async I/O and provider wire-format lives here (kernel stays firewalled).
- **Push:** `WebPushAdapter`(web-push 0.11), `ApnsAdapter`(a2 0.10, **one persistent HTTP/2 client per hub**),
  `FcmAdapter`(fcm_v1 0.3). Dead-token mapping: `410 Gone → DeadToken(WebPush410Gone)`,
  `410 BadDeviceToken → DeadToken(Apns410BadDeviceToken)`, `UNREGISTERED → DeadToken(FcmUnregistered)`.
- **SMS:** hand-rolled `reqwest` REST — `TwilioAdapter` (form + `.basic_auth`), `PlivoAdapter`, `VonageAdapter`,
  `SnsAdapter`(aws-sdk-sns), `SmsGateAdapter` (self-host, flagged). Provider selected per market by config.
- **Email:** `SesAdapter`(aws-sdk-sesv2) / `ResendAdapter`(resend-rs) — the **default**; `SmtpAdapter`(lettre)
  — **opt-in only, flagged `degraded/reputation-risk`** (§1.3).
- **RED (behind the network feature):** `apns_410_maps_to_dead_token`; `web_push_410_maps_to_dead_token`;
  `fcm_unregistered_maps_to_dead_token`; `apns_uses_single_persistent_client` (a per-send-new-connection
  adapter fails this). `smtp_adapter_is_flagged_degraded` — constructing `SmtpAdapter` without the explicit
  opt-in flag is a `Config` error.
- **Adversarial:** `legacy_fcm_crate_absent` — a `cargo tree` gate asserts neither `fcm` nor `gcm` (legacy)
  is in the dependency graph (R4 §1.1).

### 4.7 M7 — `EmailSenderIdentity` + §4-F managed default with a REAL opt-out

Spec (confirm-by-default, §4-F): the shipped default is `ManagedDefault { subdomain: "<hub>.hub.dowiz.email" }`
— a dowiz-run, DMARC-aligned sending domain, so a fresh Hetzner hub is deliverable on day one. The opt-out is
`VendorDomain { domain, dkim_selector }` and **must be honored from the first release, not stubbed**: a hub
config setting `email.sender = vendor` switches every send's identity with no code change. This is a **soft
ongoing dowiz dependency** for notification email (R4 risk #4), mitigated by the opt-out being real day-one.
- **RED:** `managed_default_is_the_default` — a hub with no email override reports
  `EmailSenderIdentity::ManagedDefault`. `vendor_optout_is_honored_not_stubbed` — setting the vendor domain
  makes `EmailPort::sender_identity()` return `VendorDomain{..}` and the outgoing envelope's `From`/DKIM
  reflect it; a stub that ignores the override fails this test.
- **Adversarial:** `smtp_direct_from_fresh_ip_is_flagged` — selecting `SmtpAdapter` surfaces the
  `degraded/reputation-risk` flag in config; it is never a silent default. `dmarc_alignment_recorded` — the
  managed default carries the DMARC-aligned domain (documented; the alignment is a DNS fact the blueprint
  names, not code P61 executes).

### 4.8 M8 — credential lifecycle

Spec: VAPID keypair generated once and persisted to the hub's local secret store (never regenerated per
subscription); APNs JWT rotated at `APNS_JWT_MAX_AGE_S`; FCM OAuth token cached and refreshed at margin.
- **RED:** `vapid_keypair_is_stable_across_subscriptions` — two subscriptions from the same hub sign under
  the same persisted key. `apns_jwt_rotates_under_hour` — a JWT older than 3300 s is rotated before send.
- **Adversarial:** `expired_fcm_token_refreshes_not_reacquires_per_message` — N sends trigger ≤1 refresh,
  not N (R4 §1.2's "don't reacquire per message").

---

## 5. The coverage matrix proof (X10) — built and PROVEN, not assumed

This is the most load-bearing section of the blueprint. §16.52 makes SMS/email a *mandatory* fallback; X10
requires P61 to **prove** every customer who placed an order has ≥1 working status channel — because on iOS
Safari web, the push channel does not exist at all.

### 5.1 The per-platform × per-channel matrix (the enumerated claim)

Legend: **✓** = works; **✗** = structurally unavailable; **~** = works if the customer registered it at
checkout (the mandatory-fallback lane).

| Order-placing context (`PlatformContext`) | Web Push | APNs | FCM | SMS | Email | Messenger (P43) | Push available? |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **iOS Safari web, NOT installed** (§16.8 zero-friction) | **✗** | ✗ | ✗ | ~ | ~ | ~ | **NONE — the gap** |
| iOS Safari web, installed as home-screen PWA | ✓ | ✗ | ✗ | ~ | ~ | ~ | web push |
| Android web (Chrome/Firefox) | ✓ | ✗ | ✗ | ~ | ~ | ~ | web push |
| Desktop web (Chrome/Edge/Firefox; Safari macOS 16+) | ✓ | ✗ | ✗ | ~ | ~ | ~ | web push |
| Native app (Tauri) — iOS | ✗ | ✓ | ✗ | ~ | ~ | ~ | APNs |
| Native app (Tauri) — Android | ✗ | ✗ | ✓ | ~ | ~ | ~ | FCM |
| Native app (Tauri) — desktop | ✓ | ✗ | ✗ | ~ | ~ | ~ | web push / OS |
| Any platform, **push denied/blocked** | ✗ | ✗ | ✗ | ~ | ~ | ~ | **NONE** |

**Two rows have no push channel at all** — the iOS-Safari-web-uninstalled row (the exact §16.8 case the
task names) and the push-denied row. For those, SMS/email/messenger is not a redundancy; it is *the only
way the order can honestly report status*.

### 5.2 The invariant, stated precisely (item 6 — hazard-safety as math, not prose)

Define `coverage(S)` for a `ChannelSet S` with platform `p`:

```
coverage(S) := ∃ t ∈ registered_transports(S) . reachability(kind(t), p) = Reachable
```

`reachability` is a pure total function (§3): on `IosSafariWeb`, `reachability(WebPush, _) =
Unreachable(IosSafariWebPushRequiresPwa)`; SMS/Email/Messenger are `Reachable` on every platform. Therefore:

```
p = IosSafariWeb  ∧  registered = {WebPush}     ⟹  coverage = false   (the gap, provably caught)
p = IosSafariWeb  ∧  SMS ∈ registered           ⟹  coverage = true    (fallback makes it honest)
```

**The gate is a checkout precondition, fail-closed:** `channel_coverage(&S)` is evaluated *before* an order
is accepted. `Err(NoReachableChannel)` ⇒ checkout must collect an SMS/email/messenger contact (or surface
"this order will have no status updates" and refuse under §16.52) — it is **structurally impossible to
accept an order with zero reachable status channels**. This is item-13 *self-termination as a hard invariant
boundary*: the state "accepted order with no reachable channel" is made unrepresentable at the checkout seam,
not policed by a runtime supervisor.

### 5.3 The falsifiable done-check (this exact case must be tested — §7 DoD-3)

The task's binding requirement: a falsifiable test proving coverage for the web-first-iOS case.

- **RED → GREEN A (`ios_safari_web_push_only_has_no_coverage`, §4.3):** `ChannelSet{ platform: IosSafariWeb,
  push:[web-push sub], sms:None, email:None }` ⇒ `channel_coverage` = `Err(NoReachableChannel)`. RED before
  the classifier exists; GREEN after — proving the gap is *detected*, never assumed away.
- **RED → GREEN B (`sms_fires_even_when_push_unreachable`, §4.4):** the same set + an SMS number ⇒
  `coverage = Ok`, and `Notifier::notify` on a status change puts `WebPush` in `skipped_unreachable` and
  `Sms` in `reached`. This proves the mandatory fallback is what actually carries status on that platform —
  the product is honest here by construction, tested, not hoped.

---

## 6. Cross-cutting design obligations (standard items 6, 8, 9, 11–16)

### 6.1 Hazard-safety as math (item 6)

Three unsafe states, each made unreachable by type/invariant structure, not policy:
1. **"Order accepted with no reachable channel"** — unrepresentable via the §5.2 checkout precondition
   (`channel_coverage` gate). Argued from the totality of `reachability` + the `Result` return.
2. **"Dead token retried forever / live token evicted on a blip"** — the `NotifyError` sum type (§3) makes
   `DeadToken` and `Transient` distinct variants; `DEAD_TOKEN_RETRIES = 0` and the retry arm match only
   `Transient`. A mis-handling is a non-exhaustive `match` — a *compile* error, not a runtime surprise.
3. **"Order A's status leaks to order B's contact"** — the `channel_ref`-keyed registry + the preserved
   `customer.rs:586-599` cross-order property. Fan-out never widens the routing key.

### 6.2 Schemas designed for scaling (item 8)

- **Scaling axis: transports per order (`ChannelSet` cardinality) and orders per hub.** `ChannelSet` is
  O(push subs + 3) — bounded per customer (a handful of devices). `ChannelRegistry` is O(live orders on one
  hub) — a single venue's in-flight orders, hundreds not millions; a `HashMap` is correct and the point it
  would change (multi-thousand concurrent live orders per hub) is far beyond a single venue's physics.
- **Fan-out is O(transports)** per transition, bounded and independent of fleet size (each hub notifies only
  its own orders — node-local, §6.3).

### 6.3 Isolation/bulkhead (item 11) + mesh awareness (item 12) + living memory (item 15)

- **Isolation:** each port's failure is contained — a dead SMS provider does not block push or email;
  `FanoutOutcome` records each transport's fate independently (no all-or-nothing). Retry throttling reuses
  `TokenBucket` (`token_bucket.rs:26`, degrade-closed on poison) so a flapping provider cannot exhaust the
  hub.
- **Mesh awareness:** notification state is **strictly node-local** — tokens/contacts never gossip, never
  leave the hub (§16.22). The **one soft, named exception** is the §4-F managed email sending domain: a
  dowiz-run DNS/DMARC dependency, *not* a data dependency (no customer data flows to dowiz) — mitigated by
  the vendor-domain opt-out being real day-one. Push/SMS talk to the customer's chosen provider directly
  from the hub; no dowiz relay ever sees a token or a message.
- **Living memory:** a `ChannelSet` is temporally scoped — bound at checkout, evicted on terminal
  (`is_terminal`, `order_machine.rs:64`); dead tokens are pruned on `410`/`UNREGISTERED`. The registry is a
  short-TTL live-order working set, never an accreting archive (cross-refs
  `internal-retrieval-living-memory-arc-2026-07-14`: demote/evict, never accrete central contact history).

### 6.4 Error-propagation isolation + smart index (item 14)

- The **`NotifyError` sum type** turns "retry a dead token" / "evict a live one" into a compile-time
  exhaustiveness obligation.
- A **CI grep-gate** (mirroring `scripts/ci-no-courier-scoring.sh`, cited in R4 §0) asserts:
  (a) no `dowiz.`-central token/contact host string in `notification.rs` (§16.22 red-line), and
  (b) neither legacy `fcm` nor `gcm` crate in `cargo tree` (R4 §1.1). Both become CI-time failures, not
  runtime ones.
- The coverage invariant is a **checkout precondition** — a bug that accepts an unreachable order fails
  `channel_coverage`'s test at build time (§5.3), not in production.

### 6.5 Rollback/self-healing vocabulary (item 13, used precisely)

- **Self-healing = error-correcting property:** dead-token eviction is the redundancy-repair — the registry
  self-corrects to only-live tokens; a subsequent re-subscribe re-populates it. No supervisor decision.
- **Self-termination = hard invariant boundary:** the §5.2 "no order without a reachable channel" gate —
  the unsafe state is unrepresentable, not watched.
- **Snapshot re-entry:** notification state is derivable — a lost `ChannelRegistry` is rebuilt from the next
  checkout/subscribe; there is no durable notification state to restore, so re-entry is trivially cheap.

### 6.6 Linux discipline (item 9) + tensor/spectral/eqc (item 16)

- **Linux verdict framework:** `Notifier` fan-out = **REINFORCES** the existing `ports/` seam pattern
  (`llm.rs`); the three-adapters-behind-one-trait shape is **ALREADY-EQUIVALENT** to the `LlmBackend` +
  `llm-adapters` split — reused, not re-derived. Dead-token eviction is **EXTENDS** (a new error pole on the
  same taxonomy shape as `event_log.rs`'s `StoreError` transient-vs-fatal split).
- **Tensor/spectral/eqc: DOES-NOT-TRANSFER, honestly.** Notification fan-out is O(1) branchy I/O
  orchestration with no closed-form math — there is no equation to compile via `eqc-rs`, no Laplacian, no
  spectral structure. Forcing one would be ritual math (the Anu/Ananke discipline forbids exactly this).
  Stated so a future reader does not "add spectral" for symmetry.

---

## 7. DoD — falsifiable, RED→GREEN, per item (standard item 2)

Every line is a test that is RED before the change and GREEN after; none is a prose checkbox.

| # | DoD check | RED→GREEN artifact |
|---|---|---|
| DoD-1 | The three ports + `NotifyError` taxonomy exist; port module has no HTTP/serde | `dead_token_is_not_transient` (§4.1) + `cargo tree -p dowiz-kernel` shows no HTTP client in the port |
| DoD-2 | `ChannelRegistry` is hub-local; binds at checkout, unbinds on terminal | `registry_is_hub_local_only`, `bind_then_terminal_unbinds` (§4.2) |
| **DoD-3** | **X10 coverage PROVEN for the web-first-iOS case** | `ios_safari_web_push_only_has_no_coverage` (RED: gap caught) **+** `sms_fires_even_when_push_unreachable` (GREEN: fallback carries status) — §5.3 |
| DoD-4 | Fan-out fires SMS/email by availability, not gated on push failure (§16.52) | `sms_fires_even_when_push_unreachable`, `zero_reachable_is_no_send` (§4.4) |
| DoD-5 | Dead-token eviction is per-provider, transient never evicts | `dead_token_evicted_mid_batch`, `transient_never_evicts`, `{apns,web_push,fcm}_410_maps_to_dead_token` (§4.4/§4.6) |
| DoD-6 | The order-machine hook delivers exactly to the bound channel; idempotent | **un-ignore `customer.rs:650` `b2_real_notification_reaches_bound_channel`** + `duplicate_transition_notifies_once` (§4.5) |
| DoD-7 | §4-F managed-default is default; vendor opt-out is honored, not stubbed | `managed_default_is_the_default`, `vendor_optout_is_honored_not_stubbed` (§4.7) |
| DoD-8 | Legacy FCM absent; SMTP-direct flagged degraded | `legacy_fcm_crate_absent`, `smtp_adapter_is_flagged_degraded` (§4.6) |
| DoD-9 | Credential lifecycle: stable VAPID key, JWT rotation, cached FCM token | `vapid_keypair_is_stable_across_subscriptions`, `apns_jwt_rotates_under_hour`, `expired_fcm_token_refreshes_not_reacquires_per_message` (§4.8) |
| DoD-10 | Cross-order isolation preserved through fan-out | `cross_order_isolation` (§4.2), reusing `customer.rs:586-599` |
| DoD-11 | Regression: the coverage matrix + dead-token taxonomy are named in `docs/regressions/REGRESSION-LEDGER.md` (item 17) | ledger entry `P61-coverage-matrix` + `P61-dead-token-taxonomy` |

---

## 8. Benchmark plan (standard item 10) — measured numbers, existing harness, zero new infra

All against **fake in-process adapters** (deterministic, no network) so the numbers are reproducible in CI;
the `notify-adapters` crate carries an optional real-network bench behind its feature flag.

| Bench | What it measures | Pass condition |
|---|---|---|
| `bench_fanout_latency` | `Notifier::notify` over an N-transport `ChannelSet` (fake adapters) | O(N), sub-µs per transport dispatch; recorded before/after, not estimated |
| `bench_coverage_classify` | `channel_coverage` on a full `ChannelSet` | O(1), constant regardless of platform |
| `bench_apns_persistent_vs_per_send` | throughput of one persistent HTTP/2 client vs per-request (R4 §1.2 warning) | persistent ≥ per-send throughput, measured — justifies M8's single-client rule with a number |
| `bench_registry_bind_unbind` | `ChannelRegistry` churn at venue order volume | linear, no pathological rehash at hundreds of live orders |

Telemetry hook: `FanoutOutcome` counts (`reached`/`evicted`/`skipped_unreachable`/`transient_failures`) are
emitted per transition so a coverage regression (rising `skipped_unreachable` with flat `reached`) surfaces
automatically, not only at review (item 10).

---

## 9. Links to docs & memory (standard item 7)

- **Consumes (input):** `kernel/src/order_machine.rs` (`OrderStatus`, `assert_transition`/`fold_transitions`,
  `is_terminal` — the transitions P61 notifies on); `kernel/src/event_log.rs` (`commit_after_decide`, the
  committed-transition + idempotency seam). **The order machine is P61's declared input dependency.**
- **Binds against (seam):** `kernel/src/ports/customer.rs` (P49 — `NotificationRouter`/`NotificationBinding`,
  the `channel_ref` handle, the `b2` ignored test P61 un-ignores).
- **Reconciles with:** `BLUEPRINT-P43-external-integration-ports.md` (messenger channels + generic
  `ChannelSend` seam — §2.1); `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` (P22 marketing lane — out of
  scope). R4 §2.1 supersedes P43's httpSMS-first SMS framing.
- **Supplied by:** the data-wallet at checkout (§16.23 / **P66**) hands the hub the consented contact set;
  P61 consumes it, does not build the wallet.
- **Feeds (consumers, declared):** **M1 (first real order)** — the synthesis §3 definition of M1 is "…with
  status notifications delivered"; the P61 row's `Feeds` column is "M1 (status updates are mandatory)."
  **M2 (first delivery order)** — courier/dispatch status updates ride the same fan-out. P61 is a launch
  blocker for M1.
- **Governing decisions:** `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 W1 (P61 scope), X10 (coverage
  matrix), §4-F (managed email domain); `docs/research/OPUS-R4-…-2026-07-18.md` §1–§2 (crate research).
- **Standard:** `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (this compliance map, §11).
- **Memory:** `integration-ports-reactive-arc-2026-07-13` (IP-* ports convention), `ecosystem-strategy-arc-2026-07-13`
  (caching-only-gap), `internal-retrieval-living-memory-arc-2026-07-14` (evict-never-accrete).

---

## 10. Hermetic principles honored (standard item 20 — load-bearing only)

- **Correspondence ("as above, so below"):** the hub-local `ChannelSet` mirrors *exactly* the contact set the
  customer consented to hand *this* hub — no central shadow copy exists. The `channel_ref` is the
  correspondence handle between the anonymous grant (above) and the hub-local transports (below); nothing
  above the hub holds what is below it. Tested by DoD-2's hub-local-only assertion.
- **Polarity (reachable ↔ unreachable):** the coverage invariant (§5.2) is *defined* on this polarity; the
  X10 gap is precisely the point where the polarity flips against push. Making the polarity explicit
  (`Reachability` enum) is what lets the gap be proven rather than assumed.
- **Cause & Effect:** an order transition (cause) deterministically produces a fan-out (effect); a dead-token
  effect feeds back as eviction, closing the loop — no effect without a named cause (no default channel, no
  silent send).

---

## 11. Standard-compliance map (all 20 points, checkable)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth w/ `file:line` | §0 (every row re-verified this pass) |
| 2 | Falsifiable DoD (RED→GREEN) | §7 (11 checks, incl. the un-ignore of `customer.rs:650`) |
| 3 | Spec→test→code, event-driven | §4 (each M spec-first, hook on committed transitions §4.5) |
| 4 | Predefined types & constants | §3 (all types + constant table before impl) |
| 5 | Adversarial/breaking tests | §4 (every item has an adversarial case; §5.3's RED gap-catch) |
| 6 | Hazard-safety as math | §6.1 + §5.2 (three unsafe states made unrepresentable) |
| 7 | Links to docs & memory | §9 |
| 8 | Schemas w/ scaling axis | §6.2 (transports-per-order, orders-per-hub) |
| 9 | Linux engineering discipline | §6.6 (REINFORCES/ALREADY-EQUIVALENT/EXTENDS verdicts) |
| 10 | Benchmarks + telemetry | §8 (four benches + `FanoutOutcome` telemetry) |
| 11 | Isolation/bulkhead | §6.3 (per-port containment, `TokenBucket` reuse) |
| 12 | Mesh awareness | §6.3 (node-local; the one soft §4-F email-domain exception named) |
| 13 | Rollback/self-healing as math | §6.5 (eviction=error-correcting; coverage gate=self-terminating) |
| 14 | Error-propagation + smart index | §6.4 (`NotifyError` sum type + two CI grep-gates) |
| 15 | Living-memory awareness | §6.3 (TTL-scoped registry, evict-never-accrete) |
| 16 | Tensor/spectral where applicable | §6.6 (**DOES-NOT-TRANSFER**, argued honestly) |
| 17 | Regression tracking | §7 DoD-11 (`REGRESSION-LEDGER.md` entries) |
| 18 | Instructions for other agents | §12 |
| 19 | Reuse-first, upgrade-if-needed | §0/§2.1 (reuses `NotificationRouter`, `TokenBucket`, `ports` seam, `llm-adapters` shape) |
| 20 | Hermetic principles | §10 (Correspondence/Polarity/Cause-Effect, load-bearing) |

---

## 12. Clear instructions for other agentic workers (standard item 18 — zero session context assumed)

**Goal:** build the hub-side notification fabric that turns a committed `OrderStatus` transition into a
multi-transport fan-out, with proven coverage and hub-local storage.

**Exact file targets:**
1. `kernel/src/ports/notification.rs` (NEW) — M1–M5, M7 types, M8 policy. Register at `kernel/src/lib.rs`
   near `:186` (`pub mod` inside `ports/mod.rs`). **Compile firewall: no HTTP/serde/tokio imports here**
   (verify `cargo tree -p dowiz-kernel`, per `llm.rs:6`).
2. `notify-adapters/` (NEW crate at repo root; mirror `llm-adapters`) — M6 concrete adapters + M7 sender
   identities + M8 credential I/O. All async/HTTP/provider wire-format lives here.
3. `kernel/src/ports/customer.rs` — remove the `#[ignore]` at `:650` and wire `b2_real_notification_reaches_bound_channel`
   to the real `Notifier` (DoD-6). Do **not** weaken the cross-order property at `:586-599`.
4. `docs/regressions/REGRESSION-LEDGER.md` — add `P61-coverage-matrix`, `P61-dead-token-taxonomy` (DoD-11).
5. `scripts/` — the two CI grep-gates (§6.4), modeled on `ci-no-courier-scoring.sh`.

**Acceptance criteria (all must be GREEN):** the 11 DoD checks in §7. The load-bearing two are DoD-3 (the
X10 web-first-iOS coverage proof — RED gap-catch then GREEN SMS-fallback) and DoD-6 (un-ignoring
`customer.rs:650`).

**Hard constraints (a violation fails review regardless of test state):**
- **No dowiz-central token/contact store** — everything hub-local (§16.22). CI grep-gate enforces it.
- **No legacy FCM** (`fcm`/`gcm`) — only `fcm_v1` (R4 §1.1). CI `cargo tree` gate enforces it.
- **Email default = managed API; `lettre` SMTP is opt-in, flagged degraded** — never a silent SMTP default (§1.3).
- **SMS/email fire by availability, not gated on push failure** (§16.52) — the web/no-app customer never had push.
- P61 adds **no** order state, edge, or fulfillment discriminator — it only observes transitions.

**Order of work:** M1 (ports/taxonomy) → M2 (registry) → M3 (coverage — do this before fan-out; it gates
checkout) → M4 (fan-out) → M5 (hook + un-ignore `b2`) → M6 (adapters) → M7 (sender identity) → M8
(credentials). M3 and M4 carry the X10 proof; do not defer them.
