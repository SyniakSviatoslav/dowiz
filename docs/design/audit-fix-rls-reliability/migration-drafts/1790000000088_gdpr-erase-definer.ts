import type { MigrationBuilder } from 'node-pg-migrate';

// ============================================================================
// OPERATOR-GATED DRAFT — NOT APPLIED. Do NOT place into packages/db/migrations/
// without the standing red-line review (packages/db/migrations/** is a 🔴 protect-path).
// ============================================================================
//
// The STRUCTURAL post-flip *success* fix for N1 (docs/design/audit-fix-rls-reliability/
// resolution-r2.md §1 N1.1) — the companion to the worker-local fail-loud backstop that already
// shipped (workers/anonymizer-gdpr.ts + anonymizer-gdpr-backstop.test.ts, ledger #61).
//
// WHY THIS EXISTS: post-NOBYPASSRLS + MIG-2, the anonymizer erases on a context-free connection that
// `customers` RLS renders ∅ (customers has NO `app.current_tenant` policy arm; the RC4 arm is
// orders-only — 1790000000077:44-67). A table-wide `app.current_tenant` arm on `customers` was
// REJECTED (resolution-r2 §1 N1-b: repeats N2 on the PRIMARY PII table — hands every
// courier-shift/webhook principal read/update on all customers at their location). A SECURITY DEFINER
// erase function runs as the function owner, so RLS visibility is a non-issue; scoping is the
// function's OWN `WHERE id = p_customer AND location_id = p_location` predicate — the SAME discipline
// the anonymizer already uses (lib/anonymizer/index.ts:135,148-156). It fixes N1 (visibility) and N2
// (no arm on customers OR on the two GDPR tables) together, and matches the proposal's existing
// "tiny auditable DEFINER ingress-resolver" convention (gdpr_claim_due, resolve_telegram_chat, …).
//
// GATING: rides LC4-MIG / GATE-FLIP-E2E. The worker keeps its side effects (avatar storage.delete,
// bus publish) app-side, keyed off the returned avatar_key. Once this lands, the worker's N1 backstop
// keys `completed` off this function's returned `out_anonymized_at` (visibility-safe) instead of a
// plain re-read — the plain re-read is only correct under BYPASSRLS (today). NEW P-proof required:
// on a NOBYPASSRLS+MIG-2 rehearsal DB, drive one erasure end-to-end and assert customers.anonymized_at
// IS NOT NULL + phone tokenised; negative: fn absent/withheld ⇒ worker lands `failed`+DLQ, never
// `completed` (red on the current design, green on the combined fix). Assumes `customers.avatar_key`
// exists in the target env (referenced by the app anonymizer under a columnExists guard); if absent,
// add it first or drop the avatar_key column from the RETURNS TABLE + SELECT below.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- Idempotent, tenant-scoped erasure that runs as the function OWNER (RLS-visibility-independent).
    -- Returns the resulting anonymized_at (NULL only if the row does not exist at this tenant) and the
    -- avatar_key to purge app-side. Already-anonymized rows are a no-op success (return the existing ts).
    CREATE OR REPLACE FUNCTION gdpr_erase_customer(p_customer uuid, p_location uuid)
      RETURNS TABLE(out_anonymized_at timestamptz, out_avatar_key text)
      LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
    DECLARE
      v_existing timestamptz;
      v_avatar   text;
    BEGIN
      -- Lock the exact tenant-scoped row; 0 rows → the subject is not at this tenant → return nothing
      -- (the caller MUST treat an empty result as "no effect" → failed, never completed).
      SELECT c.anonymized_at, c.avatar_key
        INTO v_existing, v_avatar
        FROM customers c
       WHERE c.id = p_customer AND c.location_id = p_location
       FOR UPDATE;

      IF NOT FOUND THEN
        RETURN;  -- empty result set: no row erased
      END IF;

      IF v_existing IS NOT NULL THEN
        -- Idempotent: already anonymized. Return the existing timestamp (goal-state reached).
        out_anonymized_at := v_existing;
        out_avatar_key := NULL;  -- avatar already purged on the first erase
        RETURN NEXT;
        RETURN;
      END IF;

      UPDATE customers
         SET phone = 'anon_' || gen_random_uuid()::text,
             name = NULL,
             marketing_opt_in = false,
             anonymized_at = now()
       WHERE id = p_customer AND location_id = p_location
       RETURNING anonymized_at INTO out_anonymized_at;

      out_avatar_key := v_avatar;  -- app-side storage.delete purges this key after the fn returns
      RETURN NEXT;
    END;
    $fn$;

    REVOKE ALL ON FUNCTION gdpr_erase_customer(uuid, uuid) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION gdpr_erase_customer(uuid, uuid) TO dowiz_app;
    COMMENT ON FUNCTION gdpr_erase_customer(uuid, uuid) IS
      'GDPR Art.17 erasure (DEFINER, RLS-visibility-independent, tenant-scoped by predicate). resolution-r2 N1.1.';
  `);
}

// Forward-only (repo convention — CREATE OR REPLACE is re-runnable; no destructive down).
export async function down(): Promise<void> {
  // no-op: forward-only. To remove, DROP FUNCTION gdpr_erase_customer(uuid, uuid) in a new migration.
}
