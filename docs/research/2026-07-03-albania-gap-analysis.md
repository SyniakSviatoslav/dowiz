# dowiz / DeliveryOS — Albania Gap Analysis for Small Food Vendors

**Date:** 2026-07-03 · **Scope:** does dowiz fit the reality of small food vendors (byrek shops,
fast-food, pizzerias, small restaurants) in Tirana / Durrës?

**Method:** (A) codebase mapping via repowise MCP tools + direct grep/read verification of every
"current behavior" claim (file:line cited, and re-verified in this pass — several claims from an
earlier incomplete draft of this document were found **stale** and are corrected below); (B) Albania
market research via web search, with the highest-stakes claims (the fiscalization law) independently
re-verified by directly fetching the tax authority's own FAQ page and dedicated fiscal-compliance data
providers, not just secondary blog summaries. Every Albania claim carries a source; anything not
confirmed by an Albania-specific primary source is explicitly marked **UNVERIFIED — best estimate**
rather than stated as fact.

**One-line verdict:** dowiz nails the *strategic wedge* for Albania — cash-on-delivery-first,
zero-decimal-Lek-native money model, Albanian-default UI, landmark+GPS addressing, 0%-commission
own-brand storefront — and the one **BLOCKER**-class money bug found by the prior audit
(inclusive-tax double-charge) **has already been fixed** (verified in this pass, see §1.4). What
remains is (1) a genuine, un-worked-around **legal blocker**: dowiz issues no fiscal receipt, and
Albanian law requires one for every sale, cash included; and (2) a cluster of **half-built
revenue/usability features** — promo codes, modifier groups (sizes/extras), bulk menu edits,
category/product reorder, local card payment — that are cheaper to fix and materially raise the
product's value to a small vendor once fiscalization is addressed.

---

## 1. Current product map (BUILT / DARK / MISSING)

Status legend: **BUILT** = live by default · **DARK** = built but flag-off by default · **MISSING** =
not in code (or schema-only seam, no owner-facing UI).

### 1.1 What dowiz already does well for this market

| Capability | Status | Evidence | Why it fits Albania |
|---|---|---|---|
| **Cash-as-proof COD**, courier cash-ledger + change-making | BUILT | `apps/api/src/lib/payments/provider.ts:48-57` (`cashProvider`, no-op, stays `unpaid` until courier confirms); `apps/api/src/routes/orders.ts:518-521` (`CASH_AMOUNT_TOO_LOW` 422 guard); `apps/api/src/lib/deliveryCompletion.ts` (single `completeDelivery` primitive) | ~99% of AL payment *volume* and ~65-78% of value/COD-share is still cash; correct default, no card-chargeback exposure. |
| **Zero-decimal Lek money model**, integer/BigInt math, no float | BUILT | `packages/db/migrations/1780338982014_location_commerce.ts:6-7` (`currency_code DEFAULT 'ALL'`, `currency_minor_unit DEFAULT 0`); `packages/shared-types/src/utils.ts:13-16` (`CURRENCIES.ALL.decimals=0`); `apps/api/src/lib/money.ts` (integer tax math) | Qindarka (1/100 lek) lost legal-tender status in 1992 — Lek genuinely has no circulating subunit. Schema is correct for this, unlike a naive "assume cents" model. |
| **VAT (TVSH) rate + price-includes-tax per venue, default-inclusive** | BUILT | `location_commerce.ts:8-9` (`tax_rate NUMERIC DEFAULT 0`, `price_includes_tax BOOLEAN DEFAULT true`); checkout VAT line `apps/web/src/pages/client/checkout/OrderSummarySection.tsx:58-66`; Telegram receipt TVSH line `apps/api/src/notifications/locales.ts:73` | Albanian consumer law (9902/2008) mandates VAT-inclusive displayed prices; standard TVSH is 20% flat for restaurants (no reduced rate) — `price_includes_tax` defaulting `true` is the legally-correct AL setting. |
| **Albanian (`sq`) default UI**, 3-way sq/en/uk parity (~1,450 keys) | BUILT | `packages/ui/src/lib/i18n.ts:3,8` (`Locale='sq'|'en'|'uk'`, default `'sq'`); `packages/ui/src/lib/i18n-catalog.ts` | Standard (Tosk-based) Albanian is the correct national written/business register — no dialect (Gheg) handling is needed for a Tirana/Durrës product. |
| **Albania-shaped phone normalization** | BUILT | `apps/web/src/pages/client/checkout/phone.ts:1-13` (`normalizeAlbanianPhone` — accepts `069…`, `0 69…`, `00355…`, bare `69…`, coerces to `+355` E.164) | Matches how Albanians actually type numbers; backend validation stays generic E.164 (`packages/shared-types/src/utils.ts:1-2`). |
| **Free-text landmark + mandatory GPS-pin addressing** (no rigid street/number form) | BUILT | `apps/web/src/pages/client/checkout/DeliveryDetailsSection.tsx:63-66` (free-text + map pin) | Albania has no reliably-used postal-code/structured-address system in daily life; landmark+pin is the workable model. |
| **Menu OCR/photo import** (PDF/image → OCR + LLM, heuristic fallback) | BUILT | `apps/api/src/routes/owner/menu-import.ts` | Vendors have menus as printed cards/photos, not spreadsheets — this is the correct onboarding shape for the observed low-digital-maturity vendor segment (§2, R11). |
| **0%-commission own-brand storefront** (`/s/:slug`) | BUILT | `apps/web/src/pages/client/MenuPage.tsx`; palette/font theming | Direct counter-positioning to Wolt (~20-30%) and local incumbent Baboon; genuinely differentiated. |
| **Free-tile maps** (MapLibre + openfreemap) | BUILT | `apps/web/src/lib/tileConfig.ts` | No Google Maps billing dependency — fits a low-margin small-vendor SaaS. |

### 1.2 DARK (built, flag-off) — relevant to Albania

| Capability | Flag (default) | Evidence |
|---|---|---|
| Crypto payment (Plisio, USDT/USDC hosted invoice) | `PAYMENTS_PREPAID_ENABLED` / `PAYMENTS_CRYPTO_ENABLED` off | `apps/api/src/lib/payments/plisio.ts`; `apps/api/src/routes/orders.ts:646` |
| Menu characteristics (L2 compare/filter lenses) | `VITE_MENU_CHARACTERISTICS_*` off | `MenuPage.tsx` |
| Rich media gallery | `MEDIA_RICH_ENABLED=false` + requires `plan='business'` | `packages/config/src/index.ts` |
| Voice ordering (client-side, read-only) | `VOICE_CONTROL_ENABLED` off + no storefront UI wired | `packages/voice/src/capability-table.ts` |
| Telegram per-category prefs / storefront action | `TG_CATEGORY_GATING` / `TG_STOREFRONT_ACTION` off | `apps/api/src/routes/telegram-webhook.ts:20-27` |
| OTP phone verification | `OTP_ENABLED=false`, send step is a stub | `apps/api/src/routes/customer/otp.ts` |

### 1.3 MISSING (no code, or backend seam with zero owner-facing UI)

**Admin menu-authoring gaps** (given facts, re-confirmed with citations this pass):

| Capability | Evidence |
|---|---|
| **Category rename** in admin UI | `apps/web/src/pages/admin/MenuManagerPage.tsx` only has `handleAddCategory` (623-634) and `handleDeleteCategory` (636-646) — no rename handler/control. Backend seam exists but is orphaned: `apps/api/src/routes/owner/categories.ts:112-147` (`PATCH /api/owner/locations/:locationId/categories/:id`, accepts `name`) — but the route the admin UI actually calls (`/api/owner/menu/categories`, `categories.ts:189-259`) has only GET/POST/DELETE, no PATCH alias. |
| **Category / product drag-reorder** in admin UI | Zero `drag|sortable|reorder` hits in `MenuManagerPage.tsx` (the only drag-related code is the unrelated PDF-import file-drop, `:393,1254-1257`). Backend has `sort_order` on the non-aliased category route above; for products, `apps/api/src/routes/owner/products.ts:117-166` supports `sort_order` but the alias the UI calls (`products.ts:440-499`) uses a `.strip()`-mode schema (446-459) that silently drops `sort_order` even if sent. |
| **Bulk edit** (multi-select price/availability/category change) | No multi-select state anywhere in `MenuManagerPage.tsx`. `apps/api/src/routes/owner/menu-confirm.ts:9` states explicitly: "bulk is a client loop over this, each authenticated" — i.e. no batch endpoint exists anywhere in `apps/api/src/routes/owner/*`; true zero seam, not just missing UI. |
| **Modifier-group management UI** (sizes, extras, option groups) | Zero `modifier`/`ModifierGroup` references anywhere under `apps/web/src/pages/admin` or `apps/web/src/components/admin`. Backend is fully built: full CRUD in `apps/api/src/routes/owner/modifier-groups.ts`, product-linking in `products.ts:289-345`, schema across `1780338982010_menu_modifiers.ts` / `1780338982019_product_modifier_groups_loc.ts` / `1790000000060_modifier-display-type.ts`, and an authz test suite (`apps/api/tests/modifier-groups-authz.test.ts`). This is a backend-complete, UI-absent gap — the cheapest high-leverage fix on this whole list. |
| **Per-product translations UI** (manual sq/en/uk field edit) | No manual locale editor in `MenuManagerPage.tsx`. Schema exists: `packages/db/migrations/1780338982011_content_i18n.ts:5,13` (`product_translations`, `category_translations`, RLS-forced). The only backend action is `apps/api/src/routes/owner/menu-translate.ts` — an AI **bulk auto-translate-everything** trigger, rate-limited 1/min — not a per-product manual override for a vendor who wants to fix one wrong translation. |

**Other missing capabilities:**

| Capability | Evidence of absence |
|---|---|
| **Fiscal receipt / e-invoice (NIVF/NSLF/QR, DPT/CIS submission)** | Zero `fiscal`/`NIVF`/`NSLF` matches anywhere in the repo (verified by fresh grep this pass); "invoice" in code refers only to the Plisio crypto hosted-payment page. See §3 G1. |
| **Business tax-ID (NIPT) field** | No `nipt`/`tax_id` column anywhere. |
| **Card / local-gateway payment** (POK, Paysera, bank e-commerce) | `'card'` exists only as a schema enum value (`payments-ledger.ts`); no PSP adapter code — and Stripe itself is not available in Albania (§2 R5), so this needs a local-rail adapter, not the obvious default. |
| **Receipt printing** (thermal/Bluetooth kitchen ticket) | No print code found; only cosmetic `ti-receipt` icon classes on unrelated UI (`AnalyticsPage.tsx`, `OrderSummaryAccordion.tsx`). |
| **Promo/coupon redemption at checkout** | Full owner CRUD exists (`apps/api/src/routes/owner/promotions.ts`), but `orders.ts:514` hardcodes `discountTotal = 0` at checkout; the validate endpoint is owner-gated with zero customer-facing callers. |
| **SMS to customer** | OTP send step is a `console.log` stub (`customer/otp.ts`); no SMS gateway (Twilio/Vonage/local) wired anywhere. |
| **Order confirmation / receipt email** | Resend is wired only for internal ops/waitlist alerts (`apps/api/src/notifications/adapters/email.ts`); no customer- or owner-facing order email. |
| **Instagram as an order/contact channel** | Not one of the 6 checkout `MessengerKind`s (`apps/web/src/lib/messenger.ts:6-8`: `phone, whatsapp, viber, telegram, signal, simplex`); Instagram only appears as a static profile-URL footer link (`location_themes.social_instagram`). |
| **Pickup / scheduled ("order for later")** | Delivery is the only live order type; pickup/scheduled UI branches are dead code (`DeliveryDetailsSection.tsx:158-162`); no `scheduled_for` column. |
| **Loyalty / points** | `customers.loyalty_points` column exists, zero readers/writers. |
| **Public ratings on menu** | `order_ratings` captured post-delivery, never surfaced on the public storefront. |
| **Address autocomplete/geocoding** | Pin-drag + browser geolocation only. |
| **Plan/billing/subscription enforcement** (ADR-020) | No `plans`/`billing`/`subscription` tables; only `locations.plan CHECK IN ('free','business')` gating the dark media feature. Pricing is doc-only. |
| **Multi-location switcher for one owner** | Schema supports multiple locations; sessions pin to one, no admin switcher UI. |

### 1.4 Corrections to the prior (incomplete) pass of this document, and newly-found bugs

- **[RESOLVED, not a live blocker] Inclusive-tax double-charge.** An earlier version of this document
  flagged `price_includes_tax=true` (the Albania-correct schema default) as causing every taxed order to
  be overcharged (VAT extracted for display, then re-added to the charge). **Verified fixed in this
  pass**: `apps/api/src/routes/orders.ts:513` now reads
  `const chargedTax = location.price_includes_tax ? 0 : taxTotal;` — the inclusive branch contributes
  `0` to the charge, with an explicit inline reference to `ADR-audit-fix-money D1 / LC1`. This lands in
  commits `682efe35` / `561560eb`, already on this branch. Kept in the gap table (§3, marked
  **[RESOLVED]**) for audit-trail continuity, not as an active risk.
- **100× minor-unit inconsistency — still live.** Two surfaces divide Lek amounts by 100 despite
  `currency_minor_unit=0`: `apps/api/src/pages/admin` — `apps/web/src/pages/admin/PromotionsPage.tsx:406`
  (`` `${(p.discount_value / 100).toFixed(0)} ALL` ``) and `:426` (`min_order_amount / 100`), so a
  500-Lek discount renders as "5 ALL"; and `apps/api/src/lib/ssr-renderer.ts:99`
  (`price: (prod.price / 100).toFixed(2)`) inside the SEO structured-data (`schema.org`) generator,
  while the human-visible price render in the same file correctly uses a minor-unit-aware helper
  (`ssr-renderer.ts:178-181`) — so a search-engine snippet and the visible menu price can disagree.
- **Opening-hours / "closed venue" gate is client-side only.** `POST /orders` (`orders.ts:139`) checks
  only `location.published_at == null`; there is no server-side check against `busy_mode`/hours before
  accepting an order (`busy_mode` at `orders.ts:542` only doubles the confirm-timeout, it never blocks
  order creation). A crafted request can still place an order on a closed/paused venue.
- **Tenant `default_locale` is never read by the SPA.** `packages/ui/src/lib/i18n.ts:7-9` seeds
  `currentLocale` from `localStorage` only, defaulting hardcoded to `'sq'` — the `locations.default_locale`
  column (confirmed to exist and is typed through `MenuPage.tsx:100`) is never consulted, so every
  first-time visitor gets Albanian regardless of a venue's configured default (relevant for an
  English-first or tourist-facing coastal venue).
- **Analytics dashboard has a dead filter and fabricated trend figures.** `apps/api/src/routes/spa-proxy.ts:365`
  hardcodes `trend: '+15%'` regardless of actual data; the `?period=` query param is not wired to the
  underlying SQL window (still true on re-check this pass).
- **Crypto (dark) webhook path has a latent 100×-for-ALL trap, not yet a live bug.**
  `apps/api/src/routes/orders.ts:660`: `minorUnit: location.currency_minor_unit ?? 2` — falls back to
  **2** decimals if the column is ever null. Low risk today (the column is `NOT NULL DEFAULT 0`), but a
  trap waiting behind the `PAYMENTS_CRYPTO_ENABLED` flag; combined with no amount/currency reconciliation
  on the Plisio webhook (`payments-webhook.ts`), this should be closed before any crypto flag-flip.
- **i18n coverage has real, silent gaps — new finding this pass.** A structural scan of
  `packages/ui/src/lib/i18n-catalog.ts` found **31 keys with an `en` value but no `sq`/`uk`** (e.g.
  `start.found_items`, `checkout.min_order_error`, `admin.import_result`, and six `message.preset.*`
  canned courier/customer messages). Per the fallback logic in `i18n.ts:38` (`hit || fallback || key`),
  these silently render in **English** for Albanian and Ukrainian users today — for a product whose
  entire pitch rests on being Albanian-native, this is a real (if narrow) crack.

---

## 2. Albania market reality checklist

Confidence is stated per row; where the fresh research explicitly could not confirm a claim with an
Albania-specific primary source, it is marked so rather than asserted.

| # | Reality | Detail | Confidence |
|---|---|---|---|
| R1 | **Fiscalization is mandatory for every sale, no small-business exemption** | Law No. 87/2019 "On the Invoice and Turnover Monitoring System," phased in B2G→B2B→**B2C (1 Sept 2021)**, penalties enforced since 1 Jul 2022. Directly confirmed by fetching the tax authority's own FAQ (tatime.gov.al): for cash transactions "printimi i një fature është i detyrueshëm" — the invoice is mandatory — **except** that an app/online-shop order does not need a *paper printout*, but must still be issued and sent electronically (email/e-invoice). No turnover-based exemption exists; a simplified Self-Care/Central-Invoicing-Platform path exists for small taxpayers *without* certified software, but the duty to fiscalize itself is universal. | **HIGH** (directly fetched primary source) |
| R2 | **NIVF vs NSLF — corrected from an earlier draft** | **NIVF** = government-assigned code confirming real-time validation by the Central Invoicing System (CIS); **NSLF** = the seller's own code, used for the offline-fallback path (earlier version of this document had these reversed). Offline continuity: **48 hours** to restore connectivity and sync, **5 days** grace if the fiscal device itself fails (verbatim from tatime.gov.al FAQ). | HIGH |
| R3 | **Technical requirements** | Certified software + AKSHI-issued digital certificate/seal; invoices route through the government Central Invoicing System; format is UBL 2.1 / UN-CEFACT Cross-Industry Invoice (EU-standard schema) with Albania-specific NIVF/NSLF/QR fields. 5-year e-invoice archiving required. Certified local providers: easyPos/easyInvoice, devPOS, Elif, fature.al, Bills.al. **UNVERIFIED** whether a foreign SaaS can get direct AKSHI certification vs. needing to integrate through an already-certified local provider — several tatime.gov.al pages returned DNS errors during this research and this specific point wasn't confirmed from a primary source. | MED-HIGH |
| R4 | **Penalties, directly verified** | Under Law No. 83/2022 (effective 1 Jan 2023): second failure to issue an invoice within a year → **ALL 50,000** (natural person), **ALL 100,000** (VAT-registered natural person), **ALL 500,000** (corporate). Separate ALL 50,000-per-invoice fine for transmission/reporting failures. An unfiscalized invoice is not legally valid (buyer loses input-VAT deduction); repeat/serious cases risk a "tax risk" flag or suspension order. Enforcement is tightening: **Law No. 79/2025** (effective **Jan 2026**) adds automated VAT filing, new cash-payment caps, and mandatory POS terminals for coastal/hospitality businesses by May 2026. | HIGH |
| R5 | **Stripe is not available in Albania** | Card checkout needs a local rail: **POK** (BoA-licensed e-money institution, QR, Apple Pay, merchant app), **Paysera Albania** (EMI, e-store gateway), bank wallets (RaiPay, BKT SmartPay), easypay.al. | HIGH |
| R6 | **Cash still dominates by value; card is rising fast by count** | Bank of Albania data (first 9 months of 2025, via Hashtag.al): card payments = 51% of transaction **count** (just overtook ATM cash withdrawals) but cash withdrawals still = **65% of total value** vs 17% card POS. National POS terminal count 29,261 (+24.5% y/y). COD is ~77-78% of Albanian e-commerce transactions (Mordor Intelligence, 2025), consistent with a separate Western-Balkans retailer survey finding COD dominant at 72.5%. | HIGH |
| R7 | **Currency = lek (ALL), integer, VAT-inclusive display** | Qindarka (1/100 lek) lost legal-tender status 1 Jan 1992 — all prices are whole-number Lek in practice. Consumer Law 9902/2008 requires the displayed price to be the final, VAT-inclusive price. No documented rounding-to-10s/50s convention was found (**UNVERIFIED — best estimate** only; treat any rounding rule as a design choice, not a market norm). | HIGH (integer/inclusive); UNVERIFIED (rounding) |
| R8 | **VAT (TVSH) standard 20%, no reduced rate for ordinary restaurants** | 6% applies only to tourist accommodation/agritourism (explicitly excluding food except breakfast bundled into a room rate); 10% only to agricultural inputs. Ordinary restaurant/takeaway/delivery meals are taxed at the standard 20%. Most small food vendors sit below the **ALL 10M/year VAT-registration threshold** (PwC, reviewed Feb 2026) → often VAT-exempt in practice, but **still required to fiscalize** (R1) — these are separate obligations. 0%-profit-tax regime applies up to ALL 14M/year turnover through 2029. | HIGH |
| R9 | **Messaging-app habits — genuinely under-evidenced for Albania specifically** | An earlier draft of this document asserted "92% use WhatsApp/Viber," "Instagram is the top social-commerce channel," and "Telegram is weak" at HIGH confidence. Fresh research **could not independently confirm any of these three as Albania-specific facts** — no dataset breaks out WhatsApp/Viber/Telegram penetration for Albania at all. What *is* confirmed (DataReportal, Digital 2025 Albania): Facebook and Instagram each reach ~43% of the population (1.20M users), Messenger 31.6%. "Viber is dominant in Albania" is a plausible **regional-pattern inference** (Viber is confirmed dominant in Bulgaria, Greece, Serbia, Belarus, all near/bordering Albania) but Albania itself is never named in any Viber-penetration dataset found. One concrete but dated data point: SPAR Albania's 2020 home-delivery launch explicitly offered ordering via "WhatsApp, Viber and telephone," suggesting all three are treated as normal ordering channels by at least one large retailer. **Recommendation: treat channel prioritization as an open question and validate with a small vendor survey before over-investing engineering in any one channel's automation.** | **LOW-MED** (materially downgraded from the prior draft) |
| R10 | **Delivery competition is real but not from the giants everyone assumes** | **Wolt**: live since 13 Mar 2024 in Tirana (130+ venues), expanded to **Durrës in Mar 2025** (50+ venues) and Vlorë mid-2025; a dedicated AL/Kosovo GM was appointed Apr 2025 (continued investment). **Glovo**: began hiring for a Tirana launch Feb 2024 but **suspended the plan in Apr 2024**, never went live (confirmed: its Tirana URL is a dead marketing page). **Bolt Food**: not present in Albania at all (absent from Bolt's own city/support lists; only Croatia/Bulgaria/Greece/Cyprus in the Balkans). The real incumbent is local: **Baboon** (live since 2016, 1,000+ partner venues claimed, 80,000+ users, profitable since 2021, covers Tirana/Durrës/Korça/Vlorë) plus smaller local players **Hajde** ($1M seed, Tirana→Kosovo), **Snapfood**, **Foodini**, **ToGo Express**. Commission rates for the Albania market specifically are not publicly disclosed by any platform; the general 20-35% industry range is well-documented globally but no Albania-specific vendor-complaint evidence was found (plausibly because Wolt is too new locally for organized backlash to have surfaced in indexed sources). | HIGH (platform presence); MED (commission specifics) |
| R11 | **Vendor tech baseline is shallow and social-media-centric** | Adjacent-sector evidence (UNDP 2025 tourism-sector survey): social media is "the dominant digital marketing approach" for small AL businesses, only 22% use any formal ERP/ordering system, ~50% have no digital-transformation strategy. Regionally (OECD SME Policy Index for Western Balkans 2026): ~33% of Western Balkans SMEs have no e-commerce engagement at all; where they do sell online, e-commerce turnover share (5.3%) is far below the EU average (12%) — "selling online" often means a page, not real transaction volume. Only 23.3% of Albania's population meets the EU basic-digital-skills threshold (RCC, WB DESI 2024). Customer-side connectivity is not the bottleneck: 88.4% internet penetration (DataReportal, Digital 2026), ~93% mobile penetration, 99.6% 4G coverage — the gap is vendor-side enablement, not customer demand. | MED (vendor-side, adjacent-sector); HIGH (customer connectivity) |
| R12 | **Language: standard Tosk-based Albanian, Gheg/Tosk split is a non-issue for this scope** | Codified 1972 Congress of Orthography on the southern Tosk dialect, used uniformly in all formal/business writing nationwide (even in Gheg-speaking Kosovo). Tirana/Durrës sit within the Tosk zone. No evidence any Albania-market digital product has ever built a Gheg UI variant. This would only become relevant on a future Kosovo expansion, not for the current Tirana/Durrës scope. | HIGH |
| R13 | **Tipping is light and cash-preferred** | Not mandatory; round-up or small cash tips are the norm even when the bill is paid by card. A mandatory tip prompt would be a mismatch for this market. | HIGH |
| R14 | **Phone numbering** | `+355`; mobile prefixes 067/068 (One Albania), 069 (Vodafone Albania); number portability means prefix no longer reliably identifies the carrier. | HIGH |

---

## 3. GAP TABLE

Columns: **Gap** · **Sev** (BLOCKER / HIGH / MED / LOW) · **Business value** · **AL-specific?** ·
**Current state (file:line)** · **Recommended fix + effort** (S ≤2d · M ≤2wk · L >2wk).

| # | Gap | Sev | Business value | AL-specific? | Current state (file:line) | Recommended fix + effort |
|---|---|---|---|---|---|---|
| G1 | **No fiscal receipt / e-invoice (NIVF/NSLF/QR, CIS submission)** | **BLOCKER** | Legally required for every sale; non-compliance risks per-invoice fines starting ALL 50,000, escalating on repeat, plus buyer-side input-VAT invalidity. This is the gate between "demo" and "sellable." | **Very high** — Law 87/2019 is Albania-specific, no exemption | Zero fiscal code repo-wide (verified, §1.3) | **Phase 0 (S):** confirm with an Albanian accountant/lawyer whether a relay-only SaaS (vendor's own certified POS fiscalizes) is sufficient, vs. dowiz needing its own certification if it ever collects payment. **Phase 1 (M):** integrate a certified provider's API (devPOS/easyPos/Elif) — on order completion, call the provider, store `nivf`/`nslf`/`qr_url` on the order, surface the e-invoice on the tracking page + as an email/Telegram attachment. **Phase 2 (L):** pursue direct AKSHI certification if partnership terms are unfavorable. |
| G2 | **[RESOLVED] Inclusive-tax double-charge** | was BLOCKER | Was overcharging every taxed order under the AL-correct `price_includes_tax=true` default. | Medium (generic bug, AL-default-triggered) | **Fixed**: `orders.ts:513` (`chargedTax = price_includes_tax ? 0 : taxTotal`), commits `682efe35`/`561560eb` | No action needed; verify no regression in the next money-path test run. Kept here for audit-trail continuity only. |
| G3 | **Promo/coupon codes can't be redeemed at checkout** | HIGH | Discounts/first-order campaigns are the top conversion lever vendors expect from any ordering platform; half-built wastes existing owner-side investment. | Low (universal need) | Owner CRUD built (`owner/promotions.ts`); `orders.ts:514` hardcodes `discountTotal=0`; validate endpoint has zero customer callers | Add a customer promo-code field at checkout → public validate endpoint → server-authoritative `discountTotal`. Fix the 100× unit bug on `PromotionsPage:406,426` in the same change. **M.** |
| G4 | **No local card gateway (POK / Paysera / bank e-commerce)** | HIGH | Card is now 51% of transaction *count* and rising; Stripe cannot fill this — a local PSP is the only viable path. | **High** — must specifically be a *local* rail | `'card'` is schema-only; cash (live) + crypto (dark) are the only payment adapters | Build a POK or Paysera adapter behind the existing `PaymentProvider` seam (`lib/payments/provider.ts`); keep COD the default. **M-L.** Sequence after G1 (payment-collection may affect the fiscalization regime, R1/R3). |
| G5 | **Modifier-group management UI missing (sizes, extras, add-ons)** | HIGH | Table-stakes for pizzerias/fast-food (size, extra toppings, drink size) — a very common Tirana vendor category. Backend is fully built; this is pure UI debt. | Low (universal, but the underlying vendor mix in Tirana/Durrës is pizza/fast-food-heavy, raising practical value here) | Full CRUD API + schema exist (`owner/modifier-groups.ts`, `products.ts:289-345`); zero admin UI | Build the owner-facing screen on top of the existing API — no new backend work needed. Highest value-per-effort item on this list. **S-M.** |
| G6 | **Bulk edit missing (menu-wide price/availability changes)** | HIGH | Real operational pain for a vendor updating e.g. all coffee prices at once; `menu-confirm.ts:9` documents there is no batch endpoint even server-side. `MenuManagerPage.tsx` is a top-5 90-day churn hotspot, consistent with ongoing pain in this exact screen. | Low (universal) | No batch endpoint anywhere under `owner/*`; explicit "bulk is a client loop" comment | Add one batch-mutation endpoint (array of `{id, patch}`, per-row authz) + multi-select UI. **M.** |
| G7 | **Owner order-notifications are Telegram-only, and the Albania channel-preference assumption is itself unverified** | MED-HIGH | Owners must know a new order arrived; if the real-world channel preference (WhatsApp/Viber vs. Telegram) doesn't match what's built, owners risk missing orders. | **High** — but per §2 R9, the underlying "which channel Albanians actually prefer" claim is not established by any primary source found | Telegram bot fully built (`bootstrap/notifications.ts`); WhatsApp notification channel was *deliberately removed* for privacy/ToS reasons (`1790000000043_remove-whatsapp-channel.ts`); no SMS/email fallback | **S (research first):** run a 10-15 vendor phone survey on actual order-notification channel preference before building. **Then M:** wire whichever channel wins (Viber has an official Business/bot API) as a second owner-alert channel, or add SMS as a channel-agnostic fallback. |
| G8 | **Category rename + category/product reorder missing in admin UI** | MED | Renaming a category or reordering menu sections/items is a routine day-2 request; currently requires a support ticket / direct DB edit. `MenuManagerPage.tsx` churn (26 commits/90d) is consistent with unresolved friction here. | Low (universal) | Backend PATCH exists for categories (`owner/categories.ts:112-147`) but on a route the UI doesn't call; product `sort_order` is silently stripped by the UI's own PATCH schema (`products.ts:446-459`) | Add the missing PATCH alias for category rename/reorder; stop stripping `sort_order` on the product PATCH; add drag-handles in the UI. **S-M.** |
| G9 | **No Instagram order/contact channel** | MED | Instagram is confirmed to be a top-reach social platform in Albania (~43% of population, DataReportal) even if "top *social-commerce* channel for food orders" specifically isn't independently confirmed (§2 R9). Missing it forecloses a plausible acquisition path regardless. | Medium-High (needs validation, see G7) | 6 checkout kinds = phone/whatsapp/viber/telegram/signal/simplex (`messenger.ts:6-8`); no Instagram | Add Instagram as a checkout deep-link kind (`ig.me`/profile); pair with a "link in bio" storefront CTA. **S-M.** |
| G10 | **100× minor-unit inconsistency (SSR structured data + PromotionsPage)** | MED | Visibly wrong prices on two surfaces (a 500-Lek discount shows as "5 ALL"; SEO snippet price can disagree with the visible menu price). | Medium (generic bug, but ALL's zero-decimal-ness is exactly what exposes it) | `ssr-renderer.ts:99`; `PromotionsPage.tsx:406,426` | Standardize on the existing minor-unit-aware helper (`ssr-renderer.ts:178-181` pattern) everywhere; delete the two hardcoded `/100`s. **S.** |
| G11 | **Opening-hours / closed-venue gate is client-only** | MED | A closed/paused venue can still receive an order it can't fulfill → refund, angry customer, wasted courier trip. | Low (universal) | `orders.ts:139` checks only `published_at`; `busy_mode` (`:542`) only affects confirm-timeout, never blocks creation | Add a server-side `VENUE_CLOSED` 422 check at order creation. **S.** |
| G12 | **`formatMoney` renders "800 ALL" not "800 L / Lekë"** | MED | Reads as foreign/technical to an Albanian user; undercuts the "feels native" selling point. | High — cosmetic but AL-idiom | `packages/shared-types/src/utils.ts` returns the ISO code though the `L` symbol is defined but unused in the format path | Render the local symbol/word in the UI, keep ISO code only in data/logs. **S — quick win.** |
| G13 | **No SMS to customer (OTP or order-status updates)** | MED | Web-push requires the customer to keep the PWA around; SMS is the universal fallback, especially given the low e-commerce/digital-skills baseline (§2 R11). | Medium | `OTP_ENABLED` off; send step is a `console.log` stub | Wire a local SMS gateway (or a reachable international one) behind OTP + order-confirm hooks. **M.** Gate on per-SMS cost given thin vendor margins. |
| G14 | **Tenant `default_locale` ignored — every first-time visitor gets Albanian** | MED | Wrong first impression for a tourist-facing or English-first venue (e.g. a Durrës/Vlorë coastal spot). | Medium | `i18n.ts:7-9` hardcodes fallback `'sq'`, never reads `locations.default_locale` | Seed the SPA's initial locale from the tenant's `default_locale` before falling back to `localStorage`/`'sq'`. **S.** |
| G15 | **No customer/owner order-confirmation email** | MED | Expected confirmation channel; also the natural electronic-delivery path for an app-paid order's fiscal e-invoice once G1 ships. | Medium | Resend wired only for internal ops/waitlist (`notifications/adapters/email.ts`) | Add order-confirmation email (customer) + daily owner digest; reuse the existing Resend integration. **S-M.** Pairs directly with G1. |
| G16 | **No pickup / "order for later" scheduling** | MED | Pickup is common for byrek/coffee-style vendors; scheduling smooths kitchen load at peak times. This was actively removed, not just never built. | Low (universal) | Pickup/scheduled UI branches are dead code (`DeliveryDetailsSection.tsx:158-162`); no `scheduled_for` column | Re-enable pickup (skip address/delivery-fee flow); add `scheduled_for` for later-orders. **M.** |
| G17 | **Per-product translations UI missing (manual sq/en/uk edit)** | MED | A vendor who wants to fix one wrong English/Ukrainian product name currently has no way to do it without triggering a full AI re-translate of the whole menu. | Low (generic i18n UX gap; not Albania-unique) | `product_translations`/`category_translations` tables exist; only action is a bulk AI auto-translate trigger (`owner/menu-translate.ts`) | Add a simple per-locale text-field editor on the product form, independent of the bulk AI trigger. **S.** |
| G18 | **31 i18n catalog keys silently fall back to English for sq/uk users** | MED (small blast radius, high symbolic cost) | Directly undercuts the "Albanian-native" pitch on the specific strings affected (e.g. `checkout.min_order_error`, six `message.preset.*` canned messages a courier/customer would actually see). | High — directly about Albanian-language completeness | `i18n-catalog.ts` (31 `en`-only keys identified this pass); silent fallback in `i18n.ts:38` | Backfill the 31 keys; confirm the CI parity gate (`scripts/i18n-parity.ts`) actually fails the build on future gaps rather than just warning in dev. **S.** |
| G19 | **Analytics dashboard: dead period filter + fabricated trend %** | MED | Owner-facing dashboard shows invented trends ("+15%" literal) → distrust and bad decisions if noticed. | Low (universal) | `spa-proxy.ts:365` hardcodes `trend: '+15%'`; `?period=` not wired to the SQL window | Compute real deltas from the query window or remove the delta chips entirely. **S.** |
| G20 | **Crypto (dark) webhook: no amount/currency reconciliation + `minorUnit` null-fallback of 2** | MED (dark, pre-launch) | If the crypto flag is ever flipped on without this fix, a null `currency_minor_unit` would silently misprice an ALL-denominated ledger entry by 100×. | High — the 100× trap exists specifically because ALL is zero-decimal | `orders.ts:660` (`?? 2` fallback); no amount/currency check in `payments-webhook.ts` | Gate `paid` status on `amount >= charged && currency match`; make the `?? 2` fallback fail loudly instead of silently defaulting. **S.** Must land before any `PAYMENTS_CRYPTO_ENABLED` flip. |
| G21 | **No thermal/Bluetooth receipt/kitchen-ticket printing** | LOW-MED | Vendors want a physical kitchen ticket; also the natural print path for a fiscal receipt once G1 exists. | Medium | No print code found | Web-print or ESC/POS Bluetooth ticket for the kitchen; fiscal-document printing rides on G1's provider integration. **M.** |
| G22 | **No business tax-ID (NIPT) field** | LOW alone / prerequisite for G1 | Needed to actually issue a fiscal or B2B invoice. | High | No `nipt` column anywhere | Add to tenant/owner settings; feed directly into G1. **S.** |
| G23 | **No loyalty/points despite the column existing** | LOW | Retention lever some aggregators use; not urgent. | Low | `customers.loyalty_points` unread/unwritten | Defer until after G3 (promos) ships. **M.** |
| G24 | **Plan/billing/quota enforcement is doc-only (ADR-020)** | LOW (pre-monetization) | No revenue-capture mechanism yet; not an MVP blocker for a pilot vendor. | Low | No `plans`/`billing`/`subscription` tables; only a `plan` enum gating one dark feature | Build order-counter + soft-threshold when actually monetizing. **L.** |
| G25 | **Tipping is cash-only and un-nudged** | N/A — correct as-is | AL tipping is light and cash-preferred (§2 R13); the current model already fits. | Fits, don't change | `orders.tip_amount`, cash-only | Leave as-is; do **not** add a mandatory tip prompt — that would be a market mismatch. |

*Additional lower-priority MISSING items not fully tabled above (see §1.3): address autocomplete/geocoding,
public display of post-delivery ratings, multi-location switcher for one owner. All three are low
urgency for a first Albania pilot.*

---

## 4. Top 10 highest-value gaps (ranked)

1. **G1 — No fiscal receipt / e-invoice (fiskalizimi).** BLOCKER, most Albania-specific item on the
   list. Every sale legally needs a fiscalized invoice; dowiz issues none. → integrate a certified
   provider (devPOS/easyPos/Elif) to generate NIVF/QR per order; confirm the intermediary-regime
   question with a local accountant first.
2. **G4 — No local card gateway (POK/Paysera).** HIGH. Card is now the majority payment method by
   transaction count; Stripe is unavailable in Albania, so a local-rail adapter is the only path.
3. **G5 — Modifier-group management UI missing.** HIGH, cheapest high-leverage fix on the list: backend
   is fully built, only the owner-facing screen is missing, and pizza/fast-food size+extras pricing is
   table-stakes for the Tirana/Durrës vendor mix.
4. **G6 — Bulk edit missing.** HIGH. Real, evidenced (hotspot-churn) operational pain for any vendor
   maintaining a non-trivial menu; no batch endpoint exists at all today.
5. **G3 — Promo/coupon codes unredeemable at checkout.** HIGH. Full owner-side CRUD exists but
   `discountTotal` is hardcoded to 0 — the single most expected conversion lever is half-built.
6. **G7 — Owner notifications are Telegram-only, and the underlying Albania-channel-preference
   assumption is itself unverified.** MED-HIGH. Risk of missed orders *and* a strategic assumption that
   should be validated (10-15 vendor calls) before more channel automation is built on top of it.
7. **G8 — Category rename + category/product reorder missing in admin UI.** MED. Routine day-2
   friction; backend seams already exist for most of it, making this a cheap fix relative to its
   recurring annoyance.
8. **G9 — No Instagram order/contact channel.** MED. Instagram has strong, confirmed reach in Albania;
   missing it forecloses a plausible acquisition path even though "top channel for food orders
   specifically" isn't independently confirmed.
9. **G10 — 100× minor-unit inconsistency (SSR + PromotionsPage).** MED. Live, visible money-display
   bug on a market where "does the price shown match what's real" is a trust-critical detail; cheap to
   fix.
10. **G11 — Opening-hours/closed-venue gate is client-side only.** MED. Cheap fix for a real
    operational-safety gap (unfulfillable orders on a closed venue).

*(Runners-up: G18 the 31 silently-English i18n keys — cheap and symbolically important for the
"Albanian-native" pitch; G12 the "800 ALL" vs "800 L" money-symbol fix — a one-line quick win; G20 the
dark crypto webhook reconciliation gap — must close before any flag-flip, not urgent today.)*

---

## 5. The single most important Albania-specific thing dowiz is missing or getting wrong

**Fiscalization ("fiskalizimi," Law No. 87/2019) — and it is a blocker, not a nice-to-have.**
Every sale by an Albanian food vendor — cash included, delivery included, down to a 100-Lek byrek —
must be reported in real time to the tax authority and produce a legally valid invoice carrying an
NIVF verification code (confirmed directly by fetching the tax authority's own FAQ page: for
app/online orders the paper printout is optional, but issuing and electronically delivering the
invoice is not). There is no turnover threshold or micro-vendor carve-out — the obligation is
universal — and penalties escalate on repeat non-compliance (from ALL 50,000 upward under Law
83/2022), with enforcement tightening further under Law 79/2025 effective January 2026.

dowiz currently computes VAT correctly (`tax_rate`, `price_includes_tax`, a TVSH line in the Telegram
receipt) but issues **zero** fiscal documents — there is no NIVF/NSLF/QR concept anywhere in the
codebase. This cuts both ways: it is the **hardest current blocker** (a vendor cannot run fully
compliant sales through a tool that never fiscalizes a single order), and it is simultaneously the
**biggest available moat** — Wolt and Baboon both leave fiscalization entirely to the restaurant's own
POS/register; a dowiz that integrates a certified provider (devPOS/easyPos/Elif all expose APIs) and
issues the NIVF/QR *inside its own order flow* would be doing something neither major competitor
does today. Everything else in this analysis is polish or revenue upside on top of a product that
already fits Albania's cash-first, Lek-native, Albanian-language reality unusually well; fiscalization
is the one thing standing between "an excellent demo" and "a product a small Albanian food vendor can
legally run their whole business on."

---

## Appendix — key research sources

**Fiscalization (R1-R4, primary-verified this pass):** tatime.gov.al fiscalization FAQ (direct fetch,
verbatim quotes) and key-points page; fiscal-requirements.com (Albania country page + penalty article
citing Law 83/2022 + Law 79/2025 briefing); vatupdate.com (Dec 2025 e-invoicing briefing); ClearTax
and EDICOM Albania e-invoicing guides; dddinvoices.com (fiscalization + e-invoicing explainers).

**Payments/VAT/currency (R5-R8):** Bank of Albania data via Hashtag.al (Nov 2025 card-vs-cash);
Mordor Intelligence (Albania e-commerce COD share); NORBr (Western Balkans payment methods); PwC Tax
Summaries (reviewed Feb 2026, VAT threshold/rate); International Tax Review (6% reduced-rate scope);
WIPO Lex Law 9902/2008 (VAT-inclusive pricing); Wikipedia (Albanian lek, qindarka); AlbaniaVisit
(currency guide); kryeministria.al (0%-profit-tax regime).

**Messaging/social (R9):** DataReportal Digital 2025/2026 Albania; Sinch and Techjury (regional Viber
penetration, Albania not directly named); SPAR International 2020 press release (WhatsApp/Viber/phone
ordering, dated).

**Delivery platforms (R10):** Wolt Newsroom, Top Channel, SeeNews, Albania Tech, TradingView/Reuters
(Wolt entry/expansion/GM appointment); SeeNews (Glovo suspension); Bolt's own cities/support pages
(absence confirmed by direct check); Albania Tech (Baboon); The Recursive (Hajde); Snapfood/Tracxn/ToGo
Express company pages.

**Vendor tech level (R11):** UNDP (Nov 2025 tourism-sector digitalization survey); OECD SME Policy
Index for Western Balkans and Türkiye 2026; Regional Cooperation Council WB DESI 2024; DataReportal;
INSTAT ICT Usage Survey 2024; World Bank Albania E-Commerce Diagnostic.

**Language/tipping/phone (R12-R14):** Wikipedia (Albanian Orthography Congress, Gheg Albanian);
Ethnologue; Talkpal; OnMeTravel (tipping norms).

*Note: several `tatime.gov.al` pages returned DNS errors from the research sandbox during this pass;
every load-bearing fiscalization fact was either fetched directly from a working tatime.gov.al page or
cross-confirmed by ≥2 independent fiscal-compliance sources. The R3 "can a foreign SaaS get direct
AKSHI certification" question remains open and should be confirmed with local counsel before committing
to a specific integration architecture for G1.*
