# Breaker findings — Order acquisition-attribution `channel`

Attacker: System Breaker DeliveryOS. Target: `docs/design/order-channel-attribution/proposal.md`
(+ `docs/adr/ADR-order-channel-attribution.md`). Read-only source verification only; no product code
touched. Format per finding: `[SEVERITY] vector · finding · break-scenario/number · violated invariant`.

Verdict up front: the design is mostly sound and several of its load-bearing claims were **verified true**
(see "Claims that hold"). The findings below are where the proposal's stated invariants are **factually
contradicted by the code** it points at, plus under-specified capture semantics.

---

## HIGH

### H1 — [B-DATA / B-SEC] The anonymizer never touches `orders.metadata`; the proposal's GDPR dismissal (R4, §8) rests on a false premise
- **Finding.** §8 asserts "the GDPR anonymizer needs **no change** … `client_ip_hash`, which is already
  handled," and R4 disposition: "Redaction/anonymizer already owns `client_ip_hash`." This is **false for
  the jsonb copy.** `AnonymizerService.anonymizeOrder` (`apps/api/src/lib/anonymizer/index.ts:210-222`)
  nulls the dedicated **column** `client_ip_hash` (mig `…054`) plus `delivery_address`,
  `delivery_instructions`, `receiver_*` — but issues **zero writes against `metadata`**. Yet
  `insertOrderWithItems` writes a *second* copy of `client_ip_hash` **inside** the metadata jsonb
  (`order-persistence.ts:95`: `JSON.stringify({ otp_verified, client_ip_hash })`).
- **Break scenario.** Customer requests GDPR erasure → order is "anonymized" → `orders.client_ip_hash`
  (column) = NULL, but `orders.metadata->>'client_ip_hash'` **still holds the sha256(ip‖salt)** forever.
  A daily-salted IP hash is a quasi-identifier; it survives erasure and (see H2) is served to the owner
  dashboard. The `channel` field itself adds **no** PII, so its own blast radius is nil — but a reviewer
  who trusts the proposal's R4 ("no anonymizer change needed") will approve on a **false GDPR claim** and
  leave the pre-existing erasure gap unaddressed. The proposal actively mis-describes the metadata sidecar
  as GDPR-clean.
- **Violated invariant.** "No PII / GDPR anonymizer needs no change" (§8) — the premise is untrue; metadata
  is a jsonb the anonymizer ignores entirely, so it is **not** a GDPR-scrubbed sidecar. (Pre-existing gap;
  the design's sin is perpetuating it via an incorrect claim, a red-line-class error.)

---

## MEDIUM

### M1 — [B-SEC / B-CONSIST] "Written once and never consulted / no reader anywhere" (§8) is false — the owner dashboard reads and returns the whole `metadata` object
- **Finding.** §8: "It is written once and never consulted"; appendix invariant: "`channel` is **write-only**
  — no reader …". But `apps/api/src/routes/owner/dashboard.ts:112-130` does
  `metadata = JSON.parse(row.metadata)` and returns `metadata` **verbatim** in the dashboard API response
  (alongside `preflight`). The moment `channel` is folded into metadata it is **read and shipped** to the
  owner client with zero design/test/i18n consideration.
- **Break scenario.** Day 1 after ship, `GET /owner/.../dashboard` starts returning
  `metadata: { otp_verified, client_ip_hash, channel: "qr" }` per order to the owner UI — an unplanned,
  untested API-contract expansion. (Not a decision-gate reader → not a pricing/dispatch/authz violation,
  and same-tenant only, so not cross-tenant; but the categorical "no reader anywhere / never consulted"
  security claim is simply wrong.) This same path already forwards the raw `client_ip_hash` to the owner
  (compounds H1).
- **Violated invariant.** Appendix: "`channel` is write-only — no reader …" / §8 "never consulted."

### M2 — [B-CONSIST] Under-specified sessionStorage capture → cross-storefront attribution bleed within one tab
- **Finding.** Capture is "read `?ch=` once on shell mount → sessionStorage; missing → default `web-direct`"
  (ADR §5, R1). sessionStorage is per-tab but the design implies **one session-global key**, not per-slug,
  and does not specify whether a mount **without** `?ch=` **overwrites** the stored value or leaves it
  stale. R1 only discusses same-storefront first-vs-last touch.
- **Break scenario.** Same tab: customer opens restaurant **A** via `…/s/restA?ch=qr` (stores `qr`), then
  navigates (SPA client-side or bookmark within the tab) to restaurant **B** at `…/s/restB` with **no**
  `?ch=`. If the no-param mount does not clear the key, B's checkout reads `qr` from sessionStorage → **B's
  order is mis-attributed to `qr`** despite B being a direct visit. Cross-tenant attribution bleed. Impact
  = wrong/lossy analytics, not an incident — but it is a concrete correctness defect the proposal's R1 does
  not cover.
- **Violated invariant.** Implicit "channel reflects *this* order's true acquisition source" — a single
  session-global key across tenants breaks it; the proposal's own §1 goal ("how a customer arrived at *a
  storefront*") is per-storefront but the storage is not.

---

## LOW

### L1 — [B-ANTIPATTERN] `.default('web-direct')` makes `channel` a **required** field in the exported `CreateOrderInput` output type + changes parsed shape (latent foot-gun)
- **Finding.** `CreateOrderInput = z.infer<typeof CreateOrderInput>` (`packages/shared-types/src/legacy.ts:79`)
  is the **output** type. A field declared `channel: OrderChannel.optional().default('web-direct')` is
  **non-optional in the output type** (default guarantees presence post-parse). So the exported
  `CreateOrderInput` TS type gains a **required** `channel`, and `.parse()` output now always contains it.
- **Break scenario (latent).** Any object literal typed `: CreateOrderInput` built without `channel` →
  TS2741 "Property 'channel' is missing"; any test using `toStrictEqual` on a parsed body without `channel`
  → fails on the new key. **Verified there are none today** (grep: no literal `: CreateOrderInput`
  constructors in `apps/`/`packages/`; no `toStrictEqual`/`toEqual` on parsed order bodies in
  `orders-guards.test.ts` / `orders-authz.test.ts`). So this is a **latent** foot-gun for the impl lane and
  future typed callers, not a live break — hence LOW, not the "existing clients break" the task probed for
  (they don't: the field is optional on the wire, `.strict()` only rejects *undeclared* keys).
- **Violated invariant.** Appendix: "`.strict()` behavior for every other field unchanged; no existing
  client breaks" — holds for the **wire**; the caveat is the **type-level** required-in-output shift.

### L2 — [B-SCALE / completeness] A second order-POST client is not instrumented
- **Finding.** Two checkout clients POST to `/orders`: `apps/web/src/pages/client/CheckoutPage.tsx` (React —
  the one the proposal plans to instrument via `apps/web/src/lib/channel.ts`) **and**
  `apps/api/src/client/checkout/app.ts::confirmOrder` (vanilla JS, `POST /api/orders`). The proposal only
  covers the React path.
- **Break scenario.** If `client/checkout/app.ts` is a live storefront path, **all** its orders always send
  no `channel` → server default `web-direct`, regardless of `?ch=` — silent under-attribution. Liveness of
  that client not confirmed in this pass. Worst case = missing data, not a break → LOW.
- **Violated invariant.** §1 goal completeness ("every order attributionally… ") — one order-entry path is
  left un-instrumented.

### L3 — [B-CONSIST, informational] Idempotency: hash-exclusion is **correct**, residual is only first-touch-on-retry
- **Finding.** The task probed whether excluding `channel` from `request_hash` creates a replay/dedup
  problem. **It does not.** `buildRequestHash` (`apps/api/src/lib/order-canonical.ts:29-52`) hashes an
  **explicit field allowlist** (locationId/type/items/pin/address/cash/currency/menuVersion/customerId) —
  adding/omitting `channel` requires zero change to the hash and cannot alter it. The reuse guard
  (`orders.ts:381-386`) 422s only on `request_hash` mismatch; since `channel` is excluded, a legit retry
  that re-derives a *different* channel does **not** trigger a spurious `IDEMPOTENCY_KEY_REUSED` — folding
  it into the hash **would** have (a bug the proposal correctly avoids). Residual: first successful INSERT's
  channel wins; a retry's differing channel is silently dropped (returns cached order via
  `orders.ts:387-390`). Pure first-touch-on-retry non-determinism → lossy analytics, not a dedup/replay
  break.
- **Violated invariant.** None — the design got this right; recorded to close the probe.

---

## Claims that hold (verified true — no finding, logged to prevent re-litigation)

- **Metadata CHECK.** `orders.metadata` carries only `CHECK (jsonb_typeof(metadata) = 'object')`
  (`packages/db/migrations/1780421100057_anti-fake-signals.ts:100-102`). Adding a string key keeps the value
  an object → **constraint untouched.** No other CHECK references `orders.metadata`.
- **`$18` binding.** `metadata` is bound as `$18` inside
  `JSON.stringify({ otp_verified, client_ip_hash })` (`order-persistence.ts:87,95`) — the proposal's write
  site is accurate; extending the object needs no new positional param / column-list change.
- **No new dependency to import `OrderChannel`.** `apps/web` already declares
  `"@deliveryos/shared-types": "workspace:*"` (`apps/web/package.json:14`) and already imports from it in
  ~10 files → importing `OrderChannel` needs **no** `package.json` change. Constraint respected.
- **Customer status page does not leak channel.** `customer/orders.ts` order-status query selects explicit
  columns (`o.id, o.status, o.type, o.delivery_address, …`) — **no `metadata`**; `CustomerOrderStatusResponse`
  (`legacy.ts:132-158`) is `.strict()` and has no metadata/channel field. The 201/200 order responses
  return only `id,status,subtotal,total,created_at,timeout_at` (RETURNING at `order-persistence.ts:88`,
  sent at `orders.ts:390,668`). No channel echo to the customer, no WS broadcast of it. (Owner side leaks —
  see M1.)
- **Back-of-envelope numbers.** 1,000 orders/min = 16.7 ≈ **17/s** ✓; 26 B × 17 = **442 B/s** ✓;
  60,000 orders/hr × 26 B ≈ **1.56 MB/hr** ✓; zero new rows/tx/pool/index ✓. The BoE is honest and the load
  is genuinely negligible — no scaling finding.
- **13-value enum.** ADR lists exactly 13 distinct tokens ✓.
- **Idempotency hash-exclusion** — correct (see L3).

---

## Severity summary
| # | Sev | Vector | One-liner |
|---|-----|--------|-----------|
| H1 | HIGH | B-DATA/B-SEC | Anonymizer never scrubs `metadata`; R4/§8 "anonymizer already owns client_ip_hash" is false → hashed-IP survives erasure (channel PII-free, but the GDPR premise is wrong). |
| M1 | MED | B-SEC/B-CONSIST | Owner dashboard JSON.parses + returns whole `metadata` → "written once, never consulted / no reader" (§8, appendix) is false; channel ships to owner API day 1, untested. |
| M2 | MED | B-CONSIST | Session-global (non-per-slug) sessionStorage + unspecified no-`?ch=` overwrite → cross-storefront attribution bleed in one tab (A's `qr` tags B). |
| L1 | LOW | B-ANTIPATTERN | `.default()` makes `channel` required in the exported `CreateOrderInput` output type / parsed shape; latent (no live constructor/snapshot break found today). |
| L2 | LOW | completeness | Second POST `/orders` client (`client/checkout/app.ts`) uninstrumented → always default `web-direct` if live. |
| L3 | LOW | B-CONSIST(info) | Hash-exclusion correct; residual = first-touch-on-retry non-determinism only. No dedup break. |
