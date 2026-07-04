# Customer Distribution Channels — one ordering system, N surfaces (Lane F)

**Date:** 2026-07-04 · **Status:** research (docs-only) · **Scope:** how the `/s/:slug` ordering
experience reaches customers beyond "a link in a browser" — under the HARD constraint of **no
App Store / Play Store native apps** — plus (scope extension) the omnichannel **channel-hub
architecture** that lets ONE system serve web, messengers, social, maps, QR, kiosk and agentic
surfaces.

**Method.** Every channel is grounded twice: (a) in the codebase as it exists today (file:line),
and (b) in live 2026 web sources fetched/verified this week (URLs cited inline; anything
unverifiable is flagged). Albania-market claims lean on the existing
`docs/research/2026-07-03-*` corpus — including its own honesty downgrade (see §0.2).

**Architectural prior (decided):** the rebuild stack is Rust (axum) + **Astro 5 shell + Svelte 5
islands**, with `/s/[slug]` SSR/prerendered (`docs/design/rebuild-plan/06-complete-rebuild-stack.md:24`).
Consequence: the storefront becomes a near-instant-loading URL, which is precisely the asset every
channel below distributes. **Every channel is a different wrapper around the same link.**

**Money rule (standing):** all conversational/social/agentic channels terminate in a **signed
cart-token handoff into the ONE web checkout**. No channel gets its own payment surface.
🔴 Any in-chat payment (Telegram Payments, WhatsApp payments, ACP delegated payments) is
council-gated and out of scope for adoption tiers here (§14).

---

## §0 What already exists in the codebase (grounding)

### §0.1 Channel-relevant assets, verified today

| Asset | State | Evidence |
|---|---|---|
| `/s/:slug` storefront (SSR shell for bots, SPA for humans) | LIVE | `apps/api/src/routes/spa-proxy.ts`; test-selector memo `docs/research/…` (storefront-test-selectors) |
| **Subdomain white-label** `<slug>.dowiz.org` → internal rewrite to `/s/:slug` | LIVE | `apps/api/src/lib/subdomain-rewrite.ts:17-32` (reserved: `www/api/app`); wired in `apps/api/src/server.ts:181` |
| **Custom vendor domains** (`order.vendor.al`) | NOT BUILT | rewrite hard-codes `host.endsWith('dowiz.org')` (`subdomain-rewrite.ts:19`) |
| **Embeddable widget** (script + SRI + iframe auto-resize) | BUILT | `apps/api/src/client/widget/loader.ts` (usage: `<script src=".../widget.js" data-slug=... integrity=... crossorigin>`), built artifacts `apps/api/public/dist/widget.js`, `widget.integrity.txt`, `embed-helper.js` (postMessage `dowiz:resize`) |
| **PWA manifest** | BUILT, single-tenant-flavored | `apps/api/public/manifest.json` — ONE global manifest (`name: "DeliveryOS"`, `start_url: "/?source=pwa"`, `scope: "/"`), hardcoded shortcut `/s/dubin-sushi` (line 33). Not per-tenant. |
| **A2HS install prompt** | BUILT | `apps/web/src/components/pwa/InstallPrompt.tsx` — captures `beforeinstallprompt` (Android/Chromium), iOS-Safari feature-detected fallback instructions |
| **Service worker** | BUILT, discrepancy | source `apps/api/src/public/sw.js` has `push` (line 11) + `notificationclick` (line 39) handlers; the checked-in built `apps/api/public/sw.js` is a cache-shell ONLY (`dowiz-shell-v1`, no push handler). Matches this file's "prior defect" biomarker — re-verify build pipeline before relying on customer push display. |
| **Web-push vertical (customer)** | BUILT | checkout opt-in `apps/web/src/pages/client/checkout/push.ts` → `/push/vapid-public-key` (`apps/api/src/routes/public/vapid.ts`) → `/customer/push/subscribe` (`apps/api/src/routes/customer/push.ts`) → sender `apps/api/src/notifications/adapters/webpush.ts` |
| **Telegram bot** (owner-side only) | LIVE | `apps/api/src/routes/telegram-webhook.ts`, poll worker `apps/api/src/notifications/workers/telegram.poll.ts`; flags `TG_CATEGORY_GATING` / `TG_STOREFRONT_ACTION` default off |
| **Checkout messenger deep-links** (customer→courier comms, ADR-0016) | LIVE | `apps/web/src/lib/messenger.ts:6` — 6 kinds `phone/whatsapp/viber/telegram/signal/simplex`; links `wa.me`, `viber://chat?number=`, `t.me/<user>`, `signal.me` (lines 48-80). **Instagram absent** (gap G9). |
| **WhatsApp owner-notification channel** | DELIBERATELY REMOVED | migration `1790000000043_remove-whatsapp-channel.ts` (privacy/ToS) — cited in `2026-07-03-albania-gap-analysis.md` G7 |
| **QR generation** | PARTIAL | `qrcode` dep exists; only the owner→Telegram-bot deep-link QR is generated (`apps/web/src/pages/admin/SettingsPage.tsx:37,118`). **No storefront QR kit** (recommended by trust research T4/T7 but unbuilt). |
| Kiosk / wake-lock / `share_target` | NOT BUILT | no hits repo-wide |

### §0.2 Albania market grounding — with the honesty downgrade

The task framing ("Viber+WhatsApp strong, Telegram weak, Instagram top social-commerce") comes
from earlier drafts. The 2026-07-03 gap analysis **explicitly downgraded** it (R9, LOW-MED):
no dataset breaks out WhatsApp/Viber/Telegram penetration for Albania specifically. What IS
confirmed (DataReportal Digital 2025 Albania): **Facebook and Instagram each reach ~43% of the
population (~1.20M users), Messenger ~31.6%**. Viber dominance is a *regional-pattern inference*
(confirmed #1 in Serbia/Bulgaria/Greece/Belarus at ~90% claimed penetration —
[Infobip, 2026-05-29](https://www.infobip.com/blog/most-popular-messaging-apps-by-country),
[Sinch/GWI, 2025-11-07](https://sinch.com/blog/most-popular-messaging-apps-by-country/); for the
Western Balkans incl. Albania only a 2021 SimilarWeb datapoint via
[Telegrafi](https://telegrafi.com/en/viberi-eshte-aplikacioni-qe-se-shumti-perdoret-ne-kosove-dhe-kryesisht-ne-vendet-e-ballkanit-perendimor/)).
This week's fresh research (§4) again failed to find a 2025-26 Albania-specific Viber number.
**Consequence: channel priority below favors zero/low-cost channels that don't bet engineering
on any single messenger, and keeps the G7 recommendation — survey 10-15 vendors before paying
for any messenger platform.** Competitive context: Wolt entered Tirana 2024-03, incumbent Baboon
has chronic quality complaints; cash is first-class (`2026-07-03-vendor-pain-established-apps.md:202`).

---

## §1 QR + NFC physical layer — "the storefront IS the app"

**How it works.** Table tents / door stickers / packaging stickers / receipt footers carry a QR
of `https://<slug>.dowiz.org` (or `/s/:slug?t=<table>`); NFC NDEF stickers carry the same URL for
tap-to-open. Zero install, zero account, the Astro storefront loads SSR-fast.

**2026 facts (verified).**
- **NFC cost:** NTAG213 PET stickers from an EU seller: **€0.29/pc @100 → €0.15 @16k**
  ([Shop NFC live pricing, fetched 2026-07](https://shopnfc.com/en/nfc-stickers/401-nfc-stickers-ntag213-10x19mm.html));
  China-direct roughly half that (~$0.15-0.18 @1k, 2024 data —
  [DO RFID](https://www.dorfidreader.com/ntag215-vs-ntag213-vs-ntag216-nfc-tag-price-guide-2024/)).
  NTAG213's 144B is plenty for a short URL; 215/216 buy nothing here.
- **iOS tap behavior:** iPhone XS/XR (2018) **and all later models background-read NDEF URL tags
  with no app** — banner notification → tap → Safari
  ([Apple Core NFC docs](https://developer.apple.com/documentation/corenfc/adding-support-for-background-tag-reading),
  [GoToTags](https://gototags.com/help/ios/nfc/reading/background),
  [Shop NFC iPhone matrix](https://shopnfc.com/en/content/20-nfc-iphone)). iPhone 7–X need the
  Control Center reader (iOS 14+). No iOS 26 change to background reading found (only anecdotal
  in-app Core NFC bug reports — [forum thread](https://developer.apple.com/forums/thread/800624)).
- **Android:** NDEF URL tags dispatch straight to the default browser, screen-on/unlocked —
  universal on 2026 handsets ([ref](https://www.andreasjakl.com/nfc-tags-ndef-and-android-with-kotlin/)).
- **QR normalization:** Europe ≈ $770-850M restaurant-QR-ordering market 2024, ~16% CAGR
  (vendor-grade sources: [dataintelo](https://dataintelo.com/report/restaurant-qr-ordering-market),
  [qrcode.co.uk](https://qrcode.co.uk/blog/qr-code-statistics-for-restaurant-usage-in-2024/) —
  directionally credible, exact %s unverified).

**Codebase today:** `qrcode` dep exists; only the TG-bot QR is generated (§0.1). No printable
storefront QR/NFC kit.

- **Build effort:** **S** (current AND rebuild): an owner-side "Printable kit" page — QR PNG/PDF
  (per-table `?t=` variants), A6 tent + sticker layouts. NFC = ops (buy pre-encoded or encode via
  any NFC phone app), zero code.
- **Running cost:** ~€0 software; print ~€5-15/venue; NFC ~€0.20-0.30/table.
- **Friction-to-first-order:** scan → menu in **~3-5s, 0 taps** before browsing (NFC: 1 tap on
  the iOS banner). Lowest friction of any channel for physically present customers.
- **Albania fit:** perfect — dine-in/walk-in heavy, cash-first, and the trust research already
  calls for a "printed QR kit" as a physical artifact of ownership
  (`2026-07-03-trust-and-likeability.md:87,135`). Works for every customer regardless of which
  messenger they use (immune to the R9 uncertainty).
- **Ownership:** 100% vendor-owned surface; no intermediary.
- **Risks:** QR-sticker fraud (overpasted malicious QR) — mitigate with branded tents + the
  domain printed as text; that's also the anti-impersonation anchor
  (`2026-07-03-vendor-pain-established-apps.md:190`).

---

## §2 Google Business Profile / Maps + Apple Business (place-card layer)

**How it works.** The vendor's Maps listing carries the ordering URL where high-intent locals
already look. This is configuration + a playbook, not engineering.

**2026 facts (verified).**
- **"Order with Google" native checkout is DEAD** — discontinued **2024-07-01**; the ordering
  button now redirects out to external ordering sites
  ([BentoBox 2024-06-10](https://www.getbento.com/blog/end-of-order-with-google/),
  [SER](https://www.seroundtable.com/order-with-google-discontinued-37399.html)). The old help
  article (answer/10918858) no longer resolves. Replacement on the partner side is the Actions
  Center **"Ordering Redirect"** deep-link feed
  ([feed spec, live 2026](https://developers.google.com/actions-center/verticals/ordering/redirect/reference/feeds/action-feed));
  legacy end-to-end food feeds are marked Deprecated.
- **Merchants can add their OWN ordering URL** in GBP → "Manage online ordering options" →
  "Add a link", and mark it **preferred for pickup and/or delivery**; third-party links can be
  ordered removed (5-day compliance)
  ([Google help, fetched live 2026-07](https://support.google.com/business/answer/10842217)).
  Link categories incl. menu + food orders; links must be crawlable and actually complete the
  action — Google checks up to daily
  ([answer/6218037](https://support.google.com/business/answer/6218037),
  [policies answer/13769188](https://support.google.com/business/answer/13769188); note: those
  two pages state 10-per-category vs 20-per-type — unreconciled discrepancy). Links are
  **dashboard-only** (not settable via the Business Profile API).
- **Apple:** Apple Business Connect merged into the new free **"Apple Business"** platform
  (announced 2026-03-24, live 2026-04-14, 200+ countries)
  ([Apple Newsroom, fetched live](https://www.apple.com/ne/newsroom/2026/03/introducing-apple-business/)).
  Place cards support **custom action links ("Order Food") pointing at any business URL**
  (Apple approval ≤3 business days; customer needs iOS 17.4+) plus **Showcases** for promos
  ([Apple guide](https://support.apple.com/guide/apple-business-connect/about-actions-abcba5e65376/web),
  [custom action links PDF](https://business.apple.com/partners/assets/custom-action-links.pdf)).

**Codebase today:** Maps *scraping* exists for demo enrichment (venue data memo 2026-07-01);
nothing manages the vendor's own GBP/Apple listing. The claimable-demo motion already produces
the URL to place.

- **Build effort:** **S** — an onboarding checklist step + illustrated Albanian how-to ("vendos
  linkun në Google/Apple Maps"); optionally a concierge step in the white-glove setup visit.
  Nothing to build in the product beyond ensuring `/s/:slug` stays crawlable (SSR shell already
  bot-serving; the Astro rebuild makes this stronger).
- **Running cost:** €0. No partner program needed for the merchant-own-link path.
- **Friction-to-first-order:** Maps → "Order" tap → storefront, ~5-10s. Captures the
  highest-intent searches ("pizza afër meje").
- **Albania fit:** strong — Google Maps is the de-facto local directory; Wolt/Baboon don't own
  the vendor's listing, the vendor does.
- **Ownership:** vendor-owned listing pointing to vendor-owned storefront; the "Preferred by
  business" badge actively demotes marketplace links.
- **Risks:** link-compliance crawls (must complete an order — fine), listing hijack/imposter
  links (monitor via the claim playbook); Apple path needs the vendor enrolled in Apple Business
  (free but an extra account).

---

## §3 Telegram Mini App (wrap the storefront)

**How it works.** A Mini App is literally the storefront URL loaded in Telegram's WebView plus
`telegram-web-app.js`; `t.me/<bot>/<app>?startapp=<slug>` deep-links straight into a tenant.

**2026 facts (verified against Bot API 9.6 docs, updated 2026-04-03).**
- Capabilities: `initData` HMAC auth, `MainButton`/`SecondaryButton`, `themeParams` (CSS vars),
  fullscreen + safe-area insets, `CloudStorage`/`DeviceStorage`/`SecureStorage`,
  `addToHomeScreen()` ([core.telegram.org/bots/webapps](https://core.telegram.org/bots/webapps)).
- **Payments policy — food is exempt from Stars:** "Payments for digital goods and services must
  be carried out exclusively in Telegram Stars… If you're selling physical goods or services,
  you may use different currencies or payment providers"
  ([payments-stars doc, verbatim](https://core.telegram.org/bots/payments-stars)). Physical-goods
  Bot Payments take no Telegram commission ([bots/payments](https://core.telegram.org/bots/payments)).
  Cash-on-delivery / external checkout: allowed **by omission** + the physical-goods carve-out
  (no written prohibition in the [Mini App ToS](https://telegram.org/tos/mini-apps)) — high
  confidence for COD, and our cart-token handoff (§14) never pays in-chat anyway.
- Discovery: built-in Mini App Store since 2024-08
  ([Telegram blog](https://telegram.org/blog/w3-browser-mini-app-store)) + tApps Center
  ([tapps.center](https://tapps.center/)).
- WebView constraints: cookies/localStorage volatile, no reliable `beforeunload`, OAuth popups
  unreliable → authenticate per-launch from `initData`, keep cart server-side (which the
  cart-token model does anyway). Fetch/WS work normally.

**Codebase today:** owner-side bot infra LIVE (`telegram-webhook.ts`, poll worker) — the bot,
token handling and webhook plumbing already exist; only the customer-facing `web_app` surface is
missing. `TG_STOREFRONT_ACTION` flag already reserved (off).

- **Build effort:** **S-M** — register the app on the existing bot, add a thin `tg-entry` route
  that loads `/s/:slug` with a `TelegramAdapter` island (theme vars → tenant palette, MainButton
  → "View cart", initData → optional prefill). On the Astro rebuild this is one adapter island.
- **Running cost:** €0 (Bot API free — already relied on by the social-pipeline research).
- **Friction-to-first-order:** from a shared t.me link inside Telegram: 1 tap → menu (~2-3s),
  no browser context switch.
- **Albania fit:** **honest caveat — weakest AL consumer reach of the big three** (R9: Telegram
  is owner-side today for good reason; no AL consumer-penetration evidence). Build it because
  it's nearly free given existing infra, not because it's the AL growth channel.
- **Ownership:** vendor keeps the customer (orders land in the same system; Telegram is just a
  browser with distribution).
- **Risks:** none material at €0 — 🔴 only if in-chat Telegram Payments were ever added
  (council-gated; not proposed).

---

## §4 Viber for business

**How it works (pattern).** A vendor-branded (or platform-shared) Viber bot: carousel of dishes
or a keyboard menu → every CTA is an `open-url` into `/s/:slug` (cart-token checkout); plus
`viber://chat?number=` deep links for plain human chat (already shipped in checkout, §0.1).

**2026 facts (verified — pricing changed materially).**
- **Chatbots now cost EUR 100/month maintenance PER BOT** (every bot billed on/after
  2024-02-05), plus per-message fees for bot-initiated (out-of-session) messages; user-initiated
  24h sessions are free/unlimited
  ([official FAQ](https://help.viber.com/hc/en-us/articles/15383950711197-Rakuten-Viber-chatbot-commercial-model-FAQ)
  — page 403s to bots, content confirmed via search index +
  [respond.io corroboration, 2025-07-10](https://respond.io/blog/viber-bot-pricing)). **No free
  tier for new bots.** Per-country outbound rates are partner-negotiated (unverifiable publicly).
- **Viber Business Messages** (via CPaaS partners): per-delivered-message + **minimum monthly
  commitment per sender: EUR 115/mo** (EUR 175 for BG/GR/etc.)
  ([Infobip pricing, fetched live 2026](https://www.infobip.com/viber-business/pricing)) —
  reseller terms; Viber publishes no direct rates.
- Deep links: `viber://pa?chatURI=<uri>&context=<ctx>&text=<prefill>` (bot),
  `viber://chat?number=` (person) — [official](https://developers.viber.com/docs/tools/deep-links/).
- Rich media: carousels (up to 6×7 button grid/card), keyboards with `reply`/`open-url` actions
  ([REST Bot API](https://developers.viber.com/docs/api/rest-bot-api/)).
- **AL position:** #1 confirmed in Serbia/Bulgaria/Greece (~90%); for Albania only a 2021
  datapoint + regional inference (§0.2). A stray "18% AL penetration" claim exists but is
  low-quality and contradicts regional evidence — treated as unreliable.

**Codebase today:** `viber://chat?number=` customer→courier deep link LIVE
(`messenger.ts:65-68`); no bot. MAX-ROI item #5 (Viber **owner alerts**) is a separate,
already-planned M-effort build — note it now carries the EUR 100/mo bot fee unless routed
through Business Messages (EUR 115/mo minimum) — **this changes the #5 cost math and argues for
the vendor survey (G7) first.**

- **Build effort:** **M** (bot webhook adapter + carousel/keyboard renderer from the menu model +
  deep-link handoff); the notification-adapter half overlaps with MAX-ROI #5.
- **Running cost:** **EUR 100/mo per bot** (platform-shared bot = one fee amortized across all
  vendors — the only sane structure at dowiz's scale) or EUR 115/mo minimum via BM partner.
- **Friction-to-first-order:** chat tap → carousel → open-url → checkout, ~4 taps/15s; or
  zero-build: customer taps the existing storefront "chat on Viber" link and orders by text
  (human-mediated, no fee at all on a personal/free business account).
- **Albania fit:** likely the strongest messenger *if* the regional pattern holds — but that's
  exactly the LOW-MED claim (§0.2). The EUR 100/mo floor makes "verify before build" mandatory
  for a 0%-commission product: at typical SaaS pricing that's several vendors' worth of revenue.
- **Ownership:** vendor owns the customer (bot hands off to vendor's storefront; contact stays
  in the order record).
- **Risks:** pricing regime can move again (it did in 2024); bot fee is a real recurring cost;
  🔴 any Viber in-chat payment — not proposed, council-gated.

---

## §5 WhatsApp Business Platform

**How it works — two very different tiers.**
(a) **Free tier (Business App):** the vendor runs the free WhatsApp Business App; customers reach
it via `wa.me/<number>?text=<prefill>` (official:
[faq 5913398998672934](https://faq.whatsapp.com/5913398998672934/)); the app's built-in
**catalog (up to 500 products, free)** supports customer **carts + a single order message**
([About Cart](https://faq.whatsapp.com/1184376605821468),
[place an order using cart](https://faq.whatsapp.com/5495780857144067) — pages live, content
JS-gated; one blog claims otherwise but conflicts with WhatsApp's own Help Center). No native
payments outside IN/BR; no order dashboard — orders are just chat messages.
(b) **Platform (API via BSP):** programmatic messaging, templates, **Flows** (in-chat
multi-screen forms — API-only, explicitly usable for COD order intake, no extra Meta fee beyond
the carrying template; [whautomate](https://whautomate.com/whatsapp-flows)).

**2026 pricing (verified).** **Per-message replaced per-conversation on 2025-07-01**; service
(user-initiated) conversations are **free without cap since 2024-11-01**; utility templates
inside an open 24h window are free; **Click-to-WhatsApp ads open a 72h fully-free window**
([Meta pricing doc, fetched live](https://developers.facebook.com/documentation/business-messaging/whatsapp/pricing)).
Albania sits in the **"Rest of Central & Eastern Europe"** band (inferred — not on the 2026-07-01
standalone-market list): **marketing $0.0989, utility/auth $0.02438 per delivered message**
(two agreeing BSP mirrors of the 2026-07-01 rate card:
[SleekFlow](https://help.sleekflow.io/en_US/whatsapp/pricing),
[Gallabox](https://docs.gallabox.com/pricing-and-billing-modules/new-per-message-pricing) — raw
Meta CSV is JS-gated, flagged). BSPs: **360dialog from €49/mo, no per-message markup**
([pricing](https://360dialog.com/pricing)); Twilio adds $0.005/msg both directions.

**Codebase today:** `wa.me` customer→courier deep link LIVE (`messenger.ts:61-64`). A WhatsApp
**owner-notification** channel existed and was **deliberately removed** for privacy/ToS
(migration `1790000000043`) — any API re-entry must re-litigate that decision.

- **Build effort:** free-tier playbook **S** (onboarding doc: set the storefront link + catalog
  items pointing at `/s/:slug` URLs; our checkout already emits wa.me links). Platform/Flows
  **M-L** (BSP account per… actually per-WABA, webhook adapter, template approval cycle).
- **Running cost:** free tier €0; Platform ≥€49/mo (BSP) + per-message; inbound-led ordering
  stays near-free (service window).
- **Friction-to-first-order:** wa.me tap → chat → storefront link tap → checkout (~3 taps).
  With Flows: order fully in-chat, but that **moves order intake off the single money surface —
  under §14 Flows should at most collect intent and still hand off a cart token.**
- **Albania fit:** WhatsApp is one of the two presumed-dominant messengers (§0.2, LOW-MED);
  SPAR AL treated it as a normal ordering channel as far back as 2020. The free tier fits
  0%-commission economics perfectly; the API tier does not (yet).
- **Ownership:** vendor's own number, vendor's own chat thread — strong; contacts stay with the
  vendor.
- **Risks:** ToS history (we removed the channel once); per-message price bands move quarterly
  ([updates page](https://developers.facebook.com/docs/whatsapp/pricing/updates-to-pricing/));
  Dec 2025 EU probe into Meta's API policy re AI chatbots
  ([Register](https://www.theregister.com/2025/12/04/eu_probes_meta_whatsapp_ai/)).
  🔴 WhatsApp payments — not available in EU anyway, and council-gated.

---

## §6 Instagram / Facebook

**How it works.** IG is a discovery surface, not a checkout: bio links, story link stickers,
DM deep links, and (paid) click-to-WhatsApp ads all route to `/s/:slug`.

**2026 facts (verified).**
- **Story link stickers: all accounts** (since 2021-10,
  [official](https://about.instagram.com/blog/announcements/expanding-sharing-links-in-stories-to-everyone));
  **bio holds up to 5 links** (since 2023-04,
  [SMT](https://www.socialmediatoday.com/news/instagram-adds-capacity-to-display-5-links-in-profile-bio/647995/)).
- **`ig.me/m/<username>` DM deep links are officially supported** (optional `?ref=` up to 2,083
  chars; requires Icebreakers for new-conversation referrals; **not supported on Instagram Web**)
  ([Meta doc, fetched live](https://developers.facebook.com/documentation/business-messaging/instagram-messaging/features/ig-me-links)).
- **Meta Shops are NOT available in Albania** (EU list cut to 11 countries in 2023-08;
  [Glowmetrics](https://glowmetrics.com/blog/everything-you-need-to-know-about-changes-to-meta-shops-june-2023/)),
  and **native in-app checkout was killed worldwide by 2025-08** — all Shops are
  website-checkout-only now
  ([ppc.land](https://ppc.land/meta-phases-out-facebook-and-instagram-shops-checkout-by-august-2025/)).
  So "send them to your own site" is not a workaround — it's the ONLY Meta commerce model left,
  which suits us exactly.
- DM automation (Instagram Messaging API): professional accounts only, strict 24h response
  window (+human-agent tag ~7d, secondary-source figure), ~200 automated DMs/hr; legacy message
  tags error since 2026-04-27
  ([Meta doc](https://developers.facebook.com/docs/instagram-platform/instagram-api-with-instagram-login/messaging-api/),
  [creatorflow](https://creatorflow.so/blog/instagram-dm-compliance-meta-rules/)).
- Click-to-WhatsApp ads from IG/FB open the **72h free WhatsApp window** (§5) — the paid-growth
  bridge between the two Meta surfaces.

**Codebase today:** IG exists only as a static profile footer link
(`location_themes.social_instagram`); **not a checkout messenger kind** — gap **G9**, already
prioritized as MAX-ROI item #4 (S-M).

- **Build effort:** **S-M** — add `instagram` as the 7th `MessengerKind` (`ig.me/m/` link,
  username input — the pattern in `messenger.ts` extends trivially), plus an owner "story-share
  kit" (pre-sized story image of the menu + link sticker how-to). The Sunday social pipeline
  (R3) already covers automated posting.
- **Running cost:** €0 organic; CTWA ads optional paid growth.
- **Friction-to-first-order:** story → link sticker tap → storefront (~2 taps, in-app browser).
- **Albania fit:** **the only social channel with CONFIRMED ~43% population reach** (§0.2) —
  highest-confidence social bet available.
- **Ownership:** IG owns discovery, but every tap lands on the vendor's storefront and every
  order lands in the vendor's system — acceptable split.
- **Risks:** in-app browser quirks (test checkout in IG's WebView); DM automation policy churn —
  keep DMs human or simple-replies-only initially.

---

## §7 Embeddable widget + custom domains (white-label)

**How it works.** Two complements: (a) a JS embed vendors paste into any existing site/CMS —
already built; (b) the storefront served on the vendor's own hostname.

**Codebase today (mostly built):**
- Widget: `<script src=".../widget.js" data-slug="..." integrity="..." crossorigin>` with SRI +
  iframe auto-resize (`apps/api/src/client/widget/loader.ts`, `public/dist/embed-helper.js`) —
  needs only docs + an owner-facing "copy this snippet" panel.
- Subdomain white-label `<slug>.dowiz.org` LIVE (`subdomain-rewrite.ts`).
- Custom vendor domains (`order.vendor.al`): NOT built — rewrite is `dowiz.org`-only.

**2026 facts (verified).** Fly.io managed certs: **$0.10/mo per hostname (first 10 free),
wildcard $1/mo**, Let's Encrypt-backed; dedicated IPv4 $2/mo (shared free); LE rate limits are
the practical ceiling ([Fly pricing, fetched live](https://fly.io/docs/about/pricing/),
[custom domains doc](https://fly.io/docs/networking/custom-domain/)).

- **Build effort:** widget **S** (docs/panel only); custom domains **M** — hostname→slug DB
  lookup replacing the hard-coded suffix check, `fly certs add` automation on vendor DNS
  verification, cert-status surfacing in admin. On the rebuild, host-based tenant resolution is
  a natural axum middleware.
- **Running cost:** ~**$1.20/vendor-domain/year** after the first 10 free — negligible.
- **Friction-to-first-order:** identical to web (it IS the web channel, relocated).
- **Albania fit:** vendors with existing sites are rare among small venues, but hotels/chains
  have them; the custom domain is a strong sales artifact ("porosit.emri-juaj.al" on the menu).
- **Ownership:** the **maximum** of any channel — vendor's own domain survives even a dowiz
  brand change; anti-impersonation ground truth (`vendor-pain` doc:190).
- **Risks:** widget iframe + third-party-cookie constraints → keep checkout as a top-level
  navigation from the widget, not inside the iframe (the built widget's overlay/button pattern
  already leans this way); cert issuance failures need admin visibility.

---

## §8 PWA hardening as distribution

**How it works.** The storefront on the customer's home screen with the **vendor's** name/icon —
the repeat-order surface. Not an acquisition channel; a retention channel.

**2026 facts (verified).**
- **iOS 26: every site added to the home screen now opens as a web app by default** (manifest
  optional; per-site toggle)
  ([WebKit WWDC25 post](https://webkit.org/blog/16993/news-from-wwdc25-web-technology-coming-this-fall-in-safari-26-beta/)).
- EU DMA drama resolved 2024-03-01: Apple reversed the iOS 17.4 kill; home-screen web apps
  remain WebKit-only everywhere
  ([Apple DMA page](https://developer.apple.com/support/dma-and-apps-in-the-eu/)). As of
  mid-2026 **no alternative browser engine has actually shipped on iOS**
  ([OWA](https://open-web-advocacy.org/blog/apples-browser-engine-ban-persists-even-under-the-dma/)).
- **iOS Web Push is still home-screen-install-gated** (unchanged since 16.4); Declarative Web
  Push (Safari 18.4) does NOT bring push to plain iPhone Safari tabs
  ([WebKit](https://webkit.org/blog/16535/meet-declarative-web-push/)). Badging works on
  installed web apps. No `beforeinstallprompt` on iOS — our `InstallPrompt.tsx` iOS-instructions
  fallback remains the correct pattern.
- Android: WebAPK + Richer Install UI live; **Web Install API (`navigator.install`) in origin
  trial (Chrome/Edge 148-153)** — watch, don't depend
  ([InfoQ 2026-03](https://www.infoq.com/news/2026/03/web-install-api-origin-trial/)).

**Codebase today:** manifest is **global and single-tenant-flavored** (§0.1 — name
"DeliveryOS", hardcoded `/s/dubin-sushi` shortcut): a customer installing from `/s/artepasta`
gets a DeliveryOS-branded app whose start_url is `/`. `InstallPrompt.tsx` built. **SW build
discrepancy (§0.1) must be resolved before trusting customer push display.**

- **Build effort:** **S-M** — per-tenant dynamic manifest (`/s/:slug/manifest.webmanifest`:
  tenant name, derived-palette theme color, tenant icon from R2, `start_url:/s/:slug`,
  `scope:/s/:slug`) is a small dynamic route; trivially natural in the Astro rebuild. Fix the
  sw.js build pipeline. Offline: **cache menu browse only — NEVER offline order-queueing**
  (money red-line: an "accepted" order without server truth is a lie; current sw.js cache-first
  shell already stops at `/api/`).
- **Running cost:** €0.
- **Friction:** install is 2-3 taps (Android prompt / iOS share-sheet); afterwards reorder is
  1 tap from home screen — the cheapest legal replacement for "our app" without stores.
- **Albania fit:** works on every phone; push (order status) already has the full server
  vertical built (§0.1).
- **Ownership:** total; the icon on the home screen is the vendor's.
- **Risks:** iOS install friction remains real (share-sheet only); don't oversell push reach on
  iOS.

---

## §9 Zero-install native-lite (App Clips / Play Instant) — verify-then-defer

**2026 facts (verified).**
- **Google Play Instant is DEAD**: "Starting December 2025, Instant Apps cannot be published
  through Google Play, and all Google Play services Instant APIs will no longer work"
  ([Android docs, fetched live](https://developer.android.com/topic/google-play-instant)).
- **Apple App Clips remain fully supported** (15 MB limit with physical invocations, 50 MB
  digital-only; QR/NFC/App Clip Codes/links invocation) — but a Clip is a target **inside a full
  App Store app**: it requires the main app, App Review on every release, and the $99/yr
  developer program
  ([Apple docs](https://developer.apple.com/documentation/appclip/choosing-the-right-functionality-for-your-app-clip),
  [App Store Connect guide](https://developer.apple.com/help/app-store-connect/offer-app-clip-experiences/overview-of-app-clips/)).
  Realistic cost: a multi-week native SwiftUI build + permanent review cadence (no authoritative
  price exists — flagged).

**Verdict: DEFER (honest pricing done).** The Android half no longer exists; the Apple half
violates the spirit of the no-native constraint (it IS an App Store app) and duplicates what QR →
instant Astro storefront already achieves in <5s. Revisit only if iOS conversion data someday
shows the browser hop itself is the bottleneck.

---

## §10 Kiosk / counter mode

**How it works.** A cheap Android tablet at the counter, fullscreen storefront in a kiosk shell:
walk-ins self-order; staff key phone orders into the same system (the multi-homing mitigation
from `vendor-pain` doc:174).

**2026 facts (verified).** Screen Wake Lock API supported in all major browsers (>94% global;
iOS home-screen bug fixed in 18.4) ([caniuse](https://caniuse.com/wake-lock),
[web.dev](https://web.dev/blog/screen-wake-lock-supported-in-all-browsers)). No standard "kiosk
API" exists; the practical stack is **Fully Kiosk Browser — €7.90 one-time per device**
(fleet cloud ~€1.18/device/mo; [license](https://license.fully-kiosk.com/license/single)) or
OSS [FreeKiosk](https://github.com/RushB-fr/freekiosk), + lock-task mode.

- **Build effort:** **S** — a `?kiosk=1` storefront mode: wake lock, cart auto-reset after
  order/timeout, bigger touch targets, suppress external links + messenger deep-links, "pay at
  counter" as the cash path (same checkout, same money surface).
- **Running cost:** ~€80-120 tablet + €7.90 once; €0 software.
- **Friction:** zero for the customer (it's already open on the menu).
- **Albania fit:** counter/walk-in culture + cash-at-counter is the native fit; also the
  "phone-order replacement" wedge.
- **Ownership:** fully vendor-owned hardware surface.
- **Risks:** unattended-device abuse (kiosk mode must not expose admin routes — serve only
  `/s/:slug`); device theft is the vendor's problem, not a data problem (no secrets on device).

---

## §11 Other current channels

- **RCS Business Messaging — WATCH, do not build.** iPhones in Albania are **unreachable via
  RCS**: neither Vodafone AL nor ONE ships the carrier bundle
  ([Apple carrier page](https://support.apple.com/en-al/108048),
  [foxt.dev tracker, 2025-12-12](https://foxt.dev/ios-rcs/)). Android users are reachable via
  Google Jibe, and aggregator coverage lists include Albania
  ([OpenMarket](https://www.openmarket.com/docs/Content/rcs-global-coverage.htm)), but there's
  no public AL rate card (aggregator-quoted) and any campaign needs SMS fallback. Re-check the
  tracker quarterly; carriers can flip the bundle at any time.
- **WhatsApp DMA interop** (third-party chats live in Europe since 2025-11, BirdyChat/Haiket
  first — [Meta](https://about.fb.com/news/2025/11/messaging-interoperability-whatsapp-enables-third-party-chats-for-users-in-europe/))
  — consumer-side only today; no business-messaging surface yet. Watch.
- **Web Share / share_target — S, nice-to-have.** A "Share this menu" button (Web Share API) on
  the storefront turns every customer into a distributor into whatever messenger they actually
  use — sidesteps the R9 uncertainty entirely. `share_target` (receiving shares into the
  installed PWA) is marginal; skip.
- **Shortlink/QR-hub — S, measurement layer.** Shlink is already in the approved social-pipeline
  plan (R3). Give every physical/social placement a distinct short URL
  (`dwz.al/<slug>/<table|tent|story|gbp>`) so channel performance is measurable — this is what
  makes the tier rankings falsifiable later.

## §12 Agentic commerce heads (2026) — mostly watch, two cheap actions

**The 2026 whiplash, verified:**
- **ChatGPT Instant Checkout is DISCONTINUED.** Launched 2025-09-29 (Etsy, then Shopify);
  OpenAI pulled native in-chat checkout ~2026-03-06 and retreated to **product discovery +
  link-out to the merchant's own checkout**
  ([Forrester 2026-03-07](https://www.forrester.com/blogs/what-it-means-that-the-leader-in-agentic-commerce-just-pulled-back/),
  [Modern Retail 2026-03-27](https://www.modernretail.co/technology/what-went-wrong-with-chatgpts-instant-checkout/)
  — Walmart cited in-chat conversion 3x lower; "<15 Shopify stores ever live" is
  secondary-source only, flagged). The 4% Shopify fee
  ([PYMNTS](https://www.pymnts.com/news/ecommerce/2026/shopify-merchants-to-pay-4percent-fee-on-sales-made-through-chatgpt-checkout/))
  is moot for native checkout. The **ACP spec itself is alive** (v2026-04-17 adds an MCP
  binding; Apache-2.0, OpenAI+Stripe maintainers —
  [repo](https://github.com/agentic-commerce-protocol/agentic-commerce-protocol)), but its
  Delegated Payment Spec is **literally card-only** (`payment_method.type: "card"` —
  [spec](https://developers.openai.com/commerce/specs/payment)) — structurally incompatible
  with cash-on-delivery today. Restaurant brands (Little Caesars, Starbucks, DoorDash apps in
  ChatGPT) all use **discovery → hand off to own checkout**
  ([NRN](https://www.nrn.com/quick-service/little-caesars-has-launched-an-app-in-chatgpt-to-help-with-orders)) —
  i.e. the industry converged on our §14 model.
- **Google is the momentum leader:** **UCP (Universal Commerce Protocol)** announced 2026-01-11
  with Shopify/Walmart/Target/Visa/MC/Stripe
  ([Google](https://blog.google/products/ads-commerce/agentic-commerce-ai-tools-protocol-retailers-platforms/));
  open source, discovery via **`/.well-known/ucp` manifest**, transports incl. **MCP**,
  AP2-compatible, merchant-of-record stays the merchant
  ([technical post](https://developers.googleblog.com/under-the-hood-universal-commerce-protocol-ucp/)).
  Checkout early-access is **US/CA/AU-only, Google-Wallet-card-based**
  ([Merchant help](https://support.google.com/merchants/answer/16837055)); at I/O 2026 Google
  announced Universal Cart with **"local food delivery" as an upcoming vertical**
  ([blog](https://blog.google/products-and-platforms/products/shopping/google-shopping-cart/)) —
  the single strongest watch-signal for this platform. **AP2** was donated to the **FIDO
  Alliance** (~2026-04-28, v0.2) — payments-network plumbing, nothing for a small merchant
  platform to integrate directly
  ([Google](https://blog.google/products-and-platforms/platforms/google-pay/agent-payments-protocol-fido-alliance/)).
  Microsoft Copilot Checkout (2026-01-08) and Perplexity's merchant program are US-only /
  gated — watch.
- **MCP is the quiet real layer:** every Shopify store now exposes a **Storefront MCP server**
  (`/api/mcp`: search_catalog, get_product, get_cart, update_cart) whose checkout **hands off
  to a web checkout URL**
  ([shopify.dev, fetched live](https://shopify.dev/docs/apps/build/storefront-mcp/servers/storefront)).
  No gatekeeper, no application, no fee, no card-rail requirement.
- **Stripe does NOT support Albania** (verified on [stripe.com/global](https://stripe.com/global),
  fetched 2026-07-04) — forecloses the Stripe-token ACP path entirely, independent of policy.
- Passive layer: Google still consumes `menu`/`servesCuisine`/hours on LocalBusiness markup
  (no Menu rich result —
  [docs](https://developers.google.com/search/docs/appearance/structured-data/local-business));
  `llms.txt` ~10% adoption, Google ignores it (single-study figure, flagged).

**Actionable NOW (both fit §14 exactly):**
1. **schema.org `Restaurant` + `hasMenu`/`MenuSection`/`MenuItem` JSON-LD** on the SSR
   storefront — S on the Astro rebuild (the menu is already server-rendered); feeds Google
   today and every agent crawler tomorrow. €0.
2. **A read-only menu/cart MCP server + `/.well-known/ucp` manifest stub** — S-M; mirrors
   Shopify's shape; the cart tool mints the same §14 cart token and returns the checkout URL.
   The cheap hedge that makes dowiz agent-visible without touching money. €0.

**Watch-only:** ACP/UCP checkout programs (US-centric, card-rails, gated), AP2, Copilot/
Perplexity. 🔴 Any delegated-payment integration is council-gated — and currently impossible
anyway (no Stripe in AL).

## §13 Channel-hub reference architectures ("one core, N channels")

All verified against live docs 2026-07-04.

### §13.1 Shopify — channel as app + publication + per-channel credential
A sales channel is an app; per-channel catalog exposure is a **`Publication`** ("a group of
products and collections that are published to an app", with `autoPublish` and a `catalog` for
channel-scoped pricing/availability); `Product`/`ProductVariant`/`Collection` implement
`Publishable` (`publishablePublish`/`publishableUnpublish`). Headless storefronts are just
another channel, each auto-provisioned a **public (publishable) + private token pair**, max 100
per shop. ([sales channels](https://shopify.dev/docs/apps/build/sales-channels),
[Publication](https://shopify.dev/docs/api/admin-graphql/latest/objects/Publication),
[headless tokens](https://shopify.dev/docs/storefronts/headless/building-with-the-storefront-api/getting-started)).
**Pattern:** one product core; channel = entity + catalog subset + credential.

### §13.2 Medusa — the cleanest OSS data model (with our own caveat)
([sales-channel module](https://docs.medusajs.com/resources/commerce-modules/sales-channel),
[links](https://docs.medusajs.com/resources/commerce-modules/sales-channel/links-to-other-modules),
[publishable keys](https://docs.medusajs.com/resources/commerce-modules/sales-channel/publishable-api-keys)):
- `SalesChannel` entity ("an online or offline channel you sell products on");
- **Product ↔ SalesChannel M2M** (per-channel availability); **StockLocation ↔ SalesChannel M2M**;
- **ApiKey ↔ SalesChannel M2M** — the storefront sends `x-publishable-api-key`, the backend
  infers allowed channels and filters everything;
- **Cart → has-one SalesChannel; Order → has-one SalesChannel** (every order carries its
  originating channel).

Our existing teardown already vetted this module and rejected its *tenancy* use: Medusa's
`sales_channel_id` is a soft app-level filter, DeliveryOS RLS `location_id` is stronger
(`docs/research/repo-1-medusa.md:87-98,226,247` — "Sales-channel / store tenancy model —
REWRITE/REJECT"). **The borrow is the channel SHAPE (channel entity + availability scoping +
channel-stamped cart/order + channel-scoped public credential), layered UNDER RLS tenancy, not
instead of it.**

### §13.3 Food-vertical hubs — the channel families and the adapter contract
- **Deliverect** (quote-only pricing) defines the best-documented **channel-adapter interface**
  ([docs](https://developers.deliverect.com/docs/building-a-channel-integration-overview)): a
  channel = an ordering surface with a `channelLinkId`, and an adapter must implement
  **(1) registration, (2) menu-push receipt incl. availabilities, (3) normalized order POST,
  (4) order-status webhook back, (5) product snooze/unsnooze, (6) busy mode** (+ optional
  prep-time/courier-tracking). Channel families: marketplaces, direct web/social, kiosk/QR,
  dispatch.
- **UrbanPiper** (40k+ restaurants claimed; Swiggy/Zomato are strategic investors) splits into
  Hub (marketplace aggregation), Meraki (direct ordering), **Orderline AI (phone/AI-voice as an
  explicit channel family)**, POS ([urbanpiper.com](https://www.urbanpiper.com/)).
- **Otter** (CloudKitchens): one POS, "update menus once — changes apply everywhere",
  **per-channel price overrides** ([tryotter.com](https://www.tryotter.com/); independent 2024
  figure: ~18% of US delivery orders — [Contrary](https://research.contrary.com/company/cloudkitchens)).

**Converged channel families across the vertical:** marketplace · direct web/app · kiosk+QR ·
social/messaging · phone/AI-voice · in-store POS. **Converged architecture:** one canonical
menu + per-channel overrides (price/availability/snooze) → menu push out per channel →
normalized order ingestion in → status webhooks back.

### §13.4 CPaaS transport normalization (Balkan-relevant)
- **Infobip** (Croatian): **Messages API = one endpoint over 10 channels** — SMS, MMS, WhatsApp,
  RCS, **Viber Business Messages, Viber Bots**, Apple Messages, Instagram DM, LINE, Messenger —
  with automatic cross-channel failover; **Telegram only via Answers/Conversations, NOT the
  Messages API** ([docs](https://www.infobip.com/docs/messages-api),
  [telegram](https://www.infobip.com/docs/telegram)). Pay-as-you-go + **Viber sender minimums
  (€115/€175 per month)** ([pricing](https://www.infobip.com/pricing)). The only normalizer with
  first-class Viber — its concrete Balkan edge.
- **360dialog**: WhatsApp-only BSP, **€49/number/month, zero markup on Meta fees**
  ([pricing](https://360dialog.com/pricing)).
- **Twilio Content API**: one template → WhatsApp/RCS/Messenger/SMS/MMS with automatic
  degradation ([docs](https://www.twilio.com/docs/content/overview)) — **no Viber**, a real gap
  for the Balkans.

### §13.5 Verdict — what maps onto the Rust core + cart-token design
**Medusa's data model + Deliverect's adapter contract, with dowiz's own twist that no channel
ever gets a checkout.** Concretely, for the rebuild:
1. A `channels` registry per location (kind: `web|subdomain|custom_domain|widget|qr|nfc|gbp|
   apple|telegram|viber|whatsapp|instagram|kiosk|agent`, config, enabled) — Medusa's
   `SalesChannel`, but under RLS `location_id` (per our teardown verdict).
2. **Orders stamped with `channel`** (Medusa's Order→SalesChannel) — this is what makes tier
   rankings falsifiable (§11 shortlinks feed the same attribution).
3. Channel adapters implement a Deliverect-shaped contract *minus order intake*: menu-render
   out (carousel/catalog/structured data), availability/snooze push, status notifications back —
   but the order itself ALWAYS arrives through the one web checkout (§14).
4. Per-channel menu overrides: defer — YAGNI at current scale; the schema slot (`channels.config`)
   is enough for now.
5. CPaaS: **do not adopt yet.** Infobip normalization + Viber minimums only pay off once ≥2 paid
   messenger channels are live and the G7 vendor survey has picked them (§0.2). Direct free APIs
   (Telegram Bot API; WhatsApp free tier) first.

---

## §14 The unifying constraint: signed cart-token handoff (single money surface)

**Rule (operator-set, standing):** every conversational/social/agentic channel terminates in a
**signed cart-token handoff into the ONE web checkout**. No channel-native payment, no
channel-native order intake. 🔴 Any in-chat payment (Telegram Bot Payments, WhatsApp Pay, ACP
delegated payments) is council-gated and excluded from all tiers below.

**Why this is the load-bearing simplification:**
- **One money surface** = one place where price/tax/idempotency invariants are enforced (integer
  money, RS256, RLS FORCE — the red-line set), one Playwright-provable checkout, one
  **fiskalizimi** integration point (gap G1 — the AL legal blocker attaches to the sale; N
  channel checkouts would mean N fiscalization surfaces).
- **Channel adapters become read-only + intent-only**: render menu, collect a dish list, mint a
  token. They can be built/discarded cheaply (Deliverect-shaped contract, §13.5) without ever
  touching money paths — keeps every future channel out of the red-line review radius.
- **It matches the 2026 platform reality anyway:** Meta killed native checkout (§6), Order with
  Google is dead (§2), Telegram exempts physical goods from Stars (§3) — the industry converged
  on "hand off to the merchant's web checkout."

**Mechanics (design sketch, for the rebuild council — not built here):**
- Cart stays **server-side** (already true); the token is an opaque signed reference
  (HMAC/short JWT: `cart_id`, `location_id`, `channel`, `exp` ≤ 30min, no PII, single-claim).
- Handoff URL: `/s/:slug/checkout?ct=<token>` — checkout hydrates the cart, stamps the order's
  `channel`, proceeds exactly as today.
- Side benefit: this neutralizes the Telegram-WebView volatile-cookie problem (§3) and any
  in-app-browser storage weirdness (IG/FB WebViews, §6) — state never lives in the WebView.
- Pure-link channels (QR/NFC/GBP/Apple/bio links) need no token at all — they enter at the menu;
  tokens exist only where a channel pre-builds intent (bot carousels, DM flows, agents).

## §15 Ranked adoption sequence (cleaner + faster + simpler, under the §14 constraint)

All tiers assume: channels are wrappers around `/s/:slug`; conversational/agentic channels end
in a cart-token handoff; no channel gets a payment surface (🔴 council otherwise).

### Tier 1 — ship with the rebuild (all ~€0 running cost, S each; days not weeks)
| # | Channel | Effort | Cost | Why this tier |
|---|---|---|---|---|
| 1 | **QR + NFC printable kit** (§1) | S | print €5-15/venue; NFC ~€0.20/tag | Lowest friction of all (scan→menu ~3s), physically present customers, immune to the R9 messenger uncertainty; already demanded by trust research T4/T7. |
| 2 | **GBP food-ordering link + Apple Business order action** (§2) | S (playbook + concierge step) | €0 | Highest-intent placement that exists; Order-with-Google's death made the merchant-own-link the official path; nothing to build. |
| 3 | **Instagram surface pack** — 7th `MessengerKind` (`ig.me`), story-share kit, bio links (§6) | S-M | €0 | The ONLY social channel with confirmed ~43% AL reach; Meta itself now mandates website checkout — our model is the only model. Closes gap G9 / MAX-ROI #4. |
| 4 | **WhatsApp free-tier playbook** — wa.me links (built) + vendor catalog→storefront (§5) | S | €0 | Zero-cost presence on a presumed-dominant messenger without betting engineering on the unverified claim. |
| 5 | **Shortlink/QR-hub + `channel` stamp on orders** (§11, §13.5) | S | ~€0 (Shlink already planned) | The measurement layer that makes every other ranking falsifiable — attribution before investment. |

### Tier 2 — next quarter (S-M, near-zero cost; the "own surface" deepeners)
| # | Channel | Effort | Cost | Why this tier |
|---|---|---|---|---|
| 6 | **Per-tenant PWA manifest + SW-pipeline fix** (§8) | S-M | €0 | Turns the storefront into "the vendor's app" on home screens (iOS 26 made every home-screen site a web app); push vertical already built server-side; retention, not acquisition — hence T2. |
| 7 | **schema.org Menu JSON-LD + read-only MCP/`.well-known/ucp` stub** (§12) | S-M | €0 | The agent-visibility hedge while UCP's "local food delivery" vertical matures; passive, no gatekeeper, no money surface. |
| 8 | **Telegram Mini App wrap** (§3) | S-M | €0 | Nearly free on the LIVE bot infra + t.me deep links; honest AL-consumer caveat keeps it out of T1 — build because cheap, not because growth. |
| 9 | **Custom domains white-label** (§7) | M | ~$1.20/domain/yr | Deepest ownership artifact (`porosit.vendor.al`); subdomain rewrite is 80% of the code; needs cert automation care. |
| 10 | **Widget productization** (§7) | S | €0 | Already built (loader + SRI + resize); only docs + an admin copy-snippet panel remain. |
| 11 | **Kiosk/counter mode** (§10) | S | ~€90-130/venue once | Walk-in + phone-order replacement wedge; wake-lock is universal now; cash-at-counter stays on the one checkout. |
| 12 | **Web Share button** (§11) | S | €0 | Customers distribute into whichever messenger they actually use — sidesteps R9 entirely. |

### Tier 3 — gated, watch, or dead
| # | Channel | Effort | Cost | Status + why |
|---|---|---|---|---|
| 13 | **Viber bot** (§4) | M | **€100/mo/bot** (or €115/mo BM minimum) | 🚧 GATED on the G7 10-15-vendor survey: real recurring cost vs LOW-MED regional-inference evidence. If the survey confirms Viber, a single platform-shared bot amortizes the fee. |
| 14 | **WhatsApp Business Platform / Flows** (§5) | M-L | ≥€49/mo BSP + per-message | Gated on order volume AND re-litigating the deliberate ToS-driven removal; Flows collect intent only — checkout stays ours. |
| 15 | **UCP / ACP checkout programs** (§12) | — | — | WATCH. US-centric, card-rails-only, Stripe absent in AL; but Google named "local food delivery" the next Universal Cart vertical — re-check quarterly. 🔴 any delegated-payment join is council-gated. |
| 16 | **RCS Business Messaging** (§11) | — | — | WATCH. AL iPhones unreachable (no carrier bundle at Vodafone AL / ONE); Android-only reach + no public AL rates = not viable yet. |
| 17 | **CPaaS normalization (Infobip)** (§13.4) | M | minimums per channel | Only after ≥2 paid messenger channels exist; premature abstraction today. |
| 18 | **Apple App Clips** (§9) | L (native) | $99/yr + native build + review cadence | DEFER — it IS an App Store app; duplicates what QR→instant storefront already does. |
| 19 | **Google Play Instant** (§9) | — | — | DEAD (publishing ended Dec 2025). |

**The through-line:** Tier 1 is pure distribution of one URL (zero new money surfaces, zero
recurring platform fees — matching 0%-commission economics); Tier 2 deepens ownership of the
same URL; Tier 3 is everything with a recurring fee, an unverified market assumption, or a
card-rail dependency — each behind its named gate (G7 survey, volume, council, carrier/vertical
launch).

---

## Appendix — verification gaps (honest residue)

1. **Albania-specific messenger penetration (2025-26)** — still unverifiable (fresh search this
   week found nothing newer than a 2021 SimilarWeb datapoint); the G7 vendor survey remains the
   only honest resolver.
2. **WhatsApp rate-band membership for Albania** — RoCEE band inferred from absence on the
   standalone-market list; exact per-message rates taken from two agreeing BSP mirrors, not the
   (JS-gated) Meta CSV.
3. **Viber per-country outbound rates** — partner-negotiated, not public; EUR 100/mo maintenance
   is the verified planning floor.
4. **WhatsApp Business App cart/order-message capability** — WhatsApp Help Center pages are
   JS-gated to fetchers; capability confirmed via the pages' indexed content, one blog dissents.
5. **GBP link caps** — Google's own two help pages disagree (10/category vs 20/type).
6. **ACP/Instant-Checkout merchant counts and post-pullback fee** — secondary sources only;
   chatgpt.com/merchants 403s.
7. **App Clip realistic build price** — no authoritative figure exists; "multi-week native
   build + $99/yr" is the honest floor.
8. **sw.js build discrepancy** (§0.1) — a codebase fact to re-verify in the build pipeline, not
   a web claim.
