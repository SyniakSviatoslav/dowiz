# DELIVERY MVP FEATURE-COMPLETENESS AUDIT — client / owner / courier / cross-cutting (2026-07-18)

> **What this is:** a research/audit document (groundwork, not a blueprint) answering the
> operator's ask: think through every aspect of a food-delivery MVP — order intake and control,
> inventory, and every key service need — across the client, owner, and courier layers, using
> BOTH the old (deleted) stack's real feature surface AND the current roadmap as evidence.
> **No code changed; no phase numbers minted** (a parallel pass is writing P51 right now —
> collision risk is real; every MISSING item below names a proposed HOME and leaves numbering
> to a later consolidation pass).
>
> **Method + sources (all verified live this session, 2026-07-18):**
> 1. Prior art built on, not re-derived: `DELIVERY-EDGE-CASES-AND-DETERMINISTIC-INVENTORY-2026-07-17.md`
>    (read in full — its §3 conflict/cancel/idempotency catalog and §4 `stock.rs` design are
>    treated as established), which itself builds on the backend audit (R1–R8) and
>    `HUB-DESIGN-VENDOR-MARKET-RESEARCH-2026-07-17.md` (G1–G8, D1–D6, O20).
> 2. Old-stack surface: `git log --all --diff-filter=A --name-only` over
>    `apps/api/src/routes/**` (69 route files) and `apps/web/src/pages/**` (62 page files) —
>    the stack deleted in the 2026-07-13 purge (`79ef316f6` apps, `f9ab28ff1` attic) but fully
>    present in history. §1 enumerates it.
> 3. Current roadmap: `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §2 (P01–P19),
>    §8 (P20–P30), §10 (P31–P46), §11 (P47–P50) — read in full for §10/§11.
> 4. Live kernel state: `kernel/src/domain.rs` (0 hits for `stock`/`address` — confirmed by
>    grep this session), `kernel/src/cart.rs` (real Cart with add/remove/price/reconcile),
>    `kernel/src/decision/mod.rs:38,202` (`DomainTag::MenuInventory` + `MenuInput{item, stock,
>    is_86} → MenuOut{order_safe}` — a decision-routing SHAPE, not a stock ledger),
>    `bebop-repo/bebop2/delivery-domain/src/pod.rs` (k-of-n hybrid-signed DeliveryClaim, built),
>    `bebop-repo/bebop2/proto-cap/src/matcher.rs:63` (`assign(order, candidates, max)` — the
>    caller supplies the candidate set; grep for `shift|on_duty|availability` across
>    delivery-domain + claim_machine: **zero hits**).
>
> **Status vocabulary:** COVERED = an existing phase owns it, cited, nothing to add here ·
> PARTIAL = a phase owns the area but a named piece inside the old stack's scope has no DoD
> line · MISSING = no phase owns it at all (candidate for the consolidation pass) ·
> REJECTED-BY-DESIGN = deliberately absent, with the standing rejection cited.

---

## 1. The old stack's real feature surface (from git history — what a working MVP actually contained)

One line per feature area; grouped, not exhaustive per-file. Where a header comment was read
this session it is cited; otherwise the description is inferred from the (self-describing)
route/page name and marked (n).

**API routes (`apps/api/src/routes/`, 69 files):**

| Area | Files | What it did |
|---|---|---|
| Orders core | `orders.ts`, `order-messages.ts`, `owner/order-meta.ts` | Order lifecycle CRUD (the untested-hotspot biomarker); per-order chat between parties (n); owner-side order annotations (n) |
| Customer | `customer/{orders,otp,track,push,ratings}.ts` | Customer's order list; phone-OTP verification; anonymous order tracking (`softVerifyAuth` precedent, cited by P49 §11); web-push subscribe; ratings submit (n) |
| Courier | `courier/{auth,me,shifts,assignments,settlements}.ts`, `couriers.ts` | Courier login/profile; **shift open/close (on-duty state)**; task/assignment list; courier-side earnings/settlements (n) |
| Owner menu | `owner/{products,categories,modifier-groups,product-media,menu-availability,menu-import,menu-translate,menu-confirm}.ts` | Full menu CRUD incl. item photos/media, option groups, per-item availability (86) toggle, bulk import, translation; `menu-confirm` = the owner's deliberate authenticated allergen-confirmation act (header read: "the owner AUTHORS allergens into empty fields… then confirms HERE") |
| Owner ops | `owner/{dashboard,couriers,courier-invites,locations,settlements,refunds,ratings,alerts,notifications,signals,dwell-settings,reveal-contact,fallback}.ts` | Live order dashboard; courier roster + invite flow; multi-location; owner settlements; refund execution; alerting/notification prefs; dwell-time settings; privacy-preserving contact reveal (n) |
| Owner lifecycle | `owner/{onboarding,activation}.ts`, `public/{claim,access-requests}.ts` | Venue onboarding/activation; public venue-claim surface (header read: "the TOKEN is the sole transfer authority"); access requests |
| Payments | `payments-webhook.ts` | Inbound payment-provider webhook (card rail) |
| Public storefront | `public/{menu,rates,client-flow,funnel,og-card,seo,ssr,pwa,vapid,theme,branding-preview,fallback-config,telemetry,voice-config}.ts` | Public menu; delivery rates; client flow config; anonymous funnel sensor (header read: "pure observation… NEVER affects" the flow); OG/SEO/SSR; PWA + VAPID push keys; theming; degraded-mode config; voice config |
| Platform | `auth.ts`, `auth/local.ts`, `admin/{backups,fallback,notification-audit}.ts`, `health.ts`, `telegram-webhook.ts`, `spa-proxy.ts`, `dev/mock-auth.ts` | Auth; admin backups; notification audit; health; Telegram webhook; SPA proxy; dev auth |
| Compliance | `owner/gdpr.ts` (+ `workers/anonymizer-gdpr.ts`, per P50) | GDPR data-subject machinery — already inventoried verbatim by P50 DoD-1 |

**Web pages (`apps/web/src/pages/`):** client (`MenuPage`, `CheckoutPage` + 6 sections,
`OrderStatusPage`, `MenuComparePanel`); **courier — a complete 7-page working app**
(`LoginPage`, `CourierInvitePage`, `TasksPage`, `DeliveryPage`, `ShiftPage`, `EarningsPage`,
`HistoryPage`); admin/owner (`DashboardPage`, `DispatchView`, `MenuManagerPage`,
`AllergenEditor`, `RecipeEditor`, `SupplyLibraryPage`, `PromotionsPage`, `CouriersPage`,
`AnalyticsPage`, `CRMPage`, `BrandingPage`, `QRKitPage`, `SettingsPage`, `OnboardingPage`,
`ActivationPage`); plus landing, `ClaimPage`, `MenuFirstOnboarding`, `PrivacyPage`.

Two structural observations this enumeration forces:

- **The old stack treated the courier as a first-class user with a full app** (7 pages, 5 route
  files, shift state, earnings). The new roadmap treats the courier as a protocol actor
  (claim_machine, matcher, PoD, payout saga — all real) but **no P-phase owns a courier-facing
  surface** (P38b Sea/Sheet is customer-facing by its own text; P48 is the owner; P49 is the
  customer). See §4 and §6.
- **Inventory in the old stack was owner tooling** (`SupplyLibraryPage`, `RecipeEditor`) — and
  its new-stack successor (`stock.rs`, edge-cases doc §4) is fully designed but owned by **no
  phase number** (the design assigns its PIECES to P07/P13/D1 but the module itself to
  "Wave-0 kernel work" with no P-id). See §3 row 2 and §6.

---

## 2. CLIENT layer

| Feature | Old stack had it? | New roadmap phase | Status | Gap / proposed home if not COVERED |
|---|---|---|---|---|
| Browse menu | `public/menu.ts`, `MenuPage` | HUB-D1 `MenuRevision`/`PriceCatalog` (kernel model) + P38b Sea surface (DZ-01..12) | **COVERED** | — |
| Search / filter within menu | Part of `MenuPage` | P38b (surface behavior, not separately specified) | **COVERED** (thin) | Surface detail inside P38b's DZ scope; no separate item warranted |
| Cart | Client-side + checkout | `kernel/src/cart.rs` — built, tested (RW-07 DONE per §10.5.3) | **COVERED** | — |
| Checkout / place order | `CheckoutPage` + 6 sections | P37 DoD-2/DoD-5 (wire + F12 local path) + P38b DoD-2 | **COVERED** | — |
| Order tracking (live) | `customer/track.ts`, `OrderStatusPage` | P49(c) DoD-4 — Kalman/EMA geo rendered via P38a | **COVERED** | — |
| Order history + re-order | `customer/orders.ts` | P49 identity is deliberately **single-order** scoped; its anti-scope: "No customer account… beyond what one order needs" | **PARTIAL** | Cross-order history/re-order has no home and is *excluded* by P49's anti-scope as written. Proposed home: P49 extension item, decidable only AFTER the §11.2-3 identity ruling (candidate 2, one-order capability grant, structurally precludes history; candidate 3, magic-link, permits it). Post-MVP — flag to the identity ruling, do not pre-empt it |
| Ratings / reviews | `customer/ratings.ts`, `owner/ratings.ts` | None. Courier-directed ratings: standing NO-COURIER-SCORING rejection (M12; matcher.rs comment: "the type literally" cannot rank) | **MISSING** (venue/dish only) / **REJECTED-BY-DESIGN** (courier) | Venue/dish ratings are unowned and philosophically adjacent to the scoring rejection — needs an ⚠ operator ruling on whether they exist at all before any home is proposed. Post-MVP |
| Complaint / refund request | `owner/refunds.ts`, `order-messages.ts` | P14 dispute/escrow (gated O3); refund = money-plane ledger entry, never a state edit (edge-cases §3.3, settled) | **COVERED** (gated) | Customer-side intake UX rides P14 once O3 is ruled; nothing to add |
| Dietary / allergen info | `AllergenEditor`, `owner/menu-confirm.ts` (deliberate owner confirmation act) | HUB-D1 menu-as-data — but D1's `MenuItem` fields as drafted don't name allergens, and no DoD carries the owner-confirmation act | **PARTIAL** | Proposed home: one field-level addendum to HUB-D1 (`allergens` on `MenuItem`, same shape as the already-proposed `AvailabilitySet.cause` addendum) + the confirmation act as a P48 surface item. The legal side (mandatory allergen disclosure) belongs to P50 DoD-1's audit — flag, don't self-certify |
| Delivery-time estimate at checkout | Checkout flow | HUB-D4 StoreState + prep-time estimator (`ema_next`/`kalman.rs` named there); P49 for in-flight ETA | **COVERED** | — |
| Minimum-order / delivery-fee rules | `public/rates.ts` + checkout | P13 — the backend audit's R2 names the missing fee authority; edge-cases §3.4 adds distance-tiered fee re-quote (address field gap R-g) | **PARTIAL** | Fee authority is P13-owned (cited). **Minimum-order-value policy is not named anywhere** — one policy line inside P13's fee authority, not a new phase. Proposed home: P13 |

## 3. OWNER layer

| Feature | Old stack had it? | New roadmap phase | Status | Gap / proposed home if not COVERED |
|---|---|---|---|---|
| Menu CRUD (item, price, category, availability toggle) | 8 route files (§1) + `MenuManagerPage` | HUB-D1 (`MenuRevision` + `AvailabilitySet` w/ manual-86) + P48 DoD-2 ("owner edits a menu item and sees it live" — that phase's stated reason to exist) | **COVERED** | — |
| Menu item **photo/media** | `owner/product-media.ts` | None. No blob/media-storage concept exists anywhere in the new stack; "photo" appears only in P13's PoD capture ("geo + optional photo"). Nearest primitive: content-addressed `kernel/src/backup.rs` store | **MISSING** | A real client's menu needs images; P48's surface implies them but no phase owns media ingest/storage/serving. Proposed home: P48 scope extension (owner uploads → content-addressed store → P37 serves), sharing whatever capture/blob path P13's PoD photo lands. Small, but genuinely unowned |
| Menu import / translate | `owner/menu-import.ts`, `menu-translate.ts` | None (AGENT P40+ could assist later) | **MISSING** | Onboarding convenience, post-MVP. Proposed home: P48 backlog note; translation later as an AGENT tool — do not build pre-P40 |
| **Inventory / stock tracking (does an item 86 itself at zero?)** | `SupplyLibraryPage`, `RecipeEditor` (owner tooling; audit row 21: the UI plan's supplies page is localStorage) | Fully DESIGNED in edge-cases §4 (`stock.rs`: `Received/Reserved/Consumed/Released/Wasted/Stocktake`, invariants I1–I4, auto-86 via `AvailabilitySet{cause: StockDerived}`) — but the module is assigned only to "Wave-0 kernel work"; **no P-number owns it**. Live kernel: `domain.rs`/`cart.rs` have zero stock concept; `decision/mod.rs:202` `MenuInput.stock` is a routing shape, not a ledger | **MISSING** (from the phase index; design exists) | Proposed home: **new DELIVERY/CORE phase** ("Deterministic stock ledger", implementing edge-cases §4 verbatim — next available number, consolidation pass assigns) OR explicit absorption as a lettered sub-phase of P48 (it is the owner's surface's data spine). Its named prerequisites stay as designed: §3.1-F1 fix, §3.2 id minting (P13), §3.3 cancel edges (P07), `AvailabilitySet.cause` (D1) |
| Order queue / kitchen display | `DashboardPage`, `DispatchView` | P48 DoD-3 (live orders as read-only fold projection) | **COVERED** | — |
| Accept / reject / cancel per order | Dashboard actions | FSM `Pending→Confirmed/Rejected` exists (`order_machine.rs:67`); post-Confirmed cancel + failed-delivery edges = P07 (edge-cases §3.3, operator-gated R3/FSM-signature bump) | **COVERED** (gated) | The gate itself is the risk: `InDelivery` has one exit — a failed delivery is unrepresentable until P07's edges land. Flagged for MVP sequencing in §7, not re-designed |
| Prep-time estimate per order | `owner/dwell-settings.ts` | HUB-D4 (StoreState + estimator) | **COVERED** | — |
| Staff / courier roster | `owner/couriers.ts`, `CouriersPage` | P48 DoD-4 (grant/revoke courier capability certs; `RevocationSet` exercised) | **COVERED** | — |
| Payout / settlement visibility | `owner/settlements.ts` | HUB-D5 owner statement projection + P47 rails + P13 ledger (F43) | **COVERED** | — |
| Reporting / analytics | `AnalyticsPage`, `public/funnel.ts` | HUB-G7 PARTIAL (`ChannelLedger` reader exists); P48 anti-scope explicitly excludes analytics dashboards | **PARTIAL** | Deliberately post-MVP; G7's deterministic-reader extension is the named path. No action for MVP |
| Promotions / discounts | `owner/promotions.ts`, `PromotionsPage` | P20 DM-1..8 (confirmed 0% built, §10.5.3); DM-1 kernel discount math hosted in P39 DoD-3 | **COVERED** | Cross-referenced only, per instructions |
| Operating hours / pause / holiday closure | `SettingsPage` (n) | HUB-D4 StoreState (busy w/ auto-expiry, pause, hours) | **COVERED** | D4 is an addendum extending P10/P16 rather than a numbered phase — the consolidation pass should confirm D1/D4 have an executing owner, same class of orphan-risk as `stock.rs` but already flagged by the HUB doc itself |
| Multi-location | `owner/locations.ts` | Open operator decision **O20** (HUB-D6) | **COVERED** (gated) | Cross-referenced, not re-decided |
| Owner/venue onboarding + claim | `owner/{onboarding,activation}.ts`, `public/{claim,access-requests}.ts`, 3 pages | None — P48 assumes an owner already exists; MESH-12 hub genesis is node-level, not owner-UX | **MISSING** | For the first client, operator-assisted onboarding (a runbook, not software) is MVP-sufficient. Self-serve venue claim/onboarding: proposed home = P48 extension, explicitly post-MVP |

## 4. COURIER layer

| Feature | Old stack had it? | New roadmap phase | Status | Gap / proposed home if not COVERED |
|---|---|---|---|---|
| **Courier working surface (the app itself)** | 7 pages + 5 route files — a complete app (§1) | **None.** P38b = customer surface (its own text); P48 = owner; P49 = customer. The protocol side (claim_machine, matcher, PoD, payout) is built/planned — the courier's screen is owned by nobody | **MISSING** | Proposed home: **new DELIVERY-component phase** ("Courier working surface": authenticate with device-bound cert → see offered claims → accept → PoD capture → mark delivered → see earnings), sibling of P48/P49, rendered per the same P48-style ⚠ rendering ruling. Number assigned by consolidation pass. This is the largest single omission this audit found |
| Shift / availability toggle | `courier/shifts.ts`, `ShiftPage` | None. `matcher.rs:63 assign(order, candidates, max)` — the CALLER supplies candidates; grep for shift/on_duty/availability across delivery-domain + claim_machine: zero hits. `DeliveryEvent` has no availability variant | **MISSING** | Who feeds the candidate set is unowned. MVP stopgap (state it, don't leave it implicit): candidates = all P48-certified, non-revoked couriers — viable at first-client scale (1–3 couriers) because the claim flow is pull-based (an off-duty courier simply never accepts; `primary_for` requeue already handles refusal). Real fix: an `AvailabilityChanged`-style event family folding to the candidate set (same event-log pattern as everything else). Proposed home: P34 wire-side follow-up or the courier-surface phase above — NOT a new event variant inside P34 itself (its anti-scope forbids that) |
| Claim / accept / reject a delivery | `courier/assignments.ts`, `TasksPage` | P34 — MESH-03 `ClaimOffered/ClaimAccepted/ClaimReleased` + MESH-04 claim_machine + MESH-05 matcher (all built, wiring is the phase) | **COVERED** | — |
| Navigation / routing to address | `DeliveryPage` | **P51 (mapping phase, being written in a parallel pass right now)** | **COVERED** (pending) | Dependency noted, zero duplication here. One seam to hand P51: kernel `Order` has no address field (edge-cases §3.4, R-g, P13/P16-flagged) — navigation needs the address to exist first |
| Proof-of-delivery (photo / signature) | `DeliveryPage` flow | F42 (ARCHITECTURE.md:111 "PoD signed by edge ML-DSA" LOCK) + P13 ("multi-signal: geo + optional photo — one capture flow feeding PoD AND the splat bootstrap"); `delivery-domain/pod.rs` k-of-n hybrid-signed DeliveryClaim is BUILT | **COVERED** | Structurally yes: the claim is cryptographic (k-of-n distinct hub signatures over `order_id‖location‖timestamp`), stronger than the old stack's photo-only evidence. The capture UX rides the courier-surface phase above |
| Earnings visibility | `courier/settlements.ts`, `EarningsPage`, `HistoryPage` | Money mechanism COVERED (F43 payout saga in P13; P47 settlement; D5 statement pattern) — but D5 is owner-facing by name; no courier-facing projection is specified | **PARTIAL** | Derive-only work (a second reader over the same ledger, per D5's own pattern). Proposed home: the courier-surface phase; explicitly NOT new money logic |
| Multi-order batching | Not established — no batching route/page existed; `TasksPage` was a list | Nothing implies it | **Out of scope** | Per this audit's ground rule: neither the product's history nor its roadmap establishes the need. One-line verdict, no proposal |
| Cash-collected attestation | Implicit in old cash flow | P47 Wave-0 (courier's signed cash-collected attestation → `SettlementRecorded`) | **COVERED** | Its input surface is, again, the courier-surface phase |

## 5. CROSS-CUTTING

| Feature | Old stack had it? | New roadmap phase | Status | Verdict / reasoning |
|---|---|---|---|---|
| OTP / verification | `customer/otp.ts` (prior-defect biomarker), `auth/local.ts` | P37 DoD-4 + §10.3 invariant 3 (capability certs PRIMARY); P23 (TOTP step-up + cert-minting enrollment); P49 identity candidates | **COVERED by replacement** | Reasoned through, per the task: OTP existed to bootstrap identity where none existed. Couriers/owners now carry device-bound certs — no OTP need remains there (TOTP is step-up only, a different role). The CUSTOMER-side need collapses into P49's §11.2-3 ruling, where candidate 3 (magic-link via email/SMS) IS the OTP pattern under another name. Conclusion: no standalone OTP phase is missing; the need survives only inside P49's decision space |
| GDPR / data-subject rights | `owner/gdpr.ts` + anonymizer worker + admin page | P50 DoD-1 — names these exact files as the audit's starting inventory | **COVERED** | Finding confirmed, not duplicated. P50's audit half is explicitly startable now (git history exists) |
| Courier invite / onboarding | `owner/courier-invites.ts` (prior-defect biomarker), `CourierInvitePage` | P48 DoD-4 (roster grant/revoke) + P23-P2 (TOTP-verified enrollment mints capability cert; QR/otpauth enrollment UX in `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` §5.2) | **PARTIAL** | The crypto mechanism is fully specified. The gap is exactly the seam the task asked about: the **invite handoff UX** — how the owner's grant reaches the courier's not-yet-enrolled device (out-of-band link/QR that bootstraps P23-P2 enrollment against P48's roster) is implied by both DoDs and named by neither. One explicit line item at the P48-DoD-4 ↔ P23-P2 boundary; not a new phase |
| Notifications / push | `*/push.ts`, `public/vapid.ts`, `owner/{alerts,notifications}.ts`, `admin/notification-audit.ts` | P43 DoD-2 (the transmitting send path — the corrected "Telegram push exists" false claim); P49 DoD-3 (customer consumer, "stays RED until P43 DoD-2 transmits"); L3 courier wake folded into P13 (§9.2) | **COVERED** with a sequencing tension | The pieces all have owners, but P49 (DELIVERY, MVP-near) depends on P43 DoD-2 (ECOSYSTEM, "explicitly LAST"). For the first transaction the P49 tracking page can substitute (customer polls); the moment real notifications are MVP-required, P43 DoD-2 alone must be pulled forward — flagged in §7, no re-design here |
| Payment webhook / rails | `payments-webhook.ts` | P47 (cash Wave-0; card rail behind §11.2-1 operator decision) | **COVERED** | — |
| PWA / installability / SEO / OG | `public/{pwa,seo,og-card,ssr}.ts`, `sw.js` | P39 (installability canon decision + shell); P20 (demo/OG assets) | **COVERED** | — |
| In-order chat (customer ↔ owner/courier) | `order-messages.ts` | None | **MISSING** | Post-MVP. The boring MVP answer already exists: `kernel/src/messenger.rs:33 telegram_link()` deep-link (non-sending by design) puts the parties in an existing channel. Proposed home: P43 messenger-port extension if ever built natively |
| Degraded-mode / fallback config | `{owner,admin}/fallback.ts`, `public/fallback-config.ts` | F12 offline canon + P41 DoD-3 degradation contract + fail-closed doctrine throughout | **COVERED** | Structurally superseded: the new stack degrades by construction rather than by owner-configured fallbacks |
| Health / backups / admin ops | `health.ts`, `admin/backups.ts` | P45 (heartbeat pattern proven; `backup.rs` built; off-site backup = its one stays-red item) | **COVERED** | — |
| Theming / branding | `owner/themes.ts`, `BrandingPage`, `public/{theme,branding-preview}.ts` | P38a FE-05 (GPU design-token table) + P38b Sheet (brand-SDF) | **COVERED** | — |
| CRM | `CRMPage` | P49 anti-scope: "no loyalty, no CRM, no marketing identity" | **REJECTED-BY-DESIGN** (current stance) | Deliberate; recorded, not contested |
| Voice ordering | `public/voice-config.ts` (49/49-test Whisper stack, deleted) | DZ-10, deliberately deferred to Phase-9b (P38b absorbs it as deferred) | **COVERED as deferred** | — |
| QR kit (table/venue QR codes) | `QRKitPage` | None named; adjacent to P20 demo assets + P39 install entry | **MISSING** (small) | Post-MVP marketing collateral; proposed home: P20 asset unit |

---

## 6. Consolidated MISSING list (candidates for the consolidation pass — no numbers minted)

Ordered by severity. "Home" = proposal only; a later pass assigns numbers (P51 is in flight in a
parallel pass — do not collide).

| # | Missing item | Layer | Proposed home | MVP-blocking? (§7) |
|---|---|---|---|---|
| M1 | **Courier working surface** (cert login → offered claims → accept → PoD capture → delivered → earnings view) — protocol fully built/planned, screen owned by nobody | Courier | New DELIVERY-component phase, sibling of P48/P49; inherits P48's ⚠ rendering ruling; absorbs the courier-side earnings projection (D5 pattern, second reader) and the P47 cash-attestation input | **YES** |
| M2 | **Stock ledger has no phase number** — edge-cases §4 `stock.rs` is a complete, invariant-carrying design assigned only to "Wave-0 kernel work" | Owner | New phase implementing edge-cases §4 verbatim, OR a lettered sub-phase under P48; prerequisites unchanged (F1 fix, P13 id minting, P07 cancel edges, D1 `cause` field) | No (manual 86 via D1 `AvailabilitySet` covers transaction #1) — but index-orphan status should be fixed NOW so swarm work can claim it |
| M3 | **Menu media/photo storage** — no blob concept anywhere in the new stack | Owner | P48 scope extension; share the capture/blob path P13's PoD photo will need anyway (one storage decision, two consumers) | **Near** — not needed for the transaction mechanics; needed for first-client parity (the waiting client tested a product whose menus had photos) |
| M4 | **Courier availability / matcher candidate-set supply** — `assign()`'s `candidates` has no producer | Courier | Availability event family; lives with M1 or as a P34 follow-up (NOT inside P34 — its anti-scope forbids new event variants) | No — stopgap stated in §4 (all-certified-couriers + pull-based claims) is honest at first-client scale, but the stopgap RULE must be written into whichever phase wires intake |
| M5 | Allergen field + owner-confirmation act | Client/Owner | HUB-D1 field addendum + P48 surface item + P50 audit line (legal side) | No for transaction #1; **watch P50** — if the audit finds a live legal duty, this jumps |
| M6 | Minimum-order-value policy | Client | One line inside P13's fee authority (R2) | No |
| M7 | Order history / re-order (cross-order customer identity) | Client | P49 extension, decidable only after §11.2-3 identity ruling | No |
| M8 | Venue/dish ratings | Client | ⚠ operator ruling first (scoring-adjacency), then home | No |
| M9 | Owner/venue self-serve onboarding + claim | Owner | P48 extension; MVP substitute = operator-assisted runbook | No |
| M10 | Courier-invite handoff UX (P48 DoD-4 ↔ P23-P2 seam) | Cross | Named line item at that seam; no new phase | **Near** — the first courier must be enrollable somehow; a manual ceremony (operator runs both sides) suffices for courier #1 |
| M11 | In-order chat; menu import/translate; QR kit; analytics surface | Various | P43 ext / P48 backlog / P20 asset / G7 ext respectively | No — all post-MVP |

**Deliberately NOT proposed** (task ground rule — no features the product's own history and
roadmap don't establish): multi-order batching, loyalty/CRM, courier scoring in any form,
marketplace/aggregator features beyond D2's existing bridge design, scheduled/pre-orders,
tipping (never existed in the old stack; a money-red-line addition that would need its own
operator ruling — recorded here as an observed non-feature, not a gap).

## 7. MVP-blocking prioritization — "the first real transaction"

Operator context: a first client has tested the product and is WAITING for the updated version;
several more wait behind them. The bar for MVP = one real transaction completes: **order placed →
paid → prepared → delivered → confirmed**, for that client.

**The transaction's path through existing phases (already owned, cited, not this doc's work):**
P34 (wire the proven delivery-domain) → P37 (order over the wire + F12 local path) → P48 (a
managed menu + live queue + accept) → P47 Wave-0 (cash rail + courier attestation) → P49
(customer places/tracks anonymously) → P50 gate (the go/no-go milestone itself, whose DoD-3
already names P47/P48/P49 as prerequisites). Two gated items INSIDE existing phases deserve
early operator attention because a real client hits them within days, not months: **P07's
post-Confirmed cancel + failed-delivery edges** (today `InDelivery` has one exit — a failed
delivery is unrepresentable; edge-cases §3.3 calls this the sharpest structural hole, R3
operator-gated) and **edge-cases F1** (local retry double-commit — live in the only tested
commit path; a double-tapped checkout mints two orders).

**Of the genuinely MISSING items (§6), exactly three touch the first transaction:**

1. **M1 — courier working surface.** Blocks "delivered → confirmed": the courier cannot see,
   accept, or attest a delivery without SOME surface. Minimal acceptable MVP shape: a
   claim-list + accept + delivered/PoD + cash-attestation screen — four actions, one screen,
   the full 7-page old app is NOT the bar. This should enter the roadmap before P38b polish.
2. **M10 — courier-invite handoff (manual ceremony acceptable).** Blocks courier #1 existing at
   all; a documented operator-run enrollment ceremony discharges it for MVP, the UX item stays
   post-MVP.
3. **M3 — menu media.** Does not block the transaction mechanics; blocks first-client
   ACCEPTANCE parity (their tested product had menu photos). Decide the blob-storage question
   once, jointly with P13's PoD photo.

**Everything else in §6 is safely post-MVP** for the first transaction, with two standing
watches: M5 jumps if P50's audit finds a live allergen-disclosure duty, and the §5 notification
sequencing tension (P49 needs P43 DoD-2) resolves for MVP by the tracking page substituting for
push — but only for as long as the customer is willing to poll.

## 8. 2-question doubt audit (AGENTS.md ritual)

**Q1 — least confident about:** (1) Old-stack one-liners marked (n) are inferred from route
names, not read — any of them could have done more/less than stated; the enumeration's
completeness (69+62 files) is solid, individual descriptions are best-effort. (2) The M1
"no phase owns the courier surface" claim rests on reading P38b/P48/P49's scope text plus a
roadmap-wide grep for "courier" — a parallel pass writing P51+ could be closing it this very
session; the consolidation pass must re-check before minting. (3) Calling M4's
all-certified-couriers stopgap "honest at first-client scale" assumes 1–3 couriers who all
consent to seeing every offer — true for the waiting client per operator context, unverified
beyond it. (4) The claim that tipping "never existed in the old stack" is grep-of-filenames
level, not a content-level search of the deleted checkout code.

**Q2 — biggest thing possibly missed:** the old stack's `public/client-flow.ts` and
`owner/signals.ts` were not read and their names under-determine them; if client-flow encoded
per-venue ORDER-FLOW configuration (e.g. pickup-vs-delivery modes), then order-mode selection
(delivery vs pickup) is a feature this audit never tabled — no new-stack phase names
pickup-only orders either. Flagged as one follow-up read for whoever executes M1/P48.

---

*Research/audit only — no code, no canon, no phase numbers. Follow-ups proposed to owners:
M1/M2 phase-minting + M3/M4 homes → consolidation pass (after P51 lands); M5 → D1+P48+P50;
M6 → P13; M7/M8 → operator rulings (§11.2-3, new); M10 seam line → P48/P23 blueprints;
P07 cancel edges + F1 fix → already owned, urgency restated in §7.*
