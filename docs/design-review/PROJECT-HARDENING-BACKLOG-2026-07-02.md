# Project improve/harden backlog — whole-project sweep 2026-07-02

Consolidated from 5 read-only lanes: Repowise health/dead-code, perf/DB/god-files, architecture/coupling,
test-integrity/coverage, infra/config. Security app-code findings live separately in
docs/security/hardening-findings-2026-07-02.md (council track). Tags: **SAFE** = mechanical/behavior-preserving,
ship-now; **RED-LINE** = auth/RLS/money/PII → council; **PROTECT-PATH** = Dockerfile/CI/package.json/migrations → operator applies.

## ⭐ The convergent root (3 independent lanes agree)
AppSec + architecture + test-integrity all land on the same structural root: **owner tenant-context
resolution was never made a single deep seam.** It's re-derived in **6 resolver copies**
(`lib/get-owner-location.ts` canonical + 5 duplicates, one of which — `spa-proxy.ts:57,112` — SKIPS the
ADR-0004 live-membership re-check AND bypasses the verifyAuth chain) and the RLS tenant GUC is hand-set in
**~15 more files**. Two scattered concerns that must agree for tenant isolation, with no home to agree in.
**The highest-leverage refactor in the whole codebase:** one preHandler exposing `request.ownerContext`
(verify token → live-membership re-check → set `app.current_tenant`), consumed by every owner route.
Enabling first step (SAFE): decompose `spa-proxy.ts` (#A1) which surfaces the hidden divergent copy;
the resolver/GUC unification behind it is RED-LINE (council). Retires findings #1/#6/#7 from the security doc.

## Tier A — SAFE, ship-now (one mechanical PR each, no behavior change)
- **A1. Delete Tier-1 dead code (~1,000 lines, confidence 1.0):** `ssr-renderer.ts` 5 unused exports (~62L),
  `mockData.ts::enrichProduct` (36L), `sentry.ts::redactValue`, `restore-sandbox.ts`, `backup-verify.ts::sleep`,
  `fallback-phone.ts::showDegradedBanner`. One deletion PR + build. (Repowise)
- **A2. Decompose `server.ts` (877L):** move 3 inline `/api/dev/*` handlers to `routes/dev/*` (mock-auth already
  exists+imported → check for a DUPLICATE registration at :525 vs :536), lift `setErrorHandler` (430-504) to
  `lib/error-handler.ts`, onRequest hooks (125-204) to `plugins/request-hooks.ts`. Pure relocation. (perf + arch)
- **A3. Decompose `spa-proxy.ts` (867L)** into `routes/owner/*` modules (14 self-contained endpoints) — the
  enabling step for the root-fix; behavior-preserving moves. (arch #1)
- **A4. Contract/SSoT dedup (drift-killers):** hand-typed menu DTOs in `MenuPage.tsx:43-94` → shared
  `contracts/public/menu.ts`; `shortId` re-impl ×9 → `shared-types/utils.ts:62`; raw `fetch()` bypassing
  `apiClient` in 7 web files; `OrderStatus` two-enum → re-export domain's list. (arch #4/#5/#6)
- **A5. Central error-code union + sweep ~81 envelope-bypass sites** (`reply.status().send({error})` clusters +
  12 thrown plain objects in `lib/*Service.ts`); add `CONTRACT_CODES` union to `api-error.ts`. (arch #3)
- **A6. Split `notifications/workers/index.ts` (688L)** into 3 workers (queues already separate). (arch #10)
- **A7. Consolidate orphaned UI molecules/hooks** (BottomSheet/Drawer/Modal/use-cart/… ~520L) — aligns with the
  in-flight design-system prune. (Repowise T2)

## Tier B — coverage & CI wiring (mostly PROTECT-PATH .github → operator; high risk-retired)
- **B1. ⭐ Wire the EXISTING `rls-adversarial` IDOR guardrail into CI.** It's well-built (bypass-pool ground truth,
  39-table IDOR sweep incl. orders/customers/otp/contact-reveals) but runs NOWHERE — `pnpm test:unit` is never
  invoked by ci.yml/verify-all, and the `fresh-provision` job stands up the exact DB it needs then discards it.
  Fix = call `pnpm test:phase5-rls-adversarial` in that job. Same for jwt-rotation + integrity phase5 tests, and
  `verify:rls` (ci:false). **The single highest-value fix on the board** — the cross-tenant guardrail has NEVER run. (test)
- **B2. Add missing coverage on red-line prior-defect surfaces (SAFE to author):** GDPR anonymizer PII-erasure
  path (zero coverage), GDPR retention-settings endpoints, OTP send/verify route (rate-limit/hash/expiry/replay),
  orders.ts core txn (atomic persist / partial-order-on-failure). (test #4/#5/#6/#9)
- **B3. Fix false-coverage smells:** OTP spec computes `hasPhoneOrOtp` and never asserts it
  (flow-orders-checkout.spec.ts:236); permissive `expect([...]).toContain(status)` arrays (incl. the B4
  platform-authz gate that accepts 503 forever + skips the positive path on a missing secret). (test #6/#8/#10)
- **B4. Diagnose the live flaky test** (`access-requests.test.ts` — DB contention under `node --test` file
  parallelism; passes 5/5 isolated, failed 1/3 in full suite). needs-human. (test #7)

## Tier C — RED-LINE → council (structural, tenant-isolation)
- **C1. The owner-tenant-context seam** (the convergent root above) — resolver unification (#2) + `withTenantScope`
  GUC helper replacing ~15 hand-set copies (#7). auth/RLS. (arch #2/#7 + AppSec)
- **C2. orders.ts POST god-handler decomposition** — extract throttle/preflight/postcommit seams, but
  coverage-before-decomposition (never refactor the untested money file); pairs with security-doc coverage. (perf #2, Repowise #10)
- **C3. Money-formatter consolidation** (5 diverged formatters, rounding/EUR drift = real display-bug surface). (arch #8)

## Tier D — PROTECT-PATH infra → operator applies (I propose, ready patches)
- **D1. ⭐ Pin the Dockerfile runtime `npm install` (Dockerfile:53)** — `npm install argon2 sharp @aws-sdk/*` with
  NO version pin/lockfile in the runtime stage bypasses `--frozen-lockfile` → live supply-chain path into prod. (infra #1)
- **D2. Add `USER node` to the Dockerfile** — runtime container runs as root; one line shrinks post-exploit blast radius. (infra #2)
- **D3. Dependency/vuln CI gate:** `pnpm.overrides` `tmp@>=0.2.6` (clears the 2 confirmed HIGHs, dev-only via lhci) +
  a `pnpm audit --audit-level high` step + `dependabot.yml`. UI-PERF P0, still not in. (infra #3)
- **D4. CSP: drop `cdn.tailwindcss.com`/remote font CDNs from script-src/style-src** (self-host; nonce is defeated
  while a CDN script origin is allowed). (infra #5)
- **D5. Consolidate the two drifting security-header systems** (server.ts inline hook has no CSP; headers.ts plugin
  is prefix-scoped) → one global plugin. SAFE app-lib but coordinate with the security council. (infra #4)
- **D6. Commit the already-done DR RPO reconcile** (backup/runbooks.md ↔ disaster-recovery.md now agree 1h/4h,
  modified-uncommitted on this branch). (infra #8)
- **D7. Rotate the Telegram bot token** (pasted in a session today → treat as exposed; BotFather + update Fly secret + repo .env). (infra #9)

## Do NOT touch
- Zombie packages (audit/analytics/eval-layer/… ~1,630L, confidence 0.5 `safe_to_delete:false`) = deliberate
  out-of-tree pilots (project memory). Investigate/confirm, never auto-delete.
- Confirmed-solid controls (JWT RS256 double-pinned, parameterized SQL, RLS FORCE, idempotency on money paths,
  SSRF guard, menu cache + pool fix) — do not "improve" working defenses.

## Recommended execution order
1. **B1** (wire the IDOR guardrail — free, retires the biggest blind spot) + **D1/D2/D3** (supply-chain, operator patches).
2. **A1–A7** SAFE mechanical batch (one PR, ~1,500+ lines net reduction, build-verified).
3. **The security council** (this doc's C1 + the 9 security findings) → then C2/C3 under coverage-first discipline.
