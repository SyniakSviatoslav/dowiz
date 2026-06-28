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

## ▶ NEXT — P6-2 (Council-light DONE → awaiting operator GO on the hardened plan)
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
