import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { createSource, advance } from '../src/modules/acquisition/service.js';
import { mintProvisionToken, provisionShadowSpine } from '../src/modules/acquisition/provisioning.js';
import { markVerified, ClaimError } from '../src/modules/acquisition/claim.js';
import { verifyShadowPreview } from '../src/modules/acquisition/provision-verifier.js';

// P6-6 ProvisionVerifier — the rendered preview must pass every external-boundary invariant before a
// shadow is offered for claim. Server-side render-check (the preview is static HTML); the live Playwright
// re-asserts the same invariants on staging (e2e/tests/p6-provision-verify.spec.ts).
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

const WITH_MENU = { categories: [{ name: 'Pizza', products: [{ name: 'Margherita', price: 850 }] }] };

async function provision(draft: unknown): Promise<{ sourceId: string; slug: string }> {
  const src = await createSource(pool, 'ChIJ_ver_' + crypto.randomBytes(6).toString('hex'));
  await advance(pool, src.id, 'PLACE_INGESTED', { website_url: 'https://x.test' });
  await advance(pool, src.id, 'MENU_EXTRACTED', { menu_draft: draft });
  await advance(pool, src.id, 'ENRICHED');
  const { token } = await mintProvisionToken(pool, src.id);
  const slug = 'ver-' + crypto.randomBytes(4).toString('hex');
  await provisionShadowSpine(pool, { acquisitionSourceId: src.id, token, name: 'Verifyco', slug });
  return { sourceId: src.id, slug };
}

maybe('a well-formed shadow preview passes ALL verifier checks', async () => {
  const { slug } = await provision(WITH_MENU);
  const v = await verifyShadowPreview(pool, slug);
  assert.equal(v.ok, true, `expected ok, failed: ${v.failed.join(',')}`);
  assert.deepEqual(v.checks, { served: true, hasItems: true, banner: true, noindex: true, genericOg: true, neverOrderable: true });
});

maybe('an EMPTY-menu shadow fails verification (hasItems) → markVerified throws', async () => {
  const { sourceId, slug } = await provision({ categories: [] }); // container only, no products
  const v = await verifyShadowPreview(pool, slug);
  assert.equal(v.ok, false);
  assert.ok(v.failed.includes('hasItems'), 'no items → not verifiable');
  await assert.rejects(
    () => markVerified(pool, sourceId),
    (e: unknown) => e instanceof ClaimError && (e as ClaimError).code === 'NOT_VERIFIABLE',
  );
});

maybe('markVerified advances a well-formed shadow to VERIFIED', async () => {
  const { sourceId } = await provision(WITH_MENU);
  await markVerified(pool, sourceId);
  const s = await adminPool.query('SELECT state FROM acquisition_sources WHERE id = $1', [sourceId]);
  assert.equal(s.rows[0].state, 'VERIFIED');
});

maybe('verifier does not serve a non-shadow (real) tenant', async () => {
  // a real published tenant is never served by read_preview_menu → served=false → not ok
  const owner = (await adminPool.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['v-' + crypto.randomBytes(4).toString('hex') + '@t.test'])).rows[0].id;
  const org = (await adminPool.query(`INSERT INTO organizations (name, owner_id) VALUES ('R',$1) RETURNING id`, [owner])).rows[0].id;
  const realSlug = 'real-' + crypto.randomBytes(4).toString('hex');
  await adminPool.query(`INSERT INTO locations (org_id, slug, name, phone, status, published_at) VALUES ($1,$2,'R','','open',now())`, [org, realSlug]);
  const v = await verifyShadowPreview(pool, realSlug);
  assert.equal(v.ok, false);
  assert.equal(v.checks.served, false, 'real tenant never served by the preview verifier');
});
