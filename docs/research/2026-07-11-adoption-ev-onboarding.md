# Max-EV adoption levers — zero-friction onboarding mechanics — 2026-07-11

> **Lens:** the operator's binding constraint — vendors try dowiz only if friction is ~zero (no
> signups, no contracts, no money asked, minutes-to-value). This report ranks the moves that take a
> Durrës vendor from "never heard of it" to "using it at their venue", using ONLY assets already in
> the repo, and scores each move's friction from the *vendor's* side.
> **Method:** repo ground truth re-verified in code this session where marked; funnel/live-probe
> state inherited from same-day docs `docs/design/gap-blueprints-2026-07-11/G11-business-validation-week.md`
> and `docs/research/2026-07-11-hub-architecture-review.md` (both dated 2026-07-11 — cited, not
> re-probed). Web claims cited with source quality. Labels: **VERIFIED** (checked in code/primary
> source this session), **VERIFIED-G11/HUB** (verified live earlier today by those docs),
> **UNVERIFIED** (secondary/aggregator web source — directionally useful, not audit-grade).
> **This file is the only artifact created; the tree (branch `feat/paleo-dinosaur-digs`) was left as found.**

---

## 1. The "already paid for" arsenal — zero-friction assets and their real state

| # | Asset | What it gives a vendor | Real state today | Evidence |
|---|---|---|---|---|
| A1 | **12 pre-built shadow demos** (real Durrës venues, scraped menus 19–107 items, real ratings 4.6–5.0, real phones) | Their restaurant already live at `/s/<slug>` — zero work done by them | **Staging: all 12 live. Prod: 11/12 are 404** (only `/s/demo` Dubin & Sushi) | VERIFIED-G11 §2 S1; slugs in memory `og-preview-demo-upgrades-2026-07-06` |
| A2 | **Claim-by-link** (`/claim#token=…` → sign in → accept; binds ownership only, publish stays a separate deliberate act) | Ownership in 2–3 taps, no form, no contract | **BROKEN everywhere: `GET /claim` = 404 on prod AND staging.** Root cause: `'/claim'` absent from `SPA_ROUTES` — VERIFIED in code this session: `apps/api/src/server.ts:858` lists `['/admin','/courier','/dashboard','/s/','/login','/branding-preview','/privacy']`. React page exists (`apps/web/src/pages/ClaimPage.tsx` — token in URL fragment, scrubbed, single-use, TTL 3d). Token minting dead until `PROVISION_OPS_SECRET` re-rotated (lost throwaway, memory `claim-happy-path-and-sops-2026-06-29`) | VERIFIED (code) + VERIFIED-G11 S3 |
| A3 | **On-page claim CTA** ("Is this your restaurant?") | Self-serve claim from the storefront itself | Built, **dark**: `SHOW_CLAIM_CTA = false` — VERIFIED `apps/web/src/pages/client/MenuPage.tsx:454` (render at `:1001`); `POST /claim/request` handler kept intact | VERIFIED (code) |
| A4 | **Menu auto-import pipeline** (`scripts/demo-from-wolt.mjs` → prospect JSON; `scripts/demo-builder.mjs` quality-gates + provisions; `scripts/acquisition-bulk-provision.mjs` bulk) | A new prospect's demo is **one command** — menu, categories, prices, authentic Albanian text, no LLM needed | Built and proven (it built the 12). Needs `PROVISION_OPS_SECRET` (same blocker as A2) | VERIFIED (scripts exist, read this session) |
| A5 | **Rich OG unfurl cards** (`/og/<slug>.png`, per-venue name + rating + dish photo, 1200×630) | A pasted link that *sells itself* in WhatsApp/Telegram | **Staging: live for all 12** (bot-UA `og:title "Apollonia — Menu Digjitale"`, noindex kept). **Prod: 404** — commit `6a89d6e8` is not on main; rides the G01 merge vehicle only | VERIFIED-G11 S2; memory `og-preview-demo-upgrades-2026-07-06` |
| A6 | **QR kit** | Printed scan-to-menu handout — the physical trust artifact | `qrcode@1.5.4` dep present — VERIFIED `apps/web/package.json:21`; admin `QRKitPage.tsx` built but dark behind build-time `VITE_CHANNEL_KIT_ENABLED` (VERIFIED files); G11 3.1: a 20-line offline script generating 12 PNGs needs **no deploy** | VERIFIED (code) |
| A7 | **Owner Telegram ops** (`/start` deep-link connect, order Confirm/Reject inline buttons, live in prod) | "Run it from your pocket" — the order arrives on their phone | LIVE (`telegram-webhook.ts`); owner-alert consumer runs in the API process, so the stopped prod worker doesn't kill it — but alert firing must be re-verified empirically (G11 1.5) | VERIFIED-HUB §5.2 |
| A8 | **Storefront-toggle via Telegram** (`/store` open/close) | Pocket control without opening the console | Built, dark behind `TG_STOREFRONT_ACTION` | VERIFIED-HUB §5.2 |
| A9 | **Telegram Mini App** | Storefront inside Telegram | Dark, **CSP blocks `telegram-web-app.js`**, owner-self-preview only, audience question explicitly open | VERIFIED-HUB §2 row 5 |
| A10 | **Outreach sender** (`scripts/report-demos-to-telegram.mjs`, defaults BASE=staging) | Structured demo pitch into a chat | Exists; preview behavior needs the G11 3.4 dry-run | VERIFIED (script exists) |
| A11 | **Legal posture for order #1** | "No money asked" is actually safe | Relay-only + cash + venue fiscalizes on own POS = **no legal blocker to a first cash order**; blockers attach only to *charging* later | VERIFIED-G11 §2 (fiskalizimi read) |

**The single controlling fact:** every zero-friction asset above is built, and the funnel they feed
is broken at its hinge — a one-line `SPA_ROUTES` fix plus one 5-minute secret rotation stand between
this arsenal and a claimable demo. Everything in §3 assumes those two land first (G11 Days 1–2).

---

## 2. Analogue evidence — what actually converts SMB restaurant owners

### 2.1 Pre-built-profile claim patterns (the closest analogue to the shadow-demo strategy)

- **Google Business Profile:** estimates of unclaimed listings range 11%–56% depending on
  methodology ([SearchLab via starfish.reviews](https://starfish.reviews/google-business-profile-statistics/);
  [bloggingwizard](https://bloggingwizard.com/google-business-profile-statistics/);
  [searchendurance](https://searchendurance.com/google-business-profile-statistics/)). UNVERIFIED
  (secondary aggregators). **Read-through for dowiz:** even with Google-scale prompts, roughly half
  of SMBs never claim a pre-built profile *passively*. A passive "claim me" CTA alone is a weak
  motion; the claim converts when a human walks the owner through it — which is exactly the
  concierge design dowiz already has.
- **Yelp:** ~5.4M claimed pages; the Luca/Harvard-lineage study found simply being listed lifts
  revenue 5–9% ([Yelp official blog](https://blog.yelp.com/news/academic-study-reveals-listing-a-business-on-yelp-can-increase-revenue/) —
  primary, VERIFIED as a published claim; magnitude UNVERIFIED here). Claiming is free and framed
  as recovering something that already exists — the psychological frame dowiz's shadow demos
  reproduce: *loss-framing ("this exists about you, take control of it") outperforms gain-framing
  ("sign up for a new thing")*.
- **TripAdvisor:** no public claimed-percentage stat found (UNVERIFIED absence); its whole
  owner-acquisition surface is the free "Claim your listing" flow ([tripadvisor.com/Owners](https://www.tripadvisor.com/Owners)),
  and its behavioral hook is that management responses (claimed-only feature) drive 77% higher
  booking intent ([Ipsos MORI via TripAdvisor IR](https://ir.tripadvisor.com/news-releases/news-release-details/tripadvisor-study-reveals-77-travelers-more-likely-book-when)) — i.e. the platform sells the claim by showing what the owner is *missing*.
- **Owner.com** (the closest commercial analogue: commission-free direct ordering for independent
  restaurants, $499/mo): its onboarding promise is *"many restaurants launch within a week"* and it
  builds the site FOR the restaurant ([owner.com](https://www.owner.com/); [Sacra](https://sacra.com/c/owner/);
  [Contrary Research](https://research.contrary.com/company/owner)). UNVERIFIED specifics. dowiz's
  shadow demo compresses Owner.com's "week" to **zero vendor-side build time** — the demo already
  exists before first contact. That is the differentiated move; protect it.

### 2.2 Steps-to-first-transaction in SMB payment/POS onboarding

- **Square:** signup ~3 minutes, accept payments same day, no approval gate; bank verification
  deferred until first payout ([Square get-started guide](https://squareup.com/help/us/en/article/5123-square-get-started-guide);
  [merchantmaverick](https://www.merchantmaverick.com/set-up-square-account/)). UNVERIFIED exact
  minutes; the pattern (defer every non-essential permission past the first transaction) is the
  canonical PLG-payments motion.
- **SumUp:** sign-up 5–10 min; **up to £5,000 of transactions allowed BEFORE full business
  verification completes** ([sumup business guide](https://www.sumup.com/en-gb/business-guide/how-to-get-started-sumup/);
  [mobiletransaction.org](https://www.mobiletransaction.org/sumup-review/)). UNVERIFIED. The
  instructive rule: *let value flow first, verify later* — dowiz's claim-binds-ownership /
  publish-later split is the same shape and should stay that way.
- **Toast (the sales-led anti-pattern):** implementation 4–6 weeks remote/onsite, ~14 days
  self-serve ([Toast support](https://support.toasttab.com/en/article/Remote-Onsite-Onboarding-Guide) —
  primary, VERIFIED as their own documentation). This is the friction ceiling dowiz must stay two
  orders of magnitude under — and does, if claim + test order fit in one table-side visit.
- **GloriaFood** (free, self-serve restaurant ordering; the repo's own Tier-3 note already cites
  it): sells "start taking orders by tonight," setup wizard, no tech knowledge
  ([gloriafood.com](https://www.gloriafood.com/)). UNVERIFIED marketing claim, but it anchors vendor
  expectations: *same-day* is the market bar for free ordering tools; dowiz's concierge+pre-built
  combo beats it only if the funnel actually works on the first try.

### 2.3 WhatsApp-first commerce + messaging channel reality

- WhatsApp Business app setup ~15–20 min, free; 44% of small businesses report sales growth within
  3 months; catalog views ~40M/month; chat conversion 18–25% vs 6–7% email
  ([Menubly](https://www.menubly.com/blog/whatsapp-for-restaurants/);
  [ycloud stats](https://www.ycloud.com/blog/whatsapp-statistics-for-businesses);
  [electroiq](https://electroiq.com/stats/whatsapp-business-statistics/)). ALL UNVERIFIED
  (aggregators). Direction: chat channels convert well *as messages carrying a link*, which is all
  dowiz needs (unfurl → web checkout) — consistent with the repo's own cart-token doctrine of never
  building in-chat checkout.
- **Albania channel mix:** Statcounter (fetched this session, VERIFIED primary): Facebook = 95.6%
  of social referral traffic in Albania, June 2026; Instagram 1.7%
  ([gs.statcounter.com](https://gs.statcounter.com/social-media-stats/all/albania)). Caveat: this
  measures *referral traffic*, not usage, and is blind to messengers. WhatsApp is the top messenger
  in most of Europe incl. the Balkans per country maps ([Sinch](https://sinch.com/blog/most-popular-messaging-apps-by-country/);
  [Infobip](https://www.infobip.com/blog/most-popular-messaging-apps-by-country)) — UNVERIFIED for
  Albania specifically; the repo's R9 "messenger penetration unresolved" stands. Practical
  conclusion: **the vendor's Facebook page + WhatsApp contact are the two placement targets for the
  storefront link; do not build any messenger transport for onboarding.**

### 2.4 Link-in-bio commerce + QR + activation science

- Linktree: 50–70M users, ~$6B annual GMV via link-in-bio, commerce clicks convert ~50% above
  e-commerce average ([TechCrunch](https://techcrunch.com/2024/05/22/linktree-surpasses-50m-users-rolls-out-beta-social-commerce-program/) —
  primary for user count; [retailbrew](https://www.retailbrew.com/stories/2024/05/31/linktree-boosts-social-commerce-business-to-target-gen-z-shoppers) for GMV/conversion — UNVERIFIED).
  Read-through: "paste one link into your bio" is a proven, zero-training distribution act an SMB
  owner already understands — it should be the scripted *first thing* a claimed owner does.
- QR menus: >75% of restaurants use QR menus; customer scan rate **73% when the purpose is
  explained vs 34% when not** ([EasyMenus 10k-restaurant dataset](https://easymenus.net/blog/qr-code-menu-adoption-statistics-data);
  [barkoder](https://barkoder.com/blog/30-shocking-qr-code-statistics-you-need-to-know-in-2025)).
  UNVERIFIED. Two implications: QR-at-the-counter is a familiar, zero-explanation artifact for a
  Durrës owner already on Wolt; and the printed sheet must carry a one-line "why" in Albanian.
- Activation science: aha-moment target 3–5 min from first touch; users experiencing value within
  15 minutes are ~4–5× likelier to retain; >4-step onboarding flows crater completion (40.5% at 4
  steps → 21% at 5) ([rework.com TTV benchmarks](https://resources.rework.com/libraries/saas-growth/onboarding-time-to-value);
  [ProductLed](https://productled.com/blog/product-led-growth-metrics)). UNVERIFIED numerics;
  the design rule they justify is hard: **every vendor flow below is budgeted at ≤4 vendor actions
  and ≤5 minutes, and anything that can be pre-done by the operator is pre-done.**

---

## 3. Ranked max-EV moves

Friction score = vendor-side **steps / minutes / permissions / € asked**. EV = reach × conversion ×
(1/effort), argued qualitatively — no fake precision. Ordering is the recommendation.

### M0 — Un-break the hinge (prerequisite, not a lever)
- **What:** `'/claim'` into `SPA_ROUTES` (+ `/courier-invite`, same class) · operator re-rotates
  `PROVISION_OPS_SECRET` on staging · run `/reliability-gate` incl. the open 07-04 `checkout-phone`
  break · verify the owner Telegram alert fires on a staging order.
- **Friction:** vendor 0/0/0/€0 — this is all operator+agent side.
- **EV reasoning:** infinite multiplier — every move below divides by zero without it. A dead claim
  link or a failed first order at the table is not neutral, it's *negative* EV (Tier-1: "a lost
  order at 19:00 = a lost venue").
- **Blocking:** `apps/api/src/server.ts:858` (VERIFIED); lost secret (memory 06-29); unclosed
  staging checkout flag (G11 S5c). Effort: one line + 5 operator-minutes + one gate run.
- **Analogue:** none needed — this is G11 Days 1–2 verbatim.

### M1 — "Scan this — that's your restaurant, live." (in-person QR + phone-in-hand demo, wave 1 on the 12)
- **What:** operator visits the venue as a customer, opens `/s/<slug>` on their own phone OR hands a
  printed QR card (offline script from the existing `qrcode` dep; 12 PNGs, per-venue A5 sheet with a
  3-bullet Albanian pitch). The owner sees their own menu, photos, Google rating, working cart — a
  thing that already exists about them.
- **Friction:** **1 step (scan) / <1 min / 0 permissions / €0.** The single lowest-friction
  demonstration physically possible.
- **EV:** reach = 12 warm targets now (50+ Wolt-Durrës venues later via M6); conversion — this is
  the "show, don't tell" pattern that puts personalized outreach in the 15–25% top-quartile reply
  band ([salesmotion](https://salesmotion.io/blog/cold-outreach-best-practices), UNVERIFIED) and it
  exploits the loss-framing that makes GBP/Yelp claims work (§2.1); effort S (script + print).
- **Blocking today:** nothing hard — staging serves all 12 (VERIFIED-G11). QR script not yet
  written (G11 3.1); operator-identity footer not in code (PLAN.md WS3, S-effort trust signal);
  `staging` hostname cosmetic until M7.
- **Analogue evidence:** QR familiarity ≥75% of restaurants, 73%-with-explanation scan rate (§2.4);
  Owner.com's "we build it for you" motion minus the week of waiting (§2.1).

### M2 — Claim-on-the-spot (token minted same morning, 2–3 taps at the table)
- **What:** during the M1 visit, operator mints a fresh claim token (TTL 3d, single-use) and the
  owner claims on their own phone: open link → sign in (Google or Telegram deep-link — both live) →
  tap Accept. Ownership binds; **publish stays the owner's later, deliberate act** (protected
  friction — keep it; it mirrors SumUp's value-before-verification pattern §2.2).
- **Friction:** **3 steps / 2–3 min / 1 permission (a login identity) / €0.** No form fields, no
  contract, no payment method.
- **EV:** conversion is the whole funnel's throat. Concierge-assisted claiming is the correct
  motion — GBP-scale evidence says ~half of owners never claim unaided (§2.1), so the human walks
  them through 3 taps. Effort ≈ 0 beyond M0.
- **Blocking today:** M0 items only (`server.ts:858`; secret). E2E spec `flow-simpl-s6-claim.spec.ts`
  already encodes the RED case (it 404s today — VERIFIED-G11).
- **Analogue:** Yelp/GBP claim flows (§2.1); ≤4-step completion cliff (§2.4).

### M3 — The 60-second pocket order (Telegram connect + owner's own test order)
- **What:** immediately after M2: owner console → Settings → Telegram connect (one deep-link tap,
  live in prod code — `SettingsPage` connect + Send-Test); then the owner (or operator on the
  owner's storefront) places a test order; the owner's phone buzzes with the real order and
  **Confirm/Reject buttons** within 60s. The loop the vendor is buying — "customer orders → I control
  it from my pocket" — demonstrated on their own venue, their own phone, before publish.
- **Friction:** **+2 steps / ~3 min / 1 permission (Telegram) / €0** on top of M2. Total journey
  M1→M3 ≈ 6 vendor actions, <10 minutes — inside the 15-minute retention window (§2.4).
- **EV:** this is the *activation event*, the strongest predictor of staying (4–5× retention for
  <15-min value, §2.4). Reach = every claimed vendor; conversion of claimed→active is where dowiz
  wins or loses; effort S (all code live — needs the M0 alert verification + G03 steering: use
  telegram/whatsapp/viber contact kinds only until the 6-kind fix ships, VERIFIED `apps/web/src/lib/messenger.ts:8`
  vs the 3-kind server enum).
- **Blocking today:** owner-alert empirical verification (G11 1.5); staging checkout break (M0);
  G03 422s on 3/6 kinds (steer around; fix rides Wave-1).
- **Analogue:** Square/SumUp same-session first transaction (§2.2); TTV benchmarks (§2.4).

### M4 — The owner's first distribution act, scripted: link in bio + Facebook page button
- **What:** close the concierge session by having the owner paste their `/s/<slug>` link into their
  Instagram bio and Facebook page ("Order" button / pinned post), and their WhatsApp Business
  profile if present. Owner does it on their own accounts — their channel, their customers. This is
  what turns a claimed demo into the GREEN event (first real order from a non-operator customer).
- **Friction:** **2–3 steps / ~3 min / 0 new permissions / €0.**
- **EV:** reach = the venue's existing followers (the only audience that converts on day 1);
  link-in-bio commerce converts ~50% above e-commerce baseline (§2.4, UNVERIFIED); Albania is
  overwhelmingly Facebook-first by referral share (95.6%, VERIFIED Statcounter) — so **Facebook
  page placement outranks Instagram bio here**, inverting the default playbook. Effort ≈ 0 (a
  scripted moment in the visit).
- **Blocking:** nothing technical; needs M1–M3 done and the owner willing. Attribution already
  stamps `?ch=` links (VERIFIED-HUB §2 row 3) — mint the bio link as `/s/<slug>?ch=instagram` /
  `?ch=facebook` so the pilot funnel is measurable later, at zero extra vendor cost.

### M5 — The link that sells itself: rich-unfurl outreach (remote wave / follow-up)
- **What:** send the demo link into the prospect's WhatsApp/Viber/Telegram — the per-venue OG card
  (name + rating + dish photo) IS the pitch. Use `report-demos-to-telegram.mjs` mechanics + the
  operator's personal accounts (anti-spam trust: real local person, identity footer).
- **Friction:** **0 vendor steps** to receive; 1 tap to see their storefront.
- **EV:** reach extends beyond walk-ins (follow-ups, referrals, the venue owner forwarding it to a
  partner/spouse — chat-forwarding is the organic loop); conversion lower than in-person but
  additive; effort ≈ 0 on staging (cards live for all 12, VERIFIED-G11 S2).
- **Blocking today:** **prod OG is 404** — `og-card.ts` not on main, rides the G01 merge only
  (VERIFIED-G11 S2); until then send staging links (fine for warm outreach); paste-test one real
  unfurl per G11 3.4 before any send.
- **Analogue:** WhatsApp message→link conversion (§2.3); OG-card unfurl is the chat-native
  equivalent of the link-in-bio card (§2.4).

### M6 — "Your demo, built while you watch": one-command demo for ANY prospect
- **What:** the reach multiplier. `demo-from-wolt.mjs <wolt-slug>` → prospect JSON →
  `demo-builder.mjs` quality-gates and provisions a full demo (VERIFIED scripts). When outreach
  meets a venue outside the 12 (waiter says "talk to the owner of X", a referral, a walk-in), the
  demo exists same-day — or, staged theatrically, *before the coffee arrives*.
- **Friction:** vendor **0 steps / 0 min / 0 permissions / €0** — the entire cost is operator-side
  and scripted.
- **EV:** converts the shadow-demo motion from a fixed 12-shot magazine into a repeatable process
  covering every delivery-active venue in Durrës (Wolt 50+ venues, Baboon 1,000+ across 4 cities —
  repo R10). Conversion inherits M1's; effort per additional prospect ≈ minutes.
- **Blocking today:** `PROVISION_OPS_SECRET` (M0); Wolt-ToS/scraping-conduct gate already noted in
  the script header (VERIFIED); privacy substitutes (noindex, honesty banner, decline/erase) must
  hold for every new shadow — they are the ethics floor of this whole strategy.
- **Analogue:** Owner.com's build-for-you motion at zero marginal cost (§2.1).

### M7 — Kill the trust tax: register the domain (~€10–30, once)
- **What:** `porosite.al` (or instantly-registrable `dowiz.app`) → point at the outreach surface →
  regenerate QRs/OG absolute URLs. Removes "staging.fly.dev" from every link, QR, and unfurl.
- **Friction:** vendor 0/0/0/€0 — operator pays ~€10–30/yr (the only money in this entire report,
  and it's not asked from vendors).
- **EV:** a multiplier on M1/M4/M5 credibility, near-zero effort. Both domains NXDOMAIN as of
  today (VERIFIED-G11 WS3). Don't let registrar delays block wave 1 (G11 D5).

### M8 — Flip the self-serve claim CTA (`SHOW_CLAIM_CTA`) — *after* wave 1, for cold scale
- **What:** one-flag flip (`MenuPage.tsx:454`, VERIFIED) exposes "Is this your restaurant?" on
  demo-class storefronts → `POST /claim/request` (handler live). Turns every forwarded link and QR
  scan into a self-serve claim channel.
- **Friction:** **2 taps / 1 min / contact info only / €0.**
- **EV:** reach = everyone who ever sees a demo without the operator present; conversion is
  materially lower than concierge (§2.1's unclaimed-GBP evidence is exactly this failure mode) —
  which is why it's ranked last of the do-list, per G11 D4: token-only for week 1, flip when
  outreach goes cold/at-scale.

**Sequence in one line:** M0 → M1+M2+M3 in a single table-side visit (one venue at a time,
ArtePasta → Dubin & Sushi → Apollonia per G11 5.2) → M4 same visit → M5 follow-ups → M7 in parallel
→ M6 as opportunities appear → M8 when warm outreach saturates.

---

## 4. Top-3 wow-moment designs (from existing assets only, ranked)

**W1 — "Scan this. That's you." (fastest wow — 15 seconds, 0 permissions)**
Choreography: operator orders a coffee like a customer, then slides a printed card across the
counter: their venue name, their Google rating, a QR. Owner scans with their own phone → their own
menu, their dishes, their photos, Albanian UI, working cart. Script line (one sentence, per the
73%-vs-34% explanation effect): *"Kjo është dyqani juaj online — pa komision. Skanoje."* Why it
works: loss-framed (it already exists), zero-permission, and the artifact stays on the counter after
the operator leaves. Assets: A1 + A6 (+ M7 domain when it lands). Ready: after M0 + a 20-line QR
script. **Rank 1 — the opener; maximizes demos-per-day.**

**W2 — "Order from yourself. Watch your pocket." (deepest wow — the activation event)**
Choreography: after the 3-tap claim (W1→M2), owner taps the Telegram connect deep-link (1 tap);
operator says "now order something from yourself" — owner places a test order on their own
storefront; ≤60s later their phone buzzes: the order, with Confirm and Reject buttons. Owner taps
Confirm. That tap is the product: *no aggregator, no tablet, your phone, your customer, 0%.*
Assets: A2 + A7 (+A8 later). Ready: after M0 (alert verification is the gate — a silent phone here
is a killed deal, so G11 1.5 is non-negotiable before the first visit). **Rank 2 — slower to reach
but the moment that predicts retention; this is the "aha" within the 15-minute window.**

**W3 — "Send yourself the link. It looks like a brand." (most viral — doubles as distribution)**
Choreography: owner pastes their `/s/<slug>` link into their own WhatsApp (to themselves or the
venue group chat). It unfurls: venue name, rating, dish photo — a card that looks like a national
brand's. Script: *"Kaq duhet — vetëm ky link. Vendose te faqja e Facebook-ut dhe bio e Instagram-it."*
The wow and the first distribution act (M4) are the same gesture — the demo teaches the habit that
produces the GREEN event. Assets: A5 (+M7). Ready: today on staging; on prod only after the G01
merge. **Rank 3 — weakest as a standalone opener (needs W1's context), strongest as the closer.**

The three chain into one ≤10-minute visit: W1 opens → M2 claims → W2 activates → W3 distributes.

---

## 5. Skip-list — high-effort / low-EV for onboarding (do NOT do now)

| Skip | Why (evidence) |
|---|---|
| **Telegram Mini App wiring** | CSP blocks `telegram-web-app.js`, flags dark, audience question explicitly open, owner-self-preview only (HUB §2 row 5). Zero onboarding leverage; the storefront link already works in Telegram via M5. |
| **WhatsApp Cloud API intake** | Weeks of build + Meta Business Verification 2–5 days + council-gated ingress (EXPANSION-PLAN 1.C). The unfurl-link motion captures WhatsApp's value at zero build. Re-entry: G7 vendor survey demand. |
| **WS4 video pipeline** (Revideo/FFmpeg/Whisper) | Polish, not load-bearing for claim #1 (G11's own not-this-week list). A live demo in the owner's hand beats a video of one. |
| **Attribution dashboard before claim #1** | Right move for week 2–3 (HUB §7 ranks it), but it converts *operators'* understanding, not vendors — no onboarding EV until there are orders to attribute. |
| **Dedicated demo Fly app** | Solves staging churn that a one-week demo-freeze solves for free (G11 Option C). |
| **Custom per-tenant domains, widget productization, kiosk, voice** | Tier-3 as sorted (Business-Value-Sort); none moves first-claim probability. |
| **Viber bot** | €100/mo floor vs LOW-MED evidence (HUB §7.2); the messenger-penetration question (R9) is unresolved. |
| **Better Auth migration mid-funnel** | Red-line auth churn during outreach week risks breaking the exact login the claim flow depends on. Existing Google/Telegram login suffices for M2. |
| **Card/crypto payments, fiskalizimi integration** | Cash relay-only is legal for order #1 (A11); payments are a *charging-phase* problem. One 30-min accountant call (G11 D7), zero code. |
| **Any new infrastructure** | G11's budget line stands: one line in SPA_ROUTES, one footer, one QR script, one probe script, one funnel digest. This report adds nothing to that list — it only sequences the vendor-facing use of it. |

---

## 6. Sources

Repo (primary): `docs/design/gap-blueprints-2026-07-11/G11-business-validation-week.md` ·
`docs/research/2026-07-11-hub-architecture-review.md` §2/§5/§7 · `DeliveryOS-Business-Value-Sort.md`
· `docs/design/dowiz-brand/EXPANSION-PLAN.md` · memories `storefront-venue-data-maps-scrape-2026-07-01`,
`og-preview-demo-upgrades-2026-07-06`, `dubin-claimable-demo-2026-06-30`,
`claim-happy-path-and-sops-2026-06-29` · code anchors verified this session: `apps/api/src/server.ts:858`,
`apps/web/src/pages/client/MenuPage.tsx:454,1001`, `apps/web/package.json:21`,
`apps/web/src/lib/messenger.ts:6-8`, `apps/web/src/pages/admin/QRKitPage.tsx`,
`scripts/demo-from-wolt.mjs`, `scripts/demo-builder.mjs`, `scripts/acquisition-bulk-provision.mjs`,
`scripts/report-demos-to-telegram.mjs`, `apps/web/src/pages/ClaimPage.tsx`.

Web: [Yelp academic-study blog](https://blog.yelp.com/news/academic-study-reveals-listing-a-business-on-yelp-can-increase-revenue/) ·
[starfish.reviews GBP stats](https://starfish.reviews/google-business-profile-statistics/) ·
[bloggingwizard GBP stats](https://bloggingwizard.com/google-business-profile-statistics/) ·
[searchendurance GBP stats](https://searchendurance.com/google-business-profile-statistics/) ·
[TripAdvisor Owners](https://www.tripadvisor.com/Owners) · [TripAdvisor/Ipsos MORI 77%](https://ir.tripadvisor.com/news-releases/news-release-details/tripadvisor-study-reveals-77-travelers-more-likely-book-when) ·
[Owner.com](https://www.owner.com/) · [Sacra on Owner](https://sacra.com/c/owner/) ·
[Contrary Research on Owner](https://research.contrary.com/company/owner) ·
[Square get-started](https://squareup.com/help/us/en/article/5123-square-get-started-guide) ·
[Merchant Maverick Square setup](https://www.merchantmaverick.com/set-up-square-account/) ·
[SumUp getting started](https://www.sumup.com/en-gb/business-guide/how-to-get-started-sumup/) ·
[mobiletransaction SumUp review](https://www.mobiletransaction.org/sumup-review/) ·
[Toast onboarding guide](https://support.toasttab.com/en/article/Remote-Onsite-Onboarding-Guide) ·
[GloriaFood](https://www.gloriafood.com/) ·
[Menubly WhatsApp-for-restaurants](https://www.menubly.com/blog/whatsapp-for-restaurants/) ·
[YCloud WhatsApp stats](https://www.ycloud.com/blog/whatsapp-statistics-for-businesses) ·
[electroiq WhatsApp Business stats](https://electroiq.com/stats/whatsapp-business-statistics/) ·
[TechCrunch Linktree 50M](https://techcrunch.com/2024/05/22/linktree-surpasses-50m-users-rolls-out-beta-social-commerce-program/) ·
[Retail Brew Linktree commerce](https://www.retailbrew.com/stories/2024/05/31/linktree-boosts-social-commerce-business-to-target-gen-z-shoppers) ·
[EasyMenus QR data](https://easymenus.net/blog/qr-code-menu-adoption-statistics-data) ·
[barkoder QR stats](https://barkoder.com/blog/30-shocking-qr-code-statistics-you-need-to-know-in-2025) ·
[Statcounter Albania social](https://gs.statcounter.com/social-media-stats/all/albania) ·
[Sinch messenger map](https://sinch.com/blog/most-popular-messaging-apps-by-country/) ·
[Infobip messenger map](https://www.infobip.com/blog/most-popular-messaging-apps-by-country) ·
[rework TTV benchmarks](https://resources.rework.com/libraries/saas-growth/onboarding-time-to-value) ·
[ProductLed PLG metrics](https://productled.com/blog/product-led-growth-metrics) ·
[Salesmotion cold-outreach playbook](https://salesmotion.io/blog/cold-outreach-best-practices).

*Authored 2026-07-11 by a read-only research session; the only file created is this report.*
