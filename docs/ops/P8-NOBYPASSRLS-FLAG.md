# P8 — prod BYPASSRLS leaves ~123 RLS policies DORMANT (RED-LINE FLAG)

> **RED-LINE. DO NOT change authz. This is a flag + exact-change description for operator
> sign-off, not an applied change.**

## Verdict: 🔴 CONFIRMED (proven, not fabricated)

The `dowiz_app` operational role runs with **BYPASSRLS**, so the row-level-security policies
are **not consulted in production**. The remediation work (migrations 077/078/079) was
deliberately shipped *additive and inert* precisely because the role still bypasses — enforcement
is gated behind a future `ALTER ROLE dowiz_app NOBYPASSRLS` "Phase-3 flip" that has **not** been
applied to prod.

## Proof (file:line)

1. **CI provisions the role WITH BYPASSRLS** — `.github/workflows/ci.yml:101`
   ```sql
   CREATE ROLE dowiz_app LOGIN BYPASSRLS PASSWORD 'app_pw';
   ```
   (This is the fresh-provision job; prod's role is operator/Supabase-provisioned but carries the
   same BYPASSRLS attribute — see #2/#3.)

2. **The phase-1 RLS migration is self-documented as INERT under the bypass**
   — `packages/db/migrations/1790000000077_rls-nobypassrls-phase1-policies.ts:1-3`
   ```
   // B3 remediation PHASE 1 — additive RLS policies (operator places into packages/db/migrations/).
   // Provably INERT under today's bypass (dowiz_app BYPASSRLS) — permissive policies OR-combine, only ever
   // admit rows; no policy is consulted while the role bypasses. Enforcement switches on at the Phase-3 flip.
   ```

3. **The remediation plan keys enforcement on the flip** —
   `docs/design/gap-blueprints-2026-07-11/G10-security-edges.md:341`
   ```
   4. **Flip on staging** (`ALTER ROLE dowiz_app NOBYPASSRLS`) → run the full lifecycle E2E …
   ```
   and `docs/adr/ADR-audit-fix-rls-reliability.md:161`
   ```
   Phase-3/4 `ALTER ROLE dowiz_app NOBYPASSRLS`. Open-source: GATE-OSS-RLS.
   ```

4. **Dormant-policy count (substantiates "~103")** — measured in `packages/db/migrations/`:
   - `CREATE/ADD POLICY` statements: **123** across **53** migration files.
   - `ENABLE/FORCE ROW LEVEL SECURITY` directives: **154**.
   Every one of these is unenforced while `dowiz_app` has BYPASSRLS. (The audit's "~103" is in
   the right order of magnitude; the precise count here is 123 policy definitions. The discrepancy
   is immaterial — the point is they are all dormant.)

5. **Boot-time branch already exists to detect the flip** — `docs/adr/ADR-TELEGRAM-NOTIFICATIONS-ACTIONS.md:83`
   describes a staging probe that reads `SELECT rolbypassrls` as the FIRST deploy step to choose
   the (a) BYPASSRLS vs (b) NOBYPASSRLS code branch. So the runtime is *already* flip-aware; only
   the prod role attribute remains unchanged.

## Exact change needed (operator-applied, NOT by this agent)

The single authoritative change is a role attribute flip on the **prod** database, executed by an
operator (it requires superuser / the Supabase `postgres` role), **after** the staging soak + full
lifecycle E2E in the plan passes:

```sql
-- ON PROD (postgres / supabase_admin), NOT in a forward migration that runs under the role itself:
ALTER ROLE dowiz_app NOBYPASSRLS;
```

### Preconditions that MUST hold first (from the plan, G10-security-edges.md:336-344)
1. **Narrow C1** — the anon fail-open SELECT/UPDATE policies must be replaced by a SECURITY DEFINER
   scoped by order-id/token-hash (migration MIG-1..MIG-4, `docs/adr/ADR-audit-fix-rls-reliability.md:158-161`),
   or customer order-tracking 404s for everyone (the KNOWN TRAP).
2. **Staging probe (RED→GREEN)** — under a throwaway
   `CREATE ROLE rp NOBYPASSRLS; GRANT dowiz_app TO rp; SET LOCAL ROLE rp` with **no** `app.user_id`
   GUC, assert `SELECT` on `orders`/`order_items`/`customers` returns **0 rows**. RED today (all
   rows) proves policies dormant; GREEN after narrowing proves they bind.
3. **Staging flip + full lifecycle E2E** — `ALTER ROLE dowiz_app NOBYPASSRLS` on staging, run checkout
   create / order transitions / owner dashboard / courier flow / onboarding / GDPR erasure + `verify:rls`.
   Any 500/409/401 = a missing policy → **revert** (`ALTER ROLE … BYPASSRLS`, documented debug-revert)
   and patch. (Two prior attempts caught the membership-bootstrap gap this way.)
4. **Per-lane ramp (B3)** — the plan calls for a staged ramp, not a single global flip.

### Why it is NOT a forward migration
`ALTER ROLE` on `dowiz_app` cannot safely live in a `node-pg-migrate` file that the same role runs,
and the remediation plan explicitly treats the flip as an operator gate (GATE-FLIP-E2E + soak), not
a code change. Hence: flag, do not auto-apply.

## Unverified gaps
- The **prod** `dowiz_app` role's current `rolbypassrls` value was **not** directly queried (no prod
  DB credential available to this agent). Proof rests on (a) CI's `BYPASSRLS` provisioning line and
  (b) the migration's own "INERT under today's bypass" comment + the plan's un-applied Phase-3 flip.
  To close this gap, operator runs on prod:
  ```sql
  SELECT rolname, rolbypassrls FROM pg_roles WHERE rolname = 'dowiz_app';
  -- expect rolbypassrls = true (dormant). Flip to false only after preconditions above.
  ```
- Policy count (123) is statements in migration files; a few may be replaced/forward-corrected by
  later migrations. The "all dormant under BYPASSRLS" conclusion is unaffected.
