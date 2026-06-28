import crypto from 'node:crypto';
import type { Pool, PoolClient } from 'pg';
import { advance } from './service.js';

// P6-2 — shadow-spine write authority via a single-use provisioning token (operator decision 1b).
// Mint a short-TTL one-time token, then write the shadow spine (organizations owner_id NULL +
// locations status='closed' + menu_versions v1) THROUGH the narrow additive `provision_shadow` RLS
// policy (migration 069) — never around it. See docs/design/p6-2-provisioning-council-verdict.md.

const TOKEN_TTL_MS = 5 * 60 * 1000; // ~5 min — the policy also gates on expires_at > now()

/** Plaintext token = hex(32 random bytes) (ASCII, so Node and pg sha256 agree byte-for-byte). */
function mintToken(): string {
  return crypto.randomBytes(32).toString('hex');
}

/** encode(sha256(token::bytea),'hex') — the exact expression the RLS policy recomputes. */
export function hashToken(token: string): string {
  return crypto.createHash('sha256').update(token, 'utf8').digest('hex');
}

export class ProvisionError extends Error {
  constructor(readonly code: string, message?: string) {
    super(message ?? code);
    this.name = 'ProvisionError';
  }
}

// P6-3 — the transient menu draft (reuse the import_sessions draft SHAPE, stored in
// acquisition_sources.menu_draft jsonb). Written to products/categories ONLY at provisioning.
export interface DraftProduct {
  name: string;
  price: number; // integer minor units (the parser integer-normalizes; guarded again at write)
  description?: string | null;
  sort_order?: number;
  attributes?: Record<string, unknown> | null;
}
export interface DraftCategory {
  name: string;
  sort_order?: number;
  products?: DraftProduct[];
}
export interface MenuDraft {
  categories?: DraftCategory[];
}

// C2 WRITE-strip (defense-in-depth on the safety red-line): keep bom ingredients (decision #4
// "extract everything") but NULL every bom[].allergens — the unverified AI allergen claim is never
// persisted. The READ-gate (migration 070) is the authoritative post-claim layer; this is belt+braces.
function stripAllergens(attributes: Record<string, unknown> | null | undefined): Record<string, unknown> {
  // products.attributes is NOT NULL DEFAULT '{}' — never write null (proven against the real schema).
  if (!attributes || typeof attributes !== 'object') return {};
  const bom = (attributes as { bom?: unknown }).bom;
  if (!Array.isArray(bom)) return attributes;
  return {
    ...attributes,
    bom: bom.map((entry) =>
      entry && typeof entry === 'object' ? { ...(entry as object), allergens: [] } : entry,
    ),
  };
}

// Write categories then products (FK order) from the draft, under the provision_shadow policy.
// Pre-gen UUIDs / no RETURNING (products/categories tenant SELECT USING would reject the shadow row).
async function writeMenuFromDraft(client: PoolClient, locationId: string, draft: MenuDraft): Promise<void> {
  for (const [ci, cat] of (draft.categories ?? []).entries()) {
    const catId = crypto.randomUUID();
    await client.query(`INSERT INTO categories (id, location_id, name, sort_order) VALUES ($1, $2, $3, $4)`, [
      catId,
      locationId,
      cat.name,
      cat.sort_order ?? ci,
    ]);
    for (const [pi, prod] of (cat.products ?? []).entries()) {
      if (!Number.isInteger(prod.price) || prod.price < 0) {
        throw new ProvisionError('INVALID_PRICE', `non-integer/negative price for "${prod.name}"`);
      }
      await client.query(
        `INSERT INTO products
           (id, location_id, category_id, name, description, price, is_available, sort_order, source, allergens_confirmed, attributes)
         VALUES ($1, $2, $3, $4, $5, $6, true, $7, 'place', false, $8)`,
        [
          crypto.randomUUID(),
          locationId,
          catId,
          prod.name,
          prod.description ?? null,
          prod.price,
          prod.sort_order ?? pi,
          stripAllergens(prod.attributes),
        ],
      );
    }
  }
}

/**
 * Mint a single-use provisioning grant for a source. Returns the PLAINTEXT token ONCE (only the
 * hash is stored). The partial-unique index `provision_grants_one_active_per_source` makes a second
 * mint while one is outstanding fail at the DB (breaker H1 defense-in-depth) — surfaced as
 * ProvisionError('ACTIVE_GRANT_EXISTS').
 */
export async function mintProvisionToken(
  pool: Pool,
  acquisitionSourceId: string,
): Promise<{ token: string; expiresAt: Date }> {
  const token = mintToken();
  const expiresAt = new Date(Date.now() + TOKEN_TTL_MS);
  try {
    await pool.query(
      `INSERT INTO provision_grants (acquisition_source_id, token_hash, expires_at)
       VALUES ($1, $2, $3)`,
      [acquisitionSourceId, hashToken(token), expiresAt],
    );
  } catch (e) {
    if ((e as { code?: string }).code === '23505') {
      throw new ProvisionError('ACTIVE_GRANT_EXISTS', 'an unconsumed grant already exists for this source');
    }
    throw e;
  }
  return { token, expiresAt };
}

export interface SpineInput {
  acquisitionSourceId: string;
  token: string; // plaintext one-time token from mintProvisionToken
  name: string; // restaurant name — a fact from the Places API (facts-only provenance)
  slug: string;
  phone?: string;
}

/**
 * Write the shadow spine in ONE transaction, ordered to make the guarded state-transition the
 * dedup chokepoint (breaker B1):
 *   set_config(token, txn-local) → SELECT … FOR UPDATE (0 rows → ROLLBACK) →
 *   INSERT org+location+menu_versions (admitted only by provision_shadow) →
 *   advance(ENRICHED→PROVISIONED) [state-pinned UPDATE; a racing 2nd runner gets 0 rows → ROLLBACK] →
 *   consume the grant LAST (single-use; 0 rows → ROLLBACK).
 * No products: an org+location+menu_versions-v1 tenant with an empty menu is a COMPLETE valid tenant
 * (exactly what /onboarding/start creates), not a partial write. published_at stays NULL (B3).
 */
export async function provisionShadowSpine(
  pool: Pool,
  input: SpineInput,
): Promise<{ orgId: string; locationId: string }> {
  const client: PoolClient = await pool.connect();
  try {
    await client.query('BEGIN');
    // txn-local provisioning context the provision_shadow policy reads.
    await client.query("SELECT set_config('app.provision_token', $1, true)", [input.token]);

    // Fast-fail + serialize: lock the grant row. 0 rows → invalid/expired/consumed → abort.
    const grant = await client.query(
      `SELECT id FROM provision_grants
        WHERE token_hash = encode(sha256($1::bytea), 'hex')
          AND consumed_at IS NULL AND expires_at > now()
        FOR UPDATE`,
      [input.token],
    );
    if (grant.rowCount === 0) throw new ProvisionError('INVALID_OR_EXPIRED_TOKEN');

    // Pre-generate ids: organizations has no SELECT policy, so we never RETURNING (architect C-arch-1).
    const orgId = crypto.randomUUID();
    const locationId = crypto.randomUUID();

    await client.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, $2, NULL)`, [
      orgId,
      `${input.name} Org`,
    ]);
    await client.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status, published_at, widget_enabled, delivery_fee_flat)
       VALUES ($1, $2, $3, $4, $5, 'closed', NULL, false, 0)`,
      [locationId, orgId, input.slug, input.name, input.phone ?? ''],
    );
    await client.query(`INSERT INTO menu_versions (location_id, version) VALUES ($1, 1)`, [locationId]);

    // P6-3: write the menu from the source's transient menu_draft (categories then products) in the
    // SAME tx, under the provision_shadow policy — "never partial-write tenant." Empty/absent draft →
    // container-only (P6-2 behavior preserved). Allergens are write-stripped; price is integer-guarded.
    const draftRes = await client.query(`SELECT menu_draft FROM acquisition_sources WHERE id = $1`, [
      input.acquisitionSourceId,
    ]);
    const draft = (draftRes.rows[0] as { menu_draft?: MenuDraft | null } | undefined)?.menu_draft ?? null;
    if (draft) await writeMenuFromDraft(client, locationId, draft);

    // B1 chokepoint: state-pinned ENRICHED→PROVISIONED. advance() throws if the row is not still
    // ENRICHED (a concurrent runner already provisioned) → ROLLBACK, undoing this spine.
    await advance(client, input.acquisitionSourceId, 'PROVISIONED', { org_id: orgId, location_id: locationId });

    // Consume LAST (single-use). 0 rows → consumed under us → ROLLBACK.
    const consumed = await client.query(
      `UPDATE provision_grants SET consumed_at = now()
        WHERE token_hash = encode(sha256($1::bytea), 'hex') AND consumed_at IS NULL`,
      [input.token],
    );
    if (consumed.rowCount === 0) throw new ProvisionError('TOKEN_ALREADY_CONSUMED');

    await client.query('COMMIT');
    return { orgId, locationId };
  } catch (e) {
    await client.query('ROLLBACK');
    throw e;
  } finally {
    client.release();
  }
}

/**
 * Day-one hard-delete (counsel C2): erase a shadow tenant + its grants by acquisition source. The
 * artifact is born with its erasure path. ON DELETE CASCADE on provision_grants.acquisition_source_id
 * handles the grants; the org/location/menu_versions are removed explicitly (no tenant rows survive).
 */
export async function hardDeleteShadow(pool: Pool, acquisitionSourceId: string): Promise<void> {
  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    const src = await client.query(
      `SELECT org_id, location_id FROM acquisition_sources WHERE id = $1`,
      [acquisitionSourceId],
    );
    const row = src.rows[0] as { org_id: string | null; location_id: string | null } | undefined;
    // Drop the acquisition_sources → org/location FK references FIRST, else deleting the location
    // violates acquisition_sources_location_id_fkey (caught by provision-rls.test.ts (h)).
    // M1 (breaker): erase the PII the pipeline ingested too — NULL place_raw + menu_draft on the
    // source row, not just the FK links. "Born with its erasure path" must actually erase the data.
    await client.query(
      `UPDATE acquisition_sources SET org_id = NULL, location_id = NULL, place_raw = NULL, menu_draft = NULL WHERE id = $1`,
      [acquisitionSourceId],
    );
    await client.query(`DELETE FROM provision_grants WHERE acquisition_source_id = $1`, [acquisitionSourceId]);
    if (row?.location_id) {
      await client.query(`DELETE FROM products WHERE location_id = $1`, [row.location_id]);
      await client.query(`DELETE FROM categories WHERE location_id = $1`, [row.location_id]);
      await client.query(`DELETE FROM menu_versions WHERE location_id = $1`, [row.location_id]);
      await client.query(`DELETE FROM locations WHERE id = $1`, [row.location_id]);
    }
    if (row?.org_id) {
      await client.query(`DELETE FROM organizations WHERE id = $1 AND owner_id IS NULL`, [row.org_id]);
    }
    await client.query('COMMIT');
  } catch (e) {
    await client.query('ROLLBACK');
    throw e;
  } finally {
    client.release();
  }
}

/** Reaper (Q6): delete expired-unconsumed grants. Wire to the existing reconcile cron. */
export async function reapExpiredGrants(pool: Pool): Promise<number> {
  const res = await pool.query(
    `DELETE FROM provision_grants WHERE consumed_at IS NULL AND expires_at < now() - interval '1 day'`,
  );
  return res.rowCount ?? 0;
}
