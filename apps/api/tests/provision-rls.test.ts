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

// Walk a fresh source to ENRICHED (the legal predecessor of PROVISIONED).
async function freshEnrichedSource(): Promise<string> {
  const src = await createSource(pool, 'ChIJ_p62_' + crypto.randomBytes(6).toString('hex'));
  await advance(pool, src.id, 'PLACE_INGESTED', { website_url: 'https://x.test' });
  await advance(pool, src.id, 'MENU_EXTRACTED', { menu_draft: { categories: [] } });
  await advance(pool, src.id, 'ENRICHED');
  return src.id;
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

maybe('(h) C2 day-one hard-delete erases the shadow tenant + grants', async () => {
  const srcId = await freshEnrichedSource();
  const { token } = await mintProvisionToken(pool, srcId);
  const { orgId, locationId } = await provisionShadowSpine(pool, { acquisitionSourceId: srcId, token, name: 'Del', slug: 'del-' + crypto.randomBytes(4).toString('hex') });
  // hard-delete is an ops erase (no tenant policy admits deleting shadow rows) → runs on the
  // bypassing operational role, exactly as every write does today (db/index.ts:34-38).
  await hardDeleteShadow(adminPool, srcId);
  const org = await adminPool.query('SELECT count(*)::int n FROM organizations WHERE id = $1', [orgId]);
  const loc = await adminPool.query('SELECT count(*)::int n FROM locations WHERE id = $1', [locationId]);
  const gr = await adminPool.query('SELECT count(*)::int n FROM provision_grants WHERE acquisition_source_id = $1', [srcId]);
  assert.equal(org.rows[0].n, 0, 'org erased');
  assert.equal(loc.rows[0].n, 0, 'location erased');
  assert.equal(gr.rows[0].n, 0, 'grants erased');
});
