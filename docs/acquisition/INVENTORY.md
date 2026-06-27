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

## ✅ STOP-Checkpoint ФАЗА A
INVENTORY complete · 5 MISSING + 1 UNCERTAIN reported above (none silently built) · module/migration plan set ·
operator decisions recorded. **Awaiting GO to enter P6-1** (migration 068 + state-machine + dedup), per the
one-PR-per-stage / STOP-GO model.
