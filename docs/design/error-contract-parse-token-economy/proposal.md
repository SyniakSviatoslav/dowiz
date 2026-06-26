# Design Proposal — `error-contract-parse-token-economy`

System-Architect, DeliveryOS Triadic Council. **DESIGN-TIME ONLY** — no production code.
Three independently-shippable areas under staging-first / red-line discipline:
A · Error-contract · B · Menu-parse LLM (legit subset, not RAG) · C · Agent token-economy (dev-only).

ADR drafts: `docs/adr/0010-error-contract-envelope.md` (A), `docs/adr/0011-menu-parse-eval-and-grounding.md` (B),
`docs/adr/0012-agent-token-economy-dev-tooling.md` (C).

---

## 0. Problem + non-goals

**Problem.** Error reporting is structurally inconsistent: one `setErrorHandler`
(`apps/api/src/server.ts:541`) emits `{ code: <httpStatus:number>, error: <string>, correlationId? }`,
but ~317 call sites bypass it with ad-hoc `reply.status(n).send({ error: '...' })` shapes (some
with `code:` = a SCREAMING_SNAKE string, some `code:` = a number, some `error:` = a sentence, some
`details:`). The FE (`apps/web/src/lib/apiClient.ts:195`) reads only `errorData?.error` and switches
on **HTTP status only** — server `code` strings are thrown away, there is no `mapApiError`. A P31
correlation system **already exists** (`server.ts:243–246`: `x-correlation-id` + `correlationStore`,
fallback `generateCorrelationId`) — so my earlier "'unknown' at `:552`" premise was **stale** (corrected
in §2b). The real gaps are: the inbound `x-correlation-id` is trusted **raw** at `:244` (log-injection +
collision surface), there is no `genReqId`/`requestIdHeader` binding it to `req.id` (`server.ts:143`),
`correlationId` is **not** in the Sentry tag allowlist (`sentry.ts:85` → `setTag` dropped), and there is
no machine-checkable contract test. This is exactly Frontend-Audit-Polish-Gate
**C6** / Convergence **X7** (error-code→UX matrix gap).

Parse: a real cascade parser already exists (`apps/api/src/lib/ai-ocr-parser.ts`,
zen→groq→openai→openrouter→heuristic with degradation) but model swaps are **unguarded** — nothing
detects a silent quality regression, no field carries `{value, confidence, grounded}`, and OCR text
is concatenated straight into the prompt (`ai-ocr-parser.ts:515`) — an injection surface.

Token-economy: agents re-derive invariants every session; no AST-semantic code search, no
git-versioned fix-memory.

**Non-goals.**
- NOT a loop / NOT a gate — three discrete agreed changes.
- A4 does **not** touch the order-create transaction, RLS policies, or any applied migration (read-path only).
- B is **not** RAG: no vector DB, no reranking, no multi-hop, no chunking. Eval does **not** replace human review.
- C tooling is **never shipped** to prod; dev-only; never indexes secrets.
- Not changing idempotency-key semantics, money representation (integer minor units), RS256, or `crypto.randomUUID`.

---

## 1. Back-of-envelope (real grep numbers)

| Quantity | Number | Source |
|---|---|---|
| Central error handlers | **1** | `server.ts:541` |
| Ad-hoc error-emitting sites (`.send({ error/code ...})`) | **~317** across **46** route files | grep count |
| Total `reply.status/code(...)` occurrences | **414** across **55** files | grep count |
| Heaviest files | orders.ts (27–36), product-media.ts (27), menu-import.ts (19–22), courier/auth.ts (17), order-messages.ts (16–17), categories.ts (10–14), products.ts (12) | grep |
| FE error chokepoints | **1** file (`apiClient.ts`, throw at :195, status-switch :157–193) | read |
| `mapApiError` today | **0** (does not exist) | grep |
| Fastify `genReqId`/`requestIdHeader`/`requestIdLogLabel` today | **0** | `server.ts:143–147` |
| List/read endpoints in A4 scope | **~3–4** real (dashboard-snapshot already keyset-cursored at `dashboard.ts:26,38–49`; `customer/orders.ts` no cursor; `owner/signals`, `owner/couriers`). **No** dedicated `crm-customers`/`courier-history` route files exist — those lists are embedded | grep (none found) |
| Eval fixtures proposed | **15** (5 pdf · 5 csv · 5 jpg) → 15 `expected/*.json` | design |
| Eval $ cost | **~$0** (zen free models + heuristic; CI runs offline against fixtures / `mock` provider) | `ai-ocr-parser.ts:80–104` |
| request-id overhead | `crypto.randomUUID()` ≈ 1µs/req + 1 response header (~36 B) | negligible |

**Reading of the numbers.** A1 (envelope shape + correlationId via the existing P31 system + Fastify
init) is a **small, central** change — 1 handler + 1 Fastify ctor + 1 FE chokepoint + the Sentry
allowlist. A2 (the 317 ad-hoc sites) is the real blast radius, a **mechanical, incremental** migration:
each site swaps `reply.status(n).send({error})` → a single `reply.sendError(code, ...)` helper that
**preserves the existing code string verbatim** (B1 — never normalize/rename, and the discriminator is
the body shape, never the status code, so business outcomes like `hard_block` are never swept). The ~50
live codes are inventoried (Appendix A); the money/price codes convert first under `verify:error-contract`.
A4 is small (~3–4 endpoints) but the reused baseline is buggy (B3) — corrected comparator + one index.
B & C add new files only; **B2 is gated on the B5 RLS-FORCE fix**.

---

## 2. Options + tradeoffs (the contentious decisions)

### (a) Error-envelope rollout strategy — FE↔BE lockstep ordering

> **REVISED after Breaker B1.** The original "additive" claim was **false**: the handler already
> emits `code:<number>` (`server.ts:546,565`) while the FE already reads `code:<string>` from ad-hoc
> sites (`CheckoutPage.tsx:523,534`; `MenuFirstOnboarding.tsx:107,127`), and ~50 string codes are live
> in routes. `code` is an **existing, overloaded** field — redefining it is a breaking type change, not
> an addition.

**Envelope shape (corrected):**
`{ code: <SCREAMING_SNAKE string>, message, fields?, correlationId, retryAfterMs?, status?: <number, legacy> }`

- `code` is the **string** machine code — exactly what the FE already consumes from ad-hoc sites. Full
  live vocabulary (~50 codes) enumerated in **Appendix A**; it is the BE↔FE contract.
- The handler's numeric `code` (`server.ts:546,565` — the *only* numeric-`code` emitter) is **renamed to
  `status`**, unifying `code` as a string across handler + ad-hoc sites; no other consumer affected.
- The A2 `reply.sendError(code, …)` helper passes the existing code string **verbatim** — it must
  **never** invent, normalize, rename, or drop a code (the B1 regression vector).

| Option | Concept | Tradeoff |
|---|---|---|
| **A. Big-bang swap** | Replace shapes everywhere + FE reads new shape, one deploy | FE↔BE breaking change deployed atomically; any missed ad-hoc site (of 317) regresses silently. Rejected. |
| **B. Code-preserving superset** (chosen) | Add `message`/`fields`/`correlationId`/`retryAfterMs`; keep the string `code` **verbatim**; keep `error`; rename only the handler's numeric `code`→`status`. FE `mapApiError` reads the (already-present) string `code` | No flag-day; BE-first then FE; revertable per step. The FE's existing `code===` branches keep matching because codes are **preserved, not normalized**. |
| **C. Versioned (`Accept-Version` / path)** | `/v2` envelope alongside `/v1` | Doubles surface for one internal FE. Rejected. |

**Deploy order (explicit):** BE adds `message`/`correlationId`/etc + renames numeric `code`→`status`
(string `code` preserved → old FE green) → validate on staging → FE adds `mapApiError` reading the
string `code` → validate. `error` retained until a separate cleanup. Widget.js (A5) is the only place
with SRI + version pin.

### (b) Inbound correlation-id trust model

> **REVISED after Counsel 5a + Breaker B6.** A correlation system **already exists** (P31,
> `server.ts:243–246`: `onRequest` sets `x-correlation-id` + an AsyncLocalStorage `correlationStore`,
> falling back to `generateCorrelationId()`). My original "'unknown' at `:552`" premise was **STALE**
> — the hook at `:244` already populates the id before `setErrorHandler`. Introducing a parallel
> `x-request-id` would create **two correlation ids** — the exact incoherence the envelope exists to
> kill. And inbound `x-correlation-id` is currently trusted **raw** (`:244`) — the live
> log-injection + support-code-collision hole.

| Option | Concept | Tradeoff |
|---|---|---|
| **A. Trust inbound as the id** (current `:244`) | Use inbound `x-correlation-id` as `req.id` | Log-injection (raw) + **collision**: attacker sets it to a victim's shown support code → poisoned trace (B6). Rejected. |
| **B. New `x-request-id` channel** (my original) | Sanitize inbound `x-request-id`, use as `req.id` | Two parallel ids (incoherent with P31); still authoritative-inbound (B6 collision). Rejected. |
| **C. Server-authoritative, inbound demoted** (chosen) | Server **always generates** its own id (existing `generateCorrelationId`/`crypto.randomUUID`); inbound `x-correlation-id` is captured as a **separate sanitized `clientTraceId`** log field for widget/WS stitching — never `req.id`, never the user-facing support code | One id, server-owned; inbound can't collide or inject; reuses P31's header/store/label. |

**Decision: C.** Reuse the **existing** `x-correlation-id`/`correlationStore` as the one id;
`requestIdHeader:'x-correlation-id'`, `requestIdLogLabel:'correlationId'`, `genReqId` wired to
`generateCorrelationId`. **Always server-generate.** Inbound → sanitized (`^[A-Za-z0-9._-]{1,128}$`,
length-capped) `clientTraceId` only.
**Explicit change (B6 caveat):** `server.ts:244` today does `const correlationId = inbound || generate()`
— raw-trusting inbound. This line is **rewritten** to always `generateCorrelationId()`; the inbound value,
if present, is captured separately as the sanitized `clientTraceId`. **`clientTraceId` is never joined to
user identity** — log-only trace attribute, never indexed against a user/session, never persisted on a
tenant row (Counsel note). *Deviation from the literal change-set header name `x-request-id` recorded:
coherence (one id) > literal spec.*

### (c) Pagination cursor encoding

> **REVISED after Breaker B3 + Counsel 5d.** The `dashboard.ts` baseline I proposed to reuse is the
> **broken one**: naive `created_at < $N` (`:43`), `ORDER BY created_at DESC` with **no id** (`:64`),
> cursor carrying only `{createdAt}` (`:153`), and a backing index without id
> (`1780310074262_orders.ts:45`). Same-`created_at` burst rows are **silently dropped** — a live bug.

| Option | Concept | Tradeoff |
|---|---|---|
| **A. Opaque signed cursor** (HMAC) | Sign the keyset payload | Tamper-proof, but tenant-safety already rests on RLS + `location_id`. **Conditionally reconsidered** (5d): if `verify:rls` stays red or `import_sessions` FORCE is unconfirmed, signing becomes a justified second lock. |
| **B. Strict composite keyset** (chosen) | base64url-JSON `{createdAt, id}`, predicate `(o.created_at, o.id) < ($c, $i)`, `ORDER BY o.created_at DESC, o.id DESC` | Fixes the live drop/dup bug; deterministic under same-millisecond ties. Forgeable cols, but a forged value repositions **in-tenant only** (RLS + `location_id=$1`). |
| **C. Offset** | `LIMIT/OFFSET` | Drifts under concurrent inserts, O(n) deep pages. Rejected. |

**Decision: B (corrected comparator)**, conditional on 5d: the forged-cross-tenant proof test **MUST
run under the operational (non-BYPASSRLS) role**, not superuser; HMAC signing is reconsidered if RLS
proof is not green. Index: forward-only `CREATE INDEX CONCURRENTLY ON orders(location_id, created_at
DESC, id DESC)` (read-path; no table/RLS change).

---

## 3. Decision per contentious point (summary)

- **(a)** Code-preserving superset; string `code` is authoritative (verbatim), numeric→`status`; BE-first then FE.
- **(b)** One server-authoritative `x-correlation-id` (reuse P31); inbound demoted to sanitized `clientTraceId`.
- **(c)** Strict composite `(created_at, id)` keyset; forged-cursor test under operational role; HMAC reconsidered if RLS red.

---

## 4. Data / migrations

- **A1–A5: ZERO schema change.** Envelope is response-shape only. `correlationId` is request-scoped
  (`req.id`), never persisted.
- **A4 scope (R2/B13): dashboard-snapshot, `owner/alerts.ts`, customer/orders, signals/couriers.**
  `owner/alerts.ts:45–55,70,92–93` carries the **identical broken naive keyset** and was missing from the
  original scope — added now (same-timestamp burst alerts must not drop = missed escalation/SLA).
- **A4: ZERO table-schema change; forward-only composite indexes (B3/B13/B14).** `orders(location_id,
  created_at DESC)` lacks `id` (`1780310074262_orders.ts:45`); the strict `(created_at,id)` keyset needs
  `CREATE INDEX CONCURRENTLY ON orders(location_id, created_at DESC, id DESC)` (and the analogous
  `location_alerts(location_id, created_at DESC, id DESC)`; any other A4 list whose `(tenant,sort)` index
  lacks the tiebreaker). **B14: each index migration MUST call `pgm.noTransaction()`** — `CREATE INDEX
  CONCURRENTLY` FATAL-fails inside node-pg-migrate's default txn (proven pattern `1790000000042:51`); on
  failure the index is left INVALID → recover via drop+recreate / `REINDEX`. Read-path only, no rewrite,
  no lock, no RLS change.
- **B5 gate is now ENFORCEABLE (R2/B12).** The original gate was a no-op: `verify-rls.ts` TENANT_TABLES
  (`:27–55`) **omits `import_sessions`** and only `exit(1)`s on missing **ENABLE** (`:130–132`) — FORCE is
  merely **logged** (`:134`), so `verify:rls` is green with FORCE absent. **Strengthen the guardrail:**
  (1) add `import_sessions` to TENANT_TABLES; (2) make `verify:rls` **fail on `!relforcerowsecurity`** for
  tenant tables. Edits a **verify script** (guardrail-strengthening, allowed), not product code/RLS.
- **B: `import_sessions` is RLS ENABLE-only, NOT FORCE — B2-grounding is GATED (B5).** Confirmed
  `levelSecurity:'ENABLE'` (`1780338982025:26`), **absent** from `1780421100051_force-rls.ts`. B2's
  `{value,confidence,grounded}` rides in `draft_json` (jsonb, **no migration**) — but **B2-grounding MUST
  NOT ship** until the strengthened `verify:rls` (B12) is green AND a forward-only `import_sessions` FORCE
  migration has landed (**separate red-line change, DB owner**). **The STOP-1 OCR-redaction (B3-privacy)
  ships INDEPENDENT of this gate** (touches only prompt construction, not the table) — the earliest-shipping
  part of B. Import still lands as a draft → explicit owner-publish (activation-gate).
- **C: ZERO schema change** (dev tooling + markdown).

All A4 index migrations are **forward-only, `noTransaction()`+CONCURRENTLY, integer-safe**; money stays
integer minor units everywhere (`ai-ocr-parser.ts:756`).

---

## 5. Consistency + idempotency

- **Envelope must not touch idempotency semantics.** The FE sends `X-Idempotency-Key` on POST/PUT/PATCH
  (`apiClient.ts:116–118`); the envelope is a *response* concern and changes nothing about the
  idempotency key, its storage (Postgres, not Redis — `import_sessions.idempotency_key` unique index,
  `1780338982025:23`), or replay behavior. A retried request gets the same envelope it got the first time.
- **`correlationId` is per-attempt, not per-idempotency-key.** Two retries of the same idempotent op get
  two different `correlationId`s (one per HTTP attempt) — correct: they are distinct log events. The
  idempotency key correlates the *business* op; `correlationId` correlates the *request*.
- **Eval determinism.** Fixtures are committed bytes; expected JSON committed. Scorer runs `temperature:
  0.1` (`ai-ocr-parser.ts:127`) OR the deterministic `heuristic`/`mock` provider in CI. Price comparison
  is **exact integer-minor-unit equality** (zero tolerance). Thresholds are explicit + version-controlled
  (B9): item-recall ≥ 0.95, modifier-structure ≥ 0.90; the fixture set **grows on each real-world miss**
  (add the failing menu). Honest scope: this blocks a **measured regression on the committed set**, not
  every regression (15 self-authored fixtures; independent oracle = future work).
- **Grounding normalizer parity (Breaker B7).** B2 `grounded` must NOT use a substring check. The parser
  normalizes prices (`"1.200"→1200`, `Math.round(value*10^minorUnit)`, `ai-ocr-parser.ts:745–756`); a
  naive substring both **false-flags** correct prices (`120000` ∉ `"1.200"`) and **false-passes**
  hallucinations (`price:1` ⊂ almost any text). Grounding compares the parsed minor-unit value against
  price-tokens extracted from the OCR text **via the same `priceOf` normalizer** — exact normalized match
  both sides.

---

## 6. Failures + degradation (failure-first)

| External call | Timeout | Fallback | No-cascade guarantee |
|---|---|---|---|
| **Sentry** (tag set) | n/a (fire-and-forget) | If Sentry init absent/down, `setErrorHandler` still returns the envelope + logs to Pino with `correlationId`. Sentry is best-effort, never on the response path. **NOTE (Counsel 5b): `correlationId` must be added to the Sentry tag allowlist (`sentry.ts:85`) or `beforeSend` silently drops `setTag` (`:87`)** — A1 adds it + a proof that the tag lands | A Sentry failure never changes the HTTP response |
| **OpenRouter/Zen/Groq/OpenAI** (parse) | 120 s `AbortController` (`ai-ocr-parser.ts:517,109`) | Cascade then **heuristic structurer** — already implemented (`:522–523`), surfaced as a `warning`, never a hard fail | Import degrades to a reviewable draft; never 0 products |
| **OCR (Tesseract/Paddle)** | Paddle 120 s subprocess (`:285`) | Tesseract default; OCR failure → `PARSE_ERROR` + `fallbackError` (`:382–385`) | One process per image, exits |
| **Missing `correlationId`** | n/a | P31 `onRequest` (`server.ts:244`) + `genReqId` always produce one server-side. The `'unknown'` branch at `:552` was a **stale premise** — the hook already populates it; A1 makes the generation unconditional regardless | Every log line + every envelope carries a real server-owned id |
| **Rate-limit (429)** | n/a | **CORRECTED (B2):** `@fastify/rate-limit` (`server.ts:486`) never enters `setErrorHandler` — it builds its own body. A3 adds an **`errorResponseBuilder`** to the plugin emitting `{code:'RATE_LIMIT', retryAfterMs, …}`; the plugin keeps setting `Retry-After` (no global hook → **no double header**) | FE shows a humane "try again in Ns" via `code:'RATE_LIMIT'` |
| **Velocity gate** | n/a | **CORRECTED (B2):** `soft_confirm` (200, `orders.ts:371`) / `hard_block` (422, `:366`) are **NON-error business outcomes** (`{outcome,reasons,requiresOtp}`), handled on the FE **success path**, **NOT** via `mapApiError`. They never carry `code` and are explicitly **out of the envelope** | The preflight/soft-confirm flow reads `outcome`, never the error envelope |

Degradation is designed before happy-path: the envelope's job is to make every failure legible.

---

## 7. Security + tenant (adversarial focus)

- **(i) No internal leak / no PII in envelope.**
  - 5xx `message` is **generic** — already enforced (`server.ts:560`); A1 keeps this + adds `code:'INTERNAL'`.
    Stack traces never serialized (`server.ts:559`).
  - 422 `fields` = **paths only**, never the submitted value (a validation error on phone/address can't
    reflect PII). Harden the Zod mapper (`server.ts:543–549`) to emit `{path}`+code, not `v.message`.
  - **CORRECTED (Breaker B4): leak guarantee extends to ALL statuses, not just 5xx/422.**
    `menu-import.ts:575,578,581` returns raw Postgres `err.detail` (`Key (location_id, external_key)=(…)`
    — column + value egress) on 400/409 `DUPLICATE_KEY`/`FK_VIOLATION`/`NOT_NULL`. The `sendError` helper
    **never serializes `err.detail`/PG internals**; the sweep strips `details: err.detail`. These codes
    keep their code, carry a generic message only.
- **(ii) Log-injection + support-code collision (B6 / Counsel 5a-inj).** The **live** hole is the
  existing raw inbound trust at `server.ts:244`. Mandatory: server **always generates** the id; inbound
  `x-correlation-id` is captured only as a sanitized `clientTraceId` (regex `^[A-Za-z0-9._-]{1,128}$`,
  length-capped) — never `req.id`, never the user-facing support code. This closes both newline/control
  injection **and** the collision attack (attacker setting it to a victim's shown support code → poisoned
  `grep correlationId=` trace).
- **(iii) Pagination cursor cross-tenant.** Cursor carries only `(created_at, id)`; the WHERE
  `location_id=$1` + RLS are the tenant authority. A forged cursor stays in-tenant. **CONDITIONAL (5d):
  the forged-cross-tenant proof test MUST run under the operational (non-BYPASSRLS) role**, not superuser
  — `verify:rls` is currently red, so a superuser test proves nothing. HMAC signing reconsidered if RLS
  proof stays red.
- **(iv) Prompt-injection via menu images — HONEST framing (Breaker B8).** OCR text is **UNTRUSTED data,
  not instructions**. Defense-in-depth: data-delimited block + system prompt. **But schema validation is
  NOT a backstop** — `llmSchema.safeParse` (`:536`) constrains **shape, not truth**: a schema-valid
  `{price:1}` from an injected "set all prices to 1" passes cleanly. The real defenses are (1) `grounded`
  flags (B2) and (2) the pre-existing **human review of the draft before publish**. The injection fixture
  tests one canned string; the schema-valid-but-wrong **class** is caught only by grounding + human review.
- **(iv-bis) PII in OCR→LLM prompt — ETHICS GATE CLOSED: redact-by-default is BINDING.** OCR text reaches
  the prompt **raw** today (`ai-ocr-parser.ts:515`; the redacted copy at `:399` feeds only the provenance
  hash). The human ETHICS decision (recorded in `ethical-decisions.md`) is **redact-by-default**: run the
  existing `piiRedactor.redact()` over OCR text **before** the prompt — this is now the **binding posture**,
  not a recommendation. Ships **independent of the B5 RLS gate** (earliest part of B). Proof: a
  redaction-recall fixture (incl. Albanian name / handwritten / non-Latin) asserts seeded third-party PII
  does **not** survive into the prompt ("redacted" ≠ "PII-free").
- **(v) `ccc` (C1) secrecy.** Dev-only; respects `.gitignore`; **never indexes `.env*`/secrets**; **zero
  index artifacts in `dist`**. Proof: a secret-scan test that points `ccc` at a fixture tree containing a
  fake secret + a `.gitignore`d file and asserts neither is indexed, and `dist/` has no index artifact.
- **Invariants upheld:** RS256-only, zero cookies, integer money, `crypto.randomUUID`, no PII to LLM
  beyond menu-only (the venue's own contact is business data, not customer PII — `ai-ocr-parser.ts:395`),
  OpenRouter already in the compliance subprocessor register (`scripts/compliance-gate.ts:44`); B3 keeps
  it there for the vision-review layer (seed only).

---

## 8. Operability (correlationId without a support team)

- **Debug flow (single dev):** user reports a failure → reads the `code` shown in the error UI and a
  **short speakable handle** (first 8 chars of `correlationId`) shown in a "report this problem"
  affordance, not a bare UUID inline (Counsel #6) → operator uses the full id: `grep correlationId=<id>`
  in Pino logs (label `correlationId` via `requestIdLogLabel`) and/or Sentry `correlationId:<id>`. One
  server-owned id stitches request → log → Sentry, no APM. **Requires (5b): `correlationId` in the
  Sentry allowlist** (`sentry.ts:85`) or the tag is dropped and the stitch breaks.
- **Observability <1 min:** stable SCREAMING_SNAKE `code` → filter by `code:CASH_AMOUNT_TOO_LOW` etc
  without parsing free text.
- **Health degraded-vs-down:** unaffected — `/health` + `/livez` stay; parse LLM→heuristic is a `warning`,
  not a health flip.
- **Rollback:** code-preserving superset (§2a) → revert BE, old shape still satisfies un-tightened FE.
  A4 keyset is read-only → revert is a redeploy (the new composite index can stay — it is strictly
  better than the old one). B/C are new files → delete.
- **Flag / scaling gate:** B2 ships only after the B5 RLS-FORCE gate clears; B3 prompt-hardening rides the
  existing import preview (draft, owner-publish). C is dev-only (never in the prod image). A5 widget
  version/SRI is the one public-boundary gate.

---

## 9. Open / accepted risks

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R1 | 317 ad-hoc sites convert incrementally → mixed shapes transitionally | **Accept** — code-preserving superset makes mixed shapes safe; the helper preserves codes verbatim (never normalizes — B1); documented set converted first under `verify:error-contract` | System-Architect |
| **STOP-1** | OCR text → LLM prompt is **raw** PII (incidental third-party names/phones) | **ETHICS GATE CLOSED — redact-by-default BINDING** (`ethical-decisions.md`): `piiRedactor` over OCR before the prompt; ships independent of B5; redaction-recall fixture proves it | Human (recorded) |
| **B5/B12** | `import_sessions` RLS **ENABLE-only, not FORCE**; AND the `verify:rls` gate was a **no-op** (omits the table, only checks ENABLE) | **GATE, now ENFORCEABLE** — strengthen `verify-rls.ts` (add table + fail on missing FORCE); B2-**grounding** blocked until that's green + a FORCE migration lands. STOP-1 redaction is **not** blocked by this | DB owner |
| R3 | Composite keyset cursor is forgeable | **Accept, conditional (5d)** — RLS + `location_id` is the authority; forge → in-tenant only. Proof test **MUST run under operational role**; HMAC reconsidered if RLS proof stays red | System-Architect |
| R4 | Eval thresholds (recall/structure) too loose | **Accept w/ guardrail** — price zero-tolerance exact; recall ≥0.95 / structure ≥0.90 explicit, fixture set grows on each miss; "blocks measured regression on committed set" (not every regression — B9) | Parse owner |
| R5/B8 | OCR prompt-injection: schema-valid-but-wrong output | **Accept (honest)** — schema validation is **shape not truth**, NOT a backstop; real floor is `grounded` flags + pre-existing human review of the draft | Parse owner |
| R6/B10 | `ccc` indexing a secret despite `.gitignore` | **Mitigated/gated** — `ccc` consults ignore rules before reading bytes; secret-scan **merge gate** must pass before C1 enabled; dev-only blast radius | Tooling owner |
| B11 | C2 two-sources-of-truth drift (`INVARIANTS.md` vs lint) | **Fixed** — executable guardrail authoritative; `INVARIANTS.md` links, never restates | Tooling owner |
| C2-gov | Self-authored agent rules bind the next agent without a human gate (Counsel 5c/§5) | **Fixed** — appends route through the librarian promotion gate (lesson → human-reviewed guardrail), never raw in-session writes | Tooling owner |

---

## 10. Proof obligations (Task-Exit / STOP-DESIGN-B mapping)

**A · `verify:error-contract`:**
- asserts `(httpStatus, envelope.code [SCREAMING_SNAKE, stable], shape {code,message,correlationId})`;
- **`code` stays a STRING** and the legacy numeric is `status` — assert no consumer sees numeric `code`;
- **money/price codes asserted specifically (B1):** `MIN_ORDER_NOT_MET` (status 422 + code, and a
  guardrail that `CheckoutPage.tsx:523` still matches), `CASH_AMOUNT_TOO_LOW` (`orders.ts:595`),
  `CASH_AMOUNT_MISMATCH` (`assignments.ts:326`), `MODIFIER_MIN_NOT_MET`/`MODIFIER_MAX_EXCEEDED`
  (`orders.ts:523,527`), `NOT_DELIVERABLE` (`:576`) — a sweep cannot silently drop/rename these;
- `x-correlation-id` response header present + matches the **server-generated** `req.id`;
- **429 via `errorResponseBuilder`** → `Retry-After` header AND `retryAfterMs` in body + `code:'RATE_LIMIT'`
  (B2); assert **no double `Retry-After`**;
- **soft_confirm(200)/hard_block(422) explicitly NOT routed through `mapApiError`** — a test that these
  carry `{outcome,reasons}` and the sweep left them untouched (regression trap);
- **TWO NAMESPACES (B15):** assert SCREAMING_SNAKE on `envelope.code` **only**; **separately** assert the
  lowercase `reasons[].code` (`item_unavailable`/`velocity`/…) + `outcome` matches at `CheckoutPage.tsx:546,551`
  still hold and were not normalized;
- 5xx → `message` generic, no stack; **no `err.detail`/PG internals at any status** (B4: assert
  `menu-import` 409 carries no `Key (...)=...`);
- 422 → `fields` paths only, no submitted PII value;
- forged inbound `x-correlation-id` (newline/oversized/victim-code) → server id used, value demoted to
  sanitized `clientTraceId`, no log injection, no collision (B6);
- **Sentry tag lands (5b):** capture an event, assert `correlationId` survives `beforeSend` (allowlist);
- FE unit: `mapApiError({status,code})` covers the full Appendix-A vocabulary (closes C6/X7).
- **A4 pagination test (B3/B13):** for **each** A4 endpoint incl `owner/alerts.ts`, seed N+1 rows incl
  **same-millisecond ties** → `nextCursor` returned, page-2 disjoint, no dropped/duped row across the tie
  boundary; forged cross-tenant cursor returns zero foreign rows **under the operational (non-BYPASSRLS)
  role** (5d). Migration test: the index migration uses `pgm.noTransaction()` and creates a valid index (B14).
- **B12 verify:rls gate test:** with FORCE absent on a tenant table → `verify:rls` **exits 1** (red); with
  FORCE present → green. `import_sessions` is in TENANT_TABLES.

**B · parse-eval thresholds:**
- price = **EXACT integer-minor-unit match, 100%, zero tolerance** on all 15 fixtures;
- item recall ≥ 0.95; modifier-group structure ≥ 0.90 (explicit, version-controlled);
- **eval blocks a measured model-swap regression** on the committed set, does NOT replace human review;
- **grounding via the same `priceOf` normalizer (B7):** false-flag fixture (`1.200 Lek` correct →
  `grounded:true`) + false-pass fixture (hallucinated `price:1` not a normalized OCR token →
  `grounded:false`/flagged), never auto-published;
- prompt-injection fixture (directive-in-OCR → prices unchanged) **plus an explicit note** that the
  schema-valid-but-wrong class is caught only by grounding + human review (B8);
- **PII redaction of OCR before the prompt — BINDING (ETHICS closed), ships independent of B5:**
  **redaction-recall fixture (Counsel a)** — a menu image with seeded third-party PII (incl. **Albanian
  name / handwritten / non-Latin script**) → assert none survives into the prompt sent to the provider
  ("redacted" ≠ "PII-free");
- OpenRouter subprocessor present in compliance-gate (vision layer);
- **B5/B12 gate (grounding only):** strengthened `verify:rls` green (fails on missing FORCE; includes
  `import_sessions`) + FORCE migration landed before **B2-grounding** ships. STOP-1 redaction is **not**
  gated by this.

**C · `ccc` secret-scan:**
- `ccc` consults ignore rules **before reading bytes**; over a fixture tree never indexes `.env*`/
  `.gitignore`d secrets; zero artifact in `dist/`; the test is a **merge gate**;
- C2 appends route through the **librarian gate**; the executable guardrail is authoritative,
  `INVARIANTS.md` links (not restates).

---

## Appendix A — error-code vocabulary (the REAL inventory, B1) + UX matrix

> This is the BE↔FE contract. Codes are SCREAMING_SNAKE, **stable**, and **preserved verbatim** by the
> A2 sweep. The ~50 string codes below are the live vocabulary (grep of `apps/api/src/routes`); the
> ~10-code Appendix in the original draft was incomplete and is replaced. FE maps `code` first, falls
> back to `status`/`error` during the transitional window. `mapApiError` must cover **all** of these.

**FE-consumed today (must not break — verified call sites):**
`MIN_ORDER_NOT_MET` (`CheckoutPage.tsx:523` ← `orders.ts:548`, 422) · `UNSUPPORTED_TYPE`
(`MenuFirstOnboarding.tsx:107` ← `menu-import.ts:187`) · `SLUG_TAKEN` (`MenuFirstOnboarding.tsx:127` ←
`spa-proxy.ts:741`/`onboarding.ts:59`, 409) · generic `err?.data?.code` read at `CheckoutPage.tsx:534`.

**Money / order-create (`orders.ts`):** `MIN_ORDER_NOT_MET` (548) · `CASH_AMOUNT_TOO_LOW` (595) ·
`MODIFIER_MIN_NOT_MET` (523) · `MODIFIER_MAX_EXCEEDED` (527) · `DUPLICATE_MODIFIER` (499) ·
`MODIFIER_UNAVAILABLE` (506) · `PRODUCT_UNAVAILABLE` (423) · `PRODUCT_NOT_FOUND` (431) ·
`NOT_DELIVERABLE` (576) · `DELIVERY_NOT_CONFIGURED` (582) · `NOT_PUBLISHED` (135) ·
`PHONE_THROTTLE` (77,265) · `IP_THROTTLE` (290) · `IDEMPOTENCY_KEY_REUSED` (398) ·
`IDEMPOTENCY_CONFLICT` (806). **Cash handoff:** `CASH_AMOUNT_MISMATCH` (`assignments.ts:326`).
**Lifecycle:** `INVALID_TRANSITION` (`shifts.ts:253`). **Ratings:** `NOT_DELIVERED`,
`RATING_WINDOW_CLOSED` (`customer/orders.ts:232,235`).

**Import (`menu-import.ts`):** `UNSUPPORTED_TYPE` (187) · `COMMIT_TOKEN_MISMATCH` (277) ·
`IMPORT_SESSION_EXPIRED` (283) · `IMPORT_SESSION_FAILED` (288) · `LOW_CONFIDENCE_REQUIRES_FORCE` (298) ·
`REPLACE_BLOCKED_BY_HISTORICAL_ORDERS` (449) · `DUPLICATE_KEY`/`FK_VIOLATION`/`NOT_NULL`/`MISSING_COLUMN`
(575–584, **B4: strip `err.detail`**). **Translate:** `UNSUPPORTED_LOCALE` (`menu-translate.ts:47`).
**Activation:** `NOT_READY_TO_PUBLISH` (`activation.ts:102`).

**Courier (`courier/auth.ts`):** `INVITE_INVALID` · `INVALID_CODE` · `INVALID_CREDENTIALS` ·
`COURIER_DEACTIVATED` · `NOT_AUTHORIZED_FOR_LOCATION` · `NO_LOCATION_ASSIGNED` · `INVALID_REFRESH_TOKEN` ·
`SESSION_NOT_FOUND` · `REFRESH_REUSED` · `REFRESH_EXPIRED`. **Auth:** `OWNER_REVOKED` (`auth.ts:300`).
**Onboarding/slug:** `SLUG_TAKEN`.

**Cross-cutting (handler/plugins):** `VALIDATION_FAILED` (400, Zod `server.ts:543`) · `UNAUTHORIZED`
(401, `server.ts:535`) · `FORBIDDEN` (403) · `NOT_FOUND` (404) · `RATE_LIMIT` (429, via
`errorResponseBuilder` — B2) · `INTERNAL` (500, generic).

**Explicitly OUT of the error envelope (business outcomes, FE success path — B2):**
`soft_confirm` (200, `orders.ts:371`), `hard_block` (422, `:366`) carry `{outcome,reasons,requiresOtp}` —
read via `outcome`/`reasons`, never `mapApiError`. The former "`VELOCITY_LIMIT`" matrix row is **removed**
as an error code (velocity surfaces only as these outcomes).

**TWO NAMESPACES (B15) — do not conflate.** The SCREAMING_SNAKE-stable contract applies **ONLY to
`envelope.code`** (the error path through `sendError`/`setErrorHandler`). The `reasons[].code` tokens
inside business-outcome payloads are a **separate namespace, OUTSIDE the envelope contract, preserved
verbatim — including lowercase**: `preflight.ts:71,78,88,95` emit `code:'item_unavailable'`,
`:113,121` `'velocity'`, `:130` `'no_show_history'`, `:138` `'otp_required'`, consumed at
`CheckoutPage.tsx:546` (`code==='item_unavailable' || outcome==='hard_block'`) + `:551` (`reasons[0].message`).
A future A2 sweep must **never normalize `reasons[]`** to SCREAMING_SNAKE — that would break
`CheckoutPage.tsx:546`. `verify:error-contract` asserts SCREAMING_SNAKE on `envelope.code` only, and
**separately** asserts the `reasons[].code`/`outcome` matches still hold.

**Representative UX mapping (`mapApiError`):**

| `code` | HTTP | FE UX |
|---|---|---|
| `VALIDATION_FAILED` | 400 | inline field errors from `fields` |
| `UNAUTHORIZED` | 401 | silent refresh → login bounce (owner only) |
| `MIN_ORDER_NOT_MET` | 422 | "add {min−subtotal} more to order" (money gate — **must not regress**) |
| `CASH_AMOUNT_TOO_LOW` | 422 | "cash must be ≥ {total}" |
| `MODIFIER_MAX_EXCEEDED` | 422 | "you can pick at most N" |
| `NOT_DELIVERABLE` | 422 | "outside delivery area" |
| `SLUG_TAKEN` | 409 | "that link is taken — pick another" |
| `RATE_LIMIT` | 429 | "try again in {retryAfterMs/1000}s" |
| `INTERNAL` | 500 | generic + fallback phone, cart intact |
