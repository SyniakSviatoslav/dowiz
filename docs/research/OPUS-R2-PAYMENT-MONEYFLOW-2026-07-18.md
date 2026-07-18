# OPUS-R2 — Payment & Money-Flow Research (Wave-0 payment adapter)

> Research pass for the payment-adapter gap named in `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`
> §16.13/§16.33 ("not yet blueprinted, named here as a gap for the next blueprint pass").
> Grounds the Tier-3 payment blueprint. Real 2026 sources cited inline; confidence flagged per claim.
> Author: Opus research pass, 2026-07-18. Read-only research — no code, no canon edits.

---

## 0. Binding constraints (from the roadmap — not re-litigated here)

These operator rulings are **inputs**, not open questions. Every finding below is judged against them.

| § | Ruling | Bearing on payment |
|---|--------|--------------------|
| 16.13 | Online payment **mandatory from Wave-0**; **multi-provider via adapter layer** (Stripe = Wave-0 candidate, not exclusive) | The whole point of this doc |
| 16.16 | Fixed per-hub subscription, **no transaction %**, vendor keeps 100% | dowiz is **never** the merchant-of-record; no platform-fee leg in the money path |
| 16.20 | Multi-language/multi-market from day one; no hardcoded currency/locale | Provider must not be single-market; adapter must be currency-agnostic |
| 16.24 | Courier payout is **fully the venue's** responsibility; dowiz touches no courier money | Split logic is **never** for couriers — only for §16.46 food-court |
| 16.29 | Media/disputes/refunds = vendor + payment-provider responsibility, not dowiz | Refund/dispute API calls belong to the vendor's provider account, not hub-core |
| 16.30 / 16.34 | **Full wgpu UI, no DOM for forms, no `<input>` overlay** — entire checkout is canvas-rendered | **THE tension** — see §6. This is the load-bearing finding |
| 16.46 | Food-court: one unified cart across N vendors, one delivery → **split payment required** | One customer charge must fan out to N vendor payout destinations, intra-hub only |
| 16.49 | Payment calls **client-side**; **hub never sees card/PAN**; token/confirmation only | PCI SAQ-A-style; split must live in the provider's Connect-style API, not hub code |
| 16.53 | **Hub-level rate-limiting** for abandoned-checkout spam, *separate* from provider fraud tools | See §7 |
| 16.59 | **No vendor quality bar at all**; offboarding grace period + data export | Anti-abuse can't lean on curation; it's purely mechanical/rate-based |

Client shape (P39): the customer/owner/courier apps are **Tauri 2.x** installables (desktop + mobile), plus a browser web client. This matters enormously for §6.

---

## 1. Executive summary (read this first)

1. **There is no pure-canvas way to collect a card and stay out of PCI scope.** PCI descoping (SAQ A) is *defined by* the card fields being rendered and captured by the **provider's own code** — an iframe (web) or a native SDK view (mobile). The moment a PAN is typed into a surface *dowiz* draws (a wgpu text field) and passes through dowiz code, the hub is in **full PCI scope (SAQ D)** and the §16.49 "hub never sees card data" invariant is broken. **This is the single most important finding and it is a hard constraint, not an engineering preference.** (§2, §6)

2. **The §16.30 "no DOM" ruling and PCI descoping are reconcilable — because Tauri's UI *is* a webview with a live DOM underneath the wgpu layer.** The wgpu canvas is composited as a native GPU layer over/under the system webview, and *the webview DOM remains functional the whole time* (verified in Tauri's own design discussions). So the card-entry moment can briefly present a real provider iframe (web) — or, on Tauri mobile, a native provider SDK sheet — as the **one deliberate, scoped DOM exception**, then return to full canvas. The alternative (hosted-redirect / Payment Link) needs zero DOM in dowiz at all. **The tension is real but resolvable; it costs exactly one scoped exception to §16.34.** (§6)

3. **Rust is production-viable on the *server/hub* side but Stripe ships no official Rust SDK.** Use community `async-stripe` (regenerated weekly from Stripe's OpenAPI spec; 1.0-alpha) or thin hand-rolled REST. The *client-side card capture* is never Rust anyway — it's the provider's JS/native SDK. (§3)

4. **Split payment (§16.46) is a solved problem and does *not* conflict with "hub never sees card data"** — Stripe Connect *separate charges & transfers* / destination charges, Adyen split-at-authorization, and Mollie Connect all split *after* tokenization, server-side, from an already-tokenized charge. The card never re-enters the picture. **But it does force a merchant-of-record decision** that collides with §16.16 (see §4.3 — the sharpest open policy question). (§4)

5. **Recommended Wave-0: Stripe first (Connect for food-court split), with the adapter boundary shaped so Adyen or Mollie drops in for EU/non-US hubs.** Model the Rust adapter trait on **Hyperswitch's connector interface** (Juspay's open-source, Rust, 200+ connector orchestrator) — reference its *design*, don't necessarily run the whole stack per hub. (§5, §8)

---

## 2. The PCI descoping principle (the rule everything else obeys)

PCI SAQ A — the lightest self-assessment — is available **only** when *all* cardholder-data capture is delegated to a PCI-DSS-validated third party, via either a **hosted redirect page** or a **provider-hosted field embedded as an iframe** (web) / **provider-hosted native view** (mobile). Card data flows **browser/device → provider**, never through the merchant server.

- "Card data never touches your servers when you use Stripe Elements, which renders a secure iframe for card input… qualifies you for SAQ A." — Stripe / cside.
- "You should use recommended payments integrations to perform this process on the client-side, which guarantees that no sensitive card data touches your server and allows your integration to operate in a PCI-compliant way." — Stripe Tokenization docs.
- Adyen: "iFrames for web and encryption for iOS and Android are used to encrypt the data… resulting in no unencrypted Card data being visible to the merchant." — Adyen PCI-DSS guide.
- Mollie Components: "JavaScript APIs that allow you to add credit card holder data fields to your own checkout in a way that is fully PCI-DSS SAQ-A compliant."

**Corollary (the hard limit):** if dowiz renders the card field itself on the wgpu canvas and reads keystrokes, the PAN transits dowiz's own application memory. That is **SAQ D**, a full on-site QSA audit, and directly violates §16.49. **No amount of "we don't persist it" saves you** — PCI scope is about *transit and access*, not storage. This is why §6's resolution cannot be "just draw card fields in wgpu and POST the number."

**Raw-card API caveat (medium confidence):** Stripe *does* expose a raw-PAN tokenization endpoint (`POST /v1/tokens` with a raw card object), but it is gated — Stripe requires you to **attest SAQ-D PCI compliance and explicitly enable "raw card data APIs"** on the account, precisely because using it puts you back in full scope. Adyen's equivalent "API-only with encrypted card data" path still requires you to handle (encrypted) card fields. **Neither is usable under §16.49** — flagged so the blueprint doesn't reach for the "just tokenize it ourselves" shortcut. (Confidence: the *existence and gating* of raw-card APIs is well-established; the exact current enablement wording should be re-verified against Stripe's API reference at build time.)

### PCI DSS v4.0.1 residual obligation (mandatory since 2025-03-31)

Even SAQ-A merchants now carry **Req 6.4.3** (inventory + integrity/authorization of every script on the payment page) and **Req 11.6.1** (tamper/change-detection on payment-page HTTP headers + content) *if the payment page loads any external script* (i.e. Elements/Components, which load `js.stripe.com`). Sources: Akamai, Foregenix, cside, Imperva.

- **A fully hosted redirect (Stripe Checkout / Payment Link, Mollie hosted page)** shifts 6.4.3/11.6.1 to the provider's own domain — dowiz's page loads *no* card script → lightest residual burden. This is a **real reason to prefer redirect over embedded** for the sovereign/minimal stance, at the cost of UX (leaving the app).
- Stripe.js/Elements and Mollie Components **must be loaded from the provider's own CDN** (`js.stripe.com`), never self-hosted or bundled — a hard requirement that also means these are **not** available offline (relevant to §16.52's offline-checkout-draft resilience: payment simply can't fire offline, which the roadmap already accepts).

---

## 3. Rust SDK reality for the hub/server side

The hub's Rust process only ever handles **PaymentIntent/charge creation, confirmation-status webhooks, transfers/splits, and refunds** — all server-side REST against the provider. It never touches a PAN. What's viable in Rust:

| Provider | Rust support | Verdict |
|----------|-------------|---------|
| **Stripe** | No official Rust SDK. Community **`async-stripe`** (arlyon) — regenerated weekly from Stripe's OpenAPI spec, ~full API coverage incl. Connect/Transfers, currently **1.0-alpha** (breaking changes expected pre-RC). Also newer `stripe-sdk` (Finite Field, updated Feb 2026). | **Production-viable via `async-stripe`**, pin a version and budget for the 1.0 migration. Thin hand-rolled `reqwest` client is a legitimate fallback since the surface dowiz uses is small (PaymentIntents + Connect transfers + webhooks). |
| **Adyen / Mollie / Rapyd** | No official Rust SDKs. All are plain HTTPS/JSON REST. | Call REST directly from the adapter with `reqwest`. This is *normal* — the adapter is a thin REST client regardless of provider. |
| **Hyperswitch** (Juspay) | **Written in Rust**, open-source, connects Stripe/Adyen/Braintree/Worldpay/Checkout.com/Cybersource + 120–200 processors, self-hostable. Has `hyperswitch-prism` — a **standalone unified connector library** usable "directly against payment processors without running the full switch." | **The reference design for dowiz's own adapter** (see §5). Running the *whole* orchestrator (app-server + vault + Postgres + Redis) inside every isolated hub is heavy and conflicts with §16.51's "без тяжких бібліотек" lean-hub stance — but its **connector trait** is exactly the port shape dowiz wants. |

**Key point:** the "zero Node/TypeScript" rule is satisfied on the **hub** trivially — all server logic is Rust REST. The *client* card-capture code (Stripe.js iframe or native SDK) is the provider's, not dowiz's, and its language is irrelevant to the sovereignty rule — dowiz writes none of it.

---

## 4. Split payment (§16.46 food-court) — how it actually works, and where it bites

### 4.1 The three provider mechanics (all post-tokenization, all server-side)

1. **Stripe Connect — separate charges & transfers.** Charge the customer **once** (one PaymentIntent), then create N `Transfer`s to N connected vendor accounts, each tied to the charge via `source_transaction`. Stripe's own docs literally name the use case: *"a restaurant delivery platform that splits payments between the restaurant and the deliverer."* One charge → many payouts. Alternatively **destination charges** with `transfer_data`/`application_fee_amount` for the simpler 1-platform-1-seller case.
2. **Adyen — split at authorization.** One authorization split across multiple *balance accounts* (sale amount, commission, fees) booked separately at auth time.
3. **Mollie Connect — split payments.** Route funds between connected seller accounts so a customer can "purchase goods from multiple sellers in a single basket." **EUR/GBP only, no multi-currency conversion within a split** — a real limit for §16.20's multi-market goal.

### 4.2 Does split conflict with "hub never sees card data"? **No.**

Splitting happens **after** the customer's card is tokenized and charged — it operates on a charge ID / balance, never on card data. The client→provider tokenization step is identical whether the money later goes to one vendor or five. §16.46 and §16.49 are **fully compatible**. The roadmap's own §16.49 already anticipated this: *"split-payment logic needing to live in the provider's own split/Connect-style API rather than inside hub code."* Confirmed correct.

### 4.3 Where it *does* bite — the merchant-of-record vs §16.16 collision (**sharpest open policy question**)

Every marketplace-split model (Connect destination charges, Adyen platforms, Mollie Connect) makes **the platform the merchant-of-record or a regulated "platform,"** which normally implies the platform *can* take an `application_fee`. dowiz's §16.16 explicitly wants **no transaction %** and **vendors keep 100%**. Two honest resolutions:

- **(A) dowiz is *not* the platform.** Each vendor holds their **own** direct Stripe/Adyen/Mollie account; the hub calls that vendor's account with the vendor's own key (or a scoped restricted key). For **single-vendor hubs this is clean and 100%-to-vendor with zero dowiz money touch.** But food-court split across N *independent* accounts from *one* customer charge is **not possible** without one entity being the platform that collects then transfers — you cannot atomically charge a customer once and settle into N unrelated merchant accounts you don't control.
- **(B) The *hub/vendor-collective* is the platform** for that food-court hub (Connect with `application_fee_amount = 0`). One customer charge, N transfers to the co-located vendors, **zero platform fee** → still honors "vendor keeps 100%," but now *someone* (the hub operator / lead vendor) is merchant-of-record and carries the Connect account + KYC. **dowiz-the-company still touches no money** (fee is zero, account isn't dowiz's) — consistent with §16.16/§16.24, but it **reopens who the food-court's merchant-of-record is**, which the roadmap has not decided.

**This is the money-leg red-line the blueprint must escalate to the operator** (per memory's "money/auth/RLS red-lines preserved even under full autonomy"). Recommendation: **single-vendor hubs use model (A) (vendor's own account, no split, cleanest); food-court hubs use model (B) with zero application fee, merchant-of-record = the hub's designating vendor/operator.** Do not paper over it.

---

## 5. Multi-provider adapter architecture (§16.13 port/adapter)

### 5.1 The industry pattern (payment orchestration)

Production multi-provider systems converge on a **connector/port interface**: a stable internal payment API, with per-provider adapters implementing a common trait. Hyperswitch (Rust) is the canonical open reference — "modular architecture… pick only the components you need (routing, retries, vaulting)… without vendor lock-in," 200+ connectors behind one interface. This is exactly the §16.5 channel-adapter / §16.1 hosting-adapter shape the roadmap already uses elsewhere.

### 5.2 The Rust adapter boundary for dowiz (concrete shape)

A thin **port trait** in hub-core, provider crates behind it. Core knows nothing provider-specific:

```rust
// hub-core: the port. Currency is minor units (i64) — never f64 (money red-line).
pub trait PaymentProvider {
    /// Create the intent to be confirmed CLIENT-SIDE. Returns an opaque
    /// client secret / session handle the client SDK consumes. Hub never sees PAN.
    async fn create_intent(&self, order: &OrderRef, amount: Money, currency: CurrencyCode,
                           split: Option<&SplitPlan>) -> Result<ClientHandoff, PayError>;

    /// Verify a provider webhook signature and normalize to a hub event.
    fn verify_webhook(&self, raw: &[u8], sig: &Headers) -> Result<PaymentEvent, PayError>;

    /// Food-court: fan one captured charge out to N vendor destinations.
    async fn settle_split(&self, charge: ChargeId, plan: &SplitPlan) -> Result<(), PayError>;

    /// §16.29 — refund routes here (vendor+provider responsibility).
    async fn refund(&self, charge: ChargeId, amount: Money, reason: RefundReason) -> Result<(), PayError>;
}
```

- `ClientHandoff` is the *only* thing crossing hub→client: a `client_secret` (Stripe), `sessionData` (Adyen), or hosted-page `checkout_url` (redirect). **No card data type exists anywhere in hub-core** — that's the type-level enforcement of §16.49.
- Provider selection is **per-hub config** (which region/vendor account), resolved at hub-provisioning time — core code never branches on provider. New provider = new crate implementing the trait, zero core change. This is the §16.13 "no hub-core code changes" requirement, satisfied.
- **Idempotency keys** on every mutating call (Stripe/Adyen/Mollie all support them) — mandatory for the §16.52 offline-reconnect-retry path so a re-fired checkout can't double-charge.
- Money is **integer minor units** end-to-end (matches the repo's existing `apply_tax_*_int` A3 organ and the "no f64 money" test-integrity rule).

**Recommendation on Hyperswitch:** reference `hyperswitch-prism`'s connector trait as the design oracle; do **not** deploy the full orchestrator per hub (too heavy for isolated lean hubs). If a hub ever needs true runtime multi-provider routing/retries, prism can be added as a hub-side library later without changing the core port.

---

## 6. ⭐ THE critical finding — PCI card capture inside a no-DOM wgpu UI

This is the point the task flagged as most important. Here is the honest resolution.

### 6.1 Why it looks impossible

PCI descoping *requires* the card fields be rendered and captured by the **provider's own code**: a web **iframe** (Stripe Elements / Mollie Components / Adyen web Drop-in) or a **native mobile view** (Stripe/Adyen iOS+Android SDK). §16.30/§16.34 forbid DOM forms and even `<input>` overlays and mandate the entire UI be wgpu-drawn. On the surface: the only PCI-safe card widgets are DOM/native, and dowiz has banned DOM. **If both held absolutely, online payment would be impossible without dowiz entering full PCI scope.**

### 6.2 Why it's actually resolvable — three viable paths

**The unlock: a Tauri app's UI *is a system webview with a live DOM*.** The wgpu content is composited as a **native GPU layer over/under the webview**, and — confirmed in Tauri's own design discussions (`tauri-apps/tauri` #11944, #8246; `wry` #677) — **"the webview DOM remains functional and usable alongside GPU rendering when properly layered"** (dual-window stacking: transparent frameless overlay + click-passthrough toggling, or webview-on-top-of-wgpu). So "no DOM" is a *design choice about what dowiz draws its UI with*, **not** a claim that no DOM exists in the runtime. The DOM is right there, unused — available for exactly one job.

**Path A — Scoped iframe exception (web + Tauri desktop).** At the card-entry moment only, dowiz overlays a real provider iframe (Stripe Payment Element from `js.stripe.com`) on top of the wgpu canvas, sized to the card field, then dismisses it on tokenization. Everything else stays full-canvas. **Cost: one deliberate, documented exception to §16.34's "no `<input>` overlay,"** narrowly scoped to PCI card entry. It's the smallest possible DOM footprint and keeps SAQ A. This is the pragmatic Wave-0 answer for the browser web client, which has **no** native-SDK option at all.

**Path B — Native provider SDK (Tauri *mobile*, the genuinely DOM-free path).** Stripe's **iOS/Android SDKs render card entry in native UIKit/Compose views (PaymentSheet / Card Element), tokenize on-device, and the PAN never touches the merchant server** (confirmed, Stripe iOS docs). Adyen's native Drop-in does the same with client-side encryption. A **Tauri plugin** bridges the wgpu app to the native SDK sheet — **zero DOM, zero iframe, still SAQ A.** This is the *cleanest* fit for §16.30 on mobile and worth building as the mobile card path. It does **not** exist for desktop/web (no native card UI there → Path A or C).

**Path C — Hosted redirect / Payment Link (zero DOM in dowiz anywhere).** Hub creates a Stripe Checkout Session / Mollie hosted-page / Payment Link server-side, returns a URL; the client opens the provider's hosted page (external browser or a transient webview), customer pays on the *provider's* domain, redirects back with a status. **dowiz renders no card UI at all** → fully honors §16.30/§16.34, lightest PCI residual (6.4.3/11.6.1 shift to the provider's page), works identically on web/desktop/mobile. **Cost: the user momentarily leaves the immersive wgpu experience** — a UX/aesthetic compromise against §16.35's seamless-intent vision, not a compliance one.

### 6.3 The one thing that is *not* viable

**Drawing the card field on the wgpu canvas and capturing the PAN in dowiz code.** It "works" technically and would look perfectly on-brand — and it puts dowiz in **SAQ D full PCI scope**, breaks §16.49, and creates exactly the card-data-liability §16.29 pushes onto vendor+provider. **Rule this out explicitly in the blueprint** so a future implementer chasing UI purity doesn't build it.

### 6.4 Recommendation for §6

- **Web client:** Path A (scoped Payment Element iframe overlay). Only option.
- **Tauri mobile:** Path B (native SDK sheet via plugin) — cleanest, truly DOM-free.
- **Tauri desktop / universal fallback / max-sovereignty:** Path C (hosted redirect) — pick this if the operator would rather take the UX hit than grant *any* DOM exception.
- **Escalate to operator:** which of "one scoped iframe exception (A/B, seamless)" vs "brief redirect out of the canvas (C, zero exception)" better serves the §16.35 vision. Both are compliant; it's an aesthetics-vs-immersion call only the operator can make. **This is the top blueprint decision.**

---

## 7. Fraud / anti-abuse for a no-quality-bar, self-serve platform (§16.53)

§16.53 is explicit: the **mandatory online-payment gate is the primary spam defense** (no card, no order), and **provider fraud tools** (Stripe Radar, Adyen RevenueProtect) cover fraudulent-but-paid orders. The gap §16.53 names is **abandoned/attempted-checkout spam that never reaches a successful payment** — a DoS/nuisance vector — plus §16.59's "no vendor quality bar," meaning anti-abuse must be **purely mechanical**, never curation/reputation (also consistent with the mesh's "trust = signed capability, never reputation/blacklist" stance in memory).

Real patterns that fit dowiz's stack:

1. **Hub-level rate limiting (the §16.53 ask).** Token-bucket per client identity on checkout-intent creation (capacity ~ small burst, low refill). The repo **already has a `TokenBucket` in the kernel** (agentic-mesh B3 work) — reuse it; don't add a dep. Key by data-wallet client id + coarse IP, degrade-closed. This throttles intent spam *before* it hits the provider (which also protects against provider-side rate limits and API-cost abuse).
2. **Edge rate-limiting + challenge at Cloudflare (dowiz already runs behind CF Tunnel, §16.2/§16.45).** Cloudflare **Turnstile** is a privacy-preserving, non-interactive proof-of-work/proof-of-space challenge (no CAPTCHA image, no ad-tracking) that can gate checkout-intent creation and is explicitly marketed for "e-commerce checkout" abuse. It's a natural, already-in-stack fit. **Caveat:** Turnstile is a browser-JS widget → same DOM tension as §6; on the no-DOM canvas it belongs at the **edge/pre-request layer or the hosted-redirect page**, not embedded in the wgpu UI.
3. **Sovereign/self-hosted alternative:** **ALTCHA** — self-hosted, open-source, proof-of-work, no third party, no cookies, no fingerprinting. Fits the sovereignty ethos better than any SaaS captcha and can run as a hub-side challenge issuing a signed token the checkout endpoint verifies (mirrors the capability-cert pattern). Worth a look if the operator rejects a Cloudflare dependency on principle.
4. **Idempotency + single-outstanding-intent per client** — cap concurrent open checkout intents per data-wallet id; an abandoned one must expire/cancel before another opens. Cheap, stateful-in-hub, no external dep.
5. **What to explicitly NOT build:** vendor scoring, customer reputation, cross-hub blocklists — all barred by §16.26/§16.59 and the mesh no-reputation red line. Anti-abuse stays rate/challenge/payment-gate only.

---

## 8. Concrete Wave-0 recommendation

1. **Provider #1: Stripe.** Best Rust story (`async-stripe`), best client SDKs across web + native iOS/Android (covers all three §6 paths), Connect *separate charges & transfers* directly matches the food-court split with a named restaurant-delivery precedent. Broadest market/currency coverage for §16.20.
2. **Provider #2 (design-in, integrate-second): Adyen or Mollie for EU/non-US hubs.** Adyen = strongest multi-market + native SDKs + split-at-auth. Mollie = simplest hosted-redirect + EU-data-residency + Mollie Connect split, but **EUR/GBP-only splits** limit multi-currency food-courts. Pick per the first real non-US market. **Do not integrate #2 in Wave-0 — just prove the adapter trait holds by stubbing it.**
3. **Card-capture flow:** native SDK sheet on Tauri mobile (Path B); scoped Payment Element iframe overlay on web/desktop (Path A); **or** hosted redirect everywhere (Path C) if the operator prefers zero DOM exception over seamlessness — **operator decision, §6.4.**
4. **Split model:** single-vendor hubs = vendor's own provider account, no split, 100% to vendor (model A). Food-court hubs = Connect with `application_fee_amount = 0`, merchant-of-record = the designated hub vendor/operator (model B). **Escalate the merchant-of-record choice — money red-line.**
5. **Adapter:** thin Rust `PaymentProvider` port (§5.2) in hub-core, provider crates behind it, `ClientHandoff` the only hub→client type, no card-data type in core, integer minor units, idempotency keys everywhere. Reference Hyperswitch/`prism`'s connector trait; don't deploy the full orchestrator per hub.
6. **Anti-abuse:** reuse the kernel `TokenBucket` for hub-level checkout-intent throttling; Cloudflare Turnstile (already in stack) or self-hosted ALTCHA at the edge/redirect layer; single-outstanding-intent cap. No reputation, ever.

---

## 9. Riskiest open unknowns for the Tier-3 blueprint (ranked)

1. **[HIGHEST — aesthetics/compliance fork] §6.4 path choice.** Scoped DOM/iframe exception (seamless, breaks §16.34 once) vs hosted redirect (zero exception, leaves the canvas). Both compliant; the §16.35 immersive vision hangs on it. **Operator decision, blocks the checkout blueprint.**
2. **[HIGHEST — money red-line] Food-court merchant-of-record (§4.3).** Who is the Connect platform for a multi-vendor hub, given §16.16 forbids a dowiz fee? Model B (zero-fee, vendor/operator as platform) is the proposed answer but it's an unresolved policy + KYC question. **Escalate.**
3. **[HIGH] Native-SDK ↔ Tauri-wgpu bridge is unproven.** Path B needs a Tauri plugin surfacing Stripe/Adyen's native card sheet over a GPU-composited window on iOS+Android. No off-the-shelf plugin found; battery/compositing interaction with §16.34's full-wgpu courier UI is untested. **Prototype early — this is where §6 could still fail in practice.**
4. **[HIGH] Multi-currency split limits.** Mollie splits are EUR/GBP-only; even Stripe cross-currency Connect transfers have FX handling to design. §16.20's "any market" + §16.46 food-court split may not be simultaneously satisfiable on every provider. **Scope which markets get food-court split in Wave-0.**
5. **[MEDIUM] `async-stripe` 1.0-alpha churn.** Pin + budget the migration, or hand-roll the ~5 endpoints dowiz uses to avoid the dependency risk entirely.
6. **[MEDIUM] PCI 6.4.3/11.6.1 residual even on SAQ A.** Any embedded-iframe path (A) still needs script inventory + tamper detection on the page hosting the iframe. Redirect (C) avoids it. Decide who owns that control (likely another reason to prefer C for lean hubs).
7. **[MEDIUM] Offline/mesh vs payment finality (§16.52).** Payment can only fire online (SDKs load from provider CDN; charges need connectivity). The "held draft, fire on reconnect" path needs idempotency + intent-expiry so a stale draft can't double-charge or charge a cancelled order. Design the intent lifecycle explicitly.
8. **[LOW] Refund/dispute routing (§16.29).** Refunds hit the vendor's provider account with the vendor's key; confirm the adapter's `refund()` is callable by the vendor/admin surface without hub-core holding provider secrets in a way that widens scope.

---

## Sources

- Stripe SDKs index — https://docs.stripe.com/sdks
- Stripe iOS SDK (native PaymentSheet/Card Element, on-device tokenization) — https://docs.stripe.com/sdks/ios.md
- Stripe Integration security guide (load Stripe.js from js.stripe.com; CSP) — https://docs.stripe.com/security/guide
- Stripe Payment Intents / two-step confirmation — https://docs.stripe.com/payments/payment-intents , https://docs.stripe.com/payments/build-a-two-step-confirmation
- Stripe Connect separate charges & transfers (restaurant-delivery split precedent) — https://docs.stripe.com/connect/separate-charges-and-transfers , https://docs.stripe.com/connect/marketplace/tasks/accept-payment/separate-charges-and-transfers
- Stripe Checkout hosted redirect / SAQ A — https://docs.stripe.com/payments/finalize-payments-on-the-server ; production guide https://tomodahinata.com/en/blog/stripe-checkout-sessions-payments-production-guide-2026
- Is Stripe PCI compliant / SAQ A via Elements — https://cside.com/blog/can-you-use-stripe-for-pci-dss ; https://stripe.com/guides/pci-compliance
- `async-stripe` (weekly-regenerated, 1.0-alpha) — https://github.com/arlyon/async-stripe , https://crates.io/crates/async-stripe ; alt `stripe-sdk` — https://github.com/finitefield-org/stripe-sdk-rust
- Hyperswitch (Rust, open-source, 200+ connectors, prism unified connector lib) — https://github.com/juspay/hyperswitch , https://hyperswitch.io/
- Adyen split at authorization / marketplaces — https://docs.adyen.com/marketplaces/split-transactions/split-payments-at-authorization ; PCI/native encryption — https://docs.adyen.com/development-resources/pci-dss-compliance-guide ; iOS Drop-in — https://docs.adyen.com/payment-methods/cards/ios-drop-in/
- Mollie hosted checkout / Components SAQ A / Connect split (EUR-GBP limit) — https://www.mollie.com/products/checkout , https://docs.mollie.com/docs/mollie-components , https://docs.mollie.com/docs/connect-marketplaces-processing-payments
- Tauri wgpu + webview compositing (DOM stays usable under GPU layer) — https://github.com/tauri-apps/tauri/discussions/11944 , https://github.com/tauri-apps/tauri/issues/8246 , https://github.com/tauri-apps/wry/issues/677
- PCI DSS v4.0.1 Req 6.4.3 / 11.6.1 (mandatory 2025-03-31) — https://www.akamai.com/blog/security/script-security-achieving-pci-dss-v4-compliance-before-deadline , https://www.foregenix.com/blog/introduction-of-new-requirements-6.4.3-and-11.6.1-for-pci-dss-v4.0 , https://www.imperva.com/blog/how-to-comply-with-pci-dss-4-0-requirements-6-4-3-and-11-6-1/
- Cloudflare Turnstile (privacy-preserving PoW challenge, checkout abuse) — https://www.cloudflare.com/products/turnstile/ , https://blog.cloudflare.com/turnstile-private-captcha-alternative/
- ALTCHA (self-hosted open-source PoW, no third party) — via https://prosopo.io/blog/top-cloudflare-turnstile-alternatives/
- Token-bucket rate limiting — https://sujeet.pro/articles/rate-limiting-strategies
