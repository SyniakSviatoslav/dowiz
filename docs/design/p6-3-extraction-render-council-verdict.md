# Triadic Council Verdict — P6-3 (menu extraction → products write + labeled preview render)

**Date:** 2026-06-28 · **Seats:** system-architect · system-breaker · counsel · all grounded against live source.
**Stage:** Council-light on a 🔴 RLS **+ AI/PII** red-line, design-only (no code).
**Proposal:** scrape a restaurant's public site → AI-parse the menu → write products into the shadow tenant →
render an honest labeled preview. Extends P6-2's provisioning mechanism. Operator decisions binding: #2 no
pre-claim allergens; #4 EXTRACT EVERYTHING + owner-approval-at-claim, **PII-redaction-before-AI NOT waived**;
facts-only/Places-API+own-site provenance, **no verbatim creative copy / scraped photos**, honor robots.txt.

## CONSOLIDATED VERDICT: **APPROVE-WITH-CONDITIONS — but NOT ready to code.**
The pipeline shape is sound and inherits P6-2's proven mechanism (fold the products write into the existing
single-tx `ENRICHED→PROVISIONED` chokepoint; extend `provision_shadow` to products/categories). BUT the
proposal's core "reuse the existing redactor/port — redaction inherited" premise is **factually broken**, two
CRITICAL safety/PII blockers are unbuilt, the labeled-render path has an architectural hole, and counsel
raises one narrow ETHICAL-STOP. **2 operator decisions are required before code** (render scope + a strategic
re-confirmation). Agent-health GREEN (the worker self-disclosed the PII gap, the advisory confidence gate, and
asked Q1/Q2/Q5 openly).

---

## 🔴 CRITICAL BLOCKERS (must be built + proven before P6-3 ships)

### C1 — Free-text NAME PII egress to the external LLM (breaker CRITICAL; the binding 🔴 AI/PII line)
Two compounding facts: (1) `AiOcrParser.parse()` only branches `kind==='pdf'|'image'` and **throws** on any
other kind (`ai-ocr-parser.ts:392-393`), so the net-new HTML-scrape path **cannot** "inherit" the `:404`
redaction — it's net-new code that doesn't exist. (2) Even when applied, `PiiRedactor` has **only 5 patterns**
(email/url-with-query/iban/card/phone, `pii-redactor.ts:10-33`) and **no name/person pattern** — so scraped
`"Meet our chef Maria Hoxha"` / testimonials / `"Owner: Jeton Berisha"` pass **verbatim** into the prompt
(`:520`) to OpenRouter/Zen. **Fix (architect + counsel agree on the shape):** add an `html`/`text` branch
*inside* `parse()` that runs a **menu-region ALLOWLIST** (positively select the menu DOM/PDF-region; everything
not affirmatively menu never reaches the AI — denylist of About/footer is too leaky) + html→text, then
**falls through to the single `:404` redaction** (one redactor, all kinds converge — do NOT replicate it). A
name-guard-flagged PII-dense region → MANUAL_REVIEW/MENU_NOT_FOUND (**fail-closed**). Guardrail: a
**redaction-recall fixture with a stated recall floor**, red→green. This control is the *entire* PII defense
and is currently unbuilt.

### C2 — Allergen-as-fact (breaker CRITICAL; consumer-safety red-line + decision #2)
The LLM is prompted to guess allergens into `attributes.bom[].allergens` (`ai-ocr-parser.ts:502-503,516`);
`read_public_menu` emits `p.attributes` wholesale (`1790000000065:113`); the FE derives the allergen surface
from `attributes.bom[].allergens` (`product-mapper.ts:6`). So an AI hallucination ("allergens: []" on a nut
dish) publishes as fact. The proposal's `source/allergens_confirmed` column gate is **insufficient twice
over**: those columns are added by migration **068 (still STAGED, not live)**, AND allergens are nested in the
opaque `attributes` jsonb so a column gate doesn't strip them. **Fix (architect, two homes):** (a) **WRITE-time
strip** — null every `bom[].allergens` before INSERT (keep ingredients per decision #4; the unverified allergen
claim is never persisted); (b) **READ-time gate** — re-version `read_public_menu` with
`CASE WHEN p.source='place' AND p.allergens_confirmed=false THEN p.attributes - 'bom' ELSE p.attributes END`.
Server-authoritative invariant + red→green guardrail (counsel: a safety floor, not a flag).

---

## 🟠 HIGH BLOCKERS

### H1 — Render-path hole: the labeled preview has no way to read the shadow menu (architect #3 + breaker HIGH)
`read_public_menu` serves only `status IN ('active','open') OR published_at IS NOT NULL`
(`1790000000065:41-42`) — a `closed`/`published_at NULL` shadow returns NULL, so §D's "render the menu" has no
data path. Relaxing that WHERE leaks the shadow through the public JSON API (`public/menu.ts:77`, no banner, no
noindex) + SSR + sitemap. **Fix:** a SEPARATE `read_preview_menu(slug)` SECURITY DEFINER fn admitting ONLY
shadows (`owner_id IS NULL` via org join, `status='closed'`, `published_at IS NULL`), applying the `- 'bom'`
allergen strip + **names+prices-only** (see ETHICAL-STOP), called ONLY by the labeled render. Do NOT widen
`read_public_menu`.

### H2 — Products `provision_shadow` token-only policy WIDENS cross-tenant writes (breaker HIGH / Q6)
A token-only `WITH CHECK` on `products` (mirroring `menu_versions`) binds nothing to the shadow — a valid token
could INSERT products into a **victim real tenant's** `location_id`. **Fix:** bind the carve-out —
`WITH CHECK ( source='place' AND location_id IN (SELECT l.id FROM locations l JOIN organizations o ON o.id=l.org_id
WHERE o.owner_id IS NULL) AND <tokenValid> )`. The `source='place'` first conjunct short-circuits owner INSERTs
(default `source='owner'`) so the shadow subquery is never paid on the owner hot path. Categories bind via their
`location_id` the same way. Re-prove on the P6-2 NOBYPASSRLS harness (extend `provision-rls.test.ts`): place
products admitted, no-token rejected, `source<>'place'` rejected, **non-shadow location_id rejected**, owner
path unaffected, FK order (categories before products), no RETURNING.

### H3 — Labeled render regresses B2: real name/OG re-exposed to unfurlers (breaker + counsel + architect)
Rendering the menu re-emits the real venue name + likely real OG (`ssr-renderer.ts:236-250`); `noindex` does
NOT stop social unfurl (the exact B2 finding). **Fix:** the labeled preview keeps **generic OG** (no real
name/logo/address in og:* tags) + `noindex` + the server-authoritative banner; the real name appears only in
the on-page labeled body, never in unfurl metadata. Prove OG stays generic.

### H4 — No-fabrication gate is advisory, not enforced (breaker HIGH + architect #5)
Confidence is warning-only (`ai-ocr-parser.ts:389,562`), grounding is default-OFF (`:574`), and the prompt
orders price invention (`:515`). With **no human in the loop** (approval is at-claim, AFTER the public preview
exists) a low-confidence parse becomes a real-looking storefront. **Fix:** wire the shipped confidence floors
to the state machine via `advance()`/`REQUIRES_REASON` — `locate` empty → MENU_NOT_FOUND; 0 items / any
`severity:'error'` / OCR<0.4 / LLM→heuristic fallback (`:527`) → LOW_QUALITY (required reason). `ENRICHED` is
the only door to `PROVISIONED`, so "low confidence cannot write" becomes structural. Reuse the existing floors,
not a new constant.

---

## 🟡 MEDIUM (carry into the build)
- **M1 — `hardDeleteShadow` leaves the PII it ingests** (breaker MED — a real bug in shipped P6-2 code):
  `provisioning.ts:147` nulls org/location FKs + deletes the tenant but leaves `acquisition_sources.place_raw`
  + `menu_draft` (extracted phone/incidental names). **Fix:** the erase must also clear `place_raw`/`menu_draft`
  on the source row; add a `place_raw`/`menu_draft` short-retention TTL (counsel C5 / Art-5(e)).
- **M2 — copyright/descriptions** — see ETHICAL-STOP (names+prices-only render resolves it).
- **M3 — SSRF/robots** — `brand-extractor.ts:182-184` concedes the DNS-rebind window isn't fully closed (needs
  a pinned-IP dispatcher); `website_url` is attacker-settable on Places; robots.txt/rate-limit/ToS ungated.
  Harden the fetch (pinned-IP) + add robots/rate-limit before the scrape ships.
- **M4 — 1.5 MB fetch cap** (`brand-extractor.ts:174`) silently truncates large menus → partial-as-full.

## ✅ Verified holding (do not regress)
Never-orderable holds via the `orders.ts:134` `published_at NULL → 409` guard (decision-3's reject is
effectively this gate) — keep `published_at` the SOLE orderability gate; no shadow publish/menu_version stamp
pre-claim. Sitemap exclusion holds (`seo.ts` `owner_id IS NOT NULL`, independent of `has_products`).

## Corrections to the proposal (factual, vs live source — 6, the P6-1/P6-2 bar)
1. `:404` redaction is the **shared convergence point** for pdf+image (NOT image-only); the real gap is the
   absent `html` kind (`parse()` throws). 2. The allergen gate site is `read_public_menu` (the SQL fn), NOT
   `public/menu.ts` (route never touches allergens). 3. The labeled render can't read the shadow via
   `read_public_menu` (H1). 4. products/categories DO have SELECT policies — RETURNING still fails (tenant
   USING-as-check), pre-gen UUIDs regardless. 5. price-grounding is default-OFF + warn-only → cannot be the
   no-fabrication gate (use the always-on confidence floor). 6. migration head is **067** live; 070 is correct
   **only after the operator places 068+069**; the 070 artifact must carry a "REQUIRES 068+069" header.

---

## ⚖️ ETHICAL-STOP (counsel — friction, lifts on a recorded render decision)
> Publicly rendering **verbatim scraped item descriptions** on the pre-claim shadow storefront crosses the
> operator's binding "no verbatim creative copy" and re-opens the GDPR Art-14 incidental-PII surface on a page
> the subject never saw. **Lifts** when the public pre-claim render is **names + prices ONLY** (facts);
> descriptions stay in `menu_draft`, surfaced to the owner for approval at claim. This is decision #4's own
> logic — nothing autofilled is authoritative/live until owner review, so non-authoritative descriptions must
> not be publicly rendered pre-claim. Art-14 notice attaches to the **claim-invite outreach** (the real first
> contact to subject); the banner is honesty, not notice.

On every other axis: no stop — conditions hold (PII guard inherited per #4; allergen no-inference per #2;
hard-delete shipped at P6-2 + completed by M1).

## 🧭 Operator decisions — RECORDED (2026-06-28, `p6-operator-decisions.md` §P6-3)
- **D-render scope = FULL DESCRIPTIONS.** Operator **overrides** counsel's ETHICAL-STOP + the binding
  "no verbatim creative copy" guard, accepting the passing-off/copyright + Art-14-in-descriptions residual
  risk. **The ETHICAL-STOP is thereby resolved by recorded human override (not lifted by names-only).** The
  technical carve-outs that REMAIN BINDING are unaffected: **C1 PII-before-AI**, **C2 allergen no-inference**,
  descriptions non-authoritative-until-claim, **generic OG (H3)**, noindex/sitemap-excluded, never-orderable,
  Art-14 notice at the claim-invite, no scraped photos.
- **D-public-vs-private = KEEP PUBLIC** (re-confirmed knowingly).

## Decision gate
The 2 operator decisions are recorded. P6-3 proceeds to code once the **technical** conditions (not
operator-waivable) are designed in: **C1** (menu-region allowlist + converged single-redaction + recall
fixture, fail-closed), **C2** (write-strip `bom[].allergens` + read-gate `read_public_menu` CASE), **H1**
(`read_preview_menu` shadow-only fn), **H2** (shadow-`location_id`-bound products/categories policy +
NOBYPASSRLS re-proof), **H3** (generic OG on the labeled render), **H4** (enforced no-fabrication state-gate);
**M1–M4** carried (esp. M1 — `hardDeleteShadow` must also clear `place_raw`/`menu_draft`); and the operator has
placed migrations **068+069** (070 carries a "REQUIRES 068+069" header). **Awaiting GO to build.**
