# ⚠️ APPROVAL-PENDING — B12 verify:rls FORCE gate (ADR-0011, DB owner)

`packages/db/scripts/verify-rls.ts` is a `protect-paths` protected zone, so this strengthening
is staged here for the DB owner to apply (same boundary as a migration).

## Why
Today `verify:rls` is a no-op for the B-grounding gate: it omits `import_sessions` entirely, and the
only FORCE visibility is a *log line* on NON_TENANT_TABLES — nothing `exit(1)`s on a missing FORCE.
So `verify:rls` is green even though `import_sessions` is RLS **ENABLE-only** (no FORCE). B2-grounding
must not ship until this gate is real **and** the `import_sessions` FORCE migration has landed.

## Scope correctness (do NOT blanket-FORCE all tenant tables)
`1780421100051_force-rls.ts` FORCEs only the courier/settlement tables. The rest of the tenant
tables (memberships, modifiers, customer_signals, …) are intentionally ENABLE-only. A blanket
"FORCE required for every tenant table" check would wrongly fail ~15 tables. So FORCE is required
only for `FORCE_REQUIRED` = the already-FORCEd set + `import_sessions`.

## The change (apply to packages/db/scripts/verify-rls.ts)

1. Add `'import_sessions'` to `TENANT_TABLES` (gets the existing anon=0 / ownerB=0 isolation test —
   it is tenant-scoped: `location_id` + a memberships-scoped policy, `1780338982025:29-39`).

2. Add the `FORCE_REQUIRED` list + a FORCE audit loop (after the isolation loop). Catalog-only, so
   it is independent of the operational-role/BYPASSRLS env quirk that affects the isolation tests:

```ts
const FORCE_REQUIRED = [
  'courier_locations', 'courier_invites', 'courier_assignments', 'courier_shifts',
  'courier_positions', 'courier_audit_log', 'courier_payouts', 'settlement_items',
  'settlement_audit_log', 'courier_dispatch_queue',
  'import_sessions',
];

console.log('\n🔒 Checking FORCE ROW LEVEL SECURITY (B12)...');
for (const table of FORCE_REQUIRED) {
  const r = await pool.query(
    `SELECT relforcerowsecurity FROM pg_class
     WHERE relnamespace = 'public'::regnamespace AND relname = $1`,
    [table],
  );
  if (r.rows.length === 0) { console.error(`❌ ${table}: not found in pg_class`); process.exit(1); }
  if (!r.rows[0].relforcerowsecurity) {
    console.error(`❌ ${table}: RLS not FORCED (relforcerowsecurity=false) — a table-owner role bypasses RLS`);
    process.exit(1);
  }
  console.log(`✅ ${table.padEnd(25,' ')} FORCE ROW LEVEL SECURITY`);
}
```

## Proof obligation (red → green)
- **RED now:** `import_sessions.relforcerowsecurity = false` on staging (confirm:
  `SELECT relforcerowsecurity FROM pg_class WHERE relname='import_sessions';` → `f`). The audit `exit(1)`s.
- **GREEN after:** the forward-only FORCE migration lands:
  `pgm.sql('ALTER TABLE import_sessions FORCE ROW LEVEL SECURITY;')` (noTransaction not required; not
  CONCURRENTLY) — staged alongside this. Then `verify:rls` passes and B2-grounding may be enabled.
- The cross-tenant cursor/grounding test must run under the **operational (non-BYPASSRLS)** role (5d).
