-- B3 remediation PHASE 1b — bootstrap-write SECURITY DEFINER fns (from the 2026-06-30 write-path re-audit).
-- The member-keyed FORCE-RLS tables (organizations/locations/memberships/menu_versions) can't self-satisfy a
-- brand-new owner's FIRST write (chicken-and-egg: app_member_location_ids() empty until the membership exists).
-- Mirrors the established claim_transfer()/provision pattern: DEFINER fn does the atomic bootstrap, GRANT op role.
-- Pinned search_path (ITEM1 guardrail). The fn trusts its p_user_id caller (the route passes the JWT-verified
-- user id — same trust model as claim_transfer trusting its token).

-- bootstrap_owner: atomic owner signup — find/create personal org, create location, owner membership, menu v1.
-- Mirrors owner/onboarding.ts:62-97 + spa-proxy.ts:734-746 exactly. RETURNS the ids the route then uses.
CREATE OR REPLACE FUNCTION bootstrap_owner(
    p_user_id uuid, p_name text, p_slug text, p_phone text,
    p_currency text, p_locale text, p_supported_locales text[])
  RETURNS TABLE(org_id uuid, location_id uuid)
  LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
DECLARE v_org uuid; v_loc uuid;
BEGIN
  IF EXISTS (SELECT 1 FROM locations WHERE slug = p_slug) THEN
    RAISE EXCEPTION 'SLUG_TAKEN' USING errcode = '23505';
  END IF;
  SELECT id INTO v_org FROM organizations WHERE owner_id = p_user_id LIMIT 1;
  IF v_org IS NULL THEN
    INSERT INTO organizations (id, name, owner_id)
      VALUES (gen_random_uuid(), p_name || ' Org', p_user_id) RETURNING id INTO v_org;
  END IF;
  v_loc := gen_random_uuid();
  INSERT INTO locations (id, org_id, slug, name, phone, currency_code, default_locale,
                         supported_locales, status, widget_enabled, delivery_fee_flat)
    VALUES (v_loc, v_org, p_slug, p_name, p_phone, p_currency, p_locale,
            p_supported_locales, 'open', true, 0);
  INSERT INTO memberships (user_id, location_id, role) VALUES (p_user_id, v_loc, 'owner') ON CONFLICT DO NOTHING;
  INSERT INTO menu_versions (location_id, version) VALUES (v_loc, 1) ON CONFLICT DO NOTHING;
  RETURN QUERY SELECT v_org, v_loc;
END $fn$;
REVOKE ALL ON FUNCTION bootstrap_owner(uuid, text, text, text, text, text, text[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION bootstrap_owner(uuid, text, text, text, text, text, text[]) TO dowiz_app;
