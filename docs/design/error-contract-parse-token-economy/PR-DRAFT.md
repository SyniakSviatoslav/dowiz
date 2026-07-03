# PR: Error-contract envelope · menu-parse grounding · agent token-economy (+ in-flight branch work)

**Branch:** `feat/mvp-sensor-seams` → `main` · 53 commits · 191 files (+11.7k / −964)

> ⚠️ This branch is the cumulative `feat/mvp-sensor-seams` line. The headline below is the
> **error-contract-parse-token-economy** change-set (ADR-0010/0011/0012), completed this cycle.
> Merging ALSO ships the earlier in-flight work already validated on staging (MVP sensor-seams,
> a11y/verify-net sweep, e2e de-staling). Everything not ready to launch is **flag-gated dark**
> (see the flag inventory). Prod deploys on push to `main` — do the **Before merge** steps first.

---

## 🔴 Before merge (DB owner — required, staging-DB first)

Three migrations are **staged as artifacts** (not applied — `packages/db/` is protected) in
`docs/design/error-contract-parse-token-economy/`. Apply on the **staging DB first**, then prod via
the release command:

1. `A4-keyset-pagination-indexes.migration.ts` — composite `(tenant, sort, id)` indexes (CONCURRENTLY,
   `noTransaction`). **Read-path only; the A4 route fix is already correct without it (perf-only).**
2. `B-grounding-import-sessions-force-rls.migration.ts` — `ALTER TABLE import_sessions FORCE ROW LEVEL SECURITY`.
3. `B12-verify-rls-force-gate.md` — strengthen `packages/db/scripts/verify-rls.ts` (add `import_sessions`
   + a `FORCE_REQUIRED` audit that `exit(1)`s on missing FORCE).

Then: `pnpm verify:rls` green → optionally set `MENU_GROUNDING_ENABLED=true` to launch grounding.

---

## What this delivers — error-contract change-set (ADR-0010)

**A1–A4 · one structured error envelope, end to end.** Every API error now returns
`{ code:<SCREAMING_SNAKE>, message, fields?, correlationId, retryAfterMs?, status, error }` with a
**server-authoritative** correlationId (inbound `x-correlation-id` demoted to a sanitized, identity-free
`clientTraceId` — closes a log-injection + support-code-forgery hole). `buildErrorEnvelope` is the one
source (setErrorHandler + `reply.sendError` + rate-limit + notFound all use it).

- **A2 sweep:** ~296 ad-hoc `reply.status(n).send({error})` sites across ~45 files → `reply.sendError`,
  preserving every FE-consumed code verbatim (money: `MIN_ORDER_NOT_MET`/`CASH_AMOUNT_TOO_LOW`/`MODIFIER_*`/
  `PRODUCT_*`/`NOT_DELIVERABLE`; auth: `INVALID_CREDENTIALS`/`OWNER_REVOKED`/…). Guarded by
  `pnpm verify:error-contract` (fails the build if a FE-consumed code is renamed on either side).
- **A3:** rate-limit 429 envelope — caught + fixed a real bug (`@fastify/rate-limit` *throws* the builder
  return → must be a throwable `ApiError`, not a plain body, else 500s).
- **A4:** strict `(created_at,id)` keyset pagination — fixes a **live drop-bug** (same-millisecond burst
  orders/alerts/signals silently skipped between pages). Proven red→green against the staging DB.
- **B4 leak fix:** menu-import stopped serializing `details: err.detail` (Postgres column+value egress).
- **21 sites deliberately NOT swept** — they carry FE-consumed extra fields (`details.min_order_value`,
  `missing`, `expected`, …) or are business outcomes; absorbing them needs an envelope `data` field
  (a B4-sensitive contract decision, deferred to council).

## Menu-parse (ADR-0011)

- **B3 redact-by-default (shipped, ETHICS-binding):** OCR text is `piiRedactor.redact()`ed **before** the
  LLM prompt — incidental third-party PII no longer egresses to the model.
- **B2 grounding (shipped DARK, `MENU_GROUNDING_ENABLED` default off):** each parsed price is grounded
  against OCR price-tokens via the **same normalizer** (no substring drift); ungrounded → a draft warning,
  never auto-published. Gated on the FORCE migration above.
- **B1 eval harness:** `pnpm verify:menu-parse` — deterministic field-level scorer (price exact / recall ≥
  0.95 / modifier-structure ≥ 0.90) that blocks a cascade-swap regression on the committed fixtures.

## Agent token-economy (ADR-0012, dev-only — never shipped to prod)

- **C1 `ccc`:** AST-semantic code search (`pnpm ccc`). Secret-safe — consults ignore rules before reading
  bytes; `pnpm verify:ccc-secrets` is the merge gate (a secret is never even opened).
- **C2 `INVARIANTS.md`:** agent-facing index, each invariant linked to its real executable gate.

## Also on this branch (prior, already staging-validated)

MVP sensor-seams (geo capture / ETA synthesis), the a11y + Non-Pixel Verification Net sweep (contrast/
button-name/nested-interactive fixes), i18n backfill, and e2e/selector de-staling.

---

## Flag inventory (dark unless set)

`MENU_GROUNDING_ENABLED` (off), plus the pre-existing launch flags this branch keeps default-off on prod
(`ALLOW_DEV_LOGIN`, access-gate, etc — see deploy-topology). Deploying the branch ships dark code; each
launch is a separate, explicit flag flip.

## Proof

Per-area red→green in each commit body. Staging-validated: `error-contract.spec` 6/6; A4 keyset vs staging
DB; A3 live 429 envelope; orders/notFound live envelope curls; `verify:error-contract` /
`verify:ccc-secrets` (4/4) / `verify:menu-parse` (7/7) / `menu-grounding` (5/5) / `ocr-redaction` (3/3) /
`rate-limit-envelope` (3/3) / `send-error` (3/3) green; full pre-commit gate (lint→typecheck→build→Docker).

## Rollback

Error-envelope + sweep are response-shape only (legacy `error` retained) → revert = redeploy. A4 indexes
can stay (additive). Grounding/ccc are flag-off/dev-only. The FORCE migration has a `down()`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
