# P6 ФАЗА A — Acquisition INVENTORY (recon, read-only)

**Date:** 2026-06-27 · Ground-truth for the P6 vertical. Decisions: `docs/design/p6-operator-decisions.md`;
Council: `docs/design/p6-provisioning-council-verdict.md`. **STOP at the end — P6-1 only after GO.**

## FOUND (reuse-first)
| Asset | Where | Note |
|-------|-------|------|
| menu-CRUD output contract (`categories`/`products` Zod) | `packages/shared-types` | **read-only** — extraction must compose this |
| Stages 11-12 AI parser | `lib/ai-ocr-parser.ts::AiOcrParser.parse({kind:pdf\|image\|csv, bytes, config})` (port `lib/ports.ts`, registered `server.ts:289`) | raw bytes in → `{draft, issues, summary}`; **integer-normalizes price** already → P6-3 PriceNormalizer demoted to a guard |
| transient draft store | `import_sessions` (`menu-import.ts:128-139`: draft_json, idempotency_key, expires_at 30m, commit_token, `_provenance`) | reuse this SHAPE for `menu_draft`; never partial-write tenant |
| auto-translate (Stage 12) | `routes/owner/menu-translate.ts` + `ai_translations` | name_en autofill |
| price grounding | `lib/menu-grounding.ts::groundItems/computeGrounding` (flag `MENU_GROUNDING_ENABLED`, RLS-gated on import_sessions) | **price grounding, NOT description-gen** |
| brand seed from site | `lib/brand-extractor.ts::extractFromWebsite` (SSRF-guarded fetch) | theme/colors/logo only — **not menu** |
| SSR storefront | `lib/spa-shell.ts` (`/s/:slug`) | renders any slug; emits real name/logo OG |
| branding/themes | `location_themes` + ThemeRenderer | |
| platform shims | `packages/platform` (`ports.ts`, `stubs.ts`, MessageBus, QueueProvider/pg-boss, jwt RS256) | new PlacesProvider/ProvisionVerifier land here |
| non-tenant table pattern | `1790000000041_access-requests.ts` (ENABLE+FORCE RLS + `USING(true)` ops policy + REVOKE anon/authenticated + mirror-orders grants) | template for `acquisition_sources` |
| spine write authority | `routes/owner/onboarding.ts:55-91` writes org/location via `db.connect()` **no tenant ctx** and succeeds | ⇒ operational pool bypasses RLS **today**; P6-2 mirrors this exact path (explicit ids, `owner_id NULL`) |
| migration head | `1790000000067_bom-seams.ts` | **next free = `1790000000068`** |
| gate commands | lint · lint:gates · verify:env/db/rls · migrate:create/up · seed · **test:unit** (new) | all FOUND |
| verify harness (P6-6) | `e2e/lifecycle-e2e/critical-lifecycle.spec.ts` · `e2e/tests/cross-tenant-realtime-qa.spec.ts` | adapt one headless vs live dev |

## 🚩 MISSING / net-new (per red-line #1: flag + STOP, do NOT silently build)
1. **SSR `noindex` path** — no `noindex`/`X-Robots-Tag` anywhere; **+ sitemap leak**: shadow `status='closed'`
   is still in `seo.ts::getActiveLocations` (excludes only deleted/disabled). → **GO-exception (a)** = emit
   noindex in the shell **AND** exclude shadow/closed from `getActiveLocations`.
2. **`POST /orders` status reject** — route is **anonymous**, gates only on `published_at`, never `status`
   (the breaker BLOCKER). → decision 3 APPROVED: add a `status`-based reject (its own red→green guardrail).
3. **`MenuSource.locate`** (fetch a restaurant's menu page/PDF + classify html|pdf_text|pdf_image|image|none)
   — **net-new**; brand-extractor only fetches for brand. Must inherit the **PII redaction** before any AI.
4. **Description-gen ("describe-product")** — does NOT exist (docs-only). P6-4 description enrich = net-new
   LLM call (allergens EXCLUDED per decision 2). Price-grounding reuses `menu-grounding`.
5. **preview-token + synthetic-order handling** — **GO-exception (b)**: signed/TTL/bearer-only token bypassing
   the (new) status-gate, tagging `orders.source='provision_probe'`, **excluded from settlement +
   analytics_events + customer_reputation + notifications + hard-deletable**.

## ⚠️ UNCERTAIN — must pre-resolve before P6-2
**Write-authority fork.** Operational pool bypasses RLS today (proof above) BUT mig 015's restricted role is
"aspirational", `verify:rls` currently fails, and `index.ts:35-38` throws if `current_user='postgres'`.
→ **ADR + human sign-off:** P6-2 mirrors the onboarding `db.connect()` path; documented fork to a
`SECURITY DEFINER` provisioning fn if the role is ever locked to NOBYPASSRLS. (🔴 RLS red-line.)

## Module plan
- `apps/api/src/modules/acquisition/` — state-machine, service (createSource/advance/flag*), MenuSource +
  MenuExtractor + PriceNormalizer **adapters** (call the existing parser port — keep deps pointing inward),
  internal/ops-only route (mirror `1780421100065_lockdown-nontenant-api-surface`).
- `packages/platform/` — **only** `PlacesProvider` (Google Places Details, key from env) + `ProvisionVerifier`
  (drives Playwright = external boundary).
- migration `1790000000068_acquisition`: `acquisition_sources` (access_requests RLS pattern, `place_id UNIQUE`,
  state enum, `menu_draft` jsonb, provenance) + `products` ALTER (`source` enum default `'owner'`,
  `allergens_confirmed` bool default false). UNIQUE-insert-FIRST + whole spine in one tx (dedup race, breaker MED).

## ✅ STOP-Checkpoint ФАЗА A — PASSED (GO received)
INVENTORY complete · 5 MISSING + 1 UNCERTAIN reported (none silently built) · operator decisions recorded
(incl. 1b: one-time-token RLS-through write, not bypass).

## ✅ P6-1 — DONE (proven)
- Staged migration `docs/acquisition/migration-1790000000068-acquisition.ts` (operator places at
  `packages/db/migrations/` + `migrate:up` on dev-Postgres — migrations dir is protected). Full SQL applies
  clean on a throwaway PG: `acquisition_sources` (place_id UNIQUE, FORCE-RLS + ops policy, access_requests
  grant-mirror) + `products` provenance (`source` default 'owner', `allergens_confirmed` default false).
- `apps/api/src/modules/acquisition/`: `types.ts` (state enum + Zod `.strict()`), `state-machine.ts`
  (assertTransition, every-non-terminal-has-exit, REQUIRES_REASON), `service.ts` (idempotent `createSource`
  dedup anchor + guarded `advance`/`flag*`), `route.ts` (ops-only `POST /api/dev/acquisition`, rides the
  global dev-guard, rate-limited, Zod). Registered `server.ts`.
- Proof: `acquisition-state-machine.test.ts` 6/6 + `acquisition-service.test.ts` 3/3 vs real PG (dedup → 1
  row/place_id, legal/illegal transitions, reason invariant). apps/api typecheck + lint clean. Ledger #22.
- **Operator step before P6-2:** place the staged migration + `migrate:up` on dev-Postgres.

## ✅ P6-2 — DONE (proven; 4 blockers designed out)
- **B1** dedup anchor moved to the in-tx guarded state-transition (`advance` ENRICHED→PROVISIONED,
  state-pinned) + consume-LAST + partial-unique one-active-grant. **B2/ETHICAL-STOP** shadow tenants
  (org.owner_id IS NULL) render NO real name/logo to humans/bots + emit noindex (`spa-shell.ts`,
  `ssr-renderer.ts`) + excluded from sitemap (`seo.ts`). **B3** spine writes `published_at` NULL
  (proven). **B4** ops surface decoupled from the dev-login flag → its own `PROVISION_OPS_SECRET`,
  mounted `/internal` (not `/api/dev`), fail-closed 404 (`ops-auth.ts`).
- Code: `apps/api/src/modules/acquisition/provisioning.ts` (mint + `provisionShadowSpine` one-tx +
  `hardDeleteShadow` C2 + `reapExpiredGrants`), `route.ts` (mint/spine/hard-delete, ops-auth hook),
  `ops-auth.ts`. Registered `/internal` in server.ts. Migration staged
  `docs/acquisition/migration-1790000000069-provision-grants.ts` (operator places + `migrate:up`,
  REQUIRES 068): `provision_grants` FORCE-RLS + additive `provision_shadow` policies (built-in sha256,
  no search_path dep).
- Proof: `apps/api/tests/provision-rls.test.ts` 8/8 under a **real NOBYPASSRLS role** (token admitted,
  no-token/bogus/expired rejected, single-use, no-widening, B1 chokepoint, one-active-grant, C2
  delete) + P6-1 regression 9/9. typecheck + lint clean. Ledger #23.
- **Operator steps before P6-3:** place migrations 068 + 069 + `migrate:up` on dev-Postgres; set
  `PROVISION_OPS_SECRET` where provisioning should be reachable (NOT a dev-login flag).

## ✅ P6-3 — DONE (proven; 2 CRIT + 4 HIGH designed out, operator overrides recorded)
Scrape → AI-parse → write products → labeled PUBLIC preview render. Council verdict
`docs/design/p6-3-extraction-render-council-verdict.md`; operator decisions (FULL DESCRIPTIONS override +
KEEP PUBLIC) in `p6-operator-decisions.md` §P6-3.
- **C1** PII-before-AI: `menu-region.ts` allowlist (drops About/Team/Reviews/footer) + name-guard fail-closed
  + anchored name pattern in `pii-redactor.ts`; the new `html`/`text` parser kind converges on the single
  `:404` redaction (one redactor, no bypass). **C2** allergens: write-strip `bom[].allergens` + `read_preview_menu`
  read-gate (`attributes - 'bom'`); live-menu re-version deferred to claim-phase (`c2-read-gate-claim-phase.sql`).
  **H1** `read_preview_menu` shadow-only. **H2** products/categories `provision_shadow` bound to a shadow
  location via SECURITY DEFINER `app_is_shadow_location()`. **H3** generic-OG labeled preview (`preview-render.ts`).
  **H4** `menu-extractor.ts::classifyExtraction` enforced no-fabrication gate. **M1** `hardDeleteShadow` clears
  place_raw/menu_draft.
- Modules: `apps/api/src/modules/acquisition/{menu-source,menu-extractor}.ts` + `apps/api/src/lib/{menu-region,preview-render}.ts`;
  render wired in `routes/public/ssr.ts` (falls through if mig 070 absent). Migration staged
  `docs/acquisition/migration-1790000000070-provision-products.ts` (REQUIRES 068+069).
- Proof: 27/27 — `provision-rls.test.ts` 12/12 (NOBYPASSRLS, incl. menu write + allergen strip + victim-location
  rejected) · `menu-region-pii.test.ts` 5/5 (recall floor) · `menu-extractor.test.ts` 6/6 · `preview-render.test.ts`
  4/4. Ledger #24. typecheck clean. **Operator: place migs 068+069+070 + migrate:up before staging.**

## ✅ P6 CLAIM PHASE — DONE (proven; shadow → consented owner → live)
Council verdict `docs/design/p6-claim-council-verdict.md`. Owner claims a shadow → authenticated → reviews/
authors menu → publishes via the existing gated path.
- **Ownership transfer** = `claim_transfer(token,user)` SECURITY DEFINER carve-out (migration 071, staged) —
  the inline RLS UPDATE policy can't work (PG requires SELECT-visibility for an UPDATE target; proven); the fn
  validates the token INSIDE + touches only the target shadow (token = sole authority, K2/IDOR-safe). Leaves
  published_at NULL + status closed (NO auto-publish — B3), erases place_raw/menu_draft (H-erase), voids grants.
- **Invite** = 256-bit opaque single-use sha256 token (`claim_invites`, provision_grants RLS template, one-active
  guard, TTL). **Decline** = token-only, no registration, equally prominent (CC2) → hardDelete. **Art-14 notice**
  (CC1) for the hostile recipient. **Approve** = `routes/owner/menu-confirm.ts` sets allergens_confirmed=true only
  (CC3 — owner authors allergens into the write-stripped empty fields; never confirms an AI guess).
- **C2 live read-gate** placed: `docs/acquisition/migration-1790000000072-c2-read-gate.ts` (verbatim 065/035 +
  bom-strip CASE, REQUIRES 068, golden-snapshot proof-on-staging plan — DO NOT place blind).
- Code: `apps/api/src/modules/acquisition/claim.ts`, `routes/public/claim.ts` (accept verifyAuth / decline
  token-only), `routes/owner/menu-confirm.ts`, ops mint/verify on the `/internal` route. Proof: claim-rls 7/7
  (NOBYPASSRLS) + claim-notice 8/8; 35/35 P6 total. Ledger #25.
- **Email-match hardening DONE** (counsel): `claim_transfer` enforces, when an invite is BOUND to a contact
  (`invited_contact_hash` set at mint), that only the invited identity (case/space-normalized email) can claim —
  even with a valid token (CONTACT_MISMATCH → 403). Token-only when the invite has no bound contact. Proven
  (claim-rls test (h)).
- **P6-6 ProvisionVerifier DONE** — `modules/acquisition/provision-verifier.ts::verifyShadowPreview` renders the
  REAL preview (read_preview_menu → renderShadowPreview) and asserts every external-boundary invariant
  (served, hasItems, banner, noindex, generic-OG, never-orderable) before VERIFIED. markVerified now gates on it.
  Proof: `provision-verifier.test.ts` 4/4 + the deploy-time Playwright `e2e/tests/p6-provision-verify.spec.ts`
  (re-asserts on live staging, runs once deployed).
- **Migrations 068-072 PLACED + committed + proven vs the REAL full schema** (147 migrations apply clean; 072
  golden-snapshot: owner menu byte-identical, place stripped, confirm restores). Full P6 suite **53/53** vs real
  schema under NOBYPASSRLS. **Operator: migrate:up on staging (next deploy applies via release_command) + set
  PROVISION_OPS_SECRET; run the Playwright verify spec on staging before PROD.** Remaining follow-ups:
  owner-initiated "this is my restaurant" verified-invite request, decline-without-complaint health metric (CC4).
- **Follow-ups DONE (2026-06-28):** (1) **retention cron** — `AcquisitionRetentionWorker`
  (`workers/acquisition-retention.ts`, wired in bootstrap/workers.ts) schedules `runRetentionSweep` daily
  (03:30, advisory-locked, .catch-wrapped) — the GDPR Art-5(e) sweep now runs automatically. (2) **CC4
  decline-without-complaint** — `declineAndErase` emits `acquisition.shadow_declined`; `recordComplaint` +
  `POST /internal/acquisition/complaint` emit `acquisition.complaint` (structured logs → a computable health
  signal, no migration; proof acquisition-metrics.test.ts 2/2). (3) **owner-initiated request** — public
  `POST /api/claim/request {slug}` records `acquisition.claim_requested` for ops (signal-only, NO auto-mint →
  no spam/IDOR vector; generic 202 ack, no enumeration). Ops then does the contact-verified mint.
- **Extraction orchestration WIRED** — `POST /internal/acquisition/extract` (orchestrate-extraction.ts) composes
  MenuSource.locate (SSRF) → AiOcrParser.parse (C1) → classifyExtraction (H4), driving SOURCED→ENRICHED (or to a
  terminal verdict). Closes the SOURCED→ENRICHED gap (no manual menu_draft seed). Proof:
  extraction-orchestrator.test.ts 4/4 (real PG, injected locate/parser).

## ✅ P6 RETENTION SWEEP — DONE (proven; GDPR Art-5(e) storage-limitation)
`apps/api/src/modules/acquisition/retention.ts`: `reapAbandonedShadows` (never-claimed PROVISIONED/VERIFIED/
CLAIM_OFFERED shadows past a short TTL → hardDelete + ABANDONED; a CLAIMED tenant is NEVER reaped) +
`runRetentionSweep` (grants + invites + shadows). Triggered by the ops route `POST /internal/acquisition/
retention/sweep` (a recurring cron hits it; cadence is operator/infra). Proof: `retention-sweep.test.ts` 4/4
(stale erased, fresh survives, claimed untouched, unified sweep). Closes the breaker HIGH + counsel C5.

## (superseded) P6-2 plan — Council-light verdict
Places → spine (no LLM) via the **one-time provisioning token** (decision 1b): a single-use grant + a narrow
additive RLS provisioning policy on organizations/locations/menu_versions honoring it ONLY for
`owner_id IS NULL`+`status='closed'` shadow rows — RLS stays enforced (once the role is NOBYPASSRLS; inert
defense-in-depth today). + GO-exception (a) noindex/sitemap.

**Council-light verdict (2026-06-28): APPROVE-WITH-CONDITIONS** → `docs/design/p6-2-provisioning-council-verdict.md`.
4 blockers to design out before code: **B1** dedup anchor is on the wrong table (mint-twice → two spines; fix
= guarded conditional state-transition inside the tx); **B2/ETHICAL-STOP** noindex doesn't stop OG real-name
unfurl/human render (fix = no real-name render to humans/bots until P6-3's labeled storefront); **B3**
ship the `published_at`-stays-NULL guardrail with P6-2 (non-orderability rests on `published_at NULL`, not the
not-yet-built status-reject); **B4** decouple the ops route from the dev-login backdoor flag for prod. Plus
fix-conditions (pre-gen UUIDs/no RETURNING, menu_versions policy, schema-qualified digest, single-tx +
consume-LAST, day-one hard-delete, reaper) + the NOBYPASSRLS red→green proof gate. Migration head = **069**.
