# ADR — Audit-fix: cross-tenant authz + the missing data-access seam

- Status: **DRAFT (design-only)** — Council STEP 1 (FRAME + PROPOSE). Not approved; Breaker + Counsel run
  next. No production code until per-finding sign-off (authz/PII red-line).
- Date: 2026-07-03
- Deciders: System Architect (proposer), Council (authz/PII/RLS red-line), Operator + DB owner.
- Relates / does NOT supersede: `ADR-security-hardening-2026-07` (this is the **PATCH sibling** left
  behind by that batch's GET/#1, spa-proxy/#6, invite/#7 fixes, plus the class-killer seam),
  `ADR-pg-privilege-hardening` (B3 / NOBYPASSRLS flip — Section-E policy gaps handed off with a
  sequencing contract), ADR-0004 (owner-token revocation), ADR-0013 (courier realtime authz),
  ADR-0010 (error envelope). Full design: `docs/design/audit-fix-authz/proposal.md`.

## Context

The 2026-07-03 audit found four live cross-tenant findings (verified against source):

- **LC2/F1 (CRIT):** `routes/orders.ts:862` PATCH `/orders/:id/status` reads `WHERE id=$1` with no
  membership JOIN → any owner cancels/mutates any tenant's orders. The GET sibling was hardened; the PATCH
  was left behind. `owner/dashboard.ts:626` (the *same* transition op, duplicated) is correctly scoped —
  proving the codebase knows the shape and simply forgot one copy.
- **F3/F4 (HIGH):** `owner/couriers.ts` + `owner/courier-invites.ts` omit `requireRole(['owner'])`;
  `requireLocationAccess` admits customers/couriers → a customer reads the decrypted courier roster, a
  courier mints invites (chains with `courier/auth.ts` `ON CONFLICT DO UPDATE password_hash` into ATO).
- **LC5/F2 (HIGH, CRIT-impact):** `owner/gdpr.ts:48` trusts body `customerId`; `lib/anonymizer/index.ts:
  119/133` UPDATE has no `location_id` → irreversible cross-tenant PII erasure.
- **F5 (MED):** `lib/signals/compute.ts:86` reads `customers WHERE id=$1`, no `location_id`.

**Root (arch F2 / synthesis R-A):** no enforced data-access seam. `withTenant` in 22/65 route files; the
membership resolver cloned with divergent failure (`null`→401 vs `throw`→500); the tenant predicate is a
per-author discipline, so it is sometimes forgotten. RLS is inert under the current BYPASSRLS pool, so a
missing predicate is a **live** leak. The existing `require-auth-hook` lint accepts `verifyAuth OR
requireRole` — which is exactly why F3/F4 (verifyAuth present, requireRole absent) passed the gate.

## Decision (proposed — to be ratified after Breaker/Counsel)

Adopt **Option C** (proposal §4): the enforced-seam lint **now** as the class-killer, per-aggregate
repositories **incrementally** as debt-paydown.

1. **Tier-1 point fixes (code-only, no schema change):**
   - LC2 — authorize-by-JOIN on the PATCH read (`JOIN memberships … role='owner' AND status='active'`),
     404 before any transition; mirrors the shipped GET branch.
   - F3/F4 — add `requireRole(['owner'])` preValidation hook to both files; F4 allow-lists `role`; scope
     courier mutations to `courier_locations`, not the global row.
   - LC5 — validate `customerId` same-tenant at the entry (404 else) **and** add `AND location_id=$2` to
     the anonymizer SELECT/UPDATE (and symmetric `anonymizeOrder`).
   - F5 — thread `locationId` + `AND location_id=$2` into `compute.ts`.
2. **Tier-1 guardrails (same batch — the reason the class stays closed):**
   - Tighten `local/require-auth-hook` to require `verifyAuth` **AND** `requireRole` on
     `routes/{owner,courier}/**` (explicit allow-list for pre-account routes); ratchet warn→error.
   - New `local/no-unscoped-tenant-query` ESLint rule: FAIL on a raw `.query` touching a tenant table
     without a tenant predicate / `withTenant` context, with a grep-auditable `-- tenant-scoped-ok`
     escape hatch. Land at warn with the current count as the ratchet floor; burn to error.
   - Wire `test:unit` (incl. the never-run `phase5/rls-adversarial.test.ts`) into CI against the
     fresh-provision service DB; skip-of-a-should-succeed-setup = FAIL.
3. **Tier-2 (subsequent tracks):** collapse resolver clones → one `getOwnerLocationId` (single
   `null→401`); per-aggregate repositories behind the lint gate.
4. **Hand-off:** Section-E RLS-policy migrations + NOBYPASSRLS flip → B3/flip council (sequencing
   contract; this batch is a prerequisite, not a conflict).

## Consequences

- **Positive:** four live cross-tenant leaks closed; the class made a build-time gate (future unscoped
  query = CI failure); the genuinely-good adversarial IDOR test finally runs; failure semantics unified.
- **Cost / risk:** status-code-only behavior change on previously-exploitable calls (200→404/403) — a
  leak correction, not a break for same-tenant callers; `promotions.ts` authz 500→401 (flag for Breaker —
  confirm no caller depends on the 500); the new lint is heuristic (escape-hatch mitigates, kept
  grep-auditable). Repository migration is L-effort and deferred/staged.
- **Proof:** every fix ships red→green (proposal §6) + a `REGRESSION-LEDGER.md` row; no cheat-green.

## Alternatives considered

- **Option A (repositories only):** strongest correctness but L-effort, high blast radius, and *partial
  until complete* — still needs Option B's lint to prevent new raw queries during the migration. Rejected
  as the *sole* first move; retained as the Tier-2 target.
- **Option B (lint/preHandler only):** chosen as the immediate gate; does not remove SQL/row-mapper
  duplication — that debt is Option A's job later. Hence the C hybrid.
- **Do-nothing / point-fixes-without-seam:** rejected — LC2 is literally a point fix (dashboard) that was
  not propagated to its clone; without the seam the class regenerates.

## Open questions for Breaker / Counsel / Operator

- OR-1: `no-unscoped-tenant-query` false-negative rate on dynamically-built SQL — is the escape-hatch +
  the standing rls-adversarial sweep sufficient, or does a subset need repository-only enforcement first?
- OR-2: does any live caller depend on `promotions.ts` returning 500 (not 401) on an authz miss?
- OR-3: F3 courier-mutation scoping to `courier_locations` — confirm the shared-courier deactivation
  semantics (should deactivating in A ever affect B? design says no).
- OR-4 (operator): confirm the CI `test:unit` job can reach the fresh-provision service DB with seeded
  owner-A/owner-B fixtures (the test `process.exit(1)`s without them — must not be silently skipped).
