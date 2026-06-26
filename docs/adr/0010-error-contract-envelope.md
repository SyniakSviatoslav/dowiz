# ADR-0010 — Structured error envelope + correlationId + error-code→UX matrix (Area A)

- Status: **Proposed** (design-time; staging-first FE+BE lockstep)
- Date: 2026-06-26
- Deciders: DeliveryOS Triadic Council
- Supersedes/relates: extends the single `setErrorHandler` (`apps/api/src/server.ts:541`); closes
  Frontend-Audit-Polish-Gate **C6** / Convergence **X7**.

## Context

One central handler emits `{ code:<number>, error:<string>, correlationId? }` but ~317 ad-hoc
`reply.status(n).send({error})` sites bypass it with divergent shapes. The FE (`apiClient.ts:195`)
reads only `error` and switches on HTTP status; server `code` strings are discarded; there is no
`mapApiError`. No `correlationId` is generated (the handler reads an inbound header nothing sets →
`'unknown'`, `server.ts:552`); Fastify has no `genReqId`/`requestIdHeader`/`requestIdLogLabel`
(`server.ts:143`). No contract test exists.

## Decision

> **REVISED in RESOLVE round (Breaker B1/B2/B3/B4/B6, Counsel 5a/5b).** The original "additive" claim
> was false and is corrected below.

1. **One envelope, one source — code-preserving superset.**
   `{ code: <SCREAMING_SNAKE STRING>, message, fields?, correlationId, retryAfterMs?, status?: <number,legacy> }`.
   `code` is the **string** machine code (the FE already reads it from ad-hoc sites). The handler's
   numeric `code` (`server.ts:546,565`) is **renamed to `status`**. The `reply.sendError(code,…)` helper
   passes the existing code **verbatim** — never invents/normalizes/renames/drops (B1). All error
   responses route through it + `setErrorHandler`. **Zero ad-hoc `reply.send({error})`.** The sweep
   discriminator is the **body shape**, never the status code — business outcomes (`hard_block`/`soft_confirm`,
   `{outcome,reasons}`) are **excluded** (regression trap).
2. **One correlation id — reuse existing P31, server-authoritative (B6/5a).** Adopt the existing
   `x-correlation-id` + `correlationStore` (`server.ts:243–246`) — do **not** add a parallel
   `x-request-id`. `requestIdHeader:'x-correlation-id'`, `requestIdLogLabel:'correlationId'`, `genReqId`
   = `generateCorrelationId()`/`crypto.randomUUID`. Server **always generates**; inbound is captured only
   as a sanitized `clientTraceId` (`^[A-Za-z0-9._-]{1,128}$`, length-capped) — never `req.id`, never the
   user-facing code. **Explicit (B6 caveat): rewrite `server.ts:244`** — today `inbound || generate()`
   (raw-trust); change to **always `generateCorrelationId()`**, inbound captured separately as a sanitized
   `clientTraceId`. **`clientTraceId` is never joined to user identity** (log-only, never persisted on a
   tenant row). **Add `correlationId` to the Sentry tag allowlist** (`sentry.ts:85`) or `setTag` is
   dropped (5b).
3. **Code-preserving rollout** (not big-bang, not versioned): add the new keys + rename numeric `code`→
   `status` (string `code` preserved → old FE green); BE first, then FE `mapApiError` reads `code`; drop
   `error` later separately.
4. **A2 matrix = the full live vocabulary (~50 codes, proposal Appendix A)**, not "documented paths".
   `verify:error-contract` asserts the money/price codes specifically (`MIN_ORDER_NOT_MET`, `CASH_*`,
   `MODIFIER_*`, `NOT_DELIVERABLE`) so a sweep can't silently drop them; FE has one `mapApiError`.
4b. **TWO NAMESPACES (B15):** SCREAMING_SNAKE-stable applies **ONLY to `envelope.code`** (the error path
   via `sendError`/`setErrorHandler`). `reasons[].code` (business-outcome tokens, e.g. lowercase
   `item_unavailable` at `preflight.ts:71` → `CheckoutPage.tsx:546`) are **outside the envelope contract,
   preserved verbatim**. A sweep must never normalize `reasons[]`. `verify:error-contract` asserts
   SCREAMING_SNAKE on `envelope.code` only + separately asserts the `reasons[].code`/`outcome` matches.
5. **A3 (B2):** 429 carries `Retry-After` + `retryAfterMs` + `code:'RATE_LIMIT'` via the rate-limit
   plugin's **`errorResponseBuilder`** (the only path that reaches the 429 body) — no global hook, **no
   double `Retry-After`**. Velocity outcomes are **out of the envelope**.
6. **Leak guarantee at ALL statuses (B4):** no `err.detail`/PG internals serialized (fixes
   `menu-import.ts:575–584`); 5xx generic; 422 `fields`=paths only.
7. **A4 — strict composite keyset (B3/B13):** `(created_at,id) < ($c,$i)`, `ORDER BY created_at DESC, id
   DESC`, cursor `{createdAt,id}`. Scope: dashboard-snapshot, **`owner/alerts.ts`** (B13 — same naive bug),
   customer/orders, signals/couriers. Read/list only — **never** the order-create txn. Forward-only
   composite indexes (`orders` + `location_alerts` + analogous), each via **`pgm.noTransaction()` +
   `CREATE INDEX CONCURRENTLY`** (B14 — CONCURRENTLY FATAL-fails in a txn; proven at `1790000000042:51`;
   INVALID-on-failure → drop+recreate). RLS + `location_id` is the tenant authority; forged-cursor test
   **runs under the operational role** (5d); HMAC reconsidered if `verify:rls` stays red.
8. **A5:** SRI + version on the public `widget.js` boundary only.

## Consequences

- (+) One legible contract; one server-owned `correlationId` stitching request→Pino→Sentry.
- (+) Code-preserving rollout is revertable per step; the FE's existing `code===` branches keep matching.
- (+) B3 also fixes a **live dashboard drop-bug** (same-`created_at` burst orders silently dropped).
- (−) Transitional window carries `error`+`code`+`status`; 317 sites convert incrementally (codes preserved).
- **Red-lines:** no order-create txn / RLS / applied-migration change; A4 read-path only, one forward-only
  CONCURRENTLY index.
- **Security:** generic 5xx, no stack, no `err.detail`; 422 `fields` paths-only; inbound demoted to
  `clientTraceId` (log-injection + support-code collision closed); forged cursor in-tenant (tested under
  operational role).

## Proof

`verify:error-contract` (string `code`+legacy `status`; money codes asserted incl `MIN_ORDER_NOT_MET`
matching `CheckoutPage.tsx:523`; **two-namespace test — SCREAMING_SNAKE on `envelope.code` only, lowercase
`reasons[].code` at `CheckoutPage.tsx:546` preserved**, B15; `x-correlation-id` echo = server id; 429
`errorResponseBuilder` Retry-After+retryAfterMs, no double header; soft_confirm/hard_block NOT via
mapApiError; no `err.detail`; forged inbound demoted to `clientTraceId`; Sentry tag lands) + FE
`mapApiError` unit over Appendix-A + A4 pagination test per endpoint incl `owner/alerts.ts` (same-ms tie
no drop/dup; forged cross-tenant → zero rows under operational role; index migration uses
`noTransaction()`, B14). Red→green guardrail + ledger row.
