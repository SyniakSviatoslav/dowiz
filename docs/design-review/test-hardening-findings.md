# Test-Hardening Loop — full-surface sweep findings (2026-06-27)

**Run:** `test-hardening-attack` workflow over **all 245 test files** (94 unit `*.test.ts` + 151 e2e
`*.spec.ts`). One combined critique+security+QA scanner agent per file + a synthesis pass over the
CRITICAL/HIGH findings. **246 agents · 5.63M tokens · 928 tool-uses · 2h17m.**

> The headline: a green test suite is **not** a correct one. 2,023 blind-spots; **217 CRITICAL**.
> The dominant failure is **tests that pass while the feature is broken** — tautologies, loose
> render proofs, permissive status arrays, no auth/IDOR controls, and whole suites that never run.

## Severity / lens

| | count | | count |
|---|---|---|---|
| CRITICAL | 217 | critique (false-green) | 977 |
| HIGH | 796 | qa (coverage) | 613 |
| MED | 816 | security (authz/IDOR/PII) | 433 |
| LOW | 194 | **total** | **2023** |

## Top 20 files by finding-weight (CRITICAL×8 · HIGH×4 · MED×2 · LOW×1)

flow-security-contracts (58) · flow-ui-admin-branding (57) · flow-ui-invite-onboarding (56) ·
flow-ui-analytics-supplies (55) · ui-polish (53) · flow-proofs (51) · flow-ui-client-order-full (51) ·
client/checkout (50) · flow-ui-client-checkout (49) · admin/dashboard (48) · admin/promotions (48) ·
client/menu (48) · flow-ui-courier-core (48) · flow-ui-images (48) · admin/menu-manager (47) ·
client/menu-interaction (47) · flow-regulatory-settlements (47) · telegram-full-flow (47) ·
flow-ui-order-lifecycle (46) · flow-ui-validation (46). (full ranking + 2023 raw findings: the
workflow result JSON.)

## Systemic patterns (the recurring roots — fix the class, not the instance)

1. **Tautological / always-green assertions** — `expect(true)`, `assert.ok(true)`, `count >= 0`,
   `x===null||x!==null`, unawaited `expect(...)`, and ~30 booleans computed then only `console.log`'d,
   never asserted. The assertion can never fail. (phase2, behavioral-proof, fe-radar/-v2, capture-*,
   phase5/integrity R3+R4, many flow-ui-*.)
2. **`body.length > N` / loose body-text regex as the ONLY render proof** — a 500 page, login
   redirect, error boundary, or spinner all satisfy it; nav/footer words match the success regex. The
   single most common e2e weakness (admin/*, client/*, courier/*, flow-ui-*, capture-*, maps, …).
3. **Permissive status arrays / negative-only checks** — `expect([200,400,409,500])`, `not.toBe(500)`,
   `[200,404]`. A 500/validation-removal/always-404 passes. (flow-*, deploy-validation, audit-fixes, …)
4. **No positive control + no negative auth control** — happy-path only; no 401/403, and no proof the
   gate isn't rejecting *everyone*. (api-integrity, orders-guards, spa-proxy, every owner/courier flow.)
5. **Zero cross-tenant/IDOR coverage, or IDOR "tested" with a nil/fake UUID** — an all-zero id 404s by
   absence, not by an ownership check, so a real foreign-id leak is never exercised. Near-universal.
6. **`?dev=true` / `/api/dev/mock-auth` bypass** — real auth never exercised; many specs default
   `BASE=dowiz.fly.dev` (PROD) and call the dev backdoor against prod with real writes + no 404 canary.
7. **Empty-fixture / conditional-skip vacuity** — assertions behind `if (count>0)`/`if (visible)`,
   silent `return`, runtime `test.skip`, or a `beforeAll` that swallows setup failure → 0 assertions run.
8. **Real-time asserted via reload / poll-buffer, not a live WS DOM update** — keyword match ('NEW
   ORDER') not orderId; WS isolation without a `wasOpened()` guard (dropped upgrade → 0 msgs → "green").
9. **Truthy-only on tokens/ids/values** — `.toBeTruthy()` on access_token (accepts ''/'null'/errors),
   ids, money, CSS vars. No JWT-shape, no UUID regex, no exact/range.
10. **Loose substring / single-char regex** — `/[1-9]/`, `toContain('3')`, `endsWith('dowiz.org')`
    (xdowiz.org passes). Counts/values/URLs not pinned.
11. **Fixed `waitForTimeout` sleeps as the synchroniser** — flaky on slow CI, false-green on fast/broken
    pages; captures loading skeletons. Across the whole e2e suite + a few unit tests.
12. **Test re-implements production logic inline (oracle drift)** — schema shadows, hand-rolled fee
    oracle, inline validateImageKey/hashIp diverge from source silently. (api-integrity, spa-proxy,
    fee-parity, send-error, access-requests.)
13. **Whole suites never execute** — `.js` import of unbuilt `.ts` (ERR_MODULE_NOT_FOUND), no test
    script / CI wiring, no RuleTester. Dead-green: ~all `tools/loop-harness/tests/*`, message-bus-notify,
    theme-renderer, eta-synthesis, tools/ccc/secret-scan, the eslint permissive-status fixture.
14. **Side-effect / DB-write / dispatch never verified** — only HTTP 200; INSERT, bus.publish,
    queue.enqueue, settings persistence never read back. (courier-assignment-idor no-op fakeBus,
    timeout-handler, notification-*, flow-sensor-*, every PUT/PATCH round-trip.)
15. **Swallowed errors** — `.catch(()=>{})` on goto/click/api/route, error branch only console.log,
    double-consumed response body. (capture-*, storefront.a11y, non-pixel-sweep, telegram-test.)
16. **Vacuous assertions on RED-LINE paths (money/RLS/PII)** — phase5/integrity R3+R4 `assert.ok(true)`,
    RLS "blocked" verified by COUNT of an empty tenant, RLS via pg_class metadata not DML, PII by
    JSON key-name not value, phone-leak by exact string not digit-run. (phase5/*, ocr-redaction,
    p0-bus-claimcheck, p0-telegram-detail, pii-leak-detector.)
17. **Boundary tests only use far-from-edge values** — count=5 (never 3/4), day=95 (never 90/91),
    ratio≥3 (AA needs 4.5) → `>` vs `>=` off-by-ones survive. (preflight, quiet-hours, money-tax, eta.)
18. **Mock-route ordering / catch-all `**/api/**` swallow + missing `await`** — the specific route never
    fires or the error-status route is registered after the action. (error-handling, ux1–ux4,
    menu-first-onboarding.)
19. **No teardown / shared mutable state / prod-write risk + hardcoded creds** — fixtures pollute the
    DB, BASE defaults to prod, plaintext test@dowiz.com/test123456 committed. (flow-order-creation,
    behavioral-proof mutates the shared demo, prod-adr0004-smoke, owner-revocation.)

## Top 10 fixes (class-level — mostly DETERMINISTIC guardrails, red→green)

1. **Tautology + unasserted-boolean ESLint rule** — ban `expect(true)`, `assert.ok(true)`, floor-only
   `>=0`, `x===null||x!==null`, unawaited `expect`, and `const has*/isVisible()` that never reaches an
   `expect`. Wire into pre-commit; red on the listed files → green after fixes.
2. **Forbid `body.length > N` / loose body-text as sole render proof** — mandate a role-specific
   `assertRendered(page,'[data-testid=…]')` that also asserts no error-boundary text.
3. **Harden + ACTIVATE the permissive-status rule** (its fixture currently has no RuleTester) — flag
   `not.toBe(500/401)` and any accepted-status array with 4xx/5xx unless `// known-bug:` annotated.
4. **Shared `assertAuthGate(request,{method,url})`** — no-token→401, wrong-role→403, valid→200-non-empty
   (positive control); require it per protected describe; retire `?dev=true`-only suites.
5. **Cross-tenant/IDOR fixture with a REAL second tenant** — ban the nil-UUID anti-pattern; per resource,
   owner-A → 403/404 on tenant-B's *real* id; RLS unit tests assert DML under `SET ROLE` with a seeded
   tenant-B row so the COUNT delta proves a block.
6. **Prod-guard all dev/mock-auth** — a shared `requireStaging()` that throws when BASE is prod/unset +
   a canary asserting `/api/dev/mock-auth` is 404 on prod.
7. **Replace conditional/skip vacuity with hard preconditions** — lint-ban `if(await x.count())` /
   `if(isVisible())` wrapping an assertion; `beforeAll` must `expect(setupRes.status()).toBe(200)`.
8. **Real-time net** — `waitForLiveUpdate(page,testid,expected)` keeps the page open, advances status via
   API, asserts the DOM changes WITHOUT reload, orderId-anchored, with `expect(ws.wasOpened()).toBe(true)`.
9. **Truthy → shape/value** — `expectJwt()` / `expectUuid()` helpers; ESLint-flag `.toBeTruthy()` on
   `*token/*id/*Url`; every mutating round-trip GETs and asserts the persisted exact value.
10. **Make dead suites executable + ban swallowed errors** — per-package `test` scripts with a tsx
    loader + CI wiring; a meta-test asserting each suite runs ≥1 assertion; ESLint-ban `.catch(()=>{})`
    on Playwright/api calls.

## What this means / next

The highest-ROI response is **deterministic guardrails** (fixes 1, 3, 7, 9, 10 are ESLint-rule-shaped) —
they fix a whole class at once and stop regression, exactly the harness doctrine (a fix isn't done
without a red→green guardrail). Then the **2 red-line clusters** (#16 vacuous money/RLS/PII proofs, #6
prod-write/dev-backdoor) are the security-critical ones to fix by hand. The flow-ui-* render-proof and
auth-control work (fixes 2, 4, 5, 8) is large but mechanical behind the shared helpers.

**Caveat (honest):** these are AGENT-PROPOSED blind-spots from one pass — high signal but not yet
adversarially verified per-finding. Before acting on any single CRITICAL, confirm it against the live
source (some "tautologies" may be intentional smoke-only specs). The guardrail approach is safe because
a lint rule that goes red→green on real files is self-verifying.
