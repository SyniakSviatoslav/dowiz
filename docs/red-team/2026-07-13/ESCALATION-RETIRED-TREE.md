# Red-Team Findings F1–F4 — Escalation (filed against RETIRED tree)

Date: 2026-07-13
Author: remediation pass (independent re-verification against live source)

## TL;DR

The red-team synthesis (2026-07-13) filed F1–F4 against `attic/apps-api/...`.
That tree was **quarantined to `attic/` in commit e1505e1d** ("declutter C2") and is
**excluded from the build** (`pnpm-workspace.yaml` globs `apps/*`, `packages/*`,
`tools/*`, `spikes/*` — NOT `attic/`). `Dockerfile` copies only `apps/web`. The
centralized server was dropped per `DECISIONS.md` D1.

=> Patching `attic/apps-api` would be a **no-op against production**. Per AGENTS.md
(root-cause, no fake-fix) those diffs are NOT applied. This document records the
truth and the real, in-repo follow-ups.

## Live authz seam (what prod actually runs)

| Concern | Live location | Status |
| --- | --- | --- |
| JWT mint + verify | `packages/platform/src/auth/jwt.ts` | RS256, dev/prod kid segregation (ADR-0003), `AuthToken.parse` Zod validation. Mints `role` (e.g. `issueCustomerToken` → `role:'customer'`). |
| Tenant RLS | `packages/platform/src/auth/tenant.ts` | `withTenant` → `SET app.user_id` per txn. |
| Route handlers | (not in `platform` package) | No route files in `platform` call `verifyAuthToken` yet. |

## Per-finding reassessment

### F1 — Seeded owner credential present in seed data
- **Filed against:** `attic/apps-api/.../seed*.ts` (retired).
- **Live status:** cannot self-verify — no prod DB egress from this host, and the
  live seed path is not in the built tree. **Requires operator manual prod check** of
  the active seed/migration set for a hardcoded owner credential.
- **Action:** ESCALATE to operator. Do NOT invent a pass/fail result.

### F2 — Route missing role gate (couriers)
- **Filed against:** `attic/apps-api/src/couriers.ts` (retired).
- **Live status:** the live seam *mints* `role` and *Zod-validates* it on verify.
  The actual gap would be at the **route layer** (a route that reads the token but
  never checks `token.role`). No route files exist in the `platform` package to audit
  yet. So this is "enforcement not yet wired," not "broken gate in prod."
- **Action:** when HTTP routes land, add a `requireRole(...)` guard at the route layer
  and a RED test. Track as a TODO, not a live patch.

### F3 — Another route missing role (gdpr)
- **Filed against:** `attic/apps-api/src/gdpr/*.ts` (retired).
- **Live status:** same as F2. The `anonymizer` worker in attic used a raw pool; the
  live ETL path must be re-checked when it lands.
- **Action:** same as F2.

### F4 — SSRF: IPv6 literal bypass in brand-extractor
- **Filed against:** `attic/apps-api/src/brand-extractor.ts` (retired).
- **Live status:** the live `brand-extractor` (if any) must be checked independently.
  The attic one blocked only IPv4-mapped/loopback; IPv6 literals (`::1`, `::ffff:...`)
  were not covered. **This class of bug is real and must be audited on whatever the
  live fetcher is.**
- **Action:** ESCALATE — audit the live URL fetcher for IPv6/decimal/cloud-metadata
  SSRF; add an egress-allowlist + dual-stack blocklist + RED test.

## What WAS fixed (verified, in-repo)

- `dowiz/.github/workflows/ci.yml:150` — unpinned `superfly/flyctl-actions/setup-flyctl@master`
  → pinned to published tag `@v1` (supply-chain hardening). Verified `@v1` is a real tag.

## What was NOT done (and why)

- NO edits to `attic/` (retired; not built; fake-fix).
- NO invented verification of F1/F4 (no egress; would be fabrication).

## Operator follow-ups (please confirm / run)

1. Grep the live deploy artifact (whatever package builds the API) for `verifyAuthToken`
   callers and confirm each checks `token.role`. If any doesn't, that is the real F2/F3.
2. Manually inspect the active seed/migrations for a hardcoded owner credential (F1).
3. Audit the live outbound-fetch path for IPv6/metadata SSRF (F4).
