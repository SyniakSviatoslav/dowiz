# Payments (card · cash · crypto) — Research Brief (pre-council)

**Status:** RESEARCH ONLY. No code. Input to the Triadic Council. Owner directive 2026-06-30:
"ability to pay with banking card / cash / crypto — research first (best practices, issues, common
mistakes), then council."

## 0. Where we are today (codebase grounding)
- `payment_method` is a Postgres **ENUM = `('cash')` only** (mig `1780310044710`); orders default
  `payment_method='cash'`. **Cash-only by construction.**
- The whole order/courier/dispatch spine is built on **cash-as-proof** (deliver-v2, mig
  `1790000000073`): delivery completion = cash collected; `completeDelivery` primitive; cash HOLD both
  paths; `payment_outcome`/`cash_amount` were folded into that model.
- Money is **integer minor units** end-to-end; delivery fee + tax are already server-authoritative and
  client-mirrored (ADR-0005). Currency per location.
- No PSP integration, no card data path, no webhooks-in for payments.
- ⇒ Adding card/crypto is **not additive** — it forks the order lifecycle into **prepaid** (paid before
  fulfillment) vs **pay-on-delivery** (today's cash-as-proof), and touches a 🔴 money/contract/migration
  surface end-to-end.

## 1. The hard regional constraint (decides everything)
- **Stripe is (almost certainly) NOT available to an Albania-domiciled merchant.** Adyen/Stripe-class
  global PSPs largely exclude Albania for *acquiring*. Confirm before any provider commitment.
- Realistic options for Albania:
  - **Local bank e-commerce acquirer** (BKT / Raiffeisen / Credins / Intesa) — a hosted-payment-page (HPP)
    redirect gateway, often built on older 3DS1/UPG/“virtual POS”. Requires a **merchant bank contract**.
  - **2Checkout (Verifone)** / **PayPal** — cross-border PSPs that onboard Albania merchants (higher fees,
    payout/withdrawal constraints).
  - **Crypto** — non-custodial processor (BTCPay self-host, or Plisio/0x-style) sidesteps the bank-acquiring
    gap entirely; settlement to stablecoin/fiat depending on off-ramp.
- **NEEDS-HUMAN:** which acquirer/PSP the operator can actually contract with in Albania is the gating fact;
  the architecture must be **provider-agnostic** behind one internal interface so the choice isn't baked in.

## 2. Best practices (synthesized, sourced)
### Card (PCI is the #1 risk)
- **Stay SAQ-A: card data must NEVER touch our servers.** Use the PSP's **hosted fields (iframe)** or a
  **full redirect/HPP**; the browser sends the PAN straight to the PSP, we only ever receive a **token**.
  Direct-post / JS-built card forms are explicitly discouraged (transparent-theft risk) and inflate scope.
- **PCI DSS v4.0.1 (req 6.4.3 + 11.6.1):** every third-party script on the checkout page must be
  inventoried, justified, and change-monitored. Our checkout already loads scripts → this is a real task.
- **3DS2 / SCA:** Albania isn't EEA (PSD2 SCA not legally mandated), but most acquirers require 3DS and it
  **shifts fraud-chargeback liability to the issuer** — treat as default-on.
- **Webhook is the source of truth, not the client redirect.** The browser return URL can be lost/forged;
  payment status MUST be confirmed by a **signature-verified, idempotent webhook** (auth → capture →
  refund). Never fulfill on the redirect alone.
- **Idempotency:** client-generated idempotency key on every charge/refund + a **DB unique-constraint dedup
  table** (insert-wins, NOT check-then-act) for both the create-charge path and webhook ingestion
  ("at-least-once" delivery → you WILL get duplicates).
- **Capture timing:** authorize at checkout, **capture on order acceptance** (not before the venue commits)
  — or auth+capture immediately with auto-refund on rejection. Decide explicitly (affects refund volume).

### Crypto
- **Volatility:** never hold volatile coin — either **auto-convert to fiat on confirmation**, or accept
  **stablecoins only (USDC/USDT)**; USDT-TRC20 is the popular low-fee rail (<$1/transfer).
- **Custody = the compliance fork:** a gateway that **holds funds is an MSB** (full KYC/AML on us,
  weeks of onboarding). A **non-custodial** gateway (funds go straight to our wallet) keeps us out of MSB
  scope. Prefer non-custodial (e.g. self-hosted BTCPay) unless the operator wants managed fiat settlement.
- **Irreversibility:** confirmed crypto = **no chargebacks** (fraud-resistant) BUT refunds are **manual,
  bespoke** — must define refund-in-crypto-vs-fiat, valuation, and timing up front.
- **Confirmation latency (1–15 min):** the order needs an explicit **"awaiting payment confirmation"**
  state; don't dispatch until on-chain confirmed. Handle **under/over-payment** and **late confirmation**.
- Settlement to bank is T+1–T+3 (off-ramp dependent).

### Cash (keep it)
- Cash-on-delivery stays the existing **cash-as-proof** model. Card/crypto are **prepaid** and must
  **bypass the cash-proof completion gate** (a paid order isn't "completed by cash collected"). Courier
  cash reconciliation unchanged for COD.

### Cross-cutting (money correctness)
- One **reconciliation source of truth**: a payments ledger that records every auth/capture/refund/chargeback
  with provider ids; a daily job reconciles provider settlements vs our ledger (partial captures, refunds,
  multi-currency drift, timing gaps are the classic mismatch sources).
- Integer minor units everywhere (we already do); never float money; explicit currency on every record.
- **RLS/tenant isolation** + FORCE-RLS on all new payment tables (red-line discipline).
- Ties into the **already-council-gated refund-ledger** (see mvp-ship memory) — unify, don't duplicate.

## 3. Common mistakes (what to NOT do)
1. **Storing/transmitting PAN** on our servers → PCI scope explosion. (Hosted fields/redirect only.)
2. **Trusting the client redirect** for payment success → fulfill unpaid / miss paid. (Webhook = truth.)
3. **No idempotency** → double charges on retry/timeout; **check-then-act** webhook race → double-process.
4. **Capturing before the venue commits** → high refund rate + customer disputes.
5. **Holding volatile crypto** → margin wiped by intraday swings. (Stablecoin/auto-convert.)
6. **Custodial crypto gateway** → unintended MSB/KYC/AML obligations.
7. **No refund workflow for irreversible crypto** → stuck disputes.
8. **Multi-currency reconciliation drift** (auth vs settlement value) → unexplained shortfalls.
9. **Baking in one provider** → trapped when the Albania acquirer choice changes. (Provider-agnostic port.)
10. **Bolting "paid" onto the cash-as-proof completion** without forking prepaid vs COD → couriers asked to
    collect cash on already-paid orders, or paid orders never marked complete.

## 4. Architecture sketch to put to the council (NOT a decision)
- **`PaymentProvider` port** (auth/capture/refund/verify-webhook) with adapters (local-acquirer-HPP,
  crypto-non-custodial, cash=no-op) → provider-agnostic.
- **`order.payment_status`** state machine: `unpaid(cash) | pending | authorized | paid | failed | refunded`
  decoupled from fulfillment state; **prepaid orders skip the cash-proof gate**.
- **`payments` + `payment_events` ledger** (idempotent, RLS-FORCE, provider ids, amounts integer/minor).
- **Inbound webhook route**: signature-verified, idempotent (unique-constraint dedup), the ONLY writer of
  `paid/failed/refunded`.
- **Checkout**: hosted-fields/redirect per provider; never touch PAN; PCI script-monitoring on the page.
- Migration expands `payment_method` enum (`+card`,`+crypto`) — 🔴 operator-gated.

## 5. Questions for the council / NEEDS-HUMAN
- Which **Albania acquirer/PSP** can the operator actually contract? (gating)
- **Capture policy**: auth-then-capture-on-accept vs immediate-capture+auto-refund?
- **Crypto stance**: support at all in v1? custodial vs non-custodial? stablecoin-only? auto-convert?
- **Refund policy** (esp. irreversible crypto) + who bears PSP fees (merchant vs customer)?
- Albania **legal/tax/AML/e-invoice** obligations for online card + crypto acceptance?
- v1 **scope**: card-only first (cash already works), crypto later? Or all three at once?

## Sources
- PCI: [SAQ-A eligibility (PCI SSC)](https://blog.pcisecuritystandards.org/faq-clarifies-new-saq-a-eligibility-criteria-for-e-commerce-merchants), [SAQ A v4 PDF](https://listings.pcisecuritystandards.org/documents/PCI-DSS-v4-0-SAQ-A.pdf), [v4.0 scope/tokenization](https://petronellatech.com/blog/pci-dss-4-0-shrink-your-scope-with-tokenization-serverless-payment/), [SAQ A hidden trap (TrustedSec)](https://trustedsec.com/blog/the-hidden-trap-in-the-pci-dss-saq-a-changes)
- Albania: [Ecwid payment options for Albania](https://support.ecwid.com/hc/en-us/articles/360003207759-Payment-options-for-Albania), [Shopify gateways Albania](https://www.shopify.com/payment-gateways/albania)
- Crypto: [Stablecoins eliminate chargebacks (Spark)](https://www.spark.money/research/payment-fraud-stablecoin-advantage), [non-custodial/MSB (Aurpay)](https://aurpay.net/aurspace/accept-crypto-payments-without-kyc-non-custodial/), [crypto AML guide (Sumsub)](https://sumsub.com/blog/crypto-aml-guide/), [accept-crypto integration guide](https://superdupr.com/blog/accept-crypto-payments)
- Integration: [Adyen API idempotency](https://docs.adyen.com/development-resources/api-idempotency), [webhook idempotency (Hookdeck)](https://hookdeck.com/webhooks/guides/implement-webhook-idempotency), [idempotency keys (Simplico)](https://simplico.net/2026/04/04/idempotency-in-payment-apis-prevent-double-charges-with-stripe-omise-and-2c2p/), [3DS2/SCA (Solidgate)](https://solidgate.com/blog/payment-authentication/), [reconciliation (Primer)](https://primer.io/blog/what-is-payment-reconciliation)
