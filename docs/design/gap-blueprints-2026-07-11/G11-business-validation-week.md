# G11 — Business validation: one week pointed at a real restaurant placing a real paid order

> **Date:** 2026-07-11 · **Gap owner doc:** audit `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md`
> §7.8 ("the repo's own sharpest self-criticism"), §5.5 (WS3/WS4 never built), §9 rec 9 ·
> **Strategy source:** `DeliveryOS-Business-Value-Sort.md` Tier-4 · **Status: RESEARCH + BLUEPRINT
> ONLY — nothing in this doc is implemented; the only file created is this one; no probe mutated
> any environment (read-only GETs, read-only git/DNS/flyctl status only).**
> Every claim is labeled **VERIFIED** (re-checked live/in code this session) or **CLAIMED**
> (from memory/docs, not independently re-checked here).

---

## 1. Gap & evidence

**The gap.** `DeliveryOS-Business-Value-Sort.md` (Tier-4, 🔴 "Сам процес-апарат") warned that
building and polishing the quality apparatus risks becoming *"витонченою заміною єдиного, що
валідує бізнес: реальний заклад, який провів реальне платне замовлення"* — an elegant substitute
for the one validating event: **a real restaurant putting a real paid order through the system.**
Its explicit recommendation: cap the apparatus at "safe to ship" and *"вивести заклад №1 наживо в
concierge-режимі"*.

**What happened since that warning (all VERIFIED in the 2026-07-11 audit):**

- A complete Rust rewrite was built dark (S1–S10, 69K LOC, 9/10 staged on staging 07-05, 0% in prod).
- A kernel was event-sourced (Sovereign Core 0b-1..0b-5 + 1.1/1.2/1.5 + 2.2/2.3, staging only).
- A post-quantum crypto library was hand-rolled in a brand-new repo (bebop2, 07-08→07-11 — the
  current center of attention).
- **No evidence exists of a single real (non-demo, non-operator) production order, or of a single
  claimed venue.** The 12 outreach demos remain shadow tenants (`owner_id NULL`). The OG-card
  outreach upgrades shipped to **staging only** (commit `6a89d6e8`). WS3 (QR, identity footer,
  domain, cold-start) and WS4 (video pipeline) were researched in
  `docs/design/demo-preview-upgrades/PLAN.md` (2026-07-06) and **never built** (audit §5.5).

**This is the largest risk in the project and it is not a code risk.** Every additional
infrastructure layer raises the cost of learning the only unanswered question: *will a Durrës
restaurant claim its demo and take a real order through it?* (audit §9 rec 9). The stack has
out-run the market test by roughly a month.

**New evidence found in this session — the funnel is not merely unexercised, it is broken at its
hinge:** the claim entry point `GET /claim` returns **404 on both prod and staging** (VERIFIED
live 2026-07-11), because `apps/api/src/server.ts:858` `SPA_ROUTES` never included `'/claim'`
(same on `origin/main:840`). Every claim link ever minted (`/claim#token=…`) is currently a dead
link. The React page exists (`apps/web/src/main.tsx:56` → `ClaimPage`), the API routes exist
(`POST /claim/accept|request|decline`, `apps/api/src/routes/public/claim.ts`) — the server just
never serves the SPA shell at that path. Nobody noticed for weeks because nobody ran the funnel.
That is the gap, in one fact.

---

## 2. Research findings — funnel-stage readiness, verified 2026-07-11

**The intended funnel** (reconstructed from `docs/design/demo-preview-upgrades/PLAN.md`,
`docs/research/2026-07-03-MAX-ROI-PLAN.md` init #2/#6, memories `og-preview-demo-upgrades-2026-07-06`,
`dubin-claimable-demo-2026-06-30`, `claim-happy-path-and-sops-2026-06-29`):

```
shadow demo (/s/<slug>, scraped Wolt/Maps data)
  → owner sees THEIR venue live (outreach message + rich OG unfurl + QR + trust signals)
    → claim (/claim#token=… accept — binds ownership+login ONLY; publish stays separate)
      → concierge onboarding (operator on-site: hours/phone/menu fixes, owner's own test order)
        → publish (owner's deliberate act) → first real order (cash; venue fiscalizes on own POS)
```

### Stage-by-stage readiness (all probes live, 2026-07-11)

| # | Stage | State | Evidence |
|---|---|---|---|
| S1 | Demos reachable | **STAGING-ONLY.** All 12 return HTTP 200 with full data on `dowiz-staging.fly.dev`. On **prod**, 11 of 12 do NOT exist — `GET dowiz.fly.dev/public/locations/apollonia/info` = 404 `NOT_FOUND`; bot-UA `/s/apollonia` = "404 Location not found". The human-UA 200s on prod are the SPA shell 200-ing for ANY slug (bogus slug also 200s) — a false-green. Prod has exactly one live storefront: `/s/demo` = "Dubin & Sushi" (published, generic OG). | **VERIFIED** live probes |
| S2 | Rich unfurl (owner's first impression) | **STAGING yes, PROD no.** Staging bot-UA `/s/apollonia`: `og:title "Apollonia — Menu Digjitale"`, `og:image=/og/apollonia.png` (200, `image/png`, 652KB, 1200×630), `noindex,nofollow` retained (the §3 substitute invariant holds). Prod: `/og/apollonia.png` = 404; `og-card.ts` **not on `origin/main`**. The rich-unfurl commit `6a89d6e8` sits on `feat/paleo-dinosaur-digs` / `feat/sovereign-core-phase-zero` / `backup-wip-2026-07-08` (two of these pushed) — **not on main** → prod is blocked on the bifurcated-history merge (the **G01 merge vehicle**). | **VERIFIED** live + `git branch --contains` |
| S3 | Claim entry | **BROKEN everywhere.** `GET /claim` = 404 on prod AND staging (`/login`, `/admin`, `/privacy` all 200 — the allowlist gap is specific). Root cause: `SPA_ROUTES` at `server.ts:858` omits `'/claim'`. One-line fix. The e2e spec `e2e/tests/flow-simpl-s6-claim.spec.ts` does `page.goto('/claim#token=…')` → it would FAIL today, i.e. the falsifiable RED case already exists in-repo. On-page CTA is hidden (`SHOW_CLAIM_CTA = false`, `MenuPage.tsx:454`, owner directive 2026-07-01 — banner pitches instead; the `POST /claim/request` handler is kept intact). Token minting requires `PROVISION_OPS_SECRET`; the staging value was rotated to a **lost throwaway** on 06-29 (memory) → no new claim token can be minted until the operator re-rotates. Claim-token TTL = 3 days. | **VERIFIED** live + code; secret state CLAIMED (memory) |
| S4 | Concierge onboarding | Claim binds ownership only; go-live is a separate deliberate act in /admin (PROTECTED-FRICTION, council CC2/CC3 — correct design for concierge). Onboarding choreography (MAX-ROI init #6: menu import → owner's own test order → printed QR) is researched, not built — acceptable, concierge is human by definition. | **VERIFIED** code comments in `ClaimPage.tsx`; choreography CLAIMED |
| S5 | First-order loop | Prod product is live and healthy (audit §4.1), BUT: (a) prod **worker machine is `stopped`** since 07-03 (`flyctl status -a dowiz`) — the worker owns the order-timeout auto-cancel sweep (`apps/worker/src/handlers.ts`); owner Telegram alerts are safe (consumer lives in the API process, `apps/api/src/bootstrap/workers.ts:57`), but stuck-PENDING orders would never auto-cancel; (b) the live 6-kind/3-kind checkout 422 (G03) fails Phone/Signal/SimpleX customers — real conversion leak; (c) a staging checkout break was flagged 07-04 (`checkout-phone` testid) and **never closed** (audit §6.4) — must re-verify before any owner sees a demo. Reliability gate L0–L11 last PASSED on staging 07-07. | **VERIFIED** flyctl/code; (c) CLAIMED-open |
| S6 | Data freshness | The 12 demos carry real scraped data: Google ratings 4.6–5.0, real phones, menus of 19–107 items (per-venue table below), scraped ~07-01 (10 days old — fine for a pitch; spot-check prices before outreach). `locations` has no `updated_at` column, so freshness is by provenance, not timestamp. | **VERIFIED** live `/info` + `/menu` for all 12 |

### Per-venue outreach data (staging, live 2026-07-11)

| Slug | Name | Google rating | Menu entries | Phone on file |
|---|---|---|---|---|
| artepasta | ArtePasta | **5.0** | 61 | yes |
| apollonia | Apollonia | 4.9 | 107 | yes |
| demo | Dubin & Sushi | 4.9 | 66 | yes |
| idua | Idua | 4.9 | 70 | yes |
| aragosta | Aragosta Restaurant | 4.8 | 71 | yes |
| liriada | Liriada | 4.8 | 84 | yes |
| otantik | Otantik | 4.8 | 89 | yes |
| ventus | Ventus Harbor | 4.8 | 55 | yes |
| dyrrah-mare | Dyrrah Mare | 4.7 | 69 | yes |
| casa-mia | Trattoria Casa Mia | 4.6 | 89 | yes |
| lamuse | L'Amuse | 4.6 | 29 | yes |
| eljos-pizza | Eljo's Pizza | (none) | 19 | yes |

### WS3 inventory — what exists vs what's missing

| WS3 item | Actual state | Load-bearing for a first claim? |
|---|---|---|
| **QR per demo** | `qrcode@1.5.4` is already a dep (`apps/web/package.json:21`); an admin `QRKitPage` exists but is dark behind build-time `VITE_CHANNEL_KIT_ENABLED` (**VERIFIED**). For outreach, a 20-line script generating 12 QR PNGs offline is enough — **no deploy needed**. | **YES** (handout = trust signal per PLAN.md WS3) — trivially cheap |
| **Operator-identity footer** | Designed in PLAN.md WS3 ("Powered by Syniak Sviatoslav · Local Developer"); **not in code** (grep: no such footer in `ssr-renderer.ts`/SPA). | **YES-ish** — it is the "developer passport" anti-spam trust signal; S-effort |
| **Own domain porosite.al** | **NXDOMAIN — unregistered** (dig: no A/NS; only the `.al` AKEP SOA in authority). `dowiz.app` also unregistered. `.al` is administered by AKEP via accredited registrars — registration is an **operator-personal** purchase (~€10–30/yr; verify registrar/foreign-registrant terms). | **NO for claim #1** (a warm intro tolerates fly.dev), **YES for cold outreach at scale** — large trust delta, near-zero effort |
| **Fly cold-start warm** | **Already satisfied by config on prod** (`fly.toml:25` `auto_stop_machines = false`); measured warm TTFB 0.10–0.11s on BOTH apps (3 samples each); staging machines `started` (v278). The staging deploy-draft even carries `min_machines_running=1`. One residual check: confirm the live staging app's own machine config once. | **NO** — effectively done; verify, don't build |

### Market + compliance context (grounding, from the repo's own 07-03 research — 8 days old)

- **Durrës delivery market** (R10, `2026-07-03-albania-gap-analysis.md`): **Wolt** live in Durrës
  since Mar 2025 (50+ venues); **Baboon** is the local incumbent (since 2016, 1,000+ venues,
  Tirana/Durrës/Korça/Vlorë, profitable); **Glovo never launched** in Albania; Bolt Food absent.
  Commissions: 20–35% general industry range (Albania-specific rates undisclosed). Implication:
  Durrës owners *already know aggregators* — the 0%-commission/own-channel pitch lands on a real,
  current pain, and the 12 demos being Wolt-sourced proves each target is already delivery-active.
- **Fiskalizimi (Law 87/2019) — the legal read on a REAL paid order:** every sale must be
  fiscalized (NIVF/QR), no exemption; dowiz has zero fiscal code (G1 BLOCKER for *sellability at
  scale*). **BUT the relay-only posture is viable today**: the venue fiscalizes on its **own
  existing POS** (legal AL vendors already do), dowiz only transmits the order; cash settles
  directly customer→venue; dowiz charges nothing during the pilot. → **No legal blocker to order
  #1 under concierge/relay-only**, provided the venue issues its own receipt. Phase-0 accountant
  confirmation of the relay regime (MAX-ROI #3) is advisable *before charging subscriptions*, not
  before order #1. (CLAIMED from R4 — primary-source-fetched there; re-confirm with the accountant.)
- **Legal entity:** `compliance/README.md` controller = **"[SHPK назва ____]" — an unnamed
  placeholder** (VERIFIED). Not a blocker for a free pilot order; it IS a blocker for invoicing a
  subscription. Crypto payments (Plisio, ADR-0017) are fully dark behind default-OFF flags with
  their own NEEDS-HUMAN launch gate — **irrelevant to a cash order #1**.
- **Privacy posture of the demos:** the P6-2/P6-3 rich-unfurl override was operator-waived on
  07-06 with five non-negotiable substitutes; staging currently honors `noindex,nofollow`
  (VERIFIED in live bot HTML) and the honesty banner is in `MenuPage.tsx` (VERIFIED code). One
  accepted-risk stays open: re-hosted Google-sourced hero photos (Maps ToS, operator accepted
  2026-07-01).

---

## 3. Options & tradeoffs — where does the outreach surface live this week?

**Option A — Prod-first: provision the 12 demos on prod + merge rich-unfurl/claim-fix to main.**
- *Pro:* durable, one canonical URL, prod DB rows = unambiguous GREEN evidence.
- *Con:* gated on the **G01 rewrite-aware merge** (bifurcated history, 500+ add/add conflicts on a
  naive attempt — operator-gated, multi-day) — and per audit rec #1 the GDPR trio must ride the
  same-or-earlier merge. Prod provisioning also needs `PROVISION_OPS_SECRET` set on the prod app
  (unknown/likely unset) and prod's *current* code would unfurl the demos GENERIC (the invariant
  reversal isn't on main). **Slowest path to first contact — days of prerequisite work.**

**Option B — Staging-as-outreach-surface (interim), prod rides G01 when it lands.**
- *Pro:* staging **already has everything**: 12 fresh demos, rich OG cards, pitch banner, the Rust
  S1 storefront serving `/s` — verified live today. The only funnel break (`/claim` 404) is a
  one-line fix deployable via the normal agent-allowed staging path (`scripts/deploy-staging.sh`).
  Outreach tooling (`report-demos-to-telegram.mjs`) already defaults `BASE=staging`. First contact
  possible **Day 2**.
- *Con:* hostname says "staging" (mitigate: warm in-person outreach doesn't parse subdomains; a
  custom domain later masks it entirely); staging is redeployed from shifting lineages (mitigate:
  declare a **demo-freeze** on staging for the outreach week — operator rule); E2E runs have
  polluted demo data before (the "E2E-Test-Location-…" rename incident — freeze covers this too).

**Option C — Dedicated demo app** (`dowiz-demo` Fly app or the new domain → pinned image).
- *Pro:* isolates outreach from both prod and staging churn.
- *Con:* new app + secrets + DB provisioning from scratch this week; solves a problem (staging
  churn) that a one-week freeze solves for free. Right move *later* if outreach scales; overkill
  before claim #1.

**Recommendation: B now, A as ride-along.** Fix `/claim` + verify the loop on staging (Day 1),
outreach off staging links (Days 4–7), and put the prod-bound set — `'/claim'` one-liner, the
rich-unfurl commits, the G03 messenger-kind fix — on **G01's merge manifest** so prod inherits the
funnel the moment the merge vehicle ships. Do NOT let the week block on G01.

---

## 4. Recommended one-week blueprint (day-by-day)

Owner legend: **AG** = agent work · **OP** = operator-personal (human) work · effort S ≤half-day,
M ≤2 days. Every step carries a falsifiable VbM proof (a defined RED input) per
`docs/operating-model/verified-by-math.md`.

### Days 1–2 — unblock the funnel (agents, operator gates only where marked)

| # | Action | Owner | VbM proof (GREEN / RED) | Effort |
|---|---|---|---|---|
| 1.1 | **Fix the claim entry:** add `'/claim'` to `SPA_ROUTES` (`apps/api/src/server.ts:858`); commit on the staging lineage; deploy staging | AG | GREEN: `GET /claim` = 200 serving the SPA shell. RED (exists today): the same probe currently returns 404 — capture the before/after pair. Falsifier: a bogus path (`/claimx`) must still 404 | S |
| 1.2 | **Rotate `PROVISION_OPS_SECRET` on staging** (`flyctl secrets set` — write op, operator-only), store via the SOPS vault | **OP** (5 min) | GREEN: `POST /internal/acquisition/claim/mint` with the new secret returns a token. RED: the old/absent secret returns 401 | S |
| 1.3 | **Prove the claim flow end-to-end with the existing spec:** mint a token for a **scratch shadow tenant** (via `e2e/scripts/provision-claim-shadow.mjs` — never a real-venue tenant), run `flow-simpl-s6-claim.spec.ts` against staging | AG | GREEN: spec passes (claim → ownership bound, publish still NULL, token scrubbed from URL). RED: the spec demonstrably fails against the pre-1.1 deploy (it 404s) — run it once before the fix to record the red | S |
| 1.4 | **Re-verify the order loop on the demo surface:** run the `/reliability-gate` skill (L0–L11) against staging `/s/demo` incl. the open 07-04 `checkout-phone` break; fix ONLY what the gate reds (checkout is Tier-1; nothing else) | AG | GREEN: gate GO verdict pasted. RED: gate is falsifiable by construction (each L-step asserts a real DOM/API state) | S–M |
| 1.5 | **Prod hygiene pair:** investigate + restart the stopped prod `worker` machine (timeout sweeps dead since 07-03); confirm owner Telegram alert fires on a staging order (it's consumed in the API process — verify empirically, not by code-read) | **OP** start/approve; AG verify | GREEN: `flyctl status -a dowiz` shows worker `started` + a staging test order produces an owner Telegram alert within 60s. RED: alert absent → escalate before any outreach | S |
| 1.6 | **G01 ride-along manifest (dependency note):** hand G01 the prod-bound list — `'/claim'` fix, rich-unfurl set (`6a89d6e8` + Rust `storefront.rs` tags), G03 messenger-kind fix, GDPR trio (already rec #1). **OG cards reach prod ONLY via G01's rewrite-aware merge vehicle — do not attempt an independent cherry-pick onto the bifurcated main.** | AG (list) / **OP** (merge itself, out of this week's critical path) | GREEN when G01 ships: prod `/og/apollonia.png` = 200 `image/png` (exact probe that 404s today) | S (list only) |
| 1.7 | **OG/unfurl regression net for all 12:** scripted probe asserting per-slug `og:title` + `/og/<slug>.png` = 200 `image/png` + `noindex` present (the §3 substitute invariant must HOLD — it is the ethics floor of the override) | AG | GREEN: 12/12 pass. RED: a bogus slug must return non-image; stripping `noindex` must fail the assert | S |

### Days 3–4 — outreach instruments (only the load-bearing subset of WS3)

| # | Action | Owner | VbM proof | Effort |
|---|---|---|---|---|
| 3.1 | **QR per demo:** offline script (existing `qrcode` dep) → 12 branded PNGs (`/s/<slug>` target; regenerate later if the domain lands) + a one-page per-venue print sheet (QR + link + 3-bullet pitch in Albanian from the `report-demos-to-telegram.mjs` copy) | AG | GREEN: decode each PNG programmatically (jsQR) and assert it equals the exact URL. RED: a corrupted/wrong-URL QR fails the decode-equals assert | S |
| 3.2 | **Operator-identity footer** (PLAN.md WS3): "Powered by Syniak Sviatoslav · Local Developer" + contact, in `ssr-renderer.ts` footer + SPA storefront footer, demo-class tenants only, small flag default-ON-for-demos; deploy staging | AG | GREEN: Playwright asserts footer text on `/s/apollonia` (staging). RED: flag off → footer absent (assert both states) | S–M |
| 3.3 | **Domain** — register `porosite.al` (or fallback `dowiz.app`, instantly registrable), point at the outreach app, `flyctl certs add`, set `APP_BASE_URL`; regenerate OG absolute URLs + QRs | **OP** purchase (personal card/registrar account; verify AKEP-accredited registrar terms for foreign registrants) + AG wiring | GREEN: `curl -sI https://porosite.al/s/apollonia` = 200 with valid cert + og:image URL on the new host. RED: cert or host mismatch fails the probe | S ops |
| 3.4 | **Telegram outreach dry-run:** `node scripts/report-demos-to-telegram.mjs --dry` with previews on; paste-test ONE real unfurl into a private chat (WhatsApp + Telegram) to confirm the card renders | AG dry-run / **OP** the real paste-test (his accounts) | GREEN: dry-run JSON ok + screenshot of a rendered card. RED: `disable_web_page_preview` regression or generic card = fail | S |
| 3.5 | **Data spot-check before contact:** for the top-3 target venues, compare 5 menu prices + phone + hours against current Wolt/Maps; fix drift via the demo-builder path. Also verify the venue doesn't render "Closed" mid-day (known past-midnight `hours_json` bug, memory 07-01) | AG | GREEN: diff report ≤ tolerance; storefront shows open during business hours. RED: any price mismatch on the pitch venue = fix before outreach | S |

### Days 5–7 — the human loop (outreach is operator-personal work; agents prep and instrument)

| # | Action | Owner | VbM proof | Effort |
|---|---|---|---|---|
| 5.1 | **Concierge onboarding script (Albanian):** agent drafts the door-knock/DM flow from the existing pitch copy + R2 trust research: open as a regular customer, show THEIR live storefront on the phone, hand the QR sheet, offer claim-on-the-spot (fresh token minted same-day — TTL is 3 days), promise concierge setup, explicitly do NOT push publish (protected friction is a feature: "you go live only when you say so") | AG draft / **OP** owns the words | GREEN: script exists + operator has rehearsed it once. (Human step — proof is the artifact + dry run) | S |
| 5.2 | **Target order (who first, and why):** ① **ArtePasta** — rating 5.0, the deepest-enriched demo (real hero photo, matched fonts, real address/hours — memory 07-01), small Italian venue = single decision-maker; ② **Dubin & Sushi** — operator provisioned it personally on 06-30 (existing relationship signal; prod `/s/demo` already carries it); ③ **Apollonia** (4.9, 107-item menu; hotel restaurant = slower org, hence third). Avoid opening with Eljo's Pizza (no rating, 19 items — weakest demo). Steer every onboarded owner/customer to **Telegram/WhatsApp/Viber** contact kinds until G03's fix is in prod (the other 3 kinds 422) | **OP** decision, AG data | The ranking table in §2 is the evidence; operator may override on relationship strength — relationship beats rating | — |
| 5.3 | **Outreach wave 1 (in person, Durrës):** 3–5 venues from the ranked list, staging links + QR sheets. **This is irreducibly human work — no agent substitutes.** Agent on-call same-day for data fixes (menu corrections, hours, phone) | **OP** | Funnel counters (5.4) move — or don't. Both outcomes are data | 5.2+5.3 = the week's critical path |
| 5.4 | **Instrument the funnel (measured pilot, Business-Value-Sort Tier-5):** small read-only script → daily Telegram ops digest: demos visited (if measurable), `claim_requests` rows, claims accepted (`owner_id NOT NULL` on demo class), published venues, orders with **non-operator** customer contact | AG | GREEN: digest posts daily with real counts. RED: counters must be provably zero-based — seed check: before outreach all = 0; any nonzero pre-outreach = instrumentation bug | S |
| 6.1 | **First claim + concierge session:** operator on-site; owner claims on their own phone; concierge fixes data; owner places their OWN test order (the "aha" — this is NOT the green event, it's operator-adjacent); publish when THE OWNER says go | **OP** (AG on-call) | Claim: `claim_transfer` accepted + `owner_id` set + owner logged in on their device. Test order: order row exists with owner's phone | — |
| 7.1 | **First real order attempt:** owner shares their link/QR to regulars (their own IG bio/story — their channel, their customers); operator does not place or seed the order | **OP** venue-side | **THE GREEN EVENT** (below) | — |
| 7.2 | **Week retro vs the stop/pivot trigger:** score the funnel honestly against the RED definition; write the memory entry + re-sort Business-Value-Sort rows with real pilot data (its own instruction) | AG draft / **OP** verdict | The GREEN/RED ledger below, filled in with real counts | S |

### The validating metrics (VbM discipline)

- **GREEN (the validating event):** ≥1 order row on the outreach surface where **(a)** the venue is
  a claimed demo (`owner_id NOT NULL`, claim accepted by a non-operator identity), **(b)** the
  customer contact (phone/messenger) is **not** the operator's or the owner's own test identity,
  **(c)** the order reaches ≥ CONFIRMED (DELIVERED = full validation of the loop), **(d)** payment
  = cash on delivery settled venue-side (relay-only posture). Proof = the DB row + the owner's
  confirmation, pasted. *Falsifiable:* the same query over the pre-outreach window must return 0.
- **Honest RED (what falsifies what):**
  - **0 claims after ≥10 personal contacts across ≥5 venues** → falsifies "a shadow demo converts
    cold/warm Durrës owners at the current pitch" — the Tier-1 wedge assumption, not a code gap.
    Consequence per Business-Value-Sort: STOP building, re-sort — pitch/price/channel pivot
    (e.g., lead with fiskalizimi-relay pain, or hybrid-GTM QR-in-the-bag motion), не new infra.
  - **Claims but 0 real orders in 72h post-publish** → falsifies "owner activation → customer
    flow" — diagnose the conversion funnel (owner's channel reach, checkout friction, the G03
    422 class), still not new infrastructure.
  - **An owner declines + asks for erasure** → not a red; the decline path working IS part of the
    ethics floor. Honor it same-day (`POST /api/claim/decline` exists — VERIFIED).
- **N is pre-committed:** the operator writes the N (contacts) and the deadline into the memory
  entry BEFORE wave 1, so the red cannot be quietly reinterpreted (cognitive-bias rule).

### What NOT to build this week (explicit, per Business-Value-Sort Tier-3/4)

- **WS4 video-montage pipeline** (Revideo/FFmpeg/Whisper) — polish, not load-bearing for claim #1.
- **WS2 per-venue identity video overlay** — gated anyway; skip.
- **Fiskalizimi integration** — one Phase-0 accountant CALL (OP, 30 min) to confirm relay-only;
  zero code.
- **Card/crypto payments** — cash only; the dark Plisio vertical stays dark.
- **Astro FE parity, rebuild cutover tail, Sovereign Core 1.3/1.4/2.x, bebop2 crypto** — explicit
  one-week pause; each is Tier-4-or-parked relative to validation (audit §7.1 four-futures risk).
- **Promotions redemption, advanced analytics, custom domains per tenant, more harness/gates** —
  Tier-3/4 as sorted.
- **A dedicated demo Fly app** (Option C) — only if staging churn actually bites during the week.
- **Any new hand-rolled infrastructure whatsoever.** The week's engineering budget is: one line in
  `SPA_ROUTES`, one footer, one QR script, one probe script, one funnel digest.

---

## 5. Risks

1. **Operator time is the critical path.** Days 5–7 are human outreach; no agent can substitute.
   If the operator's week collapses into bebop2/crypto again, the blueprint fails by default —
   that is precisely the Tier-4 pattern this gap names. Mitigation: the arbiter doc (G07) should
   rank this week explicitly; agents front-load ALL prep so outreach needs zero setup.
2. **Staging churn during outreach week.** Staging is the demo surface; a redeploy from another
   lineage mid-week could regress OG/claim/checkout. Mitigation: operator declares a demo-freeze
   (no staging deploys except this blueprint's fixes); the 1.7 probe script runs daily as a canary.
3. **The staging checkout break (07-04, unclosed).** If 1.4's gate reds on checkout and the fix is
   non-trivial, the week's Day-1 buffer absorbs it — but it MUST be green before any owner sees
   the demo. A broken order on the owner's first try kills the venue permanently (Tier-1: "втрачене
   замовлення о 19:00 = втрачений заклад").
4. **Privacy/ethics exposure of the unfurl override.** Outreach broadcasts unconsented venue
   identity into chats. The five substitutes (honesty banner, noindex, demo-class scope, prominent
   decline/erase, flag-reversibility) are the floor — 1.7 asserts noindex; decline must be honored
   same-day. A venue's public complaint would damage exactly the local-trust asset the pitch runs on.
5. **Google-photo ToS (accepted risk, 07-01).** Re-hosted Maps hero photos are on the pitch
   venues. If an owner objects, swap to their provided photo on the spot (concierge fix).
6. **Claim-token handling.** TTL 3 days, single-use; a dead token at the table is an embarrassing
   failure mode — mint same-day (1.2's rotated secret makes this possible) and test one token the
   same morning.
7. **"Staging" in the URL.** Low risk for warm in-person outreach; the domain (3.3) removes it.
   Do not let domain registration delays block wave 1.
8. **Prod/staging split-brain evidence.** The GREEN event lands in the **staging** DB under Option
   B. That is acceptable evidence for validation (a real venue + real customer + real cash), but
   record it verbatim in memory; migrate the tenant to prod when G01's vehicle ships so the venue
   never has to re-onboard.
9. **G01 slippage.** If the merge vehicle doesn't ship this week, prod stays funnel-less — but
   Option B means validation does not wait on it. The dependency is one-way by design.

---

## 6. Operator decision points

| # | Decision | Default recommendation | When |
|---|---|---|---|
| D1 | Approve **Option B** (staging as the outreach surface this week) + declare the staging demo-freeze | YES — it is the only path to first contact this week | Day 1 AM |
| D2 | Rotate `PROVISION_OPS_SECRET` on staging (`flyctl secrets set`) and store it in the SOPS vault | YES — nothing in the claim funnel moves without it | Day 1 |
| D3 | Restart/diagnose the stopped prod `worker` machine | YES (5-min check; timeout sweeps are dead on prod since 07-03) | Day 1 |
| D4 | `SHOW_CLAIM_CTA` flip (MenuPage.tsx:454) — expose the on-page "Is this your restaurant?" CTA for the demo class, or keep claim token-only via personal outreach | Keep **token-only** for week 1 (concierge = higher-touch, better data); flip the CTA when outreach goes cold/at-scale | Day 2 |
| D5 | Domain: buy `porosite.al` (registrar/legal-presence check) vs fallback `dowiz.app` vs skip this week | Buy whichever completes in <1 day; do NOT block wave 1 on it | Day 3–4 |
| D6 | First-venue pick + pre-commit the outreach N and deadline (the RED trigger) in a memory entry BEFORE wave 1 | ArtePasta → Dubin & Sushi → Apollonia; N=10 contacts / 5 venues by Day 7 | Day 4 |
| D7 | Fiskalizimi Phase-0: 30-min accountant call confirming relay-only posture (venue fiscalizes on own POS) for the pilot | Do it this week in parallel — it de-risks the *charging* conversation later | Day 3–5 |
| D8 | G01 merge manifest sign-off (claim fix + OG set + G03 fix + GDPR trio ride the same vehicle) | Sign the list; execute per G01's own blueprint timeline | Whenever G01 lands |
| D9 | If RED fires: commit to the Business-Value-Sort stop/pivot consequence (re-sort with pilot data; pivot pitch/channel; do NOT return to infrastructure by default) | Pre-commit now, in writing — that is the entire point of the trigger | Day 7 retro |

---

*Blueprint authored 2026-07-11 by a read-only research session. Live probes: prod/staging HTTP
GETs, `dig`, `flyctl status` (read-only) only. The only file created is this document.*
