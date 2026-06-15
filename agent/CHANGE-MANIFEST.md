# CHANGE-MANIFEST

Generated: 2026-06-15

---

## ARCH-01 | improve | Extract MessageBus subscriptions out of server.ts into bootstrap/messaging.ts

Server.ts had ~128 lines of inline LISTEN subscriptions mixed with server bootstrap code.
Extracted to `registerNotifySubscriptions(messageBus, queueBoss)` in a dedicated module.
No behavior change — same 10 channels, same handlers. Pure locality improvement.

### Classifier answers
1. Spec reference: No written spec — IMPROVEMENT
2. Change scope: New module added (net new behavior extraction) — IMPROVEMENT
3. File safety: Touches `apps/api/src/server.ts` only (unprotected) — OK
4. Contract integrity: No schema/contract/dep changes — OK

### Touched files
- `apps/api/src/server.ts`

---

## ARCH-02 | improve | Dissolve spa-proxy shadow routes into canonical owner handlers

`spa-proxy.ts` had 7 duplicate category/product handlers that bypassed `withTenant()`.
Added `/api/owner/menu/categories` and `/api/owner/menu/products` path aliases to the
canonical `owner/categories.ts` and `owner/products.ts` handlers using JWT-based location
resolution via `getOwnerLocationId()`. Removed the duplicate blocks from spa-proxy.

### Classifier answers
1. Spec reference: No written spec banning duplicates — IMPROVEMENT
2. Change scope: New routes added to canonical handlers; old routes removed — IMPROVEMENT
3. File safety: `apps/api/src/routes/` only — OK
4. Contract integrity: Route paths unchanged (same URLs, same HTTP methods) — OK

### Touched files
- `apps/api/src/routes/owner/categories.ts`
- `apps/api/src/routes/owner/products.ts`
- `apps/api/src/routes/spa-proxy.ts`

---

## ARCH-03 | improve | Extract useMenuData hook from MenuManagerPage

MenuManagerPage had 60+ lines of inline data-fetching state (`categories`, `loading`,
`error`, `productsLoading`, `fetchCategories`, `loadAllProducts`). Extracted to
`apps/web/src/hooks/useMenuData.ts`. Page now uses `const { ... } = useMenuData()`.

### Classifier answers
1. Spec reference: No written spec — IMPROVEMENT
2. Change scope: New file added; page simplified — IMPROVEMENT
3. File safety: `apps/web/` only — OK
4. Contract integrity: No shared-types or contracts touched here — OK

### Touched files
- `apps/web/src/hooks/useMenuData.ts` (new)
- `apps/web/src/pages/admin/MenuManagerPage.tsx`

---

## ARCH-04 | improve | Align CategoryResponse contract to camelCase + apply toCategoryApiShape

CategoryResponse in shared-types was using snake_case fields (`sort_order`, `product_count`).
Reverted to camelCase (`sortOrder`, `productCount`) with `.strict()`. Added
`toCategoryApiShape()` transformer in `apps/api/src/lib/row-transformers.ts` and
applied it to all category endpoints (POST, GET list, GET single, PATCH, and menu aliases).

### Classifier answers
1. Spec reference: ProductResponse uses camelCase — same convention expected — IMPROVEMENT
2. Change scope: Changes response shape (not a revert to a prior spec) — IMPROVEMENT
3. File safety: Touches `packages/shared-types/` — PROTECTED → IMPROVEMENT
4. Contract integrity: Changes shared-types contract — IMPROVEMENT

### Touched files
- `packages/shared-types/src/contracts/owner/categories.ts`
- `apps/api/src/routes/owner/categories.ts`

---

## TYPE-01 | fix | Fix 3 TypeScript errors caught by automated typecheck gate

Three issues caught by `pnpm typecheck` (all introduced by ARCH changes above):

1. `apps/web/src/pages/admin/MenuManagerPage.tsx:6` — `Product` and `Category` imported as values
   but they are type-only; fixed with `import type`.
2. `apps/web/src/pages/admin/MenuManagerPage.tsx:488` — `cat.product_count` referenced after
   `Category` interface changed to `productCount`; fixed reference.
3. `apps/api/src/bootstrap/messaging.ts:2` — `import type PgBoss from 'pg-boss'` uses default
   import but pg-boss v12 exports named only; changed to `import type { PgBoss }`.

### Classifier answers
1. Spec reference: TypeScript strict mode is the spec — IMPROVEMENT → FIX (restores type safety)
2. Change scope: Only brings code back to passing typecheck; no logic changes — FIX
3. File safety: All in unprotected zones — OK
4. Contract integrity: No contracts touched — OK

### Touched files
- `apps/web/src/pages/admin/MenuManagerPage.tsx`
- `apps/api/src/bootstrap/messaging.ts`

---

## FIX-01 | fix | Wrap all gdpr.ts DB queries in withTenant() — resolves GDPR POST 500

All 5 query sites in `apps/api/src/routes/owner/gdpr.ts` used bare `db.query()` directly.
`gdpr_erasure_requests` has FORCE RLS, so `app.current_tenant` must be set via `withTenant()`
before any query — without it, RLS blocks every read/write → Fastify returns 500.

Fixed: POST create, GET list, GET single (+ audit log sub-query), GET retention, PUT retention.
Also fixed: `rowCount` typed as `number | null` in pg — use `(rowCount ?? 0) > 0` pattern.

### Classifier answers
1. Spec reference: RLS contract is the spec — FIX (restore correct behavior)
2. Change scope: Only wraps queries in existing withTenant helper — FIX
3. File safety: `apps/api/src/routes/owner/gdpr.ts` unprotected — OK
4. Contract integrity: No schema/contract changes — OK

### Touched files
- `apps/api/src/routes/owner/gdpr.ts`

---

## FIX-02 | fix | Fix no-fallthrough and no-case-declarations lint errors in notifications/

Two ESLint errors blocked `pnpm lint` on notification worker files:
1. `render.ts:63-65` — blank line between `case 'cash.reconcile_discrepancy':` and
   `case 'order.delivered':` triggered `no-fallthrough`; removed the blank line.
2. `render.ts:113` and `workers/index.ts:616` — `const` declarations in `default:` case
   without a block; wrapped in `{}`.

### Classifier answers
1. Spec reference: ESLint config is the spec — FIX
2. Change scope: Formatting/syntax only, no logic change — FIX
3. File safety: Both files unprotected — OK
4. Contract integrity: No contracts touched — OK

### Touched files
- `apps/api/src/notifications/render.ts`
- `apps/api/src/notifications/workers/index.ts`

---

## IMPROVE-01 | improve | Add model fallback chain to OpenRouter bridge

`scripts/openrouter-implement.ts` previously used a single hard-coded model; if it was
rate-limited or down, the script failed immediately. Added a 5-model fallback chain
(NVIDIA Nemotron → Qwen Coder → DeepSeek R1 → Gemma 3 → Mistral Small) with automatic
retry on 429/502/503. Configurable via `OPENROUTER_MODEL` (single override) or
`OPENROUTER_MODEL_FALLBACKS` (comma-separated custom chain).

### Classifier answers
1. Spec reference: No spec — IMPROVEMENT
2. Change scope: Adds retry logic, no external contract changes — IMPROVEMENT
3. File safety: `scripts/` unprotected — OK
4. Contract integrity: No contracts touched — OK

### Touched files
- `scripts/openrouter-implement.ts`

---

## IMPROVE-02 | improve | Add Agent Discipline section to CLAUDE.md from Cline/Cursor/Devin

Synthesized non-obvious behavioral rules from leaked Cline, Cursor (Composer), and Devin
system prompts into a new "Agent Discipline" section in `.claude/CLAUDE.md`. Covers:
tool use discipline (read-before-edit, one-edit-per-turn), planning triggers (todos for
3+ steps only), error recovery (test failures = code is wrong, route around env issues),
and code standards (match conventions, non-interactive flags).

### Touched files
- `.claude/CLAUDE.md`
