// STAGED MIGRATION ARTIFACT — operator places this at
//   packages/db/migrations/1790000000070_provision-products.ts
// (migrations dir is a protected governance zone; manual-approval handoff, mirroring 068/069).
// **REQUIRES 068 + 069 applied first** (products.source/allergens_confirmed from 068; the
// provision_shadow pattern + provision_grants from 069). P6-3 · next free number after 069 = 070.
import type { MigrationBuilder } from 'node-pg-migrate';

// P6-3 — extend the shadow-provisioning carve-out to the MENU (products + categories), gate
// AI-inferred allergens, and add a shadow-only preview read path. Council verdict
// docs/design/p6-3-extraction-render-council-verdict.md (H2 + C2 + H1).
//
// Same HONEST CRUX as 069: the operational role bypasses RLS today, so these policies enforce
// nothing yet — defense-in-depth that becomes the boundary once the role is NOBYPASSRLS. Proven
// under a real NOBYPASSRLS role (provision-rls.test.ts). Built-in sha256 (no search_path dep).
const TOKEN_VALID = `EXISTS (
  SELECT 1 FROM provision_grants g
  WHERE g.token_hash = encode(sha256(current_setting('app.provision_token', true)::bytea), 'hex')
    AND g.consumed_at IS NULL
    AND g.expires_at > now()
)`;
// Breaker H2: products/categories have no owner_id; a token-ONLY policy would let a valid token
// INSERT into a VICTIM tenant's location_id. Bind the carve-out to a SHADOW location. The check is a
// cross-table predicate (location→org.owner_id) — which a policy subquery cannot evaluate, because
// the enforcing (NOBYPASSRLS) role cannot SELECT the shadow org (no SELECT policy admits it). So,
// exactly like app_member_location_ids(), use a SECURITY DEFINER helper that reads org/location as
// its owner. source='place' / token short-circuit means a normal owner INSERT (source='owner', no
// GUC) never reaches the helper → no widening of the owner hot path.
const SHADOW_LOCATION = `app_is_shadow_location(location_id)`;

export async function up(pgm: MigrationBuilder): Promise<void> {
  // SECURITY DEFINER shadow-location predicate (mirrors app_member_location_ids posture).
  pgm.sql(`
    CREATE OR REPLACE FUNCTION app_is_shadow_location(p_location_id uuid)
    RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public AS $$
      SELECT EXISTS (
        SELECT 1 FROM locations l JOIN organizations o ON o.id = l.org_id
        WHERE l.id = p_location_id
          AND o.owner_id IS NULL AND l.status = 'closed' AND l.published_at IS NULL
      );
    $$;
  `);

  // H2 — additive provision_shadow FOR INSERT on products + categories, shadow-location-bound.
  pgm.sql(`
    CREATE POLICY provision_shadow ON categories FOR INSERT
      WITH CHECK ( ${SHADOW_LOCATION} AND ${TOKEN_VALID} );

    CREATE POLICY provision_shadow ON products FOR INSERT
      WITH CHECK ( source = 'place' AND ${SHADOW_LOCATION} AND ${TOKEN_VALID} );
  `);

  // H1 — read_preview_menu(slug): the ONLY read path for the pre-claim labeled preview render.
  // Admits ONLY shadow tenants (owner_id NULL + status='closed' + published_at NULL) — it can never
  // serve a real tenant, and a real tenant can never be served BY it (so it cannot leak into the
  // normal storefront / SSR / sitemap). C2 read-gate baked in: AI-inferred allergens (attributes.bom)
  // are stripped for unconfirmed place products. Full descriptions are returned (operator decision
  // D-render = FULL DESCRIPTIONS, recorded override). SECURITY DEFINER + pinned search_path.
  pgm.sql(`
    CREATE OR REPLACE FUNCTION read_preview_menu(p_slug text)
    RETURNS jsonb
    LANGUAGE sql
    STABLE
    SECURITY DEFINER
    SET search_path = public
    AS $$
      SELECT CASE WHEN l.id IS NULL THEN NULL ELSE jsonb_build_object(
        'slug', l.slug,
        'name', l.name,
        'is_preview', true,
        'currency', jsonb_build_object('code', l.currency_code, 'minor_unit', l.currency_minor_unit),
        'default_locale', l.default_locale,
        'categories', COALESCE((
          SELECT jsonb_agg(cat ORDER BY cat->>'sort_order')
          FROM (
            SELECT jsonb_build_object(
              'id', c.id,
              'name', c.name,
              'sort_order', c.sort_order,
              'products', COALESCE((
                SELECT jsonb_agg(prod ORDER BY prod->>'sort_order')
                FROM (
                  SELECT jsonb_build_object(
                    'id', p.id,
                    'name', p.name,
                    'description', p.description,
                    'price', p.price,
                    'is_available', p.is_available,
                    'sort_order', p.sort_order,
                    -- C2: strip the bom (allergen surface) for unconfirmed scraped products.
                    'attributes', CASE WHEN p.source = 'place' AND p.allergens_confirmed = false
                                       THEN p.attributes - 'bom' ELSE p.attributes END
                  ) AS prod, p.sort_order
                  FROM products p
                  WHERE p.category_id = c.id AND p.is_available = true
                ) pj
              ), '[]'::jsonb)
            ) AS cat, c.sort_order
            FROM categories c
            WHERE c.location_id = l.id
          ) cj
        ), '[]'::jsonb)
      ) END
      FROM locations l
      JOIN organizations o ON o.id = l.org_id
      WHERE l.slug = p_slug
        AND o.owner_id IS NULL          -- shadow only
        AND l.status = 'closed'         -- never a live tenant
        AND l.published_at IS NULL;     -- never-orderable invariant (B3)
    $$;

    REVOKE ALL ON FUNCTION read_preview_menu(text) FROM PUBLIC;
  `);

  // Grant EXECUTE to whatever role can already EXECUTE read_public_menu_all_locales (mirror).
  pgm.sql(`
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT DISTINCT grantee
        FROM information_schema.role_routine_grants
        WHERE routine_schema = 'public'
          AND routine_name = 'read_public_menu_all_locales'
          AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format('GRANT EXECUTE ON FUNCTION read_preview_menu(text) TO %I', r.grantee);
      END LOOP;
    END
    $$;
  `);

  // C2 READ-GATE on the LIVE public menu (read_public_menu / read_public_menu_all_locales) is a
  // POST-CLAIM concern and is DEFERRED to the claim phase — see docs/acquisition/c2-read-gate-claim-phase.sql.
  // Rationale: pre-claim a shadow is status='closed' and is never served by those functions (their
  // WHERE requires active/open OR published_at). P6-3's pre-claim allergen safety is the WRITE-strip
  // (provisioning.ts) + read_preview_menu's `attributes - 'bom'` above — both PROVEN. Re-versioning a
  // ~150-line hot-path menu fn unproven here would risk every real tenant's menu; prove it on the
  // full schema at claim time.
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP FUNCTION IF EXISTS read_preview_menu(text);
    DROP POLICY IF EXISTS provision_shadow ON products;
    DROP POLICY IF EXISTS provision_shadow ON categories;
    DROP FUNCTION IF EXISTS app_is_shadow_location(uuid);
  `);
}
