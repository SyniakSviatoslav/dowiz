# BLUEPRINT — Online Payment Adapter: Residual Build Plan (Transport, Webhook Inbound, Desktop Shell, PSP Selection)

**Status: BLUEPRINT / PLAN — no code written, nothing built.**
**Date:** 2026-07-20
**Track:** CORE-ROADMAP-2026-07-17 lane; successor-in-scope to `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P60-payment-adapter-core.md` (P60, built) and D12 §4-A (card-capture UI ruling).
**Authorization:** operator, this session — "real payment adapter now"; framing constraint: simple / secure / configurable for food-business owners.
**Governing rulings:** D12 §4-A (Path A — scoped provider-hosted card moment, desktop shell hosts a live webview), D12 §4-D (Albania/EU, EUR/ALL), D14 (red-line resources Ledger/Auth/Secret/Migration are a hard, non-configurable, human-only ceiling — no agent identity, ever).

---

## 0. Headline correction: the payment core is already built

Any framing of this work as "build online payments" is stale. The kernel already contains the complete online-fiat payment core, built and proptest-gated under P60:

| Already built, tested, in-kernel | Where |
|---|---|
| `PaymentProvider` trait — full online-fiat/Stripe-class seam | `kernel/src/ports/payment_provider.rs` |
| `IdempotencyKey(pub [u8;32])` — `derive(order_id, wallet_id, nonce)`, domain-separated SHA3 over `b"dowiz.pay.idem\0" || order_id || wallet_id || nonce`; key-rebind refusal tested | same |
| `ClientHandoff` — `HostedRedirect{checkout_url, session_token, ttl_s}` / `NativeSdkSession{session_blob}`; opaque by construction ("NEVER a PAN, never a cvv, never a card_*") | same |
| Webhook as sole truth writer — `verify_webhook(raw, &WebhookHeaders{sig, ts})`, `WEBHOOK_TS_TOLERANCE_S = 300`, per-event-id dedup; tests prove client-reported success with no webhook stays `IntentCreated` | same |
| N-leg multi-vendor saga — `run_nleg_saga` / `assert_nleg_atomicity`; terminal is exactly one of Committed / Aborted / NeedsReconciliation; captured+voided-without-reconciliation is unrepresentable; 400-case proptest | same |
| Refund routing — `RefundRequest` / `refund()` onto existing order states `Refunding→CompensatedRefund`, reconciled via `Money::checked_neg` / `checked_sub` (P07 reversal primitive); over-refund refused by the money law | same + `kernel/src/money.rs` |
| `IdemLedger` (append-only, re-folds on hub restart), `TokenBucket` anti-abuse (`CHECKOUT_BURST = 3`), single-outstanding-intent gate, `NoOpPaymentAdapter` deterministic reference impl | same |
| Cash rail — `RailKind::CashOnDelivery`, pure `decide_settlement()` over `SettlementState`, idempotent by `order_id`, 400-case proptest + 7 adversarial fixtures | `kernel/src/ports/payment.rs` (P47 Wave-0) |
| Rail registry — `PaymentRail = {Fiat, Crypto, Stripe, GoogleApplePay, OtherLater}`; `Stripe` is already an operator-ruled-in rail name at the type level; self-source grep test bans transport/credential tokens from the file | `kernel/src/ports/payment_capability.rs` |
| No-card-data firewall — bans `pan/cvv/cvc/card_number/card_holder/exp_month/exp_year` anywhere under `kernel/src/` | `kernel/tests/no_card_data.rs` |
| Adapter firewall — cargo-tree assertion that the kernel links no payment adapter | `kernel/tests/firewall_p47.rs` |

Two structural facts worth restating because they carry the whole design:

1. **The settlement/capture bulkhead is structural, not conventional.** `kernel/src/order_machine.rs` has `Delivered => &[]` — the order FSM contains no settlement or capture state at all. Settlement (`payment.rs`) and online capture (`IdemLedger` + N-leg saga) are separate event-sourced folds that never appear as order-graph nodes. The only intersection is `Refunding→CompensatedRefund`, which already reconciles correctly through the double-entry ledger.
2. **The agent lane cannot touch money, structurally.** `kernel/src/ports/agent/scope.rs`: `Resource::Ledger = 0x02` and `Action::SettlementRecorded` are `is_red_line()`; `RedLinePolicy::DenyByDefault` returns `Err(())` for any red-line scope. A grep of `agent-facade/src/` for `payment|settle|capture|Ledger` returns nothing — no payment tool exists in the agent lane. The `no-agent-order-authority` grep gate makes agent-invocable confirm/cancel unrepresentable. This is D14, already enforced in code.

**Therefore this blueprint covers only the residual:** (A) the out-of-kernel transport crate, (B) webhook-receiving infrastructure, (C) the owner-desktop shell with a webview for the card moment, (D) PSP selection for the Albania/EU launch market. None of these exist on disk today. That is the honest scope: much smaller than "build payments," but each of the four is genuinely absent.

---

## 1. Gap inventory (what does not exist)

| Gap | Evidence |
|---|---|
| **A. `payment-adapters` crate** | Name reserved in the P60 doc; no such directory on disk. The kernel's `verify_webhook` uses a local sha3 stand-in; the module doc states the real HMAC-SHA256 over `{ts}.{payload}` lives in this crate. |
| **B. Webhook inbound endpoint** | `tools/native-spa-server` has no raw-body-preserving signed-webhook route today. Signature verification requires the unparsed request bytes; no current route provides them. |
| **C. Owner-desktop shell with webview** | `apps/courier/` is real but is the courier/rider shell only (`CourierShell::TauriMobile`, "no DOM, no webview, no JS"). The D12 §4-A desktop webview (named P39-rev/P63 in the roadmap) is unbuilt. `ClientHandoff::HostedRedirect` is exactly what it would consume. |
| **D. Market-real PSP** | Stripe's official supported-country list (~46 countries) does not include Albania. Given each food-court vendor needs their own connected account, native Albanian per-vendor Stripe onboarding is doubtful. Candidates with local standing: **Paysera** (Albanian Central Bank EMI license since 2021-04-19), **Viva Wallet** (local license), or bank acquirers. |

---

## 2. Component A — the `payment-adapters` crate

### 2.1 Position and shape

- New standalone crate at repo root: `payment-adapters/` (name as reserved in the P60 doc). Path-dependency on `kernel/` — no workspace, consistent with the repo's build model.
- Implements `PaymentProvider` (from `kernel/src/ports/payment_provider.rs`) once per provider: a `StripeAdapter` first (reference/documentation adapter — the type system already names the rail), then the Albania launch adapter (`PayseraAdapter` or `VivaAdapter`, pending decision D-1 in §7).
- The trait is already provider-agnostic — `fn id() -> "stripe:eu"` is one impl, not the design. The crate must keep that property: provider-specific code stays inside each adapter module; nothing provider-specific leaks into shared types.

### 2.2 Transport

- HTTP via `ureq` — the same blocking, no-tokio choice already established by `llm-adapters` (Ollama/vLLM/managed-API, `ureq`, no tokio). Payment API calls (create intent, create refund) are low-frequency, latency-tolerant request/response calls; an async runtime buys nothing here and violates the dependency-discipline bar.
- **Explicitly rejected: `async-stripe`.** It pulls hyper/tokio/reqwest — a full async runtime and TLS stack for what is, on our side, a handful of POSTs and one HMAC check. A webhook HMAC verification is ~15 lines over a vendored HMAC. Rejection recorded here as the DECART rationale; the dep-swap comparison is: `hmac`+`sha2`+`ureq` (pure Rust, no runtime) vs `async-stripe` (runtime + SDK surface we won't use).

### 2.3 Real webhook verification (the crate's cryptographic core)

Implements the provider scheme precisely (Stripe's, as the documented reference; other providers slot in per-adapter):

- Header `Stripe-Signature: t=<unix>,v1=<hex>`.
- Signed payload is the literal string `"{timestamp}.{raw_request_body}"` — raw bytes, not re-serialized JSON.
- HMAC-SHA256 keyed on the `whsec_...` endpoint secret, hex-encoded.
- Comparison via `hmac`'s `Mac::verify_slice` — constant-time by construction. No hand-rolled byte compare.
- Timestamp tolerance: reject outside the window. Stripe's default is 5 minutes, which matches the kernel's already-coded `WEBHOOK_TS_TOLERANCE_S = 300` exactly — the adapter takes the tolerance from the kernel constant, not its own copy.
- Accept multiple `v1` entries during endpoint-secret rotation.

Dependencies: `hmac` + `sha2` (pure-Rust). Both new deps carry DECART rationales in the crate's `Cargo.toml` header per repo convention, and `cargo-deny check` gates them.

### 2.4 Feature discipline

- The crate is out-of-kernel by definition, but the kernel side must stay clean: `kernel/tests/firewall_p47.rs` (cargo-tree assertion that the kernel links no payment adapter) is the standing proof and must remain green — the dependency arrow is `payment-adapters → kernel`, never the reverse.
- Within `payment-adapters`, each provider is its own off-by-default feature (`stripe`, `paysera`, `viva`) so a deployment compiles only its rail. Header comments state what each feature pulls in and how to verify exclusion, per the repo's feature-discipline rule.

### 2.5 Credentials (Secret red-line)

- Provider API keys and the webhook endpoint secret live only in adapter-side configuration, injected at adapter construction by the hosting binary. They never enter the kernel: `kernel/src/ports/payment_capability.rs` already carries a self-source grep test banning `req[u]est`/`ur[e]q`/`std::env`/`sec[r]et`/`key` from the registry file, and `Resource::Secret` is red-line under D14 — human-provisioned, never agent-readable, never agent-grantable.
- Owner-facing configurability ("configurable for the food owners") means: an owner selects a rail and pastes provider credentials once, through a human-only owner surface — never through any agent tool, which D14 makes structurally impossible anyway.

### 2.6 `NoOpPaymentAdapter` as test oracle

The kernel's `NoOpPaymentAdapter` is the deterministic reference implementation of the trait. The adapter crate uses it as a differential oracle: every conformance test drives the same command sequence (create intent → handoff → webhook event(s) → fold; refund; N-leg saga) through (a) `NoOpPaymentAdapter` and (b) the real adapter with a fixtured/mocked transport, and asserts **identical kernel-side fold outcomes** — same idempotency behavior, same `IntentCreated`-until-webhook invariant, same saga terminal classification. Any divergence is a bug in the adapter, not in the oracle.

---

## 3. Component B — webhook-receiving infrastructure

### 3.1 Placement decision: extend `native-spa-server` (recommended; ratify as D-4)

**Call: a new route set on `tools/native-spa-server`, not a separate edge service.** Justification against the existing architecture:

- `native-spa-server` is already the repo's native HTTP adapter over the kernel's `json-api` feature — the one place where external HTTP meets kernel folds. A second inbound HTTP service would duplicate TLS/deploy/monitoring surface and create a second process that must reach the same `IdemLedger` fold, i.e., a shared-mutable-state seam this repo's design consistently avoids.
- The genuine technical requirement — **raw-body preservation** — is a route-level property, not a service-level one: the webhook route must capture the request body bytes before any parsing, because the HMAC is computed over `"{timestamp}.{raw_request_body}"` verbatim. A JSON-first route stack that parses eagerly would destroy the signable bytes; the new route set therefore reads the body as `&[u8]` and hands `(raw_bytes, WebhookHeaders{sig, ts})` directly to `verify_webhook`. Parsing happens only after verification succeeds.
- TLS terminates at the existing edge, as already noted in the P60 research; the service itself never handles certificates.

The alternative (separate minimal webhook service) remains viable if operational isolation is later wanted (e.g., independent rate-limiting blast radius); it is listed as open decision D-4 rather than silently foreclosed.

### 3.2 Trust-boundary composition (webhook vs capability-cert)

The existing `/api/*` routes authenticate via internal capability-certs. A webhook cannot and must not use that mechanism — the sender is the PSP, holding no dowiz capability. The composition rule:

- Webhook routes mount under a **distinct prefix** (`/hooks/pay/<provider-id>`), explicitly outside the capability-cert middleware. They are the only unauthenticated-inbound POST surface in the system.
- Their trust root is different in kind: **possession of the per-endpoint HMAC secret**, proven per-request by signature-over-raw-bytes plus timestamp freshness. Trust enters the kernel boundary only as the output of `verify_webhook` — a verified, timestamp-fresh, deduplicated event handed to the payment fold. A webhook never mints, carries, or implies a capability; it can never call anything under `/api/*` semantics; it writes exactly one kind of truth (provider payment events) into exactly one fold.
- This is the same pattern the kernel already committed to: the webhook is "unauthenticated-but-signature-verified inbound," and it is the **sole** truth writer for capture. No owner session, no agent, no client confirmation can substitute for it (the kernel test proving client-reported success without a webhook stays `IntentCreated` is the standing witness).

### 3.3 Replay and dedup

- First line: `verify_webhook`'s 300 s timestamp window (`WEBHOOK_TS_TOLERANCE_S`) bounds the replay horizon.
- Second line: per-event-id dedup, wired into the already-built append-only `IdemLedger`, which re-folds on hub restart — so dedup survives crashes without any new persistence mechanism. A redelivered event (PSPs redeliver by design) folds to a no-op; an attacker replaying a captured delivery inside the window changes nothing; outside the window it is rejected before dedup is even consulted.
- Anti-abuse on the route itself reuses the existing `TokenBucket` machinery (the kernel already applies it to checkout with `CHECKOUT_BURST = 3`).

---

## 4. Decision: direct vs destination charges (multi-vendor)

This is a real architectural fork and this blueprint does not paper over it.

- **What the code already implies.** `VendorLeg.dest_account` is documented as "the VENDOR'S OWN provider account — never dowiz's." That is the shape of **direct charges** (Stripe Connect vocabulary; every serious PSP has an equivalent): each vendor is their own merchant of record; dowiz never holds the money.
- **The alternative.** **Destination charges**: the platform (dowiz) is charged and then transfers to vendors. Operationally easier for vendors — one platform account instead of N vendor accounts — but it makes dowiz a payment custodian/merchant-of-record, which the existing code's own design intent (`dest_account` semantics; "dowiz stays out of the money") rejects, and which contradicts the platform's decentralized/local-first invariants at the money layer.
- **Recommendation: direct charges**, as the only choice consistent with the code already written and with the refund path (a refund debits whoever holds the charge — under direct charges, the vendor, which is where the liability belongs). **Named tradeoff:** direct charges require **every food-court vendor to hold their own PSP account**. For a small Albanian food-court vendor this is real onboarding friction — KYC, bank linkage, possibly fees — and it interacts with PSP selection (§5): the chosen PSP must actually onboard Albanian-registered small merchants natively.
- **Status: recommendation, not ruling.** Recorded as open decision **D-2** for the operator, jointly with **D-3** (does dowiz shepherd vendor PSP onboarding as a supported workflow — a genuine support-cost commitment — or leave it to vendors). If the operator rules for destination charges instead, `VendorLeg.dest_account` semantics and the "dowiz stays out of the money" stance must be explicitly re-ruled, not quietly reinterpreted.

---

## 5. Decision: PSP selection (Stripe-as-reference vs Albania launch rail)

- **Stripe = reference adapter, documented first.** The rail name is already in the type system (`PaymentRail::Stripe`), its API pattern (Payment Intents → client `client_secret` → hosted confirm → `payment_intent.succeeded` webhook) is exactly the shape the kernel coded, and its webhook scheme is the one §2.3 implements. It is the right adapter to write first and to document against.
- **Stripe is probably NOT the Albania Wave-0 rail.** Sources conflict, but Stripe's official supported-country list (~46 countries) does not include Albania; the one aggregator listing Stripe as "available" most likely reflects indirect access via a foreign/EU entity — unusable for per-vendor direct charges, where each Albanian-registered vendor needs their own native account. Albania context: non-eurozone EU candidate (all 33 accession chapters opened Dec 2025, no euro target date); currency ALL; Visa/Mastercard dominate; domestic schemes weak.
- **Credible launch candidates:** **Paysera** (Albanian Central Bank EMI license, 2021-04-19), **Viva Wallet** (local license), or bank acquirers. Verifying which of these supports (a) hosted checkout with full redirect, (b) signed webhooks, (c) per-merchant accounts for small Albanian food vendors, (d) EUR and ALL — is Phase 0 diligence work (§7, Phase 0) that no amount of code substitutes for.
- The trait's provider-agnosticism means this decision does not block Phases 1–3: the Stripe adapter proves the transport/verification/inbound machinery; the launch adapter is a second impl of the same trait. Recorded as open decision **D-1**.

### PCI posture (fixed, not open)

`ClientHandoff::HostedRedirect` — opening the provider's own domain — is the full-redirect model, which PCI DSS v4.0.1 treats **more favorably than an embedded iframe** (iframe merchants additionally carry script-integrity/anti-skimming controls 6.4.3, 11.6.1 and written TPSP confirmation). dowiz is squarely **SAQ-A-eligible**: no cardholder data is ever stored, processed, or transmitted by dowiz, and `kernel/tests/no_card_data.rs` is the structural proof, not a policy claim. Any future proposal to embed card fields directly is a red-line-grade regression and must be refused at design time.

---

## 6. Who initiates capture — a falsifiable constraint, not a note

**The customer, only.** Capture is triggered by the payer confirming on the provider's hosted surface, reached through the storefront's own checkout flow. The hub's role is exactly two-sided: create the intent (producing `ClientHandoff`), and fold the verified webhook. The owner never initiates capture. An agent never initiates capture — D14, and the existing structure already enforces it:

- `Resource::Ledger = 0x02` and `Action::SettlementRecorded` are `is_red_line()`; `RedLinePolicy::DenyByDefault` returns `Err(())`.
- `agent-facade/src/` contains no `payment|settle|capture|Ledger` symbol; `agent-loop` imports only `agent-facade`, so it structurally cannot name kernel mutation.
- No new actor class is needed: the existing per-order customer identity (`kernel/src/ports/customer.rs`, P49) covers the payer, and the webhook — unauthenticated-but-signature-verified — is what writes capture truth. **No agent or owner capability sits on the money path at all.**

**Falsifiable form (new standing gate, this blueprint's deliverable, not yet written):** a CI-wired test in the style of the existing `no-agent-order-authority` and `no-courier-scoring` grep gates — call it `no-agent-capture-path` — asserting:

1. (source gate) no identifier matching `create_intent|capture|refund|IdempotencyKey|ClientHandoff|verify_webhook` appears anywhere under `agent-facade/src/`, `agent-loop/`, `agent-adapters/`, or `llm-adapters/`;
2. (link gate) `cargo tree` for the agent-lane crates shows no edge to `payment-adapters` (mirror of `firewall_p47.rs`, pointed the other way);
3. (runtime gate) a unit test constructing an agent scope requesting `Resource::Ledger` under `RedLinePolicy::DenyByDefault` and asserting `Err(())` — already true today; the gate pins it against regression.

RED→GREEN discipline: land the gate first against a deliberately planted violation on a scratch branch (RED), remove the plant (GREEN), then wire to CI.

---

## 7. Phased build order (RED→GREEN acceptance per phase)

**Phase 0 — PSP diligence (no code).**
Verify, with primary-source documentation per candidate (Paysera, Viva Wallet, ≥1 bank acquirer): hosted-checkout-by-redirect support; signed-webhook scheme and its exact signature format; native onboarding of Albanian-registered small merchants; EUR/ALL support; per-merchant (direct-charge-equivalent) account model. Output: a one-page comparison feeding operator decisions D-1/D-2/D-3.
*Acceptance:* each cell of the comparison cites the provider's own documentation; conflicts (like the Stripe-Albania conflict already found) stated as conflicts, not resolved by wish.

**Phase 1 — `payment-adapters` crate + real HMAC verification.**
Create the crate (§2): `StripeAdapter` implementing `PaymentProvider`; `ureq` transport; webhook verification per §2.3 with `hmac`+`sha2`.
*RED→GREEN acceptance:*
- Fixture tests using real-format webhook payloads (`Stripe-Signature: t=…,v1=…` over `"{timestamp}.{raw_body}"`): valid fixture verifies (GREEN only after implementation; the test exists first and is RED).
- Tamper rejection: flipping any single byte of body, signature, or timestamp ⇒ reject. Timestamp outside `WEBHOOK_TS_TOLERANCE_S` (imported from the kernel, not redefined) ⇒ reject. Multiple `v1` entries (secret rotation) ⇒ the valid one accepted.
- Comparison is `Mac::verify_slice` — asserted by code review + a grep gate banning `==` comparison of MAC bytes in the crate.
- Dependency proofs: `cargo tree` in `payment-adapters` shows no `tokio`/`hyper`/`reqwest`; `cd kernel && cargo test` still passes `firewall_p47` and `no_card_data`; `cargo-deny check` green.
- Differential oracle (§2.6): identical fold outcomes vs `NoOpPaymentAdapter` across the conformance sequence.
- The `no-agent-capture-path` gate (§6) lands in this phase, RED-proven against a planted violation.

**Phase 2 — webhook inbound on `native-spa-server`.**
New `/hooks/pay/<provider-id>` route set (§3): raw-body capture, verification, dedup into `IdemLedger`.
*RED→GREEN acceptance:*
- Raw-body integrity: a request whose JSON body would re-serialize differently (key order, whitespace) still verifies — proving the route signs over raw bytes, not a parse-and-reserialize (this test is the RED trap for the most likely implementation bug).
- Replay/dedup: the same event delivered twice folds exactly once (assert ledger/fold state identical after 1st and 2nd delivery); the same event replayed after a simulated hub restart (re-fold of `IdemLedger`) still folds once; a delivery with timestamp outside the window is rejected before touching the ledger.
- Trust boundary: a test asserting `/api/*` routes still refuse requests lacking capability-certs, and that `/hooks/pay/*` accepts no capability-cert as a substitute for a valid signature (a capability-cert-bearing but unsigned request is rejected).
- End-to-end no-webhook invariant re-proven at the HTTP layer: drive create-intent + a client-side "success" report with the webhook endpoint disabled ⇒ state remains `IntentCreated`.

**Phase 3 — owner-desktop shell webview for the card moment (P39-rev/P63, D12 §4-A).**
The unbuilt Tauri desktop shell hosting a live webview that consumes `ClientHandoff::HostedRedirect` — opens `checkout_url` on the provider's own domain, honors `ttl_s` expiry of `session_token`.
*RED→GREEN acceptance:*
- Webview navigates only to the provider domain for the card moment; the shell process never receives CHD — enforced by a `no_card_data.rs`-style grep gate over the shell crate's source, plus the structural fact that card entry happens on the provider's page.
- Kill-the-webview test: terminating the webview mid-checkout leaves the order at `IntentCreated`; completing checkout but suppressing the webhook (Phase-2 harness) also leaves `IntentCreated`; only the webhook advances state.
- Session expiry: an expired `ttl_s` handoff is refused a fresh navigation and forces a new intent through the single-outstanding-intent gate.

**Phase 4 — direct-charges N-leg saga against a sandbox PSP.**
End-to-end `run_nleg_saga` with ≥2 vendor legs against real sandbox/test accounts of the chosen PSP (per D-1), each leg's `dest_account` a distinct test vendor account (direct-charge model, pending D-2 ratification).
*RED→GREEN acceptance:*
- Happy path: all legs captured ⇒ terminal `Committed`; provider dashboard/sandbox records agree leg-by-leg with the kernel fold.
- Abort path: forced failure of leg k ⇒ terminal `Aborted` with zero captured legs remaining (voids proven at the provider), `assert_nleg_atomicity` green.
- Disagreement path: injected provider/kernel disagreement ⇒ terminal `NeedsReconciliation`, never a silent `Committed`.
- Refund path: sandbox refund of a captured leg drives `Refunding→CompensatedRefund`; the double-entry ledger nets to exactly zero via `checked_neg`/`checked_sub`; an over-refund attempt is refused by the money law (RED test asserting the refusal).

Phases 1–3 are independent of D-1 (Stripe reference suffices); Phase 4 requires D-1 and D-2 ruled.

---

## 8. Open decisions for the operator

| # | Decision | Options | Blueprint's recommendation | Blocks |
|---|---|---|---|---|
| **D-1** | Launch PSP for Albania/EU | Stripe (reference only) · Paysera · Viva Wallet · bank acquirer | Build Stripe as reference adapter now; choose launch rail from Phase-0 diligence — Stripe is likely not viable for native Albanian vendor accounts | Phase 4 |
| **D-2** | Charge model for multi-vendor | Direct charges (vendor is MoR, own PSP account) · destination charges (dowiz custodian) | **Direct** — matches `VendorLeg.dest_account` semantics and "dowiz stays out of the money"; tradeoff: per-vendor onboarding friction | Phase 4 |
| **D-3** | Vendor PSP onboarding | dowiz-shepherded (real support cost) · vendor-self-serve | No recommendation — pure business-cost call; interacts with D-1 (whichever PSP has the lightest small-merchant onboarding lowers this cost) | Launch, not build |
| **D-4** | Webhook infra placement | `native-spa-server` route set · separate edge service | **`native-spa-server` extension** (§3.1) — one inbound surface, one fold, raw-body handling is route-local anyway | Phase 2 |

---

## 9. Invariants this blueprint preserves (and how that stays checkable)

- **No cardholder data, ever:** `kernel/tests/no_card_data.rs` (existing) + the Phase-3 shell-crate variant. Full-redirect card moment keeps SAQ-A eligibility.
- **Kernel links no adapter:** `kernel/tests/firewall_p47.rs` (existing), untouched by every phase.
- **Webhook is the sole capture-truth writer:** existing kernel tests + Phase-2/Phase-3 end-to-end re-proofs at the HTTP and shell layers.
- **Settlement/capture never enter the order FSM:** `Delivered => &[]` in `order_machine.rs`; no phase adds an order-graph node; the FSM's five-lens golden-signature self-check stays the tripwire.
- **Agents structurally excluded from money (D14):** existing `scope.rs` red-line denial + `no-agent-order-authority` gate + the new `no-agent-capture-path` gate (§6).
- **Exact-integer money law:** refunds reconcile through the existing `checked_neg`/`checked_sub` double-entry path; no parallel accounting is introduced anywhere in this plan.
- **Dependency discipline:** `hmac`, `sha2`, `ureq` only; provider features off by default; `async-stripe` rejected with recorded rationale; `cargo-deny` gates every phase.

*End of blueprint. Nothing above is built except where explicitly marked as existing kernel code; every "acceptance" line is a test to be written RED first.*
