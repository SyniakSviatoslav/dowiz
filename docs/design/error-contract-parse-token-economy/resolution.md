# Resolution — `error-contract-parse-token-economy`

System-Architect, RESOLVE round. One row per Breaker finding + Counsel point → verdict → what changed.
All code claims re-verified against live source before ruling (citations inline). The two load-bearing
proposal premises ("additive dual-read" and "every error routes through setErrorHandler") were **both
false** as written; the design is revised, not restated.

**Verdict tally:** fix = 15 · accept-risk = 2 · defer-flag = 1 · human-decision (ETHICS gate) = 1.
**Remaining CRITICAL/HIGH open:** none unaddressed (B1/B2/B3 fixed in place); B5 is a defer-flag gate
(B2 cannot ship until `import_sessions` FORCE is confirmed/added — separate red-line change).

---

## Breaker findings

| # | Sev | Verdict | What changed (concrete) |
|---|---|---|---|
| **B1** | CRITICAL | **FIX** | Verified: handler emits `code:<number>` (`server.ts:546,565`); FE reads `code:<string>` (`CheckoutPage.tsx:523,534`, `MenuFirstOnboarding.tsx:107,127`); ~50 string codes live in routes (grep). **Reconciliation:** the envelope's machine code field is `code: <SCREAMING_SNAKE string>` — which is what the FE *already* reads from the ad-hoc sites. The handler's numeric `code` is **renamed to `status`** at the single emission site (`server.ts:546,565`) — the one place that emits numeric `code` — so `code` is unified as a string across handler + ad-hoc. The A2 `reply.sendError(code, …)` helper **preserves the existing code string verbatim** — it never invents/normalizes/drops a code. Full ~50-code vocabulary enumerated in proposal Appendix A. `verify:error-contract` asserts `MIN_ORDER_NOT_MET`/`CASH_AMOUNT_TOO_LOW`/`MODIFIER_*`/`NOT_DELIVERABLE` specifically + a guardrail that the FE branch at `CheckoutPage.tsx:523` still matches. |
| **B2** | HIGH | **FIX** | Verified: rate-limit has no `errorResponseBuilder` (`server.ts:486`); soft_confirm=200 (`orders.ts:371`), hard_block=422 (`orders.ts:366`) are `reply.send` business outcomes, not throws. **Change:** A3 now adds an `errorResponseBuilder` to `@fastify/rate-limit` (the only mechanism that reaches the 429 body) emitting `{code:'RATE_LIMIT', retryAfterMs, …}`; the plugin keeps setting `Retry-After` (no global hook → no double header). soft_confirm/hard_block declared **NON-error business outcomes** handled on the FE success path via `outcome`/`reasons`/`requiresOtp`, **never** `mapApiError`. Matrix: `VELOCITY_LIMIT` removed as an envelope code (it is an outcome, not an error); `PHONE_THROTTLE`/`IP_THROTTLE` (`orders.ts:77,265,290`) kept as real error codes. |
| **B3** | HIGH | **FIX** | Verified the baseline I proposed to reuse is the broken one: naive `created_at < $N` (`dashboard.ts:43`), `ORDER BY created_at DESC` no id (`:64`), cursor `{createdAt}` only (`:153`), index has no id (`1780310074262_orders.ts:45`). **Change:** strict composite comparator `(o.created_at, o.id) < ($c, $i)` with `ORDER BY o.created_at DESC, o.id DESC`, cursor `{createdAt, id}`. Forward-only `CREATE INDEX CONCURRENTLY idx ON orders(location_id, created_at DESC, id DESC)` (read-path, no table/RLS change). This also **fixes the live dashboard drop-bug** (same-`created_at` burst orders silently dropped). Proof obligation gains a same-millisecond tie test. |
| **B4** | MED | **FIX** | Verified `err.detail` leaked at `menu-import.ts:575,578,581` (Postgres `Key (location_id, external_key)=(…)` → column+value egress). **Change:** §7-i leak guarantee extended to **all** statuses, not just 5xx/422: the `sendError` helper never serializes `err.detail`/Postgres internals; the sweep strips `details: err.detail`. `DUPLICATE_KEY`/`FK_VIOLATION`/`NOT_NULL` keep their code but carry a generic message only. |
| **B5** | MED | **DEFER-FLAG (gate)** | Verified `import_sessions` is `levelSecurity:'ENABLE'` only (`1780338982025:26`) and **absent** from `1780421100051_force-rls.ts` (grep). This is a red-line table; FORCE add is a migration + human decision. **Gate:** B2 grounding enrichment **MUST NOT ship** until `import_sessions` is confirmed/added to FORCE RLS **and** `verify:rls` is green. Raised separately. Owner: **DB owner**. (Stronger than the original R2 — now a hard ship-gate, not a footnote.) |
| **B6** | MED | **FIX** | Verified inbound `x-correlation-id` is used **raw + authoritative** as the id (`server.ts:244`). **Change:** server **always generates** its own `correlationId` (`generateCorrelationId()`/`crypto.randomUUID`); inbound is captured as a **separate, sanitized `clientTraceId`** log field for widget/WS stitching — never `req.id`, never the user-facing support code. This kills both the collision/support-code-forgery (B6) and resolves Counsel 5a/5a-inj (one authoritative server id; the live raw-ingest hole at `:244` is closed). |
| **B7** | MED | **FIX** | Verified the parser normalizes prices (`"1.200"→1200`, `Math.round(value*10^minorUnit)`, `ai-ocr-parser.ts:745–756`). **Change:** B2 grounding compares the parsed minor-unit value against price-tokens extracted from the OCR text **via the same `priceOf` normalizer** (normalize both sides), not a substring of the raw string. Kills the false-flag (`1.200 Lek` → correct) and the false-pass (`price:1` substring) — both are now exact normalized-token matches. |
| **B8** | MED | **ACCEPT-RISK (honest revise)** | Verified: a schema-valid-but-semantically-wrong response (`{price:1}`) passes `safeParse` (`:536`). **Change:** §7-iv + ADR-0011 now state plainly that schema-validation constrains **shape, not truth** and is **not** an injection backstop. The real defenses are (1) `grounded:false` flags + (2) the pre-existing **human review of the draft before publish**. Delimiter/system-prompt is best-effort defense-in-depth, labeled as such. Accepted residual: a schema-valid hallucination that is also OCR-grounded still requires human catch. Owner: **Parse owner**. |
| **B9** | LOW | **ACCEPT-RISK** | 15 fixtures are weak + self-authored. **Change:** price-exact stays zero-tolerance; recall/structure thresholds now have explicit values (item-recall ≥ 0.95, modifier-structure ≥ 0.90) checked in; fixture set grows on each real-world miss (add the failing menu as a fixture). Independent oracle = future work. "Gates a model swap" softened to "blocks a *measured* regression on the committed set." Owner: **Parse owner**. |
| **B10** | LOW | **FIX (gate)** | **Change:** ADR-0012 now requires `ccc` to consult ignore rules **before** reading file bytes, and the secret-scan fixture test is a **merge gate** that must exist+pass in CI before C1 is enabled. Until then C1 is a gated claim, not a guarantee. |
| **B11** | LOW | **FIX** | **Change:** ADR-0012 names authority: the **executable guardrail (lint rule/test) is authoritative**; `INVARIANTS.md` is a human-readable index that **links to** each guardrail and must not restate a rule that has no gate. No two prose+code sources of the same rule. |
| **Regression trap** | — | **FIX** | Verified hard_block(422) `{outcome,reasons}` and cash/idempotency errors(422) `{error,code}` coexist at the same status. **Change:** the A2 sweep discriminator is **strictly the body shape** (`.send({error…})`/`.send({…code:'…'})`), **never** keyed on `reply.status(422)`. `{outcome,reasons}` business outcomes are explicitly excluded from the envelope sweep. Ties to B1/B2. |

## Counsel points

| # | Type | Verdict | What changed |
|---|---|---|---|
| **STOP-1** | ETHICAL-STOP (grounded) | **REVISE + HUMAN DECISION (ETHICS gate)** | Verified OCR text → prompt is **raw** (`ai-ocr-parser.ts:515` uses `rawText`; `redactedText` at `:399` is used only for the provenance hash). **Revision (recommended resolution for the ETHICS gate):** insert the existing `piiRedactor.redact()` over OCR text **before** it enters the LLM prompt — i.e. feed `redactedText` (not `rawText`) into the prompt at `:515`. This strips incidental **third-party** PII (staff names/phones, handwritten notes) that the owner's consent does not cover. **Tradeoff recorded:** the pattern-based redactor may also strip the venue's own phone, weakening onboarding pre-fill; recommended posture is **privacy-first (redact by default)**, and if venue-contact extraction regresses, add a separate consented venue-contact path rather than sending raw PII. The "venue-own-contact = business-data-not-PII" reclassification is a **red-line redraw** and is recorded here for the human ETHICS decision — does **not** block design or Area A. |
| **5 (a)** | non-blocker (coherence) | **FIX** | My "'unknown' at `:552`" premise was **STALE** — P31 (`server.ts:243–246`) already sets `x-correlation-id` + `correlationStore` before `setErrorHandler`. **Change:** A1 **does not** introduce a parallel `x-request-id`. It adopts the **existing `x-correlation-id`** as the one id: `requestIdHeader:'x-correlation-id'`, `requestIdLogLabel:'correlationId'`, `genReqId` wired to the existing `generateCorrelationId`/store. Deviation from the literal change-set header name (`x-request-id`) recorded — coherence (one id) overrides the literal spec. |
| **5 (a-inj)** | non-blocker (security) | **FIX** | The live log-injection surface is the **existing** raw ingest at `server.ts:244`. **Change:** server now always generates its own id (B6); inbound is captured as a sanitized `clientTraceId` (`^[A-Za-z0-9._-]{1,128}$`, length-capped). The raw-trust path at `:244` is replaced. |
| **5 (b)** | non-blocker (operability) | **FIX** | Verified `correlationId` is **not** in the Sentry allowlist (`sentry.ts:85` = `role,location_id,order_id,worker,db,error_code`) → `setTag` silently dropped. **Change:** A1 adds `correlationId` to the allowlist; proof obligation asserts the tag actually lands on a captured event. |
| **5 (c)** | non-blocker (governance) | **FIX** | **Change:** ADR-0012 routes C2 appends through the **librarian promotion gate** (lesson → human-reviewed guardrail), never a raw in-session write to the OVERRIDE-authority `INVARIANTS.md`. Preserves the ratchet (memory advisory, guardrail/human authoritative). |
| **5 (d)** | steel-man (HMAC cursor) | **REVISE (conditional)** | **Change:** plain keyset retained, but conditionally: (1) the forged-cross-tenant proof test **MUST run under the operational (non-BYPASSRLS) role**, not superuser, or it proves nothing; (2) signing (HMAC) is **reconsidered if `verify:rls` is not green or `import_sessions` FORCE is unconfirmed**. Honest rejection-is-conditional, since the cursor's only tenant lock is RLS and RLS's proof is currently red. Ties to B5. |
| **#6** | non-blocker (humane UX) | **FIX** | **Change:** A2 shows the user a short speakable handle (first 8 chars of the id) inside a "report this problem" affordance, not a bare UUID inline on every error; the operator gets the full id in logs/Sentry. |
| Epistemic (venue-contact reclassification) | non-blocker | folded into **STOP-1** | The boundary redraw is recorded as part of the ETHICS-gate decision, not a footnote. |

---

## Net design changes (summary for parent)

1. **Envelope shape corrected** (B1): `{ code:<STRING>, message, fields?, correlationId, retryAfterMs?, status?:<number-legacy> }`. `code` = the existing FE-consumed string vocabulary (~50 codes enumerated); handler's numeric `code`→`status`. Sweep preserves codes verbatim; never reshapes business outcomes.
2. **Coverage corrected** (B2): rate-limit gets an `errorResponseBuilder`; velocity outcomes are explicitly out-of-envelope; matrix is reachable-or-explicitly-out.
3. **Pagination comparator corrected** (B3): strict `(created_at, id)` keyset + index; fixes a live drop-bug.
4. **One correlation id** (5a/B6): reuse existing `x-correlation-id`, always server-generated, inbound demoted to sanitized `clientTraceId`; Sentry allowlist adds `correlationId` (5b).
5. **Leak guarantee widened** (B4): no `err.detail`/PG internals at any status.
6. **Grounding uses the real normalizer** (B7); schema-validation honestly **not** an injection backstop (B8); human review is the floor.
7. **Governance** (5c/B11): C2 appends via librarian gate; guardrail authoritative, `INVARIANTS.md` links not restates.
8. **B5 ship-gate:** B2 blocked until `import_sessions` FORCE confirmed + `verify:rls` green; cursor test runs as operational role; HMAC reconsidered if RLS proof stays red (5d).

**ETHICS gate — recommended resolution (STOP-1):** redact OCR text with the existing `piiRedactor` **before** the LLM prompt (privacy-first); record the venue-own-contact reclassification as a conscious human decision; if onboarding pre-fill regresses, add a separate consented venue-contact path rather than reverting to raw PII to the model.

---

## Round 2 resolution

R1's B1/B2/B3/B4/B6 are confirmed regression-clean by the re-attack. New items below; all code claims
re-verified against live source. **proposal.md was also fully updated this round** so a reader of the
proposal alone gets the corrected design (the R1 fixes already landed in §2a/§2b/Appendix; R2 adds B12–B15).

**R2 tally:** fix = 8 · accept = 0 · defer = 0. **No unresolved CRITICAL/HIGH.** Only B12 is HIGH and its
fix is mechanical + clearly-correct (a verify-script change) — **no 3rd re-attack needed** (see closing note).

| # | Sev | Verdict | What changed (concrete) |
|---|---|---|---|
| **B12** | HIGH | **FIX (guardrail-strengthening)** | Verified the B5 gate is **unenforceable**: `verify-rls.ts` TENANT_TABLES (`:27–55`) **omits `import_sessions`**, and the only `exit(1)` is on missing **ENABLE** (`:130–132`) — FORCE is merely **logged** (`:134`). So `verify:rls` is green with FORCE absent → it cannot detect the B5 trigger. **Change:** (1) add `import_sessions` (+ audit any other tenant table B reads) to TENANT_TABLES; (2) make `verify:rls` **FAIL (exit 1) on `!relforcerowsecurity`** for tenant tables, not just missing ENABLE. This edits a **verify/guardrail script** (allowed strengthening), not product code or RLS policy. **B2-grounding stays blocked until this strengthened `verify:rls` is green AND the `import_sessions` FORCE migration has landed** (the migration = separate red-line change, **DB owner**). |
| **B13** | MED | **FIX** | Verified `owner/alerts.ts:45–55,70,92–93` carries the **identical** naive keyset (`created_at < $N`, `ORDER BY created_at DESC` no id, cursor `{createdAt}` only) and was **missing** from the A4 scope. **Change:** added `owner/alerts.ts` to the A4 keyset-correction scope — strict `(created_at,id)` comparator, `ORDER BY la.created_at DESC, la.id DESC`, `{createdAt,id}` cursor, composite index on `location_alerts(location_id, created_at DESC, id DESC)`. Same-timestamp burst alerts must not drop (missed escalation = SLA miss). A4 enumeration updated. |
| **B14** | MED | **FIX** | Verified `CREATE INDEX CONCURRENTLY` cannot run inside node-pg-migrate's default txn; the proven pattern is `pgm.noTransaction()` (`1790000000042:51`). **Change:** ADR-0010 + the migration spec require `pgm.noTransaction()` for every A4 index migration; note INVALID-index-on-failure recovery (`REINDEX`/drop+recreate). Forward-only, read-path, no table/RLS change. |
| **B15** | MED | **FIX (two namespaces)** | Verified lowercase business-outcome codes ride in `reasons[]`: `preflight.ts:71,78,88,95` `code:'item_unavailable'`, consumed at `CheckoutPage.tsx:546` (`code==='item_unavailable' || data.outcome==='hard_block'`) + `:551` (`reasons[0].message`). **Change:** explicit two-namespace contract — **SCREAMING_SNAKE-stable applies ONLY to `envelope.code`** (the error path via `sendError`/`setErrorHandler`); **`reasons[].code` are business-outcome tokens OUTSIDE the envelope, preserved verbatim (incl. lowercase)**. `verify:error-contract` asserts SCREAMING_SNAKE on `envelope.code` only, and **separately** asserts the FE `item_unavailable`/`hard_block`/`reasons[].message` matches still hold. Documented so a future sweep cannot normalize `reasons[]` and break `CheckoutPage.tsx:546`. |
| **B6 caveat** | — | **FIX (explicit)** | Made the `server.ts:244` rewrite an **explicit change line**, not implied: today `:244` raw-trusts inbound `x-correlation-id`; the fix replaces it so the server **always generates** the id and inbound becomes a **sanitized `clientTraceId` (log-only) only**. |
| **Counsel (a)** | — | **FIX** | Added a **redaction-recall fixture** to the B-eval proof set: a menu image with seeded third-party PII (incl. Albanian name / handwritten / non-Latin script) → assert it does **not** survive into the prompt. "Redacted" ≠ "PII-free"; the gap is invisible without it. |
| **Counsel (b)** | — | **FIX** | Confirmed STOP-1 OCR-redaction (B3-privacy) ships **INDEPENDENT of the B5 RLS gate**. Privacy hardening is **the earliest-shipping part of B** — it only touches the parser's prompt construction, not `import_sessions`. B5 gates only the **grounding-data enrichment** of the table. |
| **Counsel note** | — | **FIX** | `clientTraceId` is **never joined to user identity** — log-only trace attribute, never indexed against a user/session, never persisted on a tenant row. |
| **ETHICS gate** | — | **CLOSED** | Human chose **redact-by-default** (recorded in `ethical-decisions.md`). This is now the **binding posture** in proposal §7 / ADR-0011 — not a recommendation. |

### Round-2 net changes (for the proposal-only reader)
- A4 scope now: dashboard-snapshot, `owner/alerts.ts`, customer/orders, signals/couriers — each gets the strict `(created_at,id)` comparator + composite index (B13).
- Every A4 index migration uses `pgm.noTransaction()` + `CONCURRENTLY` (B14).
- The B5 gate is now real: `verify:rls` fails on missing FORCE and includes `import_sessions` (B12).
- Two code namespaces documented: `envelope.code` (SCREAMING_SNAKE, contract) vs `reasons[].code` (verbatim, out-of-envelope) (B15).
- OCR→prompt **redact-by-default is binding** (ETHICS closed) and ships independent of B5 (Counsel b); redaction-recall is a proof obligation (Counsel a); `clientTraceId` is identity-free (Counsel note).

### Honest close
**0 unresolved CRITICAL/HIGH.** B12 (the only HIGH) is a mechanical, clearly-correct change to a verify
script (add a table to a list; turn a logged warning into an `exit(1)`) — it strengthens a gate, touches
no product code/RLS policy/migration, and is directly testable (red with FORCE absent → green after the
FORCE migration). **No 3rd re-attack is warranted.** Remaining real-world dependency is operational, not
design: the `import_sessions` FORCE migration must land (DB owner) before B2-grounding ships.
