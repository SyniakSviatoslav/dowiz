// B3 PHASE 1c — courier-activate + shadow-erase bootstrap DEFINER fns. operator places into packages/db/migrations/.
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- activate_courier: token-gated first courier membership. Mirrors auth.ts:357-396 (validate invite,
    -- upsert user, insert courier membership, mark invite used) atomically. RAISEs map to the route's 400s.
    CREATE OR REPLACE FUNCTION activate_courier(p_code_hash text, p_phone text, p_name text)
      RETURNS TABLE(user_id uuid, location_id uuid)
      LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
    DECLARE v_invite record; v_user uuid;
    BEGIN
      SELECT ci.id, ci.location_id, ci.used_at, ci.expires_at INTO v_invite
        FROM courier_invites ci WHERE ci.code_hash = p_code_hash FOR UPDATE;
      IF NOT FOUND THEN RAISE EXCEPTION 'INVALID_CODE'; END IF;
      IF v_invite.used_at IS NOT NULL THEN RAISE EXCEPTION 'CODE_USED'; END IF;
      IF v_invite.expires_at < now() THEN RAISE EXCEPTION 'CODE_EXPIRED'; END IF;
      SELECT id INTO v_user FROM users WHERE phone = p_phone;
      IF v_user IS NULL THEN
        INSERT INTO users (phone, display_name) VALUES (p_phone, p_name) RETURNING id INTO v_user;
      ELSE
        UPDATE users SET display_name = p_name WHERE id = v_user;
      END IF;
      INSERT INTO memberships (user_id, location_id, role) VALUES (v_user, v_invite.location_id, 'courier')
        ON CONFLICT (user_id, location_id, role) DO UPDATE SET status = 'active';
      UPDATE courier_invites SET used_at = now() WHERE id = v_invite.id;
      RETURN QUERY SELECT v_user, v_invite.location_id;
    END $fn$;
    REVOKE ALL ON FUNCTION activate_courier(text, text, text) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION activate_courier(text, text, text) TO dowiz_app;

    -- erase_shadow_tenant: GDPR/decline shadow-tenant erasure of the member-keyed rows (the audit's
    -- erasure-can't-propagate fix). Mirrors provisioning.ts:233-239. The acquisition_sources UPDATE +
    -- provision_grants DELETE stay in the route (allow_ops_* USING(true) admits them). Guard owner_id IS NULL
    -- so a claimed (real) tenant is never erased by this path.
    CREATE OR REPLACE FUNCTION erase_shadow_tenant(p_location_id uuid, p_org_id uuid)
      RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
    BEGIN
      IF p_location_id IS NOT NULL THEN
        DELETE FROM products      WHERE location_id = p_location_id;
        DELETE FROM categories    WHERE location_id = p_location_id;
        DELETE FROM menu_versions WHERE location_id = p_location_id;
        DELETE FROM locations     WHERE id = p_location_id;
      END IF;
      IF p_org_id IS NOT NULL THEN
        DELETE FROM organizations WHERE id = p_org_id AND owner_id IS NULL;
      END IF;
    END $fn$;
    REVOKE ALL ON FUNCTION erase_shadow_tenant(uuid, uuid) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION erase_shadow_tenant(uuid, uuid) TO dowiz_app;
  `);
}

export async function down(): Promise<void> {
  // Forward-only.
}
