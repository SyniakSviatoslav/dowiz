# GTM pipeline completion — demo / vendor-search / offer, to 100% "genuinely working to send"

> **Date:** 2026-07-11 · **Lens:** the operator's stated ~80%-done / ~20%-remaining pipeline that
> gates remote vendor outreach — "needed to send a genuinely working system, not raw work; quality
> matters." · **Method:** code read + live HTTP probes + read-only git this session, layered on the
> same-day research corpus (`2026-07-11-MAX-EV-SYNTHESIS.md`, `-adoption-ev-onboarding.md`,
> `-adoption-ev-market.md`, `gap-blueprints-2026-07-11/G11`). Labels: **VERIFIED** (checked in
> code / live probe / git this session) · **VERIFIED-G11 / -MAX-EV / -MKT / -ONB** (verified live
> earlier today by those docs, cited not re-probed) · **CLAIMED** (memory/doc, not re-checked) ·
> **UNVERIFIED** (secondary web source). **This file is the only artifact created. The repo is
> being changed by a parallel session (`fix/prod-blockers-P2`) — its live state is reported below
> as found; nothing here touches code.** Standing decisions respected throughout: local-first,
> COD, no courier scoring, anonymity, multichannel/no-app, storefront sovereignty; quality-first
> go-to-market with remote outreach gated on stability.

---

## 1. Pipeline inventory — what exists, and its real state

The pipeline is three stages. All three are **built and have each run at least once for real**
(that is the operator's 80%). What remains is almost entirely **delivery integrity** — the gap
between "the tool ran" and "the thing it produces works on the first tap by a stranger in Durrës"
(that is the 20%).

### Stage A — Vendor search / targeting (~90% complete)

| Asset | What it does | Real state | Evidence |
|---|---|---|---|
| `scripts/radar-scout.mjs` | Sweeps a city's food vendors from Google Maps, scores by public BUSINESS signals only (rating credibility, review volume, food fit, **delivery-intent**, chain penalty), ranks by warm-DM likelihood | **Built + ran**: 123 unique Durrës vendors swept, top-30 shortlist at `loops/offers/radar-durr-s.md` + `.json` | VERIFIED (code + output read this session) |
| `tools/demo-builder/aggregator-check.mjs` | Wolt-presence check (token-set name match against the city venue list) = the cost-pressure "HOT" signal | Built; **best-effort/flaky** — the ArtePasta packet records "aggregator check inconclusive"; falls back to "verify manually" (honest, never fabricates) | VERIFIED (code + packet) |
| Market ground truth | 9/12 demo venues confirmed live on Wolt Durrës — the complement-Wolt pitch targets | VERIFIED-MKT §1.3 |
| Ethics posture | Public business signals only; the personal-owner-dossier request was **DECLINED and recorded** (`loops/memory/offer-builder.md`) — venue signal is the lawful substitute | VERIFIED (memory file read) |

**Residual gaps (small):** the Wolt cost-pressure signal is *not yet folded into the radar score*
(the radar doc itself calls it "the tie-breaker … not in this score yet"); the aggregator check's
geo/endpoint flakiness means the sharpest targeting signal is manual for now. Neither blocks a
send; both sharpen *who* to send to.

### Stage B — Demo generation / provisioning (~75% complete — the largest 20% chunk lives here)

| Asset | What it does | Real state | Evidence |
|---|---|---|---|
| `scripts/demo-from-wolt.mjs` | Wolt slug → prospect JSON (menu extract, Albanian keyword categorisation, cuisine palette seed) — a new prospect's demo is one command | Built + proven; **14 prospect JSONs** in `loops/prospects/` (the 12 demos + **arden, dogu, saly** ready but unprovisioned) | VERIFIED |
| `scripts/demo-builder.mjs` | Quality-gates the menu (≥2 cats, ≥6 items, ≥50% described-or-photographed, sane integer prices), derives AA-contrast branding, provisions via the shipped `/internal` acquisition pipeline, then a **visual acceptance gate** (rendered DOM, never-orderable, noindex — no fake-green). Preview-only by default; `--send-invite` is the explicit opt-in that mints a claim token | Built + proven (it built the 12) | VERIFIED |
| `scripts/acquisition-bulk-provision.mjs` | Bulk path; idempotent/resumable; shadow-only writes (`owner_id NULL`, `status='closed'`) | Built | VERIFIED |
| The 12 demos | Real Durrës venues, ratings 4.6–5.0, menus 19–107 items, real phones | **Staging: all 12 live** (`/public/locations/apollonia/info` = 200 this session). **Prod: 11/12 = 404** (probed this session); prod's only storefront is `/s/demo` (Dubin & Sushi) | VERIFIED (live probes) |
| Claim flow (`/claim#token=…` → sign-in → accept; publish stays separate) | `ClaimPage.tsx` + `POST /claim/accept\|request\|decline` + `/internal/acquisition/claim/mint` (`route.ts:160`) all exist | **`GET /claim` = 404 LIVE on prod AND staging** (probed this session). **BUT: the fix is now COMMITTED** — parallel-session commit `09dcbe05` "fix(dowiz): P6 /claim 404 + P2 checkout enum/receiver" adds `'/claim'` to `SPA_ROUTES` (`server.ts:858` now includes it) **+ the G03 checkout enum/receiver fix, with tests. Committed ≠ deployed** — no live surface serves it yet | VERIFIED (probe + git + code, this session) |
| `PROVISION_OPS_SECRET` | Gates demo provisioning AND claim-token minting | **Lost** (rotated to a throwaway 06-29) — no new demo can be provisioned, no claim token minted, anywhere, until the operator re-rotates | CLAIMED (memory `claim-happy-path-and-sops-2026-06-29`, via G11/ONB) |
| Prod worker machine | Owns order-timeout auto-cancel sweep | **Stopped since 07-03** (owner TG alerts safe — consumer lives in the API process) | VERIFIED-G11 (flyctl) |

### Stage C — Offer generation / outreach send (~85% built, **0% currently deliverable at the quality bar**)

| Asset | What it does | Real state | Evidence |
|---|---|---|---|
| `scripts/offer-builder.mjs` | 1 Maps query → full **offer packet**: target facts, demo link **with live-check**, DM channels (WhatsApp `wa.me/` from the Maps phone; IG/FB best-effort), venue business signal, communication strategy (Albanian warm-list playbook: 5 lines, no scam-trigger words, no competence hits), LLM-composed Albanian draft (OpenRouter, scaffold fallback), send checklist. **No auto-send by design** — packet is prepared for a human | Built + ran: **6 packets** in `loops/offers/` (gitignored — contact info) | VERIFIED |
| The 6 packets | artepasta (demo ✅ live) + **colosseo, elia, pastarella, da-zio-libero, te-xhabarka — all 5 flagged "⚠️ demo NOT built yet"** | Offer generation has outrun demo provisioning: 5 of 6 packets carry a dead personalization link — blocked on the lost secret | VERIFIED (grep this session) |
| `scripts/report-demos-to-telegram.mjs` | Structured per-venue demo pitch (data + ready-to-send Albanian offer) into a Telegram channel; `--dry` preview; relies on the OG unfurl (`disable_web_page_preview: false` on venue messages) | Built; defaults `BASE=staging`; dry-run behavior needs the G11 3.4 exercise; **no `?ch=` channel stamp on the storefront links** (attribution supported by the storefront per HUB §2 but not minted here) | VERIFIED (code) |
| Rich OG unfurl cards (`/og/<slug>.png`) | The pasted link that sells itself | **Staging: 200 `image/png` for apollonia this session — but `content-length: 652,906 bytes`. Prod: 404** (`og-card.ts` not on origin/main; the commit `6a89d6e8` is contained in `fix/prod-blockers-P2` / paleo / sovereign lineages — VERIFIED `git branch --contains`) | VERIFIED (probes + git) |
| **NEW finding — the 652KB card likely breaks WhatsApp**, the #1 remote channel | Meta's own developer documentation: the link-preview image "should be under 600KB"; practitioner guides converge on **≤300KB for reliable display** ([Meta for Developers](https://developers.facebook.com/documentation/business-messaging/whatsapp/link-previews/); [opengraphplus](https://opengraphplus.com/consumers/whatsapp/images); [getlinkpeek](https://www.getlinkpeek.com/blog/whatsapp-link-preview-not-working)) | **652,906 > 600,000** → the WhatsApp unfurl — W3, the closer wow-moment — plausibly shows a bare link today. Telegram tolerates it (staging cards already render there per memory 07-06) | VERIFIED size (probe); limit **UNVERIFIED-leaning-VERIFIED** (Meta primary doc + 2 aggregators); resolution = the paste-test below |

**Inventory verdict.** The operator's "80%" is real: every stage has working, ethically-fenced,
quality-gated tooling with actual output artifacts. The "20%" is precisely: (a) two dead hinges
(`/claim` live-404, lost secret), (b) a prod surface that serves 1 of 12 demos and no OG cards,
(c) an OG card that exceeds the WhatsApp preview budget, (d) five offer packets pointing at
demos that don't exist, and (e) unverified last-mile behaviors (owner alert, checkout, freshness,
native-Albanian review). None of it is new construction. All of it is finishing.

---

## 2. The remaining 20%, itemized

Effort: **S** ≤ half a session · **M** ≈ 1 session · **OP** = operator-personal minutes (not agent
work). Every item names why it blocks "genuinely working" (not merely "demoable") and its VbM
proof with a defined RED.

| # | Gap | Why it blocks "genuinely working" | Effort | Dependency | VbM proof (GREEN / RED) |
|---|---|---|---|---|---|
| P1 | **Deploy the committed `/claim` fix** (`09dcbe05`) to staging | Every claim link ever minted is a dead link on every live surface; a dead claim at the decisive moment is negative-EV, not neutral | S (deploy + verify; the code work is done) | none (staging path is agent-allowed via `deploy-staging.sh`; **prod** delivery rides P5) | GREEN: `GET /claim` = 200 SPA shell on staging; `flow-simpl-s6-claim.spec.ts` passes. RED: today's live 404 (captured this session); falsifier `/claimx` must still 404 |
| P2 | **Rotate `PROVISION_OPS_SECRET`** (staging now; prod when P5 lands), store in SOPS vault | Without it: zero new demos (the 5 dead packet links stay dead) and zero claim tokens (the entire funnel throat) | **OP 5 min** | none | GREEN: `POST /internal/acquisition/claim/mint` = 200 token. RED: old/absent secret = 401/404 |
| P3 | **Restart the stopped prod worker** + investigate why | Stuck-PENDING orders never auto-cancel — a claimed owner's first bad order is unrecoverable trust damage | **OP 5 min** + S verify | none | GREEN: `flyctl status -a dowiz` worker `started` + a timeout-seeded staging order auto-cancels. RED: sweep doesn't fire on a synthetic stale order |
| P4 | **OG card ≤ 300KB (WhatsApp-safe)** — recompress `/og/<slug>.png` (JPEG or optimized PNG), keep 1200×630 | WhatsApp is the #1 remote rail (warm follow-up, ~98% open — MKT §5); at 652KB the card that "sells itself" likely doesn't render there at all | S–M | none (staging); prod via P5 | GREEN: 12/12 `curl -sI /og/<slug>.png` → 200, `image/*`, `content-length < 300000` + **operator paste-test screenshot in WhatsApp AND Telegram** (G11 3.4). RED: any card >600,000 bytes, or the paste-test shows a bare link / generic card |
| P5 | **Prod delivery of the funnel set** — `/claim` fix + OG commit set + demo provisioning on prod — via the **G01/Wave-1 merge vehicle only** | "Genuinely working" ultimately means a link on the canonical host; also removes the split-brain risk (claims landing in the staging DB) | rides G01 (operator-gated merge; multi-session program) | **G01/G02** (Master plan Wave 1; nuance: `fix/prod-blockers-P2` contains `6a89d6e8`, i.e. it sits on the paleo lineage — the `/claim`+G03 fixes are trivially re-appliable to the curated main-lineage PR, do NOT cherry-pick the branch wholesale) | GREEN: prod `/og/apollonia.png` = 200 `image/png` and prod `/public/locations/<slug>/info` = 200 for 12/12 (exact probes that 404 today). RED: today's prod probes (captured this session) |
| P6 | **Build the 5 missing demos** (colosseo, elia, pastarella, da-zio-libero, te-xhabarka) + provision arden/dogu/saly prospects | 5 of 6 ready offer packets carry a dead demo link — the packet's whole personalization premise ("could only be sent to this one person") fails | S–M (one command each, post-P2; quality gate may reject some — that's the gate working) | P2 | GREEN: per venue, `/public/locations/<slug>/info` = 200 + demo-builder's visual gate CERTIFIED (rendered items ≥3, never-orderable, noindex). RED: quality gate LOW_QUALITY exit, or visual gate needs-review — do not send that packet |
| P7 | **Freshness re-check before any send** — demos scraped ~07-01 (10 days old) | A wrong price or "Closed" at midday on the owner's own menu is an instant credibility kill (the known past-midnight `hours_json` bug class) | S per target venue | none | GREEN: 5-price diff vs live Wolt/Maps ≤ tolerance + phone matches + storefront renders "Open" during business hours. RED: any price mismatch on a pitch venue = fix via demo-builder before send |
| P8 | **Owner-alert + order-loop empirical verification** — run `/reliability-gate` (L0–L11) incl. the 07-04 `checkout-phone` item (G13 says it's test-debt/testid rename, not a product break — verify, don't assume), confirm owner Telegram alert ≤60s | W2 ("watch your pocket") is the activation event; a silent phone at the table/on the call is a killed deal | S–M | P1 deployed | GREEN: gate GO verdict + staging test order → owner TG alert with Confirm/Reject inside 60s. RED: alert absent → **hard stop on all outreach** |
| P9 | **Native-Albanian review of the drafts** (offer-builder output + the report-demos copy) | The playbook's own rule: anti-slop phrasing + a NATIVE pass; an "outsider" tone in a 3%-trust market closes the contact forever | **OP** (find/ask a native; ~1h) | none | GREEN: per-packet checklist box "Albanian verified by a native" ticked before send. RED: any packet sent with the box unticked is a process violation (checklist is the audit trail) |
| P10 | **Channel-stamped links in outreach artifacts** — mint `/s/<slug>?ch=wa\|fb\|tg\|qr` in packets, Telegram script, QR PNGs | Without it the pilot funnel can't attribute which channel produced the claim — the July walk-ins/sends ARE the survey; unattributed sends waste the information value | S | none (storefront already stamps `?ch=` per HUB §2) | GREEN: every outbound link in packets/QRs carries `?ch=`; grep of generated artifacts = 0 bare links. RED: a bare `/s/<slug>` in any generated artifact fails the check |
| P11 | **QR kit** — 20-line offline script (existing `qrcode@1.5.4` dep) → 12+N PNGs + per-venue A5 sheet | The physical trust artifact for walk-ins; also the leave-behind that makes a remote follow-up warm | S | P10 (bake `?ch=qr`) | GREEN: jsQR-decode each PNG equals the exact URL. RED: corrupted/wrong-URL QR fails decode-equals |
| P12 | **Funnel digest** — daily read-only counters to Telegram ops: claim_requests, claims accepted, published, orders with non-operator contact | Quality-first means measuring; the RED trigger (0 claims after N contacts) must be observable, not vibes | S | none | GREEN: digest posts with real counts, all-zero before wave 1 (seed check). RED: nonzero pre-outreach = instrumentation bug |
| P13 | **Domain** (`porosite.al` / `dowiz.app`, both NXDOMAIN) | Removes "staging.fly.dev" from every remote link — the trust tax matters MORE remotely than at a counter | **OP** ~€10–30 + S wiring | do not block wave 1 on it (G11 D5) | GREEN: `curl -sI https://<domain>/s/apollonia` = 200 + valid cert + OG absolute URLs on the new host. RED: cert/host mismatch |
| P14 | **Radar score upgrade** — fold the Wolt cost-pressure bit into the score; harden `aggregator-check` | Sharper targeting = fewer, better sends (quality-first) | S | none | GREEN: re-scored shortlist where a known-on-Wolt venue outranks an equal-rating non-Wolt one; check resolves for ≥80% of the top-30. RED: known Wolt venue (e.g. ArtePasta per MKT §1.3) reported "inconclusive" fails |

Explicitly **not** in the 20% (per the standing skip-lists — MAX-EV §7, G11, ONB §5): TMA/WhatsApp
Cloud API intake, video pipeline, dedicated demo app, Better Auth, payments/fiskalizimi code, paid
ads, Instagram-cold, messenger transports, `SHOW_CLAIM_CTA` flip (post-wave-1 only), it/pl locales
(A/B-graded rider, not a gate).

---

## 3. The "genuinely working system to send" bar — acceptance checklist

Mechanically checkable, each item falsifiable. **ALL must be green for the venue being sent to,
on the surface whose link is in the message** (staging until P5; prod after). This mirrors the
design blueprint's "stable enough to send" list and is the gate the operator asked for: quality
over speed.

| ✓ | Check | Mechanical probe (GREEN) | RED case (must be demonstrable) |
|---|---|---|---|
| 1 | **Demo link resolves — with data, not an SPA shell** | `GET {BASE}/public/locations/<slug>/info` = 200 with the venue's `name`; bot-UA `GET /s/<slug>` HTML contains the venue-specific `og:title` | a bogus slug must 404 on `/info` (guards the known prod false-green where the SPA 200s for ANY slug — VERIFIED-G11 S1) |
| 2 | **A claim completes end-to-end** | fresh token minted (P2 secret) → `flow-simpl-s6-claim.spec.ts` green against {BASE}: page loads, sign-in, accept, `owner_id` bound, `published_at` still NULL, token scrubbed from URL | the same spec run against a pre-P1 deploy 404s (recorded); an expired/reused token must be rejected |
| 3 | **The OG card unfurls in the target channels** | `/og/<slug>.png` = 200 `image/*`, `content-length < 300000`, correct per-venue `og:title`, `noindex,nofollow` present in bot HTML; **one real paste-test screenshot in WhatsApp + one in Telegram** per surface | >600KB (today: 652,906 — VERIFIED) or a generic card or missing noindex fails; stripping noindex must fail the assert (ethics floor of the P6-2/P6-3 waiver) |
| 4 | **The offer renders correctly in the send channel** | `report-demos-to-telegram.mjs --dry` exits ok, every message ≤ one phone screen (≤ ~1000 chars), link carries `?ch=`; for WhatsApp: `wa.me/<digits>` from the packet opens the correct chat; for Facebook: the venue's demo link posted on a test page/pinned-post renders the card (FB tolerates the current size — its limit is 8MB — but check anyway post-P4) | dry-run JSON not ok, message overflows a screen, `disable_web_page_preview` regression, or a dead `wa.me` number fails |
| 5 | **The menu is fresh** | per pitch venue: 5-price diff vs live Wolt/Maps ≤ tolerance; phone matches; storefront shows "Open" at a mid-day probe | any price mismatch / "Closed" at 13:00 fails; the hours check must red on a synthetic past-midnight `hours_json` |
| 6 | **The order loop is alive behind the demo** | `/reliability-gate` GO on {BASE}; staging test order → owner Telegram Confirm/Reject alert ≤ 60s; timeout sweep auto-cancels a synthetic stale order (worker running) | alert absent, gate NO-GO, or stale order still PENDING after the sweep window |
| 7 | **The unclaimed demo is honest and inert** | visual gate assertions: no add/cart affordance, honesty/demo banner present, noindex; decline path `POST /claim/decline` returns success | an orderable or banner-less unclaimed demo fails (never send a shadow that can take a real order) |
| 8 | **The message itself passes the human bar** | packet checklist fully ticked: native-Albanian verified, one REAL personal detail swapped in, correct ë/ç, honest "demo, not live yet" labeling, identity sign-off present | any unticked box = do not send (the checklist is the falsifier — it exists per packet, in the artifact) |

**The send rule:** a venue enters the send queue only when checks 1–8 are green *for that venue*.
The probe script for 1/3/5/6 should run as a daily canary during the outreach window (G11 risk 2:
staging churn), so a regression is caught before an owner taps, not after.

---

## 4. The remote-send flow — how an offer actually reaches an owner

Per the max-EV synthesis, remote is **not the opener** — walk-in is (+€1,155/90d vs −€510 for
cold-IG). Remote sends are three legitimate motions, in this order of expected value
(VERIFIED-MAX-EV/MKT rankings):

**R1 — Warm WhatsApp follow-up (after in-person contact or a prior "yes").** The packet's
`wa.me/<digits>` link → operator's PERSONAL WhatsApp → the 5-line Albanian message with the
venue's own `/s/<slug>?ch=wa` link → the OG card (post-P4) does the selling. Pricing lens: +€500
EV but sign-flips on reply rate — **test with 10 messages first**, pre-commit the N. One soft
follow-up max, ever.

**R2 — Referral relay (after venue #1 activates).** The activated owner forwards the link (or a
fresh demo minted same-day via `demo-from-wolt` for the friend's venue — the M6 reach multiplier).
The forwarded unfurl IS the offer; the operator's follow-up rides the introduction. 50–70% close
on referred B2B deals globally (VERIFIED-MKT as global stat) — the native channel of a 3%-trust
market.

**R3 — Facebook page placement (the owner's own distribution act, not operator spam).** Albania
is 95.6% Facebook by social referral (VERIFIED-MKT/Statcounter) — so the scripted "first
distribution act" for a *claimed* owner is the demo link as their page's Order button / pinned
post (`?ch=fb`), NOT an Instagram bio. Operator-side cold Facebook messaging is skip-listed;
Facebook is where the owner publishes, not where dowiz cold-DMs.

**Honesty + anonymity + no-spam invariants (all already encoded in the tooling — VERIFIED):**
- **No auto-send, structurally.** Every sender is preview/dry-run by default; `--send-invite` is
  an explicit flag; the offer packet is a document a human reviews and sends from their own
  account. Keep it that way — it is the anti-spam trust signal AND the legal posture (Art-14
  notice = a separate human act).
- **Honest labeling.** The message says it's a demo built from their public menu; the storefront
  carries the honesty banner + noindex; decline/erase honored same-day (`POST /claim/decline`
  live). "No money asked" is true and safe (relay-only + COD + venue fiscalizes on own POS —
  VERIFIED-G11 §2).
- **Anonymity/venue-level-only.** The DECLINED personal-dossier line stands; owner names only if
  self-published in a business context; packets are gitignored because they hold contact info.
- **Identity, not anonymity, on OUR side:** the operator-identity footer (PLAN.md WS3, not yet in
  code) plus the personal sign-off in the message — a real local person is the whole channel.

**Automated vs operator-personal (the division of labor):**

| Automated (agents/scripts) | Operator-personal (irreducible) |
|---|---|
| Radar sweep + scoring; prospect JSON; demo build + quality/visual gates; offer packet incl. LLM draft; OG cards + QR PNGs; freshness diffs; canary probes; funnel digest; dry-runs | Secret rotation + worker restart (Fly access); native-Albanian sign-off; the paste-tests from his own WhatsApp/Telegram; the decision to mint an invite; every actual send + follow-up; the walk-ins; domain purchase; the pre-committed N and the Day-7 verdict |

---

## 5. Sequencing — first remote sends vs polish, with VbM per wave

### Wave 0 — Un-break the hinges (same day; unblocks everything)
1. **P1** deploy `/claim` fix to staging (code already committed — deploy + run the spec RED→GREEN). *Dep: none.*
2. **P2** rotate `PROVISION_OPS_SECRET` (**OP, 5 min**). *Dep: none.*
3. **P3** restart prod worker (**OP, 5 min**). *Dep: none.*
4. **P8** reliability gate + owner-alert verification. *Dep: P1.*

*Wave-0 exit proof:* claim spec green on staging + mint 200 + gate GO + alert ≤60s. RED for the
wave: any of the four probes red = no outreach of any kind (the G11 rule: a broken first order
kills the venue).

### Wave 1 — Make the sendable set genuinely sendable (1–2 sessions; **gates the FIRST remote sends**)
5. **P4** OG ≤300KB + 12/12 OG probe + **paste-test in WhatsApp and Telegram** (the single
   session-new finding: without this, the flagship remote channel shows a bare link).
6. **P6** build the 5 packet demos + provision arden/dogu/saly (post-P2, one command each; accept
   quality-gate rejections as correct).
7. **P7** freshness re-check on every venue entering the send queue.
8. **P9** native-Albanian pass (**OP**) + **P10** `?ch=` stamps in all artifacts.
9. **P12** funnel digest live with the zero-baseline seed check; operator pre-commits N + deadline
   (G11 D6: N=10 contacts / 5 venues) BEFORE the first send.

*Wave-1 exit = the §3 checklist green for ≥3 venues.* First remote sends then proceed per §4 R1
(10-message WhatsApp warm test), while July walk-ins remain the primary motion — the two share
every artifact.

### Wave 2 — Prod-grade (rides the G01/G02 merge vehicle; do NOT block waves 0–1 on it)
10. **P5** `/claim` + OG set + 12 demos + G03 fix to prod via the curated main-lineage PR
    (re-apply `09dcbe05`'s diffs there; never merge the paleo-lineage branch wholesale);
    set prod `PROVISION_OPS_SECRET`; re-point packets/QRs/OG absolute URLs.
11. **P13** domain + cert + regenerate QRs/OG URLs.
12. Migrate any staging-claimed tenant to prod so no owner re-onboards (G11 risk 8).

### Polish (after first sends; never before)
13. **P11** QR kit print sheets (needed for walk-ins anyway — can float into Wave 1 if walk-ins
    start first). **P14** radar score upgrade. Operator-identity footer (S–M, staging deploy).
    `SHOW_CLAIM_CTA` flip only when warm outreach saturates (G11 D4). it/pl locales, savings
    counter — riders per MAX-EV, not gates.

**The one-line sequencing truth:** Wave 0 is two operator-minutes and one deploy away — everything
after it is finishing work on tools that already exist; the only thing that can fail the plan is
the same thing G11 named: the mornings not happening. The pipeline's job is to make sure that when
they do, nothing the owner taps is broken.

---

### Sources
Repo (verified this session unless marked): `scripts/radar-scout.mjs` · `scripts/offer-builder.mjs` ·
`scripts/demo-from-wolt.mjs` · `scripts/demo-builder.mjs` · `scripts/acquisition-bulk-provision.mjs` ·
`scripts/report-demos-to-telegram.mjs` · `tools/demo-builder/aggregator-check.mjs` ·
`apps/api/src/server.ts:858` · `apps/api/src/modules/acquisition/route.ts:143,160` ·
`apps/api/src/lib/og-card.ts` (working tree) · `loops/offers/*` (radar shortlist + 6 packets) ·
`loops/prospects/*` (14) · `loops/memory/offer-builder.md` · `e2e/tests/flow-simpl-s6-claim.spec.ts` ·
`e2e/scripts/provision-claim-shadow.mjs` · git: branch `fix/prod-blockers-P2`, commits `09dcbe05`,
`b5b03b9b`, `6a89d6e8` (`--contains` check). Live probes this session: `/claim` (404×2),
`/og/apollonia.png` (staging 200/652,906B; prod 404), `/public/locations/{apollonia,demo}/info`.
Same-day docs: `2026-07-11-MAX-EV-SYNTHESIS.md` · `2026-07-11-adoption-ev-onboarding.md` ·
`2026-07-11-adoption-ev-market.md` · `gap-blueprints-2026-07-11/{G11,MASTER-EXECUTION-PLAN}.md`.
Web: [Meta for Developers — WhatsApp link previews](https://developers.facebook.com/documentation/business-messaging/whatsapp/link-previews/) ·
[opengraphplus — WhatsApp image specs](https://opengraphplus.com/consumers/whatsapp/images) ·
[getlinkpeek — WhatsApp preview troubleshooting](https://www.getlinkpeek.com/blog/whatsapp-link-preview-not-working) ·
[ogrilla — WhatsApp preview guide](https://www.ogrilla.com/blog/whatsapp-link-preview-guide) (last three: UNVERIFIED aggregators, directionally consistent with the Meta primary).

*Authored 2026-07-11 by a read-only research session; the only file created is this blueprint.*
