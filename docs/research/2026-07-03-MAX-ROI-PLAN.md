# dowiz / DeliveryOS — MAX-ROI-FAST Roadmap

**Date:** 2026-07-03 · **Type:** product + growth strategy (synthesis, no code) · **Horizon:** next 90 days
**Evidence base (do not re-research — this synthesizes across them):**
- **R1** `docs/research/2026-07-03-vendor-pain-established-apps.md` — proven vendor pain; 0%-commission, own-the-customer, cash-first are the wedges; hybrid GTM is the honest motion.
- **R2** `docs/research/2026-07-03-trust-and-likeability.md` — cash-first = #1 trust asset; claimable-demo = strongest move; Albanian-first copy, Viber support, risk-reversal, ≤15-min onboarding, milestone celebration.
- **R3** `docs/research/2026-07-03-social-media-automation-pipeline.md` — n8n + Claude Batch + Playwright stat-cards + Telegram approval + Postiz + Shlink; the "Sunday workflow" turns the DB into content.
- **R4** `docs/research/2026-07-03-albania-gap-analysis.md` — fiskalizimi (Law 87/2019) is the #1 blocker AND moat; promo codes unredeemable; no local card gateway; no Instagram channel; owner alerts Telegram-only; "Lekë" formatting; closed-venue orders accepted; `default_locale` ignored — each with `file:line`.
- **AUDIT** `docs/design-review/AUDIT-SYNTHESIS-2026-07-03.md` — what's already being fixed this session.

> **Already committed/in-flight this session — DO NOT duplicate:** LC1 inclusive-tax double-charge (FIXED — verified `apps/api/src/routes/orders.ts:513` `chargedTax = price_includes_tax ? 0 : taxTotal`); cross-tenant IDOR class (LC2), LC3 customer-cancel, LC6 refund spine, and the FE fake-data / silent-toast / focus-trap classes are committed; undo-redo + pull-to-refresh + keyboard-shortcuts UI are in flight. This plan builds *around* that remediation, not on top of it.

---

## 1. Executive summary (one screen)

**The thesis — where the ROI actually is.** dowiz's strategic wedge is already correct and validated by R1/R2/R4: 0%-commission, own-the-customer, cash-first, Albanian-native. The product does *not* need a new strategy. ROI is unlocked by moving three distinct dials, in this order of leverage:

1. **Sellability (the gate).** In Albania a food vendor legally cannot run compliant sales through a tool that issues no fiscal receipt (fiskalizimi, Law 87/2019 — R4 §5, G1). This is simultaneously the **only hard blocker between "demo" and "sellable"** and the **biggest moat** — Wolt/Baboon make the restaurant fiscalize on its own POS; a dowiz that issues the NIVF/QR *inside the order flow* does something the aggregators don't. Interim relief: most legal AL vendors already own a NAIS certificate + certified POS (R4 R11), so dowiz can sell **relay-only** today (vendor fiscalizes on their existing POS) while the integration is built — this de-risks the timeline.
2. **Trust & native-feel (the yes).** The cheapest, fastest ROI in the whole plan. Cash-first positioning is the #1 trust asset (R2 T2, "paratë mbeten cash"), Albanian-first copy is worth 76% preference / 40% never-buy-otherwise (R2 L3), and the product currently undercuts its own native-feel selling point by rendering `800 ALL` instead of `800 L / Lekë` (R4 G7, verified `packages/shared-types/src/utils.ts:26`). Copy + a one-line format fix + a lekë savings counter is a weekend of work that moves the #1 pain and the #1 fear at once.
3. **Acquisition (the growth engine).** dowiz is the **owned channel, not a demand engine** (R1 honest caveat, line 25). Acquisition therefore cannot come from the product alone — it comes from (a) the **claimable-demo motion** (build the vendor's storefront before first contact; "your menu is already online, claim it free" — R2 T1, collapses time-to-value to zero *and is the pitch*), (b) the **Instagram channel** (top social-commerce habit in AL — R4 G5/R9), and (c) the **~$30/mo Sunday social pipeline** that turns the product DB into content (R3).

**The single sequence I would run:**
> **Week 1** — ship the quick-win "native trust bundle" (lekë format, `default_locale`, closed-venue server gate, milestone seed, Instagram channel, cash-first/0%/Albanian positioning + "Pa rrezik" risk-reversal block). **Weeks 2–6** — two parallel lanes: **(lane 1, red-line council)** fiskalizimi certified-provider integration + promo-redemption wiring + (gated) local card gateway; **(lane 2, safe)** productize the claimable demo, onboarding-to-first-order choreography, Viber owner alerts, and stand up the Sunday social pipeline. **Weeks 6–12 (strategic)** — hybrid GTM (marketplace as paid acquisition → convert regulars to the owned channel), 3 named case studies, scaled social + TikTok/Meta audits.

Everything red-line (money / auth / RLS / contract / compliance) goes through a **Triadic Council before any code** — flagged per-row below.

---

## 2. Ranked initiative table

Ranked by **(impact × speed × certainty × Albania-fit)**. Effort: **S** ≤2d · **M** ≤2wk · **L** >2wk. Red-line = money/auth/RLS/contract/compliance → **COUNCIL before code**.

| # | Initiative | Pain / gap (cite) | Expected ROI | Effort | Certainty | Red-line? | Dependency |
|---|-----------|-------------------|--------------|--------|-----------|-----------|------------|
| **1** | **Native trust bundle** — cash-first + "0% komision" positioning, Albanian-first copy pass, lekë formatting (`800 L`), "Pa rrezik" risk-reversal block + one-tap data export | R2 T2/T3/L3, R4 G7 (`utils.ts:26`, symbol `L` defined `:14` unused); R2 T6 (export exists in owner/gdpr) | Trust/conversion — hits #1 pain (commission) + #1 fear (cash/tax visibility) at once; native-feel is the whole selling point | S–M | High | No (copy + 1-line format + wire existing export) | GDPR export routes already exist |
| **2** | **Claimable-demo productization** — pre-built `/s/:slug` + "Ky është dyqani juaj? Merreni falas" CTA + ≤3-field phone-based claim | R2 T1/L2; R1 demand-gen caveat (l.25); R4 G5 (link-in-bio) | Acquisition — collapses time-to-value to zero *and is the pitch*; self-feeding referral (beautiful link shared vendor-to-vendor) | S–M | High | No | demo-builder loop + Maps scrape exist |
| **3** | **Fiskalizimi certified-provider integration** — order → provider (devPOS/easyPos/Elif) issues NIVF/NSLF/QR → store on order → show on tracking + receipt; add `nipt` to tenant | R4 §5 + G1 (zero fiscal code repo-wide), R4 R1–R4 | Compliance-unlock — the gate between demo and sellable; **the moat** (aggregators don't do in-flow fiscalization) | M | Med (integration effort provider-dependent) | **YES — compliance + money + contract** | Phase-0: AL accountant confirms intermediary regime (R4 R2) |
| **4** | **Instagram order channel + link-in-bio** — add IG as a checkout contact kind (`ig.me`/profile deep-link) + storefront IG-story share of `/s/:slug` | R4 G5 (`messenger.ts:6-8` — 6 kinds, no IG), R4 R9 (IG = top AL social-commerce) | Acquisition — routes the vendor's existing IG audience into dowiz | S–M | High | No | messenger deep-link pattern exists |
| **5** | **Viber/WhatsApp owner-alert channel** — add a Viber (official bot API, strong AL presence) or WhatsApp-Business owner-notification adapter alongside Telegram | R4 G6 (Telegram-only, weak in AL — channel/market mismatch; `event-registry.ts`) | Retention/reliability — owners *miss new orders* on Telegram-only → the one failure that permanently kills the relationship (R2 L5) | M | Med-High | No (new outbound adapter; not money/authz) | pg-boss retry/DLQ (exists); event registry |
| **6** | **Onboarding-to-first-order choreography** — photo/PDF menu → live storefront → vendor places a test order from their *own* phone (the aha) → branded printable QR | R2 L1 (first-value <15 min = strongest retention lever); R4 (PDF import `menu-import.ts` exists) | Retention — 80% vs 35–50% month-12 retention on fast time-to-value | M | High | No | PDF import, demo-builder, QR all exist — needs choreography |
| **7** | **Sunday social pipeline (phase 1)** — n8n + Claude Batch → stat-cards (Playwright) → Telegram approve → Postiz hosted → Shlink; IG/TikTok/FB/Telegram | R3 §4 phase 1 (~$30/mo, ~2-day build); R1 demand-gen caveat | Visibility/acquisition — DB *is* the content source; ~$30/mo turns orders into content | M | High (mechanics/cost verified) | No (marketing infra) — **PII caveat: per-vendor claims need consent** | Telegram bot + R2 photos exist |
| **8** | **Lekë savings counter** — landing calculator ("100 orders × 1,500 L × 25% = 37,500 L/muaj → dowiz 0") + in-admin "you kept X lekë this month" tile | R2 T3 (ChowNow "$470M saved" pattern); R1 Pain 1 | Conversion/retention — concrete money in the vendor's own currency beats abstract % | M | High | No (reads existing order totals; aggregate-safe) | order totals exist; named claims need consent |
| **9** | **Promo/coupon redemption at checkout** — customer promo field → public validate endpoint → apply `discountTotal` server-side (mirror the fee ladder); fix 100× unit bug on `PromotionsPage:406,426` | R4 G3 (owner CRUD exists, `orders.ts:514 discountTotal=0` — verified live; validate endpoint owner-gated, zero customer callers) | Revenue/conversion — the #1 conversion lever vendors expect; half-built surface wasted | M | High | **YES — money (touches order pricing)** | Fee-ladder server-authoritative pattern |
| **10** | **Milestone celebration** — first-order + 100-orders in-admin moment + message on their channel + shareable branded card | R2 L4 (+28% retention / +42% LTV analogues; self-feeding proof loop → T5) | Retention + acquisition (shared card = proof to next vendor) | M | Med-High | No | order-count events per tenant |
| **11** | **`default_locale` honored** — seed SPA locale from tenant `default_locale` before localStorage | R4 G10 (`i18n.ts:5-9` hardcodes `'sq'`; never read) | Conversion — right first impression for tourist/coast venues (Durrës/Vlora) | S | High | No | none |
| **12** | **Closed-venue server gate** — server-side opening-hours check in order creation (422 `VENUE_CLOSED`) | R4 G9 (`orders.ts:139-143` checks only `published_at`; client-only gate) | Retention/cost — stops unfulfillable orders → refunds, angry customers, wasted couriers | S | High | Light — touches order-creation contract (safe-direct + invariant check) | none |
| **13** | **`/status` page + "no lost orders" promise + review-reply discipline** | R2 T7/T5 (honest health check exists; BrightLocal: 88% vs 47% reply effect) | Trust — transparency competitors don't hold; local apps visibly flaky (Baboon) | M | Med | No | honest health check (audit wave-1) exists |
| **14** | **Local card gateway (POK / Paysera)** — adapter behind existing `PaymentProvider` seam; COD stays default, card opt-in | R4 G4 (card 51% of tx *count*; Stripe unavailable in AL — must be local PSP; `'card'` enum schema-only) | Revenue — captures the rising-card segment; second payment rail | M–L | Med | **YES — money** | **Gated on fiskalizimi intermediary-regime legal answer (R4 R2)** |
| **15** | **3 named local case studies** — photographed owner + city + one number + Albanian quote; live vendor count once >10 | R2 T5 (73% B2B find case studies crucial; peer > vendor claim) | Trust/acquisition — one recognizable Tirana neighbor outweighs any feature list | M (fieldwork) | Med | No (PII: named vendor consent) | first 3 live vendors |
| **16** | **Hybrid GTM operating motion** — marketplace kept as paid acquisition, convert repeat customers to owned channel (QR in bag, "order direct, save 10%") | R1 §strategic synthesis (l.225); R1 Pain 2 (own-the-customer compounds forever) | Strategic acquisition — the honest answer to the demand-gen gap | L (ongoing) | High (field-proven US/HR) | No | live vendors + storefront |

**Ranking note.** Raw (impact × speed × certainty × Albania-fit) pushes the quick wins (#1) and acquisition motion (#2) above fiskalizimi (#3) purely because fiskalizimi is slower (M) and integration-uncertain. That is the correct *sequencing* signal — ship the cheap trust/acquisition wins in week 1 while the council-gated fiskalizimi build runs in parallel — but do **not** read #3 as "less important." Fiskalizimi is the strategic keystone; the relay-only interim posture is what lets #1/#2 generate ROI before it lands.

---

## 3. Top 3 highest-ROI moves (business case each)

### #1 — Productize the claimable demo (the growth engine)
**Business case.** dowiz's structural weakness is the one thing marketplaces do well: *demand generation* (R1 caveat, l.25). The claimable demo is the single move that neutralizes it. You build the vendor's real storefront *before* first contact — their menu, photos, hours, colors, scraped from Maps (the certified demo-builder loop already does this) — and open with "your menu is already online, here's the link; claim it free." This is simultaneously (a) the **strongest possible risk reversal** — zero effort, money, or data handed over before value is *seen* (R2 T1); (b) **time-to-value = zero**, versus the SaaS benchmark of minutes-not-days; (c) a **credibility bomb** — 75% of vendors judge a business by its website, 94% of first impressions are design (R2 L2), and the vendor sees *their own shop looking professional*; and (d) a **self-feeding referral engine** — a beautiful `/s/:slug` link gets shared vendor-to-vendor. Effort is S–M because the loop exists; the delta is claim-CTA polish + a ≤3-field phone-based claim + batch tooling. **ROI: acquisition + trust + retention in one motion, off assets already built.**

### #2 — Fiskalizimi certified-provider integration (the sellability gate + moat)
**Business case.** Every sale by an Albanian food vendor — cash included, delivery included, down to a 100-lek byrek — must produce a real-time-fiscalized receipt with an NIVF number + QR reported to the DPT (R4 §5, R1–R4). dowiz has **zero** fiscal code (R4 G1). Today that caps the addressable market at "vendors who fiscalize elsewhere and tolerate a second system." Integrating a certified provider (devPOS/easyPos/Elif all expose APIs; vendor cost ≈ €100–200/yr) to issue the NIVF/QR *inside the order flow* does two things no competitor does: it makes dowiz **legally sellable** to any compliant vendor, and it turns compliance — a chore Wolt/Baboon push back onto the restaurant — into a **built-in feature and a moat**. The penalty for getting it wrong (≈€500/receipt + suspension, R4 R4) is exactly why "we handle your fiscal receipt" is a closing line, not a footnote. **ROI: unlocks the entire compliant-vendor TAM; converts a legal liability into differentiation.** Red-line (compliance + money + contract) — Phase-0 accountant sign-off on the intermediary regime *before* any payment-collection feature (R4 R2).

### #3 — The native trust bundle (fastest ROI in the plan)
**Business case.** This is a weekend of work that moves the two biggest levers at once. **Cash-first positioning** ("Porositë vijnë online. Paratë mbeten cash, në dorën tuaj.") is the #1 trust asset in a ~99%-cash, informality-sensitive market where any hint of "payments platform" reads as bank-friction + tax-visibility risk (R2 T2, CGAP). **0%-commission** is the most-documented pain of the segment (R1 Pain 1, R2 T3). **Albanian-first copy** is worth a 76% purchase-preference / 40% never-buy-otherwise swing (R2 L3). And the product currently **sabotages its own native-feel selling point**: `formatMoney` renders `800 ALL` (verified `packages/shared-types/src/utils.ts:26`) while the `L` symbol sits defined-but-unused at line 14 — a one-line fix. Bundle the copy pass + the lekë format fix + a "you kept X lekë this month" savings tile (off existing order totals) + a "Pa rrezik" risk-reversal block (free start · no card · no contract · one-tap export · open-source no-lock-in — R2 T6), and you hit the #1 pain and the #1 fear with near-zero engineering risk and no red-line. **ROI: highest conversion-per-hour in the roadmap.**

---

## 4. Trust & likeability mini-plan → concrete dowiz surfaces (from R2)

| R2 move | Surface(s) | Concrete delta | Effort |
|---|---|---|---|
| **T1 Claimable demo** (strongest) | `/s/:slug` storefront + onboarding | Claim CTA on every demo ("Merreni falas"); ≤3-field phone claim → instant admin handover | S–M |
| **T2 Cash-first "nothing changes about your money"** | Landing + pricing + checkout | Headline "paratë mbeten cash"; card/crypto strictly opt-in, visually secondary (`CheckoutPage.tsx` already cash-default) | S |
| **T3 0% + savings counter** | Landing calculator + admin tile | Lekë calculator vs 25% marketplace cut; in-admin "kept X lekë" tile (init #8) | M |
| **T4 Human on Viber + a face** | Landing + admin footer + printed QR kit | Viber/WhatsApp support line (local number, Albanian, same-day); founder photo/name/phone; in-person setup for early vendors | S tooling / M ops |
| **T5 Named local proof** | Landing + external reviews | 3 photographed vendor case studies (init #15); reply to every review (88% vs 47% effect) | M fieldwork |
| **T6 Risk-reversal stack** | Landing + pricing + admin settings | "Pa rrezik" block; wire existing owner/gdpr export to one visible export button (must actually work) | S–M |
| **T7 Reliability transparency** | `/status` + onboarding | `/status` off the honest health check (audit wave-1); market "çdo porosi ju vjen në Viber" delivery guarantee (pg-boss retry/DLQ) | M |
| **T8 Compliance signals sized down** | Footer + `/privacy` (Albanian) | One quiet line: "Të dhënat tuaja: në BE, kopje rezervë çdo ditë, të vetat tuajat" | S |
| **L1 Zero-to-live in one sitting** | Onboarding | Choreography: PDF/photo menu → live storefront → vendor's own test order (aha) → QR (init #6) | M |
| **L2 "My shop looks good"** | `/s/:slug` | Peak-end "shiko dyqanin tuaj" share moment; printable branded QR table-tents (palette/fonts already built) | S |
| **L3 Albanian everywhere** | All | Native-speaker warmth pass over the i18n catalog (parity gate exists) — coverage + tone, not machine-stiff | S–M |
| **L4 Celebrate wins** | Admin + comms | First-order + 100-orders card + channel message (init #10) | M |
| **L5 Reliability as love** | Admin | Vendor-facing failure honesty: bounce → retry via fallback channel, surface in admin, never silent-drop | S |
| **L6 Micro-interactions where the money is** | Order arrival → accept → complete; end-of-day recap | Feel-across-the-counter new-order pulse+sound; "Sot: 23 porosi, 41,300 L" recap | M |
| **L7 Meet them on the phone they own** | `/s/:slug` + `/admin` | Perf budget on mid-tier Android/3G; messaging channels primary, PWA optional (sw.js exists) | M |
| **L8 Make them smarter, gently** | Admin home | One rotating insight card ("E premtja është dita juaj më e fortë") — **do last**, worthless before order volume | M |

**Launch-blocking for AL GTM:** T1, T2, T3, T4, T6, L3 (they define the first impression). L1 unlocks retention. L4/T5 compound over the first ten vendors.

---

## 5. Visibility / social pipeline build order (from R3) — phase-1 cheapest-that-works

**Phase 1 (~$30–40/mo, ~2-day build) — do this to stand up distribution:**
1. **Postiz hosted ($29)** — connect dowiz IG Business + TikTok + FB Page + Telegram. Their app is Meta-approved and TikTok-audited → **public posting on day 1**, no waiting on your own audits.
2. **n8n via Docker** on existing Fly/VPS infra — one cron workflow `Sun 20:00`: query the DB for the week's angle material (**aggregate numbers only** per PII red-lines; per-vendor claims need that vendor's opt-in) → one **Claude Batch API** call → 7–10 bilingual (al/en) post objects → render stat-card PNGs from an HTML template via Playwright (already a repo dep) → push each to the operator's Telegram with ✅/❌ buttons → on ✅ `POST` to Postiz; Telegram-channel posts go direct via Bot API.
3. **Shlink** (one Docker container) — `go.dowiz.al/xyz` short links with UTM in every caption; closes the loop post → storefront visit.
4. **Skip X + LinkedIn + AI video in phase 1.** Real photos as Reels-style slideshows (FFmpeg one-liner: 5 R2 photos + music → 15s vertical MP4).

**Definition of done:** one Sunday run produces a scheduled week across IG/TikTok/FB/Telegram with ≤10 min of human tapping ✅ in Telegram.

**Phase 2 (weeks 4–12, ~$2–10/mo):** file dowiz's own Meta app review + TikTok Content-Posting audit **in week 1** (2–4 weeks lead) → self-host Postiz (AGPL) once approved → Revideo/FFmpeg video templates fed by R2 photos + WhisperProvider auto-subtitles → vendor Telegram "content-inbox" flywheel → Claude scoring loop on Postiz + Shlink + `/s/:slug` visit metrics.

**Content unfair advantage (R3 §5):** the product DB *is* the content source — real menus, real photos, real order data no competitor tool has. Pillars: P1 "0% commission" aggregate stat cards · P2 demo before→after · P3 R2 food porn · P4 build-in-public/OSS.

---

## 6. Red-line register — must pass a Triadic Council BEFORE any code

| Initiative | Red-line class(es) | Why | Council scope |
|---|---|---|---|
| **#3 Fiskalizimi integration** | compliance + money + contract | Legal receipt issuance, tax authority (DPT) reporting, order-schema change (`nivf`/`nslf`/`qr_url`/`nipt`), and the intermediary-regime question if dowiz ever touches payment | ADR + threat-model + **Phase-0 AL accountant sign-off (R4 R2)** before code |
| **#9 Promo/coupon redemption** | money (touches order pricing) | `discountTotal` becomes server-authoritative in the total ladder (`orders.ts:514`); a validation/apply bug is a money-integrity bug; also fixes the 100× unit bug | Council; mirror the fee-ladder pattern; independent-expectation test (not mirror==mirror — the LC1 lesson) |
| **#14 Local card gateway (POK/Paysera)** | money | New payment rail, settlement, refund obligations behind `PaymentProvider` | Council; **blocked on the fiskalizimi intermediary answer** — do not start until #3's legal Phase-0 clears |
| **#12 Closed-venue gate** | contract (light) | Changes `POST /orders` accept/reject semantics (new 422) — not money/authz | Safe-direct + `invariant-guardian` check (fast-track, not full council) |

**Non-red-line, ship via normal ship-discipline loop:** native trust bundle (#1, minus promo), claimable demo (#2), Instagram channel (#4), Viber owner alerts (#5), onboarding choreography (#6), social pipeline (#7), savings counter (#8, aggregate-only), milestone celebration (#10), `default_locale` (#11), `/status` (#13), case studies (#15).

> The AUDIT red-lines already in council/committed (LC1–LC9, R-A data-access seam, R-B GUC/RLS discipline, R-C CI-is-real) are prerequisites this plan assumes are landing — especially **R-C (make CI real)**: without it, none of the money-touching work above (#3/#9/#14) can be *proven* red→green. Sequence the CI root ahead of shipping any money initiative.

---

## 7. Honesty — what could fail, and the caveats

- **Demand-generation caveat (R1, load-bearing).** dowiz is the **owned channel, not a demand engine**. It wins every cost/control axis over marketplaces but generates **zero discovery by itself**. If the plan is read as "build features → vendors come," it fails. Acquisition *must* run on the claimable-demo motion + Instagram + the social pipeline + eventually the hybrid GTM (marketplace as paid acquisition, convert regulars). Do not oversell "replace Wolt on day 1" — the field-proven motion is hybrid (R1 §strategic synthesis).
- **Fiskalizimi could slip.** Integration effort is provider-dependent and the intermediary-regime legal question (R4 R2) is an *inference needing an Albanian accountant's sign-off* — it is not settled. Mitigation: sell **relay-only** to already-fiscalized vendors (R4 R11) while the integration is built; never let dowiz collect payment on the vendor's behalf until the regime is confirmed.
- **Tablet-hell / one-more-screen (R1 Pain 9).** For a multi-homed vendor, dowiz is initially *one more screen*, not fewer. Honest framing required; the mitigation is being the phone/walk-in order replacement too, not claiming consolidation dowiz can't yet deliver.
- **Own-fleet transfers the courier problem (R1 Pain 8).** The courier module gives control *and* an owned staffing burden. Great for vendors who already deliver (pizzerias with a scooter); a real barrier otherwise. Pickup-only + QR is the zero-fleet fallback — which is why re-enabling pickup (R4 G12, currently dead code) matters more than it looks.
- **Viber owner-alert channel is a real build, not a toggle.** G6 is a genuine channel/market mismatch (Telegram weak in AL); underestimating the Viber/WhatsApp-Business adapter effort would leave owners missing orders — the one failure that permanently kills the relationship (R2 L5).
- **Social pipeline PII line.** Per-vendor revenue/savings claims are PII-adjacent — **aggregate by default, explicit consent for named claims** (consistent with the owner-data-export ETHICAL-STOP precedent). Getting this wrong is a trust *and* ethics failure, not just a legal one.
- **Money-integrity without CI is unprovable.** Promo redemption and card gateway both touch the money ladder; the AUDIT shows the fee-parity test *certified* the LC1 bug (mirror==mirror). Until R-C (CI-is-real) lands, treat every money-touching ship as unverified.

---

## 8. Monday-morning first build

**Ship the lekë-format fix + `default_locale` + closed-venue server gate as the opening quick-win commit, and in the same day write the copy for the cash-first / 0% / "Pa rrezik" positioning.** The format fix is one line (`packages/shared-types/src/utils.ts:26` → render `L`/`Lekë` instead of `ALL`, symbol already defined `:14`), it repairs the native-feel selling point the whole GTM leans on, it is not a red-line, and it is provable same-day with a Playwright assertion on `/s/:slug`. It is the smallest, safest, highest-trust-per-hour first move — and it unblocks shipping the rest of the native trust bundle (#1) while the fiskalizimi council (#3) and the claimable-demo build (#2) spin up in parallel.
