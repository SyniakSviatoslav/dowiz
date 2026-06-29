# Adversarial 7-lens audit — dowiz/DeliveryOS (2026-06-29)

Seven decorrelated adversaries attacked the service: **security** (AppSec red-team, Decepticon-substitute),
**owner**, **courier**, **client**, **market**, **competitor**, **design/UX**, **sustainability/ethics**.
Read-only; findings traced to file:line in the lane transcripts. This is the prioritized synthesis +
disposition. Dispositions: 🔴 **COUNCIL** (auth/RLS/money/state-machine red-line → Triadic Council + operator
before code) · 🟢 **INLINE** (safe FE/non-contract fix) · 🧭 **ROADMAP** (product/strategy/ethics).

## TOP LAUNCH-BLOCKERS (convergent across lanes — fix before launch)

| # | Finding | Lanes | Disp |
|---|---------|-------|------|
| **B1** | **Money model is inverted.** `payouts.total_earned` = cash the courier *collected* (full COD value), but the Payouts UI frames it as what the owner *pays* the courier. No commission/per-delivery-fee/earnings model exists; cash-ledger only writes `'hold'` (never `release/settle`); no refund reversal. An owner could pay the courier the whole cash value *on top of* the cash they hold. | owner(O1)/courier(C1,C2,O7)/client(C4) | 🔴 COUNCIL |
| **B2** | **Dispatch auto-recover is dead code.** `courier_dispatch_queue` is written on reject/decline/cancel/expiry but **never drained** (0 `boss.send`); the retry calls a non-existent `this.boss` → TypeError. Orders are never auto-re-offered; `reoffered:true` + "→ re-offered" logs are false. Reject-before-accept orphans the order at IN_DELIVERY with no working recovery (re-tap → `SameStatusError` 400). No `'assigned'` acceptance timeout. | owner(O2,O3,O4)/courier(C3) | 🔴 COUNCIL |
| **B3** | **Tenant isolation rests on app `WHERE` clauses only** — hot-path role is `BYPASSRLS` so FORCE-RLS is inert (C1-sec); the `customers` anon policies lack `WITH CHECK`/scope (H3 — the landmine), DEFINER fns lack `search_path` (H2). Staged fixes exist (pg-privilege MIG-1/2 + RLS-WITH-CHECK). Order matters: H3+GUC → H2 → C1-flip. | security(C1,H2,H3,M1) | 🔴 COUNCIL (partly staged) |
| **B4** | **BOLA on `/api/admin/*`** — any restaurant *owner* JWT can read all tenants + trigger DR drills (no platform-admin role distinct from `owner`). | security(H1) | 🔴 COUNCIL |
| **B5** | **ReconciliationWorker unregistered in prod** → zero money-drift / stuck-shift / stuck-order alarms. Missed new-order auto-cancels with only Telegram/push as alert. | owner(O5,O6) | 🔴 COUNCIL |

## STRATEGIC / EXISTENTIAL (roadmap — the moat)
- **No demand side / no retention engine.** Per-restaurant `/s/:slug` pages, zero discovery → cold-start; no customer account/history/reorder/loyalty. dowiz is a *retention/margin tool, not a demand network* — position/price accordingly. (market M1/M5, competitor #1/#9, client C2) 🧭
- **Moat = price + a copyable feature.** A capitalized aggregator bundles free white-label direct-page + white-label courier fleet + cash-advance financing and collapses it. **Defensive priority:** demand-enablement (WhatsApp/Instagram/Google ordering + loyalty) → payment/cash rail (lock-in/float/card-hedge) → courier-fleet partnership (delivery liquidity beyond 1–5 staff). (competitor #1–8) 🧭
- **Genuinely defensible (pour here):** incentive alignment (SaaS, not an order tax), cash-native back-office in a 77%-cash market, owned-customer/loyalty (what aggregators withhold), white-label brand. (competitor + ethics) 🧭

## PRODUCT GAPS (council where money/state, else roadmap/inline)
- **Customer cancel before dispatch — no path + no UI at all** (client C1). 🔴 COUNCIL (state-machine) — already in the MVP plan's Phase-2.
- **No refund/dispute path** (client C4 / owner O7) — the refund credit-ledger design is unbuilt. 🔴 COUNCIL.
- **Cash-only**, **Telegram-only** (WhatsApp is the Albanian default + unbuilt), no SMS/email track link. 🧭/🔴.
- **GPS pin silently defaults to the restaurant's own coords** if undragged (client C3) → mis-delivery. 🟢/🔴 (delivery correctness).
- **Photoless menus** dominate (client C6 / design HIGH-1). Image pipeline + text-first photoless card. 🟢 (FE) + onboarding.

## DESIGN/UX (mostly 🟢 INLINE — safe FE)
- HIGH-1 sea of identical placeholders → text-first card for photoless items (no fake photo slot). HIGH-2 card info-overload → name/price/desc/one-scent on card, rest in modal. HIGH-3 **price wraps** (`ALL` 2nd line) → one baseline + nowrap. MED: category-nav overflow clipped; cryptic taste glyphs; modal duplicates dish name; **owner dashboard test-fixture clutter** ("E2E D1", "361 min late") + chrome banner; stacked sticky rows.

## SECURITY — safe inline (🟢) after the red-lines
M2 Telegram webhook fail-open on absent header (borderline — drives order state); M3 unescaped HTML in owner Telegram alerts (phishing + DoS); M4 SSRF IPv4-mapped-IPv6 bypass; M5 velocity bypass via omitted phone; M6 Sentry doesn't scrub request body; LOW: anonymizer leaves `delivery_instructions`, `items` no `.max`, JWT no `aud/iss`.

## SUSTAINABILITY / ETHICS (🧭 + one cheap win)
- **Carbon-blind + nudges *more* delivery** (no batching, no pickup-nudge, no CO₂ transparency) while free-delivery + weekly-count push volume. **Cheap high-leverage win:** reuse `tools/loop-harness/src/eco.ts` + the existing `delivery_trace.route_distance_m` to surface a per-order/month CO₂ estimate + a first-class **pickup / bike-courier** lower-impact choice. (ethics HIGH-1)
- **Product AI footprint unmeasured** + shadow-provisioning runs costly extraction **eagerly, pre-consent**, then reaps it (HIGH-2) → meter + lazy-extract.
- **Shadow-provisioning is opt-out** ("build-first-ask-later" — the squatting posture the design elsewhere condemns) — heavily mitigated (claim≠publish, Art-14, decline=claim prominence, robots) but residual; consider interest-first variant. (HIGH-3)
- **Customer no_show/refund signal is global, owner-judged, cross-tenant, invisible to the subject** → scope to same location + disclose + audit owner marks. (MED-4)
- **Courier labor:** unpaid cold-start waiting, piece-rate, no expected-pay-before-accept, no hourly floor. → surface earnings on the offer card + shortfall-dispute path. (MED-5)
- **Genuinely MORE ethical than aggregators (keep):** 0% commission + restaurant-owned brand/customer, GPS minimization with a consent boundary, "record-don't-judge" (no verdict engine, soft-confirm only), till-accountability (no fronted capital), conscientious GDPR. These ARE the charter in code.

## CONFIRMED-SECURE (verified negatives — do not re-flag)
dev-backdoor fail-closed (3 layers), JWT RS256, refresh-token rotation, server-authoritative integer money + idempotency + cash-as-proof HOLD (no double-spend), SQLi clear, `.env` gitignored, pg-boss IDs-only, `/internal/acquisition` fail-closed, strong per-route rate limits, humane error/empty states, `derivePalette` AA, motion/reduced-motion discipline.

## Recommended sequencing
1. **Security council** (B3+B4 + the safe M2–M6): B3 order is H3+GUC → H2 → C1-flip; B4 platform-admin gate. Several already staged.
2. **Order-money + dispatch council** (B1+B2+B5 + customer-cancel + refund-ledger): the biggest launch-blockers — the money model + the dead re-dispatch machinery.
3. **Inline now (no council):** the design 🟢 fixes (HIGH-1/2/3, modal dup, demo cleanup) + the CO₂/pickup eco win + the safe security M3/M6/LOW.
4. **Strategy roadmap:** demand-enablement → payment rail → courier fleet (the moat, before aggregators react).
