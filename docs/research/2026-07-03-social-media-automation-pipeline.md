# Automated Social-Media Content + Distribution Pipeline for dowiz/DeliveryOS

**Date:** 2026-07-03 · **Author:** marketing-ops/tooling research agent
**Goal:** grow product visibility for dowiz (vendor-owned, 0%-commission ordering/delivery OS; Albania-first; AGPL open-source; budget-conscious solo operator) with an automated pipeline built — where sane — from self-hostable, open-source tools the project can own.

> Pricing noted as of **July 2026**. Social-tool pricing churns quarterly — re-verify before committing annual plans.

---

## 0. Constraints that shape every choice

1. **Albania-first audience** → Instagram, TikTok, Facebook dominate; Telegram/WhatsApp for community; X and LinkedIn are marginal locally (LinkedIn matters only for the open-source/dev audience). Prioritize IG Reels + TikTok + FB, with Telegram as the owned channel.
2. **Platform API reality (the hard part of "automation"):**
   - **Instagram**: only the official Graph API is ToS-legal for automated publishing. Requires IG **Business/Creator** account + linked FB Page + a Meta developer app with `instagram_business_content_publish` **approved via app review**. Hard cap ~25–100 API-published posts / rolling 24 h per account ([Meta docs](https://developers.facebook.com/docs/instagram-platform/instagram-graph-api/reference/ig-user/content_publishing_limit/), [Elfsight guide 2026](https://elfsight.com/blog/instagram-graph-api-complete-developer-guide-for-2026/)). Any tool using unofficial/private IG APIs (browser-automation posters, "no-API" bots) **violates Meta ToS** → account-ban risk. None of the tools shortlisted below do that; all use the official API.
   - **TikTok**: Content Posting API — **unaudited apps can only post SELF_ONLY (private) content, max 5 users/24 h, account must be private** ([TikTok docs](https://developers.tiktok.com/doc/content-posting-api-get-started), [Mixpost TikTok audit doc](https://docs.mixpost.app/services/social/tik-tok/direct-post-audit/)). The audit takes 2–4 weeks with demo video + privacy policy. **Consequence: self-hosting a scheduler does NOT give you public TikTok posting until you pass TikTok's audit.** Hosted schedulers (Postiz cloud, Buffer, Metricool, Publer, Blotato) already run audited apps.
   - **X/Twitter**: the free API tier was **discontinued Feb 2026**; new developers get pay-per-use: **$0.015/post, $0.20/post containing a link** ([Postproxy 2026](https://postproxy.dev/blog/x-api-pricing-2026/), [wearefounders](https://www.wearefounders.uk/the-x-api-price-hike-a-blow-to-indie-hackers/)). Hosted schedulers absorb this in their plans; self-host = you pay X directly. Given Albania reach ≈ 0, **deprioritize X** except occasional OSS-launch posts.
   - **Telegram**: Bot API is free and unrestricted for channel posting — and the dowiz repo **already has a working Telegram integration** (webhook E2E tests exist). This is the zero-cost, fully-owned channel.
   - **YouTube Shorts**: Data API upload = 1,600 quota units of the default 10,000/day (~6 uploads/day); videos uploaded via **unverified** apps are locked private until the app passes Google's audit. Same pattern as TikTok: hosted scheduler bypasses the audit.
3. **Money reality**: Claude generation cost at this volume is a rounding error (see §3). The real monthly costs are (a) the scheduler seat and (b) video tooling. Optimize those.

---

## 1. Recommended stack (the pipeline, end to end)

```
SOURCE            GENERATE              APPROVE          SCHEDULE/PUBLISH        MEASURE
dowiz DB + R2  →  Claude API (Haiku/  →  Telegram bot  →  Postiz (hosted P1,   →  Shlink links + Postiz
storefront/menu    Sonnet, Batch API)    approve/reject    self-host P2)          analytics + storefront
photos, orders,    + HTML→PNG stat       buttons via       + Telegram Bot API     visits; n8n weekly
vendor stories,    cards (Playwright)    n8n wait-node     direct (free)          digest → Claude scores
Maps venue data    + FFmpeg/Remotion                                              hooks → next batch
        └────────────────── all orchestrated by self-hosted n8n ──────────────────┘
```

### Named tools + why + cost

| Stage | Tool | Why | Cost/mo |
|---|---|---|---|
| **Orchestration** | **n8n (self-hosted)** | Free forever under the [Sustainable Use License](https://docs.n8n.io/sustainable-use-license/) for internal marketing automation (explicitly allowed); Docker deploy on existing Fly.io/VPS infra; largest template ecosystem for exactly this pipeline (RSS→AI→schedule flows); the project already values owning its stack. | **$0** (runs on existing infra) |
| **Text generation** | **Claude API** — `claude-haiku-4-5` for volume captions, `claude-sonnet-4-6`/`claude-sonnet-5` for hero copy, via **Message Batches API (−50%)** | Already integrated in the repo (Anthropic SDK present); structured outputs → JSON post objects (hook/caption/hashtags/alt-text × al/en); batch API fits a "generate the week every Sunday night" cadence. | **≈ $1–5** (see §3 math) |
| **Stat cards / branded images** | **HTML template → PNG via Playwright** (already a repo dependency) or [satori](https://github.com/vercel/satori) | "€X saved at 0% commission" cards must be on-brand, bilingual, deterministic — a templated render beats a diffusion model on all three, and it's free. AI image gen reserved for rare hero art: gpt-image-1-mini **$0.005/img** or FLUX on fal/Replicate **$0.008–0.04/img** ([pricepertoken](https://pricepertoken.com/image), [buildmvpfast](https://www.buildmvpfast.com/api-costs/ai-image)). | **$0** (+ ~$1 if AI images used) |
| **Short video** | **FFmpeg templates** (phase 1) → **Remotion/Revideo** (phase 2) assembling *real* vendor photos/clips: Ken-Burns pans over R2 food photos + text overlays + music | Real food footage outperforms uncanny AI food video; the R2 bucket already holds 40+ storefront photos. Remotion = React-programmatic video, but **flag: Remotion requires a paid company license** (~$150+/yr) beyond small-team free tier — [Revideo](https://github.com/redotvideo/revideo) (MIT) or plain FFmpeg avoids that. | **$0** |
| **Approval** | **Telegram bot** (existing dowiz bot) + n8n `Wait for Webhook` | Every generated post lands in a private Telegram chat with ✅/✏️/❌ buttons; human-in-the-loop with zero new UI. Aligns with the project's "advisory signals, human authority" culture. | **$0** |
| **Scheduling/publishing** | **Postiz** — hosted **$29–39/mo** in phase 1; **self-hosted (AGPL-3.0, free)** in phase 2 | Only fully-OSS scheduler with wide coverage (IG, TikTok, FB, YT, LinkedIn, X, Telegram, Mastodon, Bluesky — 20-30+ networks), a **public API + MCP server on every plan**, and native n8n usability ([postiz.com](https://postiz.com/), [GitHub ~30k★, active](https://github.com/gitroomhq/postiz-app)). Hosted first because its Meta app is already approved and its TikTok app already audited — you post publicly on day 1. Self-host later once dowiz's own Meta app review + TikTok audit clear (§4). Same AGPL license as dowiz itself — philosophical fit. | **$29–39 → $0** |
| **Telegram channel** | **Telegram Bot API direct from n8n** | Free, unlimited, owned; also the distribution channel for vendor-community content in Albania. | **$0** |
| **Link tracking** | **Shlink (self-hosted, MIT)** | Every CTA link → `go.dowiz.al/xyz` with UTM; tracks clicks/referrer/geo/device; REST API n8n can read ([shlink.io](https://shlink.io/)). Closes the loop from post → storefront visit. | **$0** |
| **Analytics/feedback** | **Postiz analytics + Shlink + n8n weekly digest → Claude** | n8n pulls per-post metrics weekly, Claude (Haiku) ranks hooks/formats, top patterns feed the next generation batch's prompt. Optional booster: **Metricool Free** (1 brand) purely as an analytics/best-time-to-post dashboard. | **$0** |

**Phase-1 total: ≈ $30–45/mo** (Postiz hosted + Claude + optional AI images).
**Phase-2 total: ≈ $2–10/mo** (Claude + optional X pay-per-use + infra you already run).

---

## 2. Alternatives table

### Schedulers/publishers

| Tool | OSS / self-host | Price (Jul 2026) | Platform coverage | API/automation | Verdict for dowiz |
|---|---|---|---|---|---|
| **Postiz** | ✅ AGPL-3.0, Docker | Free self-host; hosted $29 (5ch/400 posts), $39 (10ch, unlimited, AI) ([pricing](https://postiz.com/)) | Widest: IG, TikTok, FB, X, LinkedIn, YT, Pinterest, Threads, Bluesky, Mastodon, Reddit, Discord, **Telegram**, 20–30+ | Public REST API + **MCP server** + webhooks on all plans | **Pick.** Hosted→self-host migration path; AGPL kinship |
| **Mixpost** | ⚠️ Lite is OSS but only FB Pages/X/Mastodon; Pro is source-available paid | **$299 one-time** Pro (all platforms incl. IG reels/stories, TikTok, YT Shorts; API, webhooks, approval flows); $1,199 Enterprise; 1 yr updates then renew ([pricing](https://mixpost.app/pricing)) | Pro: FB, IG, X, LinkedIn, YT, TikTok, Pinterest, Threads, Bluesky, GBP, Mastodon — **no Telegram** | REST API + webhooks (Pro) | Strong #2. One-time price beats subscriptions in year 2+; but same self-host TikTok-audit problem, PHP/Laravel stack (foreign to this repo), no Telegram |
| **Buffer** | ❌ | Free (3 ch, 10 posts/ch); Essentials $6/ch/mo; Team $12/ch/mo ([buffer.com/pricing](https://buffer.com/pricing)) | All majors incl. IG/TikTok | API in **beta**, plan-gated rate limits (100 req/day on Free) | Fine free starter for 3 channels; per-channel pricing scales badly (7 channels ≈ $42/mo) |
| **Metricool** | ❌ | Free (1 brand, 20 posts/mo, no LinkedIn/X); Starter ~$20–25; **Advanced ~$53–67 needed for API/Make/Zapier/MCP**; +$5/mo each for LinkedIn & X ([pricing](https://metricool.com/pricing/)) | All majors + GBP | API only on Advanced | Best-in-class **analytics**; use the Free tier as an analytics dashboard, not the publisher — API paywall too high |
| **Typefully** | ❌ | ~$12.50/mo+ ([pricing](https://typefully.com/pricing)) | **Text-only: X, LinkedIn, Threads, Bluesky, Mastodon — no IG/TikTok** ([docs](https://typefully.com/docs/api)) | Clean REST API v2 | **Wrong tool** for visual food content; only relevant if the OSS-devrel LinkedIn/X lane grows |
| **Blotato** | ❌ | $29 Starter (1,250 credits) / $97 Creator / $499 Agency ([pricing](https://www.blotato.com/pricing)) | 9+ platforms native | REST API + MCP + native n8n/Make nodes; faceless AI video + repurposing built-in | The "one API for post+video" convenience play. Good if you want AI faceless video without building it; credit-metered, closed, overlaps what Claude+FFmpeg do for ~$0 |
| **Publer** | ❌ | Free (3 accts, no X); Pro $12; **API locked to Business $21+** ([plans](https://publer.com/plans), [API launch](https://publer.com/blog/publer-api-for-marketers-and-developers/)) | 12+ incl. IG/TikTok, Telegram | REST API (Business+), RSS auto-post, bulk CSV | Decent value; API paywall and closed source make it strictly worse than Postiz here |

### Orchestrators

| Tool | License / self-host | Price | Notes |
|---|---|---|---|
| **n8n** | Fair-code (Sustainable Use); self-host free, unlimited executions ([docs](https://docs.n8n.io/sustainable-use-license/)) | $0 self-host; cloud from ~€20/mo | **Pick.** Internal automation explicitly allowed; huge social-media template library; code nodes for the Claude/FFmpeg steps |
| **Activepieces** | **MIT** (truly OSS); lighter Docker footprint | $0 self-host; cloud free tier 1k tasks, $25/mo Plus ([comparison](https://www.activepieces.com/blog/activepieces-vs-n8n)) | Cleaner license than n8n; fewer nodes/templates. Legitimate swap-in if MIT purity matters |
| **Make** | ❌ | Core ~$9–10.59/mo for 10k ops ([pricing review](https://workflowpick.com/pricing-guides/make-com-pricing-2026-review/)) | Cheapest hosted; only if you refuse to self-host |
| **Zapier** | ❌ | $19.99/mo for 750 tasks | ~5× Make's price per operation; skip |

### AI generation

| Modality | Options (Jul 2026) | Pick |
|---|---|---|
| Text | Claude API: Haiku 4.5 $1/$5 per MTok; Sonnet 4.6 $3/$15; Sonnet 5 intro $2/$10 (through 2026-08-31); Opus 4.8 $5/$25; **Batch API −50%** | Haiku 4.5 (batch) for volume; Sonnet for monthly pillar/hero copy |
| Image | gpt-image-1-mini $0.005; Imagen 4 Fast $0.02; Ideogram 3.0 $0.03 (best text-in-image); FLUX via fal/Replicate $0.008–0.04; Recraft V3 $0.04 ([pricepertoken](https://pricepertoken.com/image)) | HTML→PNG templates first (free, on-brand); Ideogram if AI text-in-image is ever needed |
| Short video | OSS: FFmpeg, Remotion (⚠️ company license), Revideo (MIT), HeyGen HyperFrames (OSS, HTML/CSS→video), [OpenMontage](https://github.com/calesthio/OpenMontage) (agentic OSS pipeline); SaaS: Blotato faceless (credits), Kling/Runway/Veo (expensive, uncanny for food) | FFmpeg photo-montage templates → Revideo when templates need logic. Real footage > generative for food |

---

## 3. Claude generation cost math (verified pricing, 2026-07)

Assume 60 posts/month, each generated bilingually (al + en) with hook, caption, hashtags, alt-text; system prompt with brand voice + vendor data ≈ 2.5k input tokens, output ≈ 800 tokens per post; plus 8 weekly-digest/scoring runs.

| Model | 128 generations/mo | With Batch API (−50%) |
|---|---|---|
| Haiku 4.5 ($1/$5 per MTok) | ≈ $0.83 | **≈ $0.42** |
| Sonnet 4.6 ($3/$15) | ≈ $2.50 | ≈ $1.25 |
| Opus 4.8 ($5/$25) | ≈ $4.16 | ≈ $2.08 |

**Conclusion: text generation is free-in-practice.** Use prompt caching on the brand-voice system prompt (stable prefix) and don't optimize further. The bilingual requirement is a Claude strength — one call, structured output with `{al: {...}, en: {...}}`.

---

## 4. Phase 1 vs Phase 2

### Phase 1 — cheapest thing that works **this week** (~$30–40/mo, ~2 days build)

1. **Sign up Postiz hosted Standard ($29)** (or Team $39 if >5 channels). Connect: dowiz IG Business account, TikTok, FB Page, YouTube. Their audited apps ⇒ public posting immediately, no Meta review, no TikTok audit.
2. **Deploy n8n via Docker** next to existing infra (Fly.io app or the staging VPS). One workflow, cron `Sun 20:00`:
   - Pull content sources: query the dowiz DB for this week's angle material (new vendors, top dishes with R2 photos, order counts for savings math — **aggregate numbers only, per the PII red-lines; per-vendor claims need that vendor's opt-in**).
   - One **Claude Batch API** call → 7–10 structured bilingual post objects.
   - Render stat-card PNGs from an HTML template via Playwright (already installed in the repo).
   - Push each candidate to the **operator's Telegram** with ✅/❌ inline buttons (existing bot).
   - On ✅ → `POST` to the **Postiz public API** to schedule; Telegram-channel posts go direct via Bot API.
3. **Deploy Shlink** (one Docker container) and use `go.` short links in every caption/bio.
4. Skip X and LinkedIn entirely in phase 1. Skip AI video — post real photos as Reels-style slideshows (Postiz supports multi-image → IG carousel; FFmpeg one-liner turns 5 photos + music into a 15 s vertical MP4).

**Definition of done:** one Sunday run produces a scheduled week across IG/TikTok/FB/Telegram with ≤10 min of human tapping ✅ in Telegram.

### Phase 2 — scaled & owned (~$2–10/mo, weeks 4–12)

1. **File dowiz's own Meta app review** (`instagram_business_content_publish`) and **TikTok Content Posting audit** (2–4 weeks; needs privacy-policy URL + demo video — dowiz has both a domain and Playwright to record the demo).
2. Once approved → **self-host Postiz (AGPL)** with dowiz's own OAuth apps; cancel the hosted plan. Everything else in the pipeline is unchanged (same Postiz API surface).
3. **Video templates**: Revideo/FFmpeg "dish of the day" and "vendor story" templates fed by R2 photos + Whisper-subtitled voice clips (the repo already has a WhisperProvider ASR engine — reuse it for auto-subtitling vendor voice notes).
4. **Vendor-as-source flywheel**: a Telegram "content inbox" bot for onboarded vendors — they forward a phone photo/clip, pipeline generates the post, vendor gets co-published + tagged. Their audience becomes dowiz's distribution.
5. **Feedback loop v2**: n8n pulls Postiz analytics + Shlink clicks + storefront `/s/:slug` visit counts weekly → Claude scores (hook style × format × posting time × language) → winning patterns appended to the generation prompt. Kill formats with <1% engagement after 3 tries.
6. Optional: LinkedIn/X lane for the **open-source launch** (ADR-020) using the same pipeline — X via pay-per-use (~$0.20/link-post, budget $5/mo) only around launch moments.

---

## 5. Content sourcing playbook (dowiz-specific)

The unfair advantage: **the product database *is* the content source.** No other tool in the comparison has real menus, real photos, and real order data behind it.

| Pillar | Source → asset | Cadence |
|---|---|---|
| **P1 — "0% commission" proof** | Aggregate order totals → "Vendors on dowiz kept €X this month that platforms would have taken" stat cards; per-vendor version **only with written vendor consent** (aligns with owner-data ETHICAL-STOP: aggregate/anonymous by default) | 2×/week |
| **P2 — Vendor spotlight / before-after** | demo-builder loop output: screenshot of Google-Maps-only presence → polished `/s/:slug` storefront; vendor quote; claimable-demo CTA ("this could be your menu — claim it") | 1–2×/week |
| **P3 — Food porn from R2** | Existing storefront photos (Dubin & Sushi, Artepasta, Eljo's Pizza sets) → Reels slideshows, dish-of-the-day, "guess the dish" polls | daily-ish stories, 3 reels/week |
| **P4 — Build-in-public / OSS** | Commit-log → weekly "what we shipped" (Claude summarizes git log honestly); AGPL/self-host positioning for the dev audience; timed with open-source gating milestones | 1×/week (LinkedIn/Telegram) |
| **UGC** | Vendor Telegram content-inbox (phase 2); repost with permission + tag; customers' story-mentions reposted | opportunistic |

**Faceless-channel note:** a fully-automated faceless food channel (AI voice-over + stock food clips) is buildable with this stack (Claude script → TTS → FFmpeg), but for a *trust-selling* B2B product, real vendors/storefronts convert better. Keep faceless mechanics for format (templated, no on-camera founder), not for synthetic content.

---

## 6. First 30-day content plan skeleton

Channels: **IG + TikTok + FB (cross-posted) + Telegram channel**. All posts bilingual al/en (al primary). ~26 posts + daily stories, all producible by the phase-1 pipeline.

| Week | Theme | Posts |
|---|---|---|
| **W1 — Arrival** | "Menütë e Shqipërisë, pa komision" (Albania's menus, zero commission) | Mon: launch reel — 15 s montage of 5 real storefronts. Wed: stat card "30% komision vs 0%" explainer carousel (5 slides). Fri: vendor spotlight #1 (demo before/after). Sun: Telegram — week recap + storefront links. Daily: 1 dish story from R2 photos |
| **W2 — Proof** | Show the money | Mon: "€X saved" aggregate stat card. Tue: TikTok — screen-recording of a customer ordering on `/s/demo` in 30 s (Playwright can record this). Thu: vendor spotlight #2 + quote. Sat: "guess the dish" engagement post. Sun: Telegram digest |
| **W3 — The product** | How it works, for vendors | Mon: carousel "your menu online in 24 h — no app store, no commission". Wed: reel — owner phone POV receiving a Telegram order notification (real feature). Fri: claimable-demo CTA post targeting one cuisine ("pizzeritë e Tiranës…"). Sun: Telegram digest |
| **W4 — Community + OSS** | Ownership story | Mon: "why we're open-source (AGPL)" — build-in-public post (also LinkedIn). Wed: vendor spotlight #3 — UGC if any vendor supplied content. Fri: month-1 numbers, honestly (posts, storefront visits via Shlink). Sun: Telegram — month recap + what's next |

**Mechanics:** batch-generate each week on Sunday; ~10 min Telegram approval; Postiz best-time slots (default 11:00–13:00 & 19:00–21:00 Europe/Tirane until analytics say otherwise); every caption carries one Shlink CTA to a live `/s/:slug`; week-4 review = first run of the Claude scoring loop.

---

## 7. Flags & risks

- **ToS**: never adopt IG/TikTok tools that bypass official APIs (some "unlimited free posting" tools do — instant differentiator check: do they ask for your *password* instead of OAuth? → ban risk). All shortlisted tools are official-API.
- **TikTok/Meta audits gate phase 2** — start both applications in week 1 so approvals land by week 4–8; hosted Postiz covers the gap.
- **X pay-per-use** ($0.20/post-with-link) makes X the only channel with per-post marginal cost — fine to skip.
- **Buffer API is beta** and rate-limited by plan; don't build the pipeline against it.
- **Metricool's API is paywalled** at ~$53–67/mo Advanced — use its free tier read-only if at all.
- **Remotion licensing** — company license required beyond small-team free use; prefer Revideo (MIT)/FFmpeg.
- **Vendor data in marketing** — per-vendor revenue/savings claims are PII-adjacent: aggregate by default, explicit consent for named claims (consistent with the project's owner-data-export ETHICAL-STOP precedent).
- **Pricing drift** — Postiz/Mixpost/Buffer/Metricool numbers above verified 2026-07; recheck before annual commitments.

## Sources

- Postiz: [postiz.com](https://postiz.com/) · [pricing](https://postiz.com/pricing) · [GitHub (AGPL-3.0)](https://github.com/gitroomhq/postiz-app) · [review w/ plan limits](https://socialrails.com/blog/postiz-review)
- Mixpost: [pricing (one-time)](https://mixpost.app/pricing) · [GitHub](https://github.com/inovector/mixpost) · [TikTok direct-post audit doc](https://docs.mixpost.app/services/social/tik-tok/direct-post-audit/)
- Buffer: [pricing](https://buffer.com/pricing) · [plan features](https://support.buffer.com/article/595-features-available-on-each-buffer-plan)
- Metricool: [pricing](https://metricool.com/pricing/) · [plan analysis](https://socialk.it/en/pricing/metricool)
- Typefully: [pricing](https://typefully.com/pricing) · [API docs](https://typefully.com/docs/api)
- Blotato: [pricing](https://www.blotato.com/pricing) · [platform](https://www.blotato.com/)
- Publer: [plans](https://publer.com/plans) · [API launch](https://publer.com/blog/publer-api-for-marketers-and-developers/)
- n8n: [Sustainable Use License](https://docs.n8n.io/sustainable-use-license/) · [community edition](https://docs.n8n.io/deploy/host-n8n/community-edition-features)
- Activepieces: [vs n8n](https://www.activepieces.com/blog/activepieces-vs-n8n) · [2sync comparison](https://2sync.com/blog/activepieces-vs-n8n)
- Make vs Zapier: [Make pricing 2026](https://workflowpick.com/pricing-guides/make-com-pricing-2026-review/) · [Zapier's own comparison](https://zapier.com/blog/zapier-vs-make/)
- Instagram API: [content publishing limit](https://developers.facebook.com/docs/instagram-platform/instagram-graph-api/reference/ig-user/content_publishing_limit/) · [2026 dev guide](https://elfsight.com/blog/instagram-graph-api-complete-developer-guide-for-2026/)
- TikTok API: [get started / audit](https://developers.tiktok.com/doc/content-posting-api-get-started) · [sharing guidelines](https://developers.tiktok.com/doc/content-sharing-guidelines)
- X API: [2026 tiers / pay-per-use](https://postproxy.dev/blog/x-api-pricing-2026/) · [price-hike analysis](https://www.wearefounders.uk/the-x-api-price-hike-a-blow-to-indie-hackers/)
- AI image pricing: [pricepertoken.com/image](https://pricepertoken.com/image) · [buildmvpfast API costs](https://www.buildmvpfast.com/api-costs/ai-image)
- Programmatic video: [Revideo](https://github.com/redotvideo/revideo) · [OpenMontage](https://github.com/calesthio/OpenMontage) · [faceless-video topic](https://github.com/topics/faceless-video)
- Shlink: [shlink.io](https://shlink.io/) · [feature overview](https://openalternative.co/shlink)
- Claude API pricing: per the repo's claude-api skill reference (cached 2026-06-24): Haiku 4.5 $1/$5, Sonnet 4.6 $3/$15, Sonnet 5 $3/$15 (intro $2/$10 to 2026-08-31), Opus 4.8 $5/$25 per MTok; Batch API −50%.
