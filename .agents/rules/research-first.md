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

### Phase 4: Write test cases for any data-mutating change

**AUTOMATIC — do not skip.** Any change that:
- Adds/modifies a PATCH, PUT, POST, or DELETE endpoint
- Changes a Zod schema or validation rule
- Alters how data is stored or returned
- Fixes a bug in a data-mutating endpoint

MUST include a test suite that verifies:
1. **Create → verify**: POST valid data, assert response contains all saved fields (not just `{ ok: true }`)
2. **Update → verify**: PATCH each field individually, assert response confirms the change
3. **Invalid data rejection**: POST/PATCH with bad data, assert 400 error with message
4. **Unknown fields rejection**: POST/PATCH with `.strict()` extra fields, assert rejection
5. **Data URL rejection**: If an image field exists, test that `data:` and `blob:` URLs are rejected with a clear error
6. **FE ↔ BE contract validation**: For any form submission flow, verify that every field the FE collect/sends in the payload is accepted by the BE schema, and that the BE response includes those fields. Document the full field list in the test.

**Test location**: 
- Unit: `apps/api/tests/<endpoint-name>.test.ts` using `node --test --import tsx`
- E2E schema contract: `apps/api/e2e/api-integrity.spec.ts` — add test cases there
- FE-BE mapping: Validate FE Product/interface fields against BE response schema
**Run command**: `pnpm --filter api test:<name>`
**Verification**: Tests must pass before the change is considered complete.

### Phase 5: After committing

- [ ] Run `graphify update .` — update the knowledge graph
- [ ] File drawer if the change introduces a new pattern or decision

---

### Phase 6: Event / notification / queue change (NX-specific)

**Trigger:** Any change to notification events, pg-boss, MessageBus, webhooks, Telegram, or DB queries.

- [ ] **Event wiring completeness** — for each event type, grep the full chain: publish → subscribe → handler → locale → render → type union. Every link must exist.
- [ ] **Library API pinning** — read `node_modules/<pkg>/package.json` for the installed version, then read the type declarations (not docs from memory)
- [ ] **Schema-first query verification** — before writing SQL JOINs, run `SELECT column_name, data_type FROM information_schema.columns WHERE table_name = ?` or read the migration file
- [ ] **Connection lifecycle audit** — every `.connect()` must have matching `.close()`/`.release()` in error and normal paths
- [ ] **Resilience-by-default for IPC** — every `queue.send()` must include `singletonKey`/`dedupKey` to prevent duplicate jobs
- [ ] **Backward compat first for webhooks** — start lenient (log warning on mismatch), add strict validation only after verifying all producers updated
- [ ] **Infrastructure pre-flight** — verify external deps (queues, secrets, webhooks) exist at startup; fail fast
- [ ] **Topology verification** — document which DB pool each connection uses; verify `inet_server_port()` returns correct port

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
| Notification event types | `apps/api/src/notifications/provider.ts` — `NotificationEventType` union |
| Telegram locale strings | `apps/api/src/notifications/locales.ts` — sq/en/uk messages |
| Telegram render cases | `apps/api/src/notifications/render.ts` — message template per event |
| Notification workers | `apps/api/src/notifications/workers/index.ts` — `buildTelegramData` |
| pg-boss queues | `apps/api/src/server.ts` — `createQueue()` calls (search for `createQueue`) |
| MessageBus pool config | `apps/api/src/server.ts` — search for `session pool` comments |
| DB schema (columns/types) | `packages/db/migrations/` — read the migration that added the table |
| Telegram webhook | `apps/api/src/routes/telegram-webhook.ts` — handler + validation |
| Connection topology | `apps/api/scripts/phase0-probe.ts` — port/pool mapping probe |

---

### Violation = rework

If you skip this protocol and introduce a bug that could have been caught by research, the fix counts as rework. The protocol takes 30-60 seconds. The fix takes 10-30 minutes. **Research first.**
