import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { createSource, advance, getById } from '../src/modules/acquisition/service.js';
import {
  mintProvisionToken,
  provisionShadowSpine,
  hardDeleteShadow,
  ProvisionError,
} from '../src/modules/acquisition/provisioning.js';

// P6-2 RLS PROOF — runs the REAL provisioning code under a REAL NOBYPASSRLS role (the live
// operational role bypasses RLS today and would MASK the policy, so the proof MUST use a role that
// actually obeys it — architect proof gate). Skips cleanly when PROV_TEST_DATABASE_URL is unset.
// The proof run points it at a throwaway PG seeded by scratchpad/setup.sql, connected as `provtest`.
// `pool` = the NOBYPASSRLS provtest role (proves the policy). `adminPool` = a bypassing role
// (postgres) standing in for the LIVE operational role, which bypasses RLS today (db/index.ts:34-38):
// reading/erasing shadow rows is an ops operation with no tenant policy, so — exactly like onboarding
// and order-create today — it runs on the bypassing role. The SECURITY claims are proven on `pool`;
// the FUNCTIONAL write/read/erase are verified on `adminPool`.
const url = process.env.PROV_TEST_DATABASE_URL;
const adminUrl = process.env.PROV_TEST_ADMIN_URL;
const maybe = url && adminUrl ? test : test.skip;

let pool: Pool;
let adminPool: Pool;
before(() => {
  if (url) pool = new Pool({ connectionString: url });
  if (adminUrl) adminPool = new Pool({ connectionString: adminUrl });
});
after(async () => {
  if (pool) await pool.end();
  if (adminPool) await adminPool.end();
});

// Walk a fresh source to ENRICHED (the legal predecessor of PROVISIONED). Optionally seed a menu_draft.
async function freshEnrichedSource(draft: unknown = { categories: [] }): Promise<string> {
  const src = await createSource(pool, 'ChIJ_p62_' + crypto.randomBytes(6).toString('hex'));
  await advance(pool, src.id, 'PLACE_INGESTED', { website_url: 'https://x.test' });
  await advance(pool, src.id, 'MENU_EXTRACTED', { menu_draft: draft });
  await advance(pool, src.id, 'ENRICHED');
  return src.id;
}

// A draft with a category + a product carrying AI-guessed allergens in bom (to prove write-strip).
const RICH_DRAFT = {
  categories: [
    {
      name: 'Pizza',
      sort_order: 0,
      products: [
        {
          name: 'Margherita',
          price: 850,
          description: 'Tomato, mozzarella, basil',
          attributes: { bom: [{ ingredient: 'cheese', allergens: ['milk'] }], spicy: false },
        },
      ],
    },
  ],
};

// A real (non-shadow) tenant location via the admin/bypass pool — to prove H2 cannot widen into it.
async function makeRealLocation(): Promise<string> {
  const orgId = crypto.randomUUID();
  const locId = crypto.randomUUID();
  // organizations.owner_id FKs to users on the real schema → mint a real owner first.
  const owner = (await adminPool.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['real-' + crypto.randomBytes(4).toString('hex') + '@t.test'])).rows[0].id;
  await adminPool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'Real', $2)`, [orgId, owner]);
  await adminPool.query(
    `INSERT INTO locations (id, org_id, slug, name, phone, status) VALUES ($1, $2, $3, 'Real', '', 'open')`,
    [locId, orgId, 'real-' + crypto.randomBytes(4).toString('hex')],
  );
  return locId;
}

maybe('(a) token write ADMITTED: provisionShadowSpine writes the shadow spine through RLS', async () => {
  const srcId = await freshEnrichedSource();
  const { token } = await mintProvisionToken(pool, srcId);
  const { orgId, locationId } = await provisionShadowSpine(pool, {
    acquisitionSourceId: srcId,
    token,
    name: 'Trattoria Test',
    slug: 'trattoria-' + crypto.randomBytes(4).toString('hex'),
  });
  // verify via adminPool — provtest cannot SELECT a shadow org (no SELECT policy admits owner_id NULL).
  const org = await adminPool.query('SELECT owner_id FROM organizations WHERE id = $1', [orgId]);
  assert.equal(org.rows[0].owner_id, null, 'shadow org owner_id must be NULL');
  const loc = await adminPool.query('SELECT status, published_at FROM locations WHERE id = $1', [locationId]);
  assert.equal(loc.rows[0].status, 'closed', 'shadow location must be closed');
  assert.equal(loc.rows[0].published_at, null, 'B3: published_at must stay NULL (never orderable)');
  const mv = await adminPool.query('SELECT count(*)::int n FROM menu_versions WHERE location_id = $1', [locationId]);
  assert.equal(mv.rows[0].n, 1, 'menu_versions v1 written');
  const src = await getById(pool, srcId);
  assert.equal(src?.state, 'PROVISIONED', 'source advanced to PROVISIONED');
  assert.equal(src?.org_id, orgId);
});

maybe('(b) no-token write REJECTED by provision_shadow under NOBYPASSRLS', async () => {
  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    // No set_config('app.provision_token', ...) → policy EXISTS() is false.
    await assert.rejects(
      () => client.query(`INSERT INTO organizations (id, name, owner_id) VALUES (gen_random_uuid(), 'X', NULL)`),
      /row-level security|policy/i,
      'INSERT without a token must be rejected by RLS',
    );
    await client.query('ROLLBACK');
  } finally {
    client.release();
  }
});

maybe('(c) bogus / expired token REJECTED', async () => {
  const srcId = await freshEnrichedSource();
  // bogus token never minted
  await assert.rejects(
    () => provisionShadowSpine(pool, { acquisitionSourceId: srcId, token: 'deadbeef'.repeat(8), name: 'N', slug: 's-' + crypto.randomBytes(4).toString('hex') }),
    (e: unknown) => e instanceof ProvisionError && (e as ProvisionError).code === 'INVALID_OR_EXPIRED_TOKEN',
  );
  // expired token: mint then force expiry in the past
  const { token } = await mintProvisionToken(pool, srcId);
  await pool.query(`UPDATE provision_grants SET expires_at = now() - interval '1 hour' WHERE acquisition_source_id = $1`, [srcId]);
  await assert.rejects(
    () => provisionShadowSpine(pool, { acquisitionSourceId: srcId, token, name: 'N', slug: 's2-' + crypto.randomBytes(4).toString('hex') }),
    (e: unknown) => e instanceof ProvisionError && (e as ProvisionError).code === 'INVALID_OR_EXPIRED_TOKEN',
  );
});

maybe('(d) second use of the SAME token REJECTED (single-use consume)', async () => {
  const srcId = await freshEnrichedSource();
  const { token } = await mintProvisionToken(pool, srcId);
  await provisionShadowSpine(pool, { acquisitionSourceId: srcId, token, name: 'Once', slug: 'once-' + crypto.randomBytes(4).toString('hex') });
  // reuse the now-consumed token against a NEW source → rejected (grant consumed)
  const srcId2 = await freshEnrichedSource();
  await assert.rejects(
    () => provisionShadowSpine(pool, { acquisitionSourceId: srcId2, token, name: 'Twice', slug: 'twice-' + crypto.randomBytes(4).toString('hex') }),
    (e: unknown) => e instanceof ProvisionError && (e as ProvisionError).code === 'INVALID_OR_EXPIRED_TOKEN',
  );
});

maybe('(e) NO WIDENING: a valid token cannot create a NON-shadow (owner_id NOT NULL) org', async () => {
  const srcId = await freshEnrichedSource();
  const { token } = await mintProvisionToken(pool, srcId);
  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    await client.query("SELECT set_config('app.provision_token', $1, true)", [token]);
    await assert.rejects(
      () => client.query(`INSERT INTO organizations (id, name, owner_id) VALUES (gen_random_uuid(), 'Real', gen_random_uuid())`),
      /row-level security|policy/i,
      'provision_shadow requires owner_id IS NULL → cannot mint a real org',
    );
    await client.query('ROLLBACK');
  } finally {
    client.release();
  }
});

maybe('(f) B1 dedup chokepoint: a source already PROVISIONED cannot be provisioned again', async () => {
  const srcId = await freshEnrichedSource();
  const { token: t1 } = await mintProvisionToken(pool, srcId);
  const first = await provisionShadowSpine(pool, { acquisitionSourceId: srcId, token: t1, name: 'A', slug: 'a-' + crypto.randomBytes(4).toString('hex') });
  // second attempt (state now PROVISIONED) — advance ENRICHED→PROVISIONED is illegal → rolls back.
  const { token: t2 } = await mintProvisionToken(pool, srcId);
  await assert.rejects(
    () => provisionShadowSpine(pool, { acquisitionSourceId: srcId, token: t2, name: 'B', slug: 'b-' + crypto.randomBytes(4).toString('hex') }),
    /illegal acquisition transition|changed under us/i,
  );
  const orgs = await adminPool.query('SELECT count(*)::int n FROM organizations o JOIN acquisition_sources s ON s.org_id = o.id WHERE s.id = $1', [srcId]);
  assert.equal(orgs.rows[0].n, 1, 'exactly one shadow org for the source (no double-spine)');
  // the second spine's org/location must NOT exist (rolled back)
  void first;
});

maybe('(g) one active grant per source (breaker H1 mint-side guard)', async () => {
  const srcId = await freshEnrichedSource();
  await mintProvisionToken(pool, srcId);
  await assert.rejects(
    () => mintProvisionToken(pool, srcId),
    (e: unknown) => e instanceof ProvisionError && (e as ProvisionError).code === 'ACTIVE_GRANT_EXISTS',
  );
});

maybe('(h) C2 day-one hard-delete erases the shadow tenant + grants + ingested PII', async () => {
  const srcId = await freshEnrichedSource(RICH_DRAFT);
  await adminPool.query(`UPDATE acquisition_sources SET place_raw = '{"phone":"+355..."}'::jsonb WHERE id = $1`, [srcId]);
  const { token } = await mintProvisionToken(pool, srcId);
  const { orgId, locationId } = await provisionShadowSpine(pool, { acquisitionSourceId: srcId, token, name: 'Del', slug: 'del-' + crypto.randomBytes(4).toString('hex') });
  // hard-delete is an ops erase (no tenant policy admits deleting shadow rows) → runs on the
  // bypassing operational role, exactly as every write does today (db/index.ts:34-38).
  await hardDeleteShadow(adminPool, srcId);
  const org = await adminPool.query('SELECT count(*)::int n FROM organizations WHERE id = $1', [orgId]);
  const loc = await adminPool.query('SELECT count(*)::int n FROM locations WHERE id = $1', [locationId]);
  const prod = await adminPool.query('SELECT count(*)::int n FROM products WHERE location_id = $1', [locationId]);
  const gr = await adminPool.query('SELECT count(*)::int n FROM provision_grants WHERE acquisition_source_id = $1', [srcId]);
  const src = await adminPool.query('SELECT place_raw, menu_draft FROM acquisition_sources WHERE id = $1', [srcId]);
  assert.equal(org.rows[0].n, 0, 'org erased');
  assert.equal(loc.rows[0].n, 0, 'location erased');
  assert.equal(prod.rows[0].n, 0, 'products erased');
  assert.equal(gr.rows[0].n, 0, 'grants erased');
  assert.equal(src.rows[0].place_raw, null, 'M1: place_raw PII erased');
  assert.equal(src.rows[0].menu_draft, null, 'M1: menu_draft PII erased');
});

maybe('(i) P6-3 menu write: provisionShadowSpine writes categories+products with allergens STRIPPED', async () => {
  const srcId = await freshEnrichedSource(RICH_DRAFT);
  const { token } = await mintProvisionToken(pool, srcId);
  const { locationId } = await provisionShadowSpine(pool, { acquisitionSourceId: srcId, token, name: 'Pizzeria', slug: 'pz-' + crypto.randomBytes(4).toString('hex') });
  const cat = await adminPool.query('SELECT count(*)::int n FROM categories WHERE location_id = $1', [locationId]);
  assert.equal(cat.rows[0].n, 1, 'category written');
  const prod = await adminPool.query("SELECT name, price, source, allergens_confirmed, attributes FROM products WHERE location_id = $1", [locationId]);
  assert.equal(prod.rows[0].name, 'Margherita');
  assert.equal(prod.rows[0].price, 850);
  assert.equal(prod.rows[0].source, 'place', 'provenance = place');
  assert.equal(prod.rows[0].allergens_confirmed, false, 'allergens unconfirmed');
  // C2 write-strip: bom kept (ingredients) but allergens array nulled to [].
  assert.deepEqual(prod.rows[0].attributes.bom[0].allergens, [], 'AI allergens stripped at write');
  assert.equal(prod.rows[0].attributes.bom[0].ingredient, 'cheese', 'ingredient kept (extract everything)');
});

maybe('(j) H2: a valid token CANNOT write products into a non-shadow (real tenant) location', async () => {
  const srcId = await freshEnrichedSource();
  const { token } = await mintProvisionToken(pool, srcId);
  const realLoc = await makeRealLocation();
  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    await client.query("SELECT set_config('app.provision_token', $1, true)", [token]);
    await assert.rejects(
      () => client.query(`INSERT INTO products (id, location_id, name, price, source) VALUES (gen_random_uuid(), $1, 'X', 100, 'place')`, [realLoc]),
      /row-level security|policy/i,
      'provision_shadow binds location_id to a shadow → cannot pollute a real tenant menu',
    );
    await client.query('ROLLBACK');
  } finally {
    client.release();
  }
});

maybe('(k) H2: a token-context product INSERT with source<>place is rejected', async () => {
  const srcId = await freshEnrichedSource();
  const { token } = await mintProvisionToken(pool, srcId);
  const { locationId } = await provisionShadowSpine(pool, { acquisitionSourceId: srcId, token, name: 'P', slug: 'p-' + crypto.randomBytes(4).toString('hex') });
  // a fresh token for a 2nd write attempt with source='owner' into the (now consumed-token) shadow
  const srcId2 = await freshEnrichedSource();
  const { token: t2 } = await mintProvisionToken(pool, srcId2);
  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    await client.query("SELECT set_config('app.provision_token', $1, true)", [t2]);
    await assert.rejects(
      () => client.query(`INSERT INTO products (id, location_id, name, price, source) VALUES (gen_random_uuid(), $1, 'X', 100, 'owner')`, [locationId]),
      /row-level security|policy/i,
      'provision_shadow requires source=place → cannot mint an owner-provenance row',
    );
    await client.query('ROLLBACK');
  } finally {
    client.release();
  }
});

maybe('(l) H1: read_preview_menu returns the shadow menu (bom-stripped) and ONLY for shadows', async () => {
  const srcId = await freshEnrichedSource(RICH_DRAFT);
  const { token } = await mintProvisionToken(pool, srcId);
  const slug = 'prev-' + crypto.randomBytes(4).toString('hex');
  await provisionShadowSpine(pool, { acquisitionSourceId: srcId, token, name: 'Preview Co', slug });
  const res = await adminPool.query('SELECT read_preview_menu($1) AS m', [slug]);
  const menu = res.rows[0].m;
  assert.equal(menu.is_preview, true);
  assert.equal(menu.categories[0].products[0].name, 'Margherita');
  // C2 read-gate: the whole bom (the only allergen carrier) is removed for unconfirmed place rows;
  // non-allergen attributes survive. So NO allergen surface can reach the render.
  assert.equal(menu.categories[0].products[0].attributes.bom, undefined, 'C2: bom (allergen carrier) stripped at read');
  assert.equal(menu.categories[0].products[0].attributes.spicy, false, 'non-allergen attributes survive');
  // a REAL (published) location is NOT served by read_preview_menu
  const realLoc = await makeRealLocation();
  const realSlug = (await adminPool.query('SELECT slug FROM locations WHERE id = $1', [realLoc])).rows[0].slug;
  const none = await adminPool.query('SELECT read_preview_menu($1) AS m', [realSlug]);
  assert.equal(none.rows[0].m, null, 'read_preview_menu never serves a real tenant');
});
