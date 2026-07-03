# Missing Features & Friction — Research + Plan

**Date:** 2026-07-02 · **Scope:** dowiz storefront/checkout/order-status (customer), courier, owner
**Goal A:** what leading food-delivery apps have that dowiz *lacks*.
**Goal B:** friction audit of the customer order journey toward near-zero-friction.

> Grounded in source (read: `apps/web/src/pages/client/*`, `checkout/*`, courier + admin pages,
> `apps/api/src/routes/orders.ts`, owner/customer routes) + memories (storefront-audit,
> flow-simplification-patch, checkout-communication, mvp-ship-plan) + the Durrës Wolt scout.
> Competitor feature sets researched via web (Wolt, Bolt Food, Uber Eats, DoorDash — 2025/2026).
> This is a **research + plan doc only** — no code changed, no commit.

---

## 0. TL;DR

- dowiz already has the *hard* parts most clones fake: live courier map + honest ETA, order-scoped
  in-app chat, cash-as-proof completion, tipping end-to-end, free-delivery nudge, per-slug delivery
  prefill. The gaps are almost all **retention/repeat-order** features, not core delivery.
- The single biggest structural gap is **no customer identity** (customers are anonymous;
  `apps/api/src/routes/customer/` = orders/otp/push/track, no account). That one absence is the root
  of *five* missing competitor features at once: reorder, favorites, order history, address book,
  cross-device/cross-venue memory.
- The single most embarrassing gap is **orphaned promotions**: owners can create promo codes
  (`PromotionsPage.tsx` + `owner/promotions.ts`) but the customer has **no field to enter one** and
  `orders.ts:509` hardcodes `const discountTotal = 0;` — the whole feature is dead at the till.
- Friction: the delivery path is ~7 required interactions. The worst offenders are a **mandatory
  "How to find you" free-text note on every order** (not prefilled, unlike the address) and a
  **mandatory "cash amount" entry** (type the banknote you'll hand over). Both are removable.

---

## Part A — Feature-Gap Matrix

Legend: **HAS** = built & live · **PARTIAL** = built but dark/orphaned/incomplete · **MISSING** = absent.
Priority: **P1** launch-critical for retention · **P2** high value · **P3** nice-to-have.

### A1. Customer-facing

| Feature (Wolt/Bolt/UberEats/DoorDash) | dowiz | Evidence | Priority |
|---|---|---|---|
| **Reorder / "order again"** | MISSING | Only the words appear in cancelled-help copy `OrderStatusPage.tsx:620`; no reorder action. Returning customer rebuilds the cart item-by-item. | **P1** |
| **Favorites / saved venues & items** | MISSING | No `favorite`/`favourite` token anywhere in `apps/`. | P2 |
| **Order history list** | MISSING | No customer account; no history route (only courier has `/courier/history`). | **P1** |
| **Address book (multiple, labelled home/work)** | PARTIAL | Single last address prefilled per-slug from localStorage `CheckoutPage.tsx:131-149` (`dos_last_delivery_{slug}`). No labels, no multiple, device-local only. | P2 |
| **Saved payment methods** | PARTIAL / N/A | Cash-only market (`PaymentSection.tsx`); crypto dark behind `VITE_PAYMENTS_CRYPTO_ENABLED`. No card vault. | P3 |
| **Live ETA + courier tracking** | **HAS** | `OrderStatusPage.tsx`: `CourierLiveMap`, `etaRange` (honest range, pre-assign/assigned), `routePolyline`, WS live updates. | — |
| **Scheduled / order-ahead delivery** | MISSING | UI shows a "coming soon" placeholder `DeliveryDetailsSection.tsx:159-166`; no `scheduled_for` in `orders.ts`. | P2 |
| **Group order (shared cart, split bill)** | MISSING | No `group order` token; cart is single-device (`CartProvider`). | P3 |
| **Tipping** | **HAS** | `PaymentSection.tsx:62-82` (`checkout-tip`), flows to courier `EarningsPage.tsx:12-14,102-104`. Cash tip, 100% to courier. | — |
| **Ratings / reviews** | PARTIAL | Customer submits one post-delivery rating+feedback `OrderStatusPage.tsx:108-119` (`POST /orders/:id/rating`) + Google-review invite `:90-91`. Not surfaced as public reviews; no separate courier rating; storefront shows owner-typed Google rating only. | P2 |
| **Loyalty / subscription (Wolt+/Bolt Plus)** | MISSING | Only a dormant migration seam (`loyalty_seam.ts` in ignored db pkg); no product surface. | P3 |
| **Promos / vouchers (customer code entry)** | MISSING (orphaned) | Owner CRUD exists (`admin/PromotionsPage.tsx`, `owner/promotions.ts`) but checkout has **no code field** and `orders.ts:509 const discountTotal = 0;` hardcodes zero discount. | **P1** |
| **Dietary / allergen filters** | PARTIAL | Filter predicate built on `computeAllergenSurface` but **flag-dark**: `MenuPage.tsx` `ALLERGEN_FILTER_ENABLED` off, `ALLERGENS_ENABLED = false`, characteristics lenses off. Text search HAS (`SearchInput`). | P2 |
| **Substitutions (out-of-stock handling)** | MISSING | No substitution prefs; closed/unavailable state just gates add-to-cart (storefront-audit Batch 1). | P3 |
| **In-app chat (customer ↔ order)** | **HAS** | `OrderStatusPage.tsx` `MessageThread` + `messages` state + send/fetch endpoints. | — |
| **Free-delivery / min-order nudge** | **HAS** | `ClientLayout.tsx` cart progress bar ("add X more"), from `/info` `free_delivery_threshold` (mvp-ship-plan). | — |
| **Multi-vertical / search discovery** | MISSING / N/A | Single-venue storefront model (`/s/:slug`), not an aggregator marketplace — deliberate positioning, not a gap. | N/A |

### A2. Courier-facing

| Feature | dowiz | Evidence | Priority |
|---|---|---|---|
| Live GPS heartbeat to customer | **HAS** | `DeliveryPage.tsx:179-201` (`/courier/shifts/ping`). | — |
| Swipe-to-complete + cash reconciliation | **HAS** | `DeliveryPage.tsx:223-249` (`payment_outcome:'paid_full'`, cash-as-proof). | — |
| Earnings incl. tips (day/week/month) | **HAS** | `EarningsPage.tsx:12-14,102-104,123`. | — |
| Entrance photo (customer→courier find aid) | **HAS** | shown `DeliveryPage.tsx:412-423` from `ContactInfoSection.tsx:136-155`. | — |
| Shift / tasks / history | **HAS** | `ShiftPage`, `TasksPage`, `HistoryPage`. | — |
| **Turn-by-turn navigation / route to door** | MISSING | Map + polyline shown to customer; courier has no in-app nav handoff. | P2 |
| **Proof of delivery (photo / PIN / signature)** | PARTIAL | Cash-as-proof + swipe only; no delivery photo capture, no handoff PIN/OTP, no signature. | P2 |
| **Batched / stacked multi-order pickup** | MISSING | One task at a time; no stacking. | P3 |
| **Peak/heatmap incentives, weather bonuses** | MISSING | Flat model; no surge/peak surface. | P3 |

### A3. Owner-facing

| Feature | dowiz | Evidence | Priority |
|---|---|---|---|
| Orders board + menu manager | **HAS** | `DashboardPage`, `MenuManagerPage`. | — |
| Branding / storefront / analytics / CRM | **HAS** | `BrandingPage`, `AnalyticsPage`, `CRMPage` (incl. `last_order`). | — |
| Promotions CRUD | **HAS** (but see below) | `PromotionsPage.tsx`, `owner/promotions.ts`. | — |
| **Promotion actually applied to an order** | MISSING | `orders.ts:509` never applies a discount — owner-created promos do nothing. | **P1** |
| **Loyalty program config** | MISSING | Seam only. | P3 |
| **Scheduled-order acceptance / prep windows** | MISSING | Tied to customer scheduled orders. | P2 |
| Owner→customer remarketing beyond CRM phone list | PARTIAL | CRM stores phone + last-order; no accounts, no consented re-engagement channel wired to promos/loyalty. | P2 |

---

## Part B — Friction Audit (customer order journey)

### B1. Step-by-step trace (delivery, happy path, `/s/:slug`)

| # | Step | Interaction | Notes / friction |
|---|---|---|---|
| 1 | Land on `/s/:slug` | scroll | SSR menu; vendor-info zone; category chips = filter. |
| 2 | Browse / (optional) open dish modal | tap ×N | modal add-to-cart stepper. |
| 3 | Add item(s) to cart | tap ×N | closed-state gates add. |
| 4 | Open cart / Checkout | 1 tap | cart FAB shows count+subtotal → opens checkout **bottom-sheet** over the menu (flow-simpl §1). |
| 5 | Name | type | required. Prefilled from `dos_checkout_draft` if returning. |
| 6 | **Communication channel** | 2 actions (select + type handle) | **required** dropdown then per-kind input `ContactInfoSection.tsx:74-104`. |
| 7 | "I am the receiver" | 0 (default checked) | reveals receiver fields only if unchecked. |
| 8 | Entrance photo | optional | camera/gallery. |
| 9 | Map pin | 1 tap | prefilled from `dos_last_delivery` if returning. |
| 10 | Street address | type | required. Prefilled if returning. |
| 11 | Entrance + Apartment | type ×2 | required **only if no pin**; optional with a precise pin (flow-simpl §3). Prefilled if returning. |
| 12 | **"How to find you" note** | type | **required EVERY order** `DeliveryDetailsSection.tsx:88-102`; **not** prefilled. |
| 13 | Dropoff instruction chip | optional | 5 chips. |
| 14 | **Cash amount** | type | **required**, must be ≥ total `PaymentSection.tsx:49-59` (type the banknote). |
| 15 | Tip | optional | `PaymentSection.tsx:62-82`. |
| 16 | Place order | 1 tap | → `OrderStatusPage`. |

**Minimum required interactions on a first order:** ~7 typed fields + 1 pin + 2 taps.
**Returning customer, same venue, same device:** address block + name + comm prefill, but **steps 12
(note) and 14 (cash) are still mandatory re-entry**, and the whole cart (steps 2–3) is rebuilt by
hand — there is no reorder.

### B2. Top friction points (ranked)

1. **No reorder.** The highest-frequency user (a repeat customer) re-adds every cart item manually.
   Delivery details prefill; the *order itself* does not. This is the #1 repeat-order tax.
2. **Mandatory "How to find you" free-text on every order** — required even when a precise map pin +
   address + entrance are given, and it is *not* persisted like the address is. Pure re-typing.
3. **Mandatory cash-amount entry.** Asking the customer to pre-declare the banknote (≥ total) is
   unusual friction; most flows default to "pay exact / no change needed" and let it be optional.
4. **Two-step required communication channel** (choose channel → type handle) on top of the phone
   field history. For the common case (phone) it's an extra decision before an input.
5. **Dead-end-ish scheduled option + orphaned promo.** "Scheduled" shows "coming soon"
   (`DeliveryDetailsSection.tsx:159-166`); promos can't be redeemed at all — both are visible
   affordances that lead nowhere, eroding trust.

### B3. Top 5 friction removals

1. **Ship reorder** — a "Order again" card on `OrderStatusPage` (and a device-local recent-orders
   list on the storefront) that rehydrates the cart from the last order. Biggest repeat-order win.
   *(Cart/UI only — SAFE.)*
2. **Make "How to find you" optional when a pin + address + entrance exist**, and persist it in
   `dos_last_delivery_{slug}` like the address. Removes a mandatory field for the precise-location
   case. *(FE-only — SAFE.)*
3. **Make cash amount optional / default "exact, no change".** Keep the ≥total validation only when
   the customer opts to say what they'll pay with. *(FE-only; server cash-422 backstop already
   authoritative — SAFE.)*
4. **Collapse the communication step for the phone default** — pre-select phone, fold the handle into
   one field, keep alternative channels one tap away. *(FE-only — SAFE, contract unchanged.)*
5. **Hide non-functional affordances** — remove the "scheduled — coming soon" tab and either wire or
   hide promo entry until it works. Stop showing doors that don't open. *(FE-only — SAFE.)*

---

## Part C — Ranked Build List (value / effort · SAFE vs 🔴 red-line)

Red-line globs per CLAUDE.md: money / auth / RLS / migrations / bulk-edit → Triadic Council before code.

| Rank | Item | Value | Effort | Class |
|---|---|---|---|---|
| 1 | **Reorder** (recent-orders + rehydrate cart) | Very high | Low | SAFE (FE + read-only fetch) |
| 2 | **Friction removals B3.2–B3.5** (optional note/cash, phone-default comm, hide dead affordances) | High | Low | SAFE (FE-only) |
| 3 | **Wire promo redemption** (checkout code field → apply `discountTotal`) | High | Medium | 🔴 money — Council + red tests + parity guardrail |
| 4 | **Customer identity (lightweight, phone/OTP-based)** — unlocks history + address book + cross-device reorder | Very high | High | 🔴 auth/PII/RLS — Council + ADR |
| 5 | **Proof of delivery** (courier photo/PIN handoff) | Medium-high | Medium | 🔴 state-machine/dispatch — Council (extends deliver-v2) |
| 6 | **Scheduled orders** (real `scheduled_for` + prep-window acceptance) | Medium | High | 🔴 contract/state-machine — Council |
| 7 | **Turn cost-free retention on:** un-dark allergen/dietary filters (positive-only sign-off) | Medium | Low | SAFE-ish (flip flag after safety review — see menu-characteristics guardrail #12/#15) |
| 8 | **Favorites** | Medium | Medium | needs #4 first (identity) |
| 9 | **Loyalty / group orders / batched courier / peak incentives** | Lower | High | P3 backlog |

**Sequencing logic:** ship the SAFE cart/UI wins (1–2) first for immediate repeat-order lift; take
promo (3) and identity (4) through Council because they touch money/auth. Identity (4) is the keystone
that later unlocks 8 and the retention half of the matrix — but the *reorder* win (1) is deliberately
built device-local first so it ships without waiting on the auth Council.

---

## Appendix — key file:line references

- Checkout journey: `apps/web/src/pages/client/CheckoutPage.tsx`,
  `checkout/{ContactInfoSection,DeliveryDetailsSection,PaymentSection}.tsx`
- Prefill/persistence: `CheckoutPage.tsx:131-177` (`dos_last_delivery_{slug}`, `dos_checkout_draft_{slug}`)
- Mandatory note: `DeliveryDetailsSection.tsx:88-102` · Cash amount: `PaymentSection.tsx:49-59`
- Tip: `PaymentSection.tsx:62-82` + `courier/EarningsPage.tsx:12-14,102-104`
- Order status (map/ETA/chat/rating): `OrderStatusPage.tsx` (`CourierLiveMap`, `etaRange` ~:83,
  `routePolyline` ~:85, `MessageThread`, rating :108-119/:723, Google invite :90-91)
- Orphaned promo: `orders.ts:509` (`discountTotal = 0`) vs `admin/PromotionsPage.tsx` +
  `api/src/routes/owner/promotions.ts`
- No customer identity: `apps/api/src/routes/customer/` = `{orders,otp,push,track}.ts`
- Scheduled placeholder: `DeliveryDetailsSection.tsx:159-166`
- Courier completion: `courier/DeliveryPage.tsx:223-249`
- Allergen/dietary dark flags: `MenuPage.tsx` (`ALLERGEN_FILTER_ENABLED`, `ALLERGENS_ENABLED=false`)
</content>
</invoke>
