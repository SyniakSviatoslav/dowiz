# Demo Preview & Outreach Upgrades — Plan

**Date:** 2026-07-06 · **Type:** growth + product plan (research + design, no code yet) · **Author:** lead
**Scope:** the next upgrades to the 12 live pitch demos (`/s/<slug>`, Durrës) — rich link previews,
per-venue OG images, QR + trust footer, and a raw-video-with-subtitles outreach pipeline.

> **Operator decision (2026-07-06, AskUserQuestion):**
> 1. Unclaimed-demo unfurl → **"Full per-venue unfurl now"** (name + dishes + per-venue image, and
>    identity allowed in outbound video). This **reverses the signed P6-2/P6-3 privacy invariant** on
>    unconsented venues → it is a **red-line override** (see §3) and does **not** authorize code by
>    itself: build only after a counsel/Triadic-Council ethics pass + explicit operator go (§3, §5).
> 2. Build order → **WS1 + WS3 + WS4**, and **write this plan doc first**.
>
> Council is mechanically *optional* since 2026-07-05 (`[[council-gate-disabled-2026-07-05]]`), but the
> override in WS2 revises a **signed** verdict — counsel's own rule is "re-ratify through the seats."
> Recommended, not forced. The operator may waive it in-session (as with the rebuild surfaces); if so
> that waiver is logged here and the honesty/erase invariants in §3 become non-negotiable substitutes.

---

## 1. Goal

Make a pasted `/s/<slug>` link — and a short founder video — convert cold Albanian restaurant owners
by looking like *"my restaurant, my dishes, done for me"* instead of a *"suspicious developer link."*
Everything is built on assets that already exist: the demo storefronts, the R2 photo bucket, the
Telegram bot, Playwright, and a Whisper ASR provider.

## 2. Current state (verified, file:line)

| Concern | Where | Today |
|---|---|---|
| Demo tenants | `scripts/demo-builder.mjs`, `scripts/acquisition-bulk-provision.mjs:13` | **Shadow** tenants: `owner_id NULL`, `status='closed'`, `published_at NULL`. Built from public Wolt/Maps data. Never published. |
| Human storefront (live) | `apps/api/src/lib/spa-shell.ts` | Node SPA shell (S1 Astro reverted). Per-tenant `<title>`+OG injected; `og:image` = tenant **logo** *if present* (`:86`). |
| Bot/unfurler storefront | `apps/api/src/lib/ssr-renderer.ts` | SSR menu + JSON-LD. `og:image` = **single static `/og-image.png`** for every venue (`:352`). |
| Shadow gate (the block) | `spa-shell.ts:103-176`, `preview-render.ts:66`, Rust `rebuild/crates/api/src/routes/storefront.rs:90` | `owner_id IS NULL` → **generic OG** (no name, no dishes) + `noindex`. Real name only in an on-page *"preview mockup — not a live store"* banner. **Council-bound: P6-2/P6-3, breaker B2, counsel C1.** |
| Bot UA match | `spa-shell.ts:15` (`BOT_UA`) | WhatsApp/Telegram/Facebook crawlers match → they hit the **bot/shadow** branch, i.e. the generic preview. |
| Outreach | `scripts/report-demos-to-telegram.mjs:170` | Sends the link with `disable_web_page_preview: true` → **no preview at all** right now. Albanian pitch copy is solid. |
| R2 storage | `apps/api/src/lib/r2-storage.ts`, `image-url.ts` | S3 client `put/get/delete`; public URL via `R2_PUBLIC_URL`. **`contentType()` handles webp/png/jpg only — no `mp4`.** |
| Montage / video | `docs/research/2026-07-03-social-media-automation-pipeline.md` | Pipeline scoped (OpenMontage / Revideo-MIT / FFmpeg + Playwright stat-cards + Whisper subtitles). **Not built.** |

**Net:** for the 12 shadow demos, a pasted link unfurls **generic today, by design.** The whole "unfurl
their name + dishes" move is blocked by a deliberate privacy invariant — not a missing feature.

## 3. Consent architecture — the pivotal gate (read before WS2)

The `owner_id IS NULL` → generic-OG rule exists so an **unconsented** venue's identity is never broadcast
into a chat. It is the Ethics Charter and the owner-data ETHICAL-STOP precedent, written in code.

The operator has chosen to **override it for the demo/outreach class**. That is a legitimate GTM motion
(claimable-demo is initiative #2 in `docs/research/2026-07-03-MAX-ROI-PLAN.md`; the menu is public
business data) — **but the exposure it adds is real**: a venue's name+dishes appearing in a *third-party
group chat* before they've consented. So the override ships only with these **non-negotiable substitutes**
for the invariant it removes (these are what a counsel pass would harden; absent a council, they are the
minimum bar):

- **Honesty invariant stays.** The unfurl description and the on-page banner keep an explicit *"preview /
  not a live store — is this your restaurant?"* line. The unfurl must never imply the venue itself set it up.
- **Keep `noindex`.** Rich *chat unfurl* ≠ *search indexing*. Allow the OG unfurl but keep `noindex,nofollow`
  so we don't pollute Google with unconsented venue pages. (Unfurl and index are separable — different bots.)
- **Scope narrowly.** Override applies only to tenants flagged as demo/outreach sources — not all shadow
  tenants. A generic shadow with no outreach intent stays generic.
- **Decline/erase prominent + working.** The claim link already doubles as decline/erase
  (`POST /api/claim/decline {token}`; `acquisition-bulk-provision.mjs:229`). It must be one tap from the page
  and honored fast. GDPR erasure path unchanged.
- **Feature-flagged, default OFF, reversible.** A single flag flips the override off globally in one step.

**Recommended path:** a *focused* counsel/ethics pass (not a full 4-seat build council) that ratifies the
five substitutes above + the legal read on "public menu in third-party chat preview," then operator go.
This is cheap friction on a genuinely irreversible-once-sent action.

---

## WS1 — Per-venue OG image generator  *(consent-neutral for claimed venues; ships now)*

**What:** replace the static `/og-image.png` with a real 1200×630 per-venue card (venue name + top dishes
from R2 photos + theme colors + optional ▶ play overlay). This is the single highest-impact preview upgrade.

**Design:**
- **Render:** HTML template → PNG via **Playwright** (already a repo dep; research §1 recommends exactly
  this — deterministic, on-brand, bilingual, free). A dedicated internal route `/internal/og/:slug` renders
  the card HTML using the tenant theme (`location_themes`) + up to 3 dish images; Playwright screenshots it.
- **Cache, don't render per-request.** Generate on menu publish / `menu_version` bump → `put` to R2 at
  `og/<location_id>-<menu_version>.png` → serve via `R2_PUBLIC_URL`. Bots re-hit; per-request Playwright
  would be a DoS on ourselves. Fallback to `/og-image.png` if the key is absent.
- **Wire the tag** in all three emitters so unfurl parity holds:
  - `ssr-renderer.ts:352` (bot SSR) — swap static PNG for `getImageUrl('og/<id>-<ver>.png')`.
  - `spa-shell.ts:87` (human SPA) — prefer the OG card over bare `logo_url`.
  - `rebuild/crates/api/src/routes/storefront.rs` `bot_menu_html` — **currently emits no `og:image` at all**;
    add it (parity gap for the eventual Astro/Rust cutover).
  - Add `og:image:width=1200`, `og:image:height=630`, `twitter:image`.
- **▶ play-overlay variant** (`og/<id>-<ver>-play.png`) for the WS4 video tie-in.

**Applies to:** published/claimed venues immediately (no gate). Extending it to *unclaimed* demos is WS2.

**Proof (Mandatory Proof Rule):** Playwright vs staging asserts `<meta property="og:image">` on a published
slug resolves to a 1200×630 image (real `Image()` load + dimension assert); visual snapshot of the card;
a fetch-the-HEAD unfurl check. API assert: the R2 key exists after a publish.

---

## WS2 — Unclaimed-demo unfurl override  *(RED-LINE — §3 gate before code)*

**What:** make the 12 unclaimed demos unfurl their real name + a dish teaser + the WS1 per-venue image,
and allow venue identity in outbound video (WS4). Operator chose this.

**What the ethics pass must ratify** (enumerate → decide → then code):
1. Reverse P6-2/P6-3 for the demo/outreach tenant class only (add an outreach flag; do not touch generic
   shadows). Sites to change: `spa-shell.ts:136` (`isShadow` gate), `ssr.ts` preview branch,
   `preview-render.ts`, and the Rust `storefront.rs:90` `is_shadow` port.
2. Keep the honesty banner + `noindex` (§3). Rich `og:*` **with** `noindex,nofollow`.
3. Decline/erase link in the unfurl description + on page.
4. Legal read: public business menu shown in a third-party chat preview (AL/EU).
5. Feature flag `DEMO_RICH_UNFURL` default OFF; per-tenant opt-out.

**Do not start WS2 code until §3 is satisfied.** WS1/WS3/WS4-infra do not depend on it and proceed now.

---

## WS3 — Outreach quick-wins  *(consent-neutral; ships now)*

- **QR per demo.** A branded QR → `/s/<slug>` PNG (reuse the existing QR-kit infra if present —
  `admin.qr_kit*`/`QRKitPage` per memory; else a small `qrcode` render). Store to R2 / attach in the
  Telegram report so an owner can hand it to staff. "Roздатковий матеріал" = trust signal.
- **Operator-identity footer** on the storefront (`ssr-renderer.ts:426` footer + SPA): *"Powered by Syniak
  Sviatoslav · Local Developer"* + LinkedIn/GitHub. This is **your** identity (the "developer passport" from
  the strategy) — no third-party consent issue; it *increases* transparency on a demo page.
- **Enable Telegram preview.** Flip `disable_web_page_preview` → `false` in
  `report-demos-to-telegram.mjs:170` **after** WS1 (published) / WS2 (unclaimed) lands. Note Telegram
  previews only the first URL in a message — keep the demo link first.
- **Own domain (ops).** Buy a short domain (`porosite.al` / `dowiz.app`), point to Fly, set `APP_BASE_URL`.
  A subdomain of `fly.dev` reads as "developer project"; a short domain is a large trust delta for ~$10/yr.
- **Cold-start (ops).** `fly.toml` `min_machines_running=1` (or a warmer ping) on the storefront app — a
  sleeping demo that spins up slowly reads as "broken," which kills the pitch harder than a plain link.

**Proof:** Playwright asserts the footer identity + a working QR target on staging; a dry-run of the
Telegram script with preview enabled showing the unfurl.

---

## WS4 — Raw-video + dynamic-subtitles pipeline  *(infra now; identity overlay = WS2-gated)*

**What:** the "Reels-style" founder video from the strategy — you talking, quick cuts to the app + their
menu, Hormozi-style word-by-word captions.

**Design (build order within WS4):**
- **A. R2 video support (infra, now).** Add `mp4`/`webm` to `R2StorageProvider.contentType()`
  (`r2-storage.ts:41`) so videos can be hosted. Serve via `R2_PUBLIC_URL`.
- **B. Subtitles (now).** Reuse the repo's **WhisperProvider** → SRT → burn word-by-word with FFmpeg
  (`drawtext`) or Revideo. No new ASR dep.
- **C. Montage engine.** Start with an **FFmpeg template** (Ken-Burns pan over R2 photos + your clip + music
  + caption track) — zero license cost. Graduate to **Revideo (MIT)** when layout needs logic. **OpenMontage**
  (the agentic OSS pipeline you opened) is the orchestrator to evaluate for auto-assembling per-venue cuts.
  Avoid Remotion (company license, research §7).
- **D. Per-venue identity overlay** (logo/name/their dishes) = **same consent gate as WS2.** Until then the
  video uses only *generic* frames (you, the app UI, `/s/demo`).
- **E. Play-button OG** — the WS1 `-play.png` variant so the link "reads as a video" (strategy's key trick).
  Prefer the OG-image-with-play trick over `og:video` (research/strategy: messengers handle `og:video`
  poorly and flag it as phishing-adjacent).

**Proof:** assert the MP4 exists on R2 with a burned subtitle track + expected duration; a Playwright check
that the demo page (or Telegram message) surfaces the play-overlay preview.

---

## 5. Sequencing

- **Phase 1 (this week, no gate):** WS1 (claimed venues) · WS3 quick-wins · WS4-A/B/C/E with a generic
  founder video. Deploy staging → prove → paste-test the unfurl in a real WhatsApp/Telegram.
- **Phase 2 (gated on §3):** counsel/ethics pass → operator go → WS2 override + WS1-for-unclaimed +
  WS4-D identity overlay, all behind `DEMO_RICH_UNFURL` default-OFF.

## 6. Risks / open questions

- **The override is the whole risk surface.** Everything else is upside-only. Get §3 right or the project's
  own ethics charter is the thing that breaks, not the code.
- **`noindex` vs unfurl** — confirm messengers still render the OG card with `noindex` present (they do; it's
  a crawler directive, not an unfurl blocker). Validate on WhatsApp + Telegram + Facebook.
- **R2 public bucket** — confirm `R2_PUBLIC_URL` is set on staging/prod (image-url.ts falls back to the
  `/images/*` proxy if not; OG images want the CDN URL, not the proxy).
- **Rebuild parity** — the Rust `storefront.rs` bot path lacks `og:image` entirely; fold WS1 into it so the
  eventual cutover doesn't regress the preview.
- **Per-venue OG generation cost** — cache on `menu_version`; never render per unfurl request.
```
