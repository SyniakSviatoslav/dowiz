---
trigger: always_on
description: Research-first protocol — mandatory before any code change. Prevents breaks and inconsistencies.
---

## Research-first protocol (always-on)

**Before making ANY code change**, agent MUST complete the relevant research checklist below. No exceptions.

### Why this exists
Every bug found in the 2026-06-07 audit was caused by code written without checking existing patterns:
- Double-prefixed routes (`/api/courier/api/courier/...`) — route prefix not checked
- Duplicate settings routes in spa-proxy.ts — existing route not checked
- 3 copies of `maskStr` — shared utility not checked
- 12 copies of tenant resolution hook — existing hook not checked
- 8 different auth patterns — no standard was followed
- 10+ error response formats — no convention enforced

**All preventable. All cost hours.**

---

### Phase 1: Understand the change (30s)

Ask yourself:
1. **What am I changing?** (route, component, utility, config, migration)
2. **What already exists for this area?** (don't assume — verify)
3. **Who consumes what I'm changing?** (breaks downstream?)

---

### Phase 2: Research checklist (by change type)

#### A. API route / endpoint change
- [ ] `graphify query "where is <resource> defined"` — find all existing routes for this resource
- [ ] Grep for the route path pattern (e.g., `/api/owner/settings`) — check for duplicates
- [ ] Check the prefix: if route is registered with `prefix: '/api/courier'`, paths INSIDE the file must be relative (e.g., `/me` not `/api/courier/me`)
- [ ] Check existing auth pattern in similar routes — use the SAME hook style (don't invent a new one)
- [ ] Check error response format — use `{ error: string }` consistently (not `{ message }`, `{ msg }`, or raw strings)
- [ ] Check if shared utility exists before writing inline: `pii-mask.ts`, `tenant.ts`, `error-response.ts`

#### B. Frontend component / page change
- [ ] Check if similar component exists in `packages/ui/src/components/`
- [ ] Check CSS variable usage — grep for any hex you're about to write
- [ ] Check auth token pattern — use `sessionStorage` (not `localStorage` for tokens)
- [ ] Check error handling pattern — look at sibling pages for the convention
- [ ] Check for embed mode: no `position: fixed`, no `target="_blank"`

#### C. Shared package / utility change
- [ ] Grep for existing implementations — don't create duplicate utilities
- [ ] Check the package's `index.ts` exports — new exports must be re-exported
- [ ] Check cross-package imports — `packages/shared-types`, `packages/domain`, `packages/ui` have strict boundaries

#### D. Database migration change
- [ ] Read the latest migration to understand naming conventions
- [ ] Check if column/table already exists before `ADD COLUMN`
- [ ] Include `down()` function (not empty)
- [ ] Check RLS policies — tenant-scoped tables need `WHERE location_id = current_setting('app.current_tenant')::uuid`

#### E. E2E test change
- [ ] Check `e2e/helpers/` for existing helpers — don't duplicate
- [ ] Use existing `loginAs()`, `seedLocation()` etc. — don't re-implement inline
- [ ] Check test file structure — follow the pattern of sibling test files

---

### Phase 3: Before committing

- [ ] Run `pnpm lint` — fix any new violations
- [ ] Run `pnpm typecheck` — fix any type errors
- [ ] Grep for your change pattern — did you introduce duplicates?
- [ ] Check imports — are you importing from the right shared location?

---

### Phase 4: After committing

- [ ] Run `graphify update .` — update the knowledge graph
- [ ] File drawer if the change introduces a new pattern or decision

---

### Quick-reference: where things live

| What you need | Where to look |
|---|---|
| Existing API routes | `apps/api/src/routes/` — grep for the resource name |
| Auth hooks | `apps/api/src/lib/auth.ts` — `verifyAuth`, `requireRole` |
| Tenant isolation | `apps/api/src/lib/tenant.ts` — `withTenant`, `getLocationId` |
| Error responses | `apps/api/src/lib/error-response.ts` — standard format |
| PII masking | `apps/api/src/lib/pii-mask.ts` — `maskStr`, `maskPhone` |
| CSS variables | `packages/ui/src/styles/tokens.css` — all colors |
| Shared types | `packages/shared-types/src/contracts/` — Zod schemas |
| Domain logic | `packages/domain/src/` — pure functions |
| UI components | `packages/ui/src/components/` — shared library |
| E2E helpers | `e2e/helpers/` — auth, seed, api utilities |
| Migrations | `packages/db/migrations/` — numbered by timestamp |

---

### Violation = rework

If you skip this protocol and introduce a bug that could have been caught by research, the fix counts as rework. The protocol takes 30-60 seconds. The fix takes 10-30 minutes. **Research first.**
