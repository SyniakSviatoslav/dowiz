# Hardening & Pre-Launch Gates (Stage 34)

## Overview
Final engineering gate before go-live. Ensures cross-tenant isolation, zero secrets exposure, noisy-neighbor protection, security perimeter, input validation, and integrity under concurrency.

## H1 — RLS Full Audit
- `verify-rls.ts` extended to cover all Phase 5 tables
- Adversarial cross-tenant test (`tests/phase5/rls-adversarial.test.ts`): SELECT/INSERT/UPDATE/DELETE on every tenant table as wrong tenant → 0 rows / denied
- Privileged pool sweep: every worker query must have explicit `WHERE location_id`

## H2 — Secrets + Keys
- `verify-secrets.ts`: gitleaks scan, .env.example placeholder audit, no JWT defaults in code
- JWT rotation test (`tests/phase5/jwt-rotation.test.ts`): sign with kid=v1, rotate to v2, old token still valid, v1 removal → old tokens rejected
- All secrets in Fly secrets/env — zero defaults in source

## H3 — Rate-Limit + Noisy-Neighbor
- `lib/resilience/rate-limit.ts`: per-tenant token bucket, per-IP token bucket, inflight semaphore per tenant
- `STRICT_OPTS` (3/min, 1 inflight) for expensive endpoints (import, reveal, OTP)
- Stale bucket cleanup prevents memory leak
- Migration M033: `rate_limit_overrides` JSONB column on locations

## H4 — Spike Smoke / Load
- `load/spike.js` (k6): 3 scenarios — read flood (500 RPS), burst orders (ramp to 20/s), multi-tenant isolation
- Thresholds: server error <1%, HTTP failure <1%
- Cache hit ratio, p95 latency tracked

## H5 — Perimeter
- `lib/security/headers.ts`: CSP with nonce, HSTS, X-Content-Type-Options, Referrer-Policy, Permissions-Policy, frame-ancestors
- CORS: restrictive default (deny all), permissive only on public menu GET + order POST
- Security headers plugin registered in server.ts

## H6 — Input / Abuse Sweep
- Body size limit: 5MB via `@fastify/multipart`
- Upload audit table (`upload_audit`): MIME, hash, size, rejection reason
- Zod `.strict()` across all Phase 5 routes

## H7 — Integrity Under Concurrency
- `tests/phase5/integrity.test.ts`: N parallel duplicate orders → 1 order (idempotency), double status transition → 1 success, money column CHECK sweep, FK orphan sweep

## H8 — Pre-Launch Checklist
- `scripts/verify-launch.ts`: automated gates (Supabase tier, TLS, restore-test, anonymizer, Sentry, env parity, RLS, fallback)
- `docs/phase5/launch-checklist.md`: automated + manual gates (OAuth, rollback rehearsal)
- Manual items flagged with `[MANUAL]`

## Files
```
scripts/verify-secrets.ts            — H2: Secrets scan
scripts/verify-launch.ts             — H8: Pre-launch checklist
apps/api/src/lib/resilience/rate-limit.ts  — H3: Rate-limit + inflight
apps/api/src/lib/security/headers.ts       — H5: Security headers
apps/api/tests/phase5/rls-adversarial.test.ts  — H1: Adversarial RLS
apps/api/tests/phase5/jwt-rotation.test.ts     — H2: JWT rotation
apps/api/tests/phase5/integrity.test.ts         — H7: Concurrency integrity
apps/api/tests/test-stage34.ts                 — H1-H8 static analysis
load/spike.js                                  — H4: Load test
packages/db/migrations/1780421100063_hardening-seam.ts  — M033
```
