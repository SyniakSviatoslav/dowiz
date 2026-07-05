# pg-privilege-hardening — operator handoff

Design + rationale: `docs/design/pg-privilege-hardening/proposal.md` + `docs/adr/ADR-pg-privilege-hardening.md`.

Everything below touches **protect-paths zones** (`packages/db/`, `package.json`) — human/operator-applied,
not auto-edited. The deterministic, non-protected parts already ship on the branch:

- ✅ `scripts/guardrail-definer-search-path.mjs` + `scripts/definer-baseline.json` (static gate, red→green proven).
- ✅ `docs/security/SECURITY-DEFINER-search-path.migration.ts` (MIG-ITEM1, value corrected to the strong triple).
- ✅ `docs/security/OPERATIONAL-ROLE-nobypassrls.migration.ts` (MIG-ITEM2).

Apply order is **MIG-ITEM2 → MIG-ITEM1** (RLS must be real before ITEM 1's gate is observable — proposal §4).

---

## 1. Place the two migrations (`packages/db/migrations/` — protected)

Current head is `1790000000073`. Assign sequential numbers at placement, ITEM2 before ITEM1, e.g.:

```
cp docs/security/OPERATIONAL-ROLE-nobypassrls.migration.ts \
   packages/db/migrations/1790000000074_operational-role-nobypassrls.ts
cp docs/security/SECURITY-DEFINER-search-path.migration.ts \
   packages/db/migrations/1790000000075_secdef-search-path.ts
```

Staging-first (per ship-discipline + proposal §9):

```
flyctl proxy 5433:5432 -a dowiz-staging-db &
# DATABASE_URL_MIGRATIONS rewritten to localhost:5433, then:
pnpm migrate:up
```

**Gate (red→green) on staging — before vs after:**

```sql
-- ITEM2 role probe — RED returns 1 row, GREEN returns 0:
SELECT rolname FROM pg_roles WHERE rolname='deliveryos_api_user' AND rolbypassrls;

-- ITEM1 definer probe — RED returns the 4 deficient fns, GREEN returns 0:
SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
WHERE n.nspname='public' AND p.prosecdef
  AND NOT EXISTS (SELECT 1 FROM unnest(coalesce(p.proconfig,'{}')) c WHERE c LIKE 'search_path=%');
```

Then run `pnpm verify:rls` + the full lifecycle E2E under the now-NOBYPASSRLS role. **Any newly-failing
query is a real tenant-isolation bug to fix before prod — never re-grant BYPASSRLS.** Prod is a separate
explicit step (migrations are idempotent and can be applied independently of the code deploy).

---

## 2. Boot-guard: also reject a BYPASSRLS connection (`packages/db/src/index.ts` — protected)

ITEM 2C, belt-and-suspenders (proposal §9 / R6). In `createOperationalPool()`, extend the existing
`pool.on('connect', …)` handler that already rejects `current_user='postgres'`:

```ts
  pool.on('connect', async (client) => {
    await client.query("SET statement_timeout = '10s'");
    const res = await client.query(
      "SELECT current_user, (SELECT rolbypassrls FROM pg_roles WHERE rolname = current_user) AS bypassrls",
    );
    if (res.rows[0].current_user === 'postgres') {
      client.release(true);
      throw new Error("SECURITY FAULT: Operational pool connected as 'postgres' superuser. This bypasses RLS. Use a dedicated restricted role.");
    }
    if (res.rows[0].bypassrls === true) {
      client.release(true);
      throw new Error("SECURITY FAULT: Operational pool role has BYPASSRLS — FORCE RLS is a no-op. Run the NOBYPASSRLS migration (deliveryos_api_user).");
    }
  });
```

Fail-fast on boot; ships with the next API deploy. (A superuser already has implicit BYPASSRLS, so the
`postgres` check is kept as a distinct, clearer message and an extra guard.)

---

## 3. `verify:rls` live gate addition (`packages/db/scripts/verify-rls.ts` — protected)

Wire both probes into the gate so a regression fails CI forever (proposal §9, B12 pattern). After the
existing non-tenant perimeter block, before the final success log:

```ts
    // ── pg-privilege-hardening gates ──
    // ITEM2: operational role must not bypass RLS.
    const bypass = await pool.query(
      `SELECT rolname FROM pg_roles WHERE rolname = 'deliveryos_api_user' AND rolbypassrls`,
    );
    if (bypass.rowCount && bypass.rowCount > 0) {
      console.error('❌ deliveryos_api_user has BYPASSRLS — FORCE RLS is a no-op on the hot path.');
      process.exit(1);
    }
    console.log('✅ operational role NOBYPASSRLS verified');

    // ITEM1: every SECURITY DEFINER fn in public must pin search_path.
    const unpinned = await pool.query(
      `SELECT p.oid::regprocedure AS sig
       FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
       WHERE n.nspname = 'public' AND p.prosecdef
         AND NOT EXISTS (
           SELECT 1 FROM unnest(coalesce(p.proconfig, '{}'::text[])) c WHERE c LIKE 'search_path=%'
         )`,
    );
    if (unpinned.rowCount && unpinned.rowCount > 0) {
      console.error('❌ SECURITY DEFINER functions without a pinned search_path:');
      for (const r of unpinned.rows) console.error(`   ${r.sig}`);
      process.exit(1);
    }
    console.log('✅ all SECURITY DEFINER functions pin search_path');
```

> Note: the runtime `verify:rls` probe checks search_path **presence**; the static guardrail
> (`scripts/guardrail-definer-search-path.mjs`, no DB needed) is the CI/pre-commit counterpart and
> additionally pins the *value* expectation via its frozen baseline.

---

## 4. Register the static guardrail (`package.json` — protected)

Add to `scripts` (mirrors the sibling `guardrail:*` entries) and wire into the gate aggregate / pre-commit:

```json
"guardrail:definer-search-path": "node scripts/guardrail-definer-search-path.mjs",
```

Run it wherever the other `guardrail:*` scripts run (CI `lint:gates` step / pre-commit). It needs no DB.

---

## 5. Accepted follow-ups (proposal §10 — not in this change)

- **R1** `read_public_menu_all_locales` is now `SECURITY INVOKER` (silently lost DEFINER at `…033`) —
  decide keep-INVOKER vs restore-DEFINER+pin, then lock with a guardrail. *(DB owner + architect.)*
- **R2** optionally `REVOKE TEMPORARY ON DATABASE … FROM PUBLIC`/the role — kills the `pg_temp` vector at
  source (staging-test first). *(DB owner.)*
- **R4** long-term: consolidate to a single NOBYPASSRLS role with unified grants (Option 2B done properly).
