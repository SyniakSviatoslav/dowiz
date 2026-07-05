# Triadic Council Verdict — P6 Provisioning / Demo-Build vertical

**Date:** 2026-06-27 · **Seats:** system-architect · system-breaker · counsel · all grounded against live source.
**CONSOLIDATED VERDICT: CONDITIONAL — do NOT start P6 code as written.** The *pipeline* is worth building;
the *unconsented public shadow store* and one hard technical invariant are blockers. ФАЗА A (recon) is safe
and required; P6-1+ only after the conditions below are accepted + the blockers designed-out.

---

## 🔴 HARD BLOCKER (system-breaker, CRITICAL) — the core invariant has no enforcement

"Shadow-tenant never accepts a real public order" **does not exist in the code.** `POST /orders`
(`orders.ts:66`) has **no preHandler — it's anonymous** — and never reads `locations.status`; the *only*
readiness gate is `if (published_at == null) → 409 NOT_PUBLISHED` (`orders.ts:134-137`). This forces a
lose-lose:
- Shadow **published** (needed so P6-6's synthetic order can use the real path) → **any anonymous person who
  resolves the slug can place a REAL order on the shadow tenant.** Red-line #1 broken.
- Shadow **unpublished** → the synthetic preview-token order itself hits `409 NOT_PUBLISHED` (the route
  consults neither `status` nor the token) → the P6-6 mechanic contradicts itself.
**The preview-token is irrelevant to the actual gate.** → A real `status`-based reject must be added inside
`POST /orders` (a touch of a 🔴 untested-hotspot) before P6-6 is possible. **GO blocked until designed.**

## Architect's gating unknown — shadow write authority is unverified
- **No guaranteed BYPASSRLS runtime pool.** Mig 015's restricted role is "aspirational" (its own comment),
  `verify:rls` currently fails, and `/onboarding/start` writes org/location with **no tenant context** yet
  succeeds → the live operational role bypasses RLS *today*, but that's unverified/fragile.
- **ФАЗА A must resolve the fork:** mirror the onboarding write path · vs · a `SECURITY DEFINER` provisioning
  fn · vs · a dedicated provisioning role. If the role is ever locked to NOBYPASSRLS, writing `owner_id NULL`
  shadow orgs is structurally impossible → P6-2 forks. **This is the one assumption that can sink P6-2.**

## Three premises in the prompt are factually wrong (architect)
1. Migrations are 13-digit timestamps — **next free = `1790000000068`**, not "007/008/010".
2. **`describe-product` does not exist** (docs only) → P6-4 reuses `lib/menu-grounding.ts` (flag
   `MENU_GROUNDING_ENABLED`, RLS-gated) or MISSING+STOP.
3. **No "Convergence harness"** → P6-6 adapts `e2e/lifecycle-e2e/critical-lifecycle.spec.ts` or
   `cross-tenant-realtime-qa.spec.ts`.

## Two "minimal" GO-exceptions are wider than one line (architect + breaker)
- **(a) noindex is insufficient:** the shadow `status='closed'` is still **sitemap-eligible**
  (`seo.ts:23-24` only excludes deleted/disabled) and `spa-shell.ts` emits the **real restaurant
  name/logo OG tags with no `noindex`/`X-Robots-Tag` anywhere**. → must ALSO exclude shadow from
  `getActiveLocations` + emit noindex in the SSR shell. Slug-obscurity is not a security control.
- **(b) preview-token must also tag the order `source='provision_probe'` and exclude it from settlement,
  analytics_events (cross-tenant, non-RLS), customer_reputation, notifications + provide a hard-delete** —
  not just "a row."

## Other graded breaker findings
- **HIGH — PII-to-AI:** redaction is inline in ONE function (`ai-ocr-parser.ts:404`), applied only to
  owner-uploaded bytes. A website-menu-scrape path is NET-NEW (`brand-extractor` only does brand signals)
  and would feed raw scraped HTML (staff names, owner phone, testimonials) to a third-party LLM **without
  inheriting redaction** → violates the BINDING zero-PII-in-AI (ADR-0011). `place_raw` also stores owner
  PII at rest unredacted.
- **HIGH — non-RLS `acquisition_sources`:** "no tenant reads it" is unenforceable convention (precedent
  `analytics-events.ts:5-10`); a wrong-`location_id` write has no RLS backstop → silent cross-tenant
  corruption. Architect fix: use the **access_requests pattern** (ENABLE+FORCE RLS + `USING(true)` ops
  policy + REVOKE anon/authenticated + mirror-orders grants) — "no isolation" yes, "no RLS" no (the latter
  fails `verify:rls`).
- **MED — confidence gate is warning-only** (`ai-ocr-parser.ts:388` warns + continues); with no human in
  the loop (unlike owner-upload), garbage extraction is written → "menu never fabricated" is not enforced.
- **MED — dedup race:** `place_id UNIQUE` guards only the acquisition row; the org+location spine is
  multi-insert → two concurrent runners leak an orphan shadow tenant. Need UNIQUE-insert-first, whole spine
  in one tx.
- **MED — no allergen-confirmed gate** in `public/menu.ts` (grep: zero hits) → AI allergens can surface as
  fact today.
- **LOW — scraper has no robots.txt/rate-limit/ToS gate**; MANUAL_REVIEW has no reaper (silent-stall sink).

## Counsel — ETHICAL-STOP (friction, recorded decision) + binding conditions
**The deepest finding: the unconsented *public* shadow store is structurally the aggregator move the
project exists to oppose — the means contradict the end** (ethics + system-coherence + inverts the No-CB
threat model: the cheap, likely first legal contact is one annoyed owner's C&D over their name on a page
they never saw — a PR gift to the coalition you're staying quiet to avoid).
- **ETHICAL-STOP** on un-consented third-party PII → AI (scraped staff names/testimonials/owner mobile are
  free-text the redactor doesn't match; GDPR Art 14 notice-to-subject triggers, unaddressed). Lift by:
  **facts-only extraction (item names + prices; discard "About us"/testimonials/bios BEFORE the AI
  boundary)** + redact + name-guard + redaction-recall fixture; `place_raw` minimized + short-retention +
  hard-delete (it's unconsented third-party data — actual delete, not anonymize).
- **No allergen inference pre-claim** — a shadow store can't take orders, so allergen data has zero
  pre-claim utility and catastrophic downside + no accountable human. Generate it only post-claim, behind
  an authenticated owner confirming each field. `allergens_confirmed=false` is necessary but not sufficient.
- **Decouple the pipeline from the unconsented public artifact** — default the demo to a **private,
  authenticated, sales-context preview**, not a silently-persisting public noindex URL. If public, it's
  **born with first-contact notice**, honestly labeled "preview mockup built from your public site — not a
  live store," server-authoritative "never accepts orders," instant owner kill.
- **Provenance gaps (your 🔴 is necessary but leaky):** facts only (no verbatim creative copy or scraped
  photos — copyright), **"GBP" = the official Places API within ToS, NOT scraping Google Maps**, honor
  robots.txt, conservative/removable trademark+logo use.
- **Strategic:** build the pipeline (real M0 leverage — forges the demo + integrates Phase-2 in one move),
  but the bankruptcy-clock constraint is **Stage-35: a real paid order from a CONSENTING restaurant**,
  which P6 does not itself produce. Let P6 accelerate the funnel without becoming the largest liability.

---

## What the Council unanimously endorses (the shape)
State-machine with every-state-has-an-exit + non-empty `failure_reason`; transient `menu_draft` (reuse the
existing `import_sessions` shape, never partial-write tenant); reuse-first (the Stages-11-12 `AiOcrParser`
port is real — raw bytes in); `place_id` idempotency; verify-against-live-not-mocks. Module seam: only
`PlacesProvider`/`ProvisionVerifier` in `packages/platform`; MenuExtractor/PriceNormalizer stay thin
adapters in `apps/api/src/modules/acquisition` (PriceNormalizer demoted to a guard — the parser already
integer-normalizes).

## Decision gate (the verdict is conditional — needs the operator)
P6 does NOT proceed to P6-1 until: (1) the `POST /orders` status-reject is designed (blocker); (2) the
ФАЗА-A write-authority fork is resolved; (3) the public-vs-private artifact decision is made (counsel's
reshape); (4) the allergen-no-inference + facts-only-PII conditions are accepted (recorded ETHICAL-STOP
decision). **ФАЗА A (recon → INVENTORY.md) is safe to run now** and will close the factual gaps.
