import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { createSource, advance } from '../src/modules/acquisition/service.js';
import { mintProvisionToken, provisionShadowSpine } from '../src/modules/acquisition/provisioning.js';
import { markVerified, mintClaimInvite, acceptClaim } from '../src/modules/acquisition/claim.js';
import { reapAbandonedShadows, runRetentionSweep } from '../src/modules/acquisition/retention.js';

// P6 retention sweep — GDPR Art-5(e): a never-claimed PUBLIC shadow must self-erase on a short TTL.
// The sweep is an ops op (hard-deletes) → runs on the bypassing operational pool (adminPool here).
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

const DRAFT = { categories: [{ name: 'C', products: [{ name: 'P', price: 100 }] }] };

async function provisionShadow(): Promise<{ sourceId: string; orgId: string; slug: string }> {
  const src = await createSource(pool, 'ChIJ_ret_' + crypto.randomBytes(6).toString('hex'));
  await advance(pool, src.id, 'PLACE_INGESTED', { website_url: 'https://x.test' });
  await advance(pool, src.id, 'MENU_EXTRACTED', { menu_draft: DRAFT });
  await advance(pool, src.id, 'ENRICHED');
  const { token } = await mintProvisionToken(pool, src.id);
  const slug = 'ret-' + crypto.randomBytes(4).toString('hex');
  const { orgId } = await provisionShadowSpine(pool, { acquisitionSourceId: src.id, token, name: 'Ret', slug });
  return { sourceId: src.id, orgId, slug };
}

async function backdate(sourceId: string, days: number) {
  await adminPool.query(`UPDATE acquisition_sources SET created_at = now() - ($1 * interval '1 day') WHERE id = $2`, [days, sourceId]);
}

maybe('a stale, never-claimed shadow is hard-deleted + ABANDONED past the TTL', async () => {
  const { sourceId, orgId } = await provisionShadow();
  await backdate(sourceId, 40);
  const n = await reapAbandonedShadows(adminPool, 30);
  assert.ok(n >= 1, 'at least the stale shadow reaped');
  const org = await adminPool.query('SELECT count(*)::int c FROM organizations WHERE id = $1', [orgId]);
  assert.equal(org.rows[0].c, 0, 'shadow org erased');
  const s = await adminPool.query('SELECT state FROM acquisition_sources WHERE id = $1', [sourceId]);
  assert.equal(s.rows[0].state, 'ABANDONED', 'source abandoned with reason');
});

maybe('a FRESH unclaimed shadow survives the sweep', async () => {
  const { sourceId, orgId } = await provisionShadow(); // created_at = now
  await reapAbandonedShadows(adminPool, 30);
  const org = await adminPool.query('SELECT count(*)::int c FROM organizations WHERE id = $1', [orgId]);
  assert.equal(org.rows[0].c, 1, 'fresh shadow NOT reaped');
  const s = await adminPool.query('SELECT state FROM acquisition_sources WHERE id = $1', [sourceId]);
  assert.notEqual(s.rows[0].state, 'ABANDONED');
});

maybe('a CLAIMED (consented) tenant is NEVER reaped, even when stale', async () => {
  const { sourceId, orgId } = await provisionShadow();
  await markVerified(pool, sourceId);
  const { token } = await mintClaimInvite(pool, sourceId);
  const u = (await adminPool.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['o-' + crypto.randomBytes(4).toString('hex') + '@t.test'])).rows[0].id;
  await acceptClaim(pool, token, u);
  await backdate(sourceId, 999);
  await reapAbandonedShadows(adminPool, 30);
  const org = await adminPool.query('SELECT owner_id FROM organizations WHERE id = $1', [orgId]);
  assert.equal(org.rows[0].owner_id, u, 'claimed tenant + owner intact (state CLAIMED is excluded)');
});

maybe('runRetentionSweep reaps expired grants + invites + abandoned shadows together', async () => {
  // expired grant
  const g = await provisionShadow();
  await adminPool.query(`INSERT INTO provision_grants (acquisition_source_id, token_hash, expires_at) VALUES ($1,$2, now() - interval '2 days')`, [g.sourceId, crypto.randomBytes(8).toString('hex')]);
  // expired invite
  const i = await provisionShadow();
  await markVerified(pool, i.sourceId);
  await adminPool.query(`INSERT INTO claim_invites (acquisition_source_id, token_hash, expires_at) VALUES ($1,$2, now() - interval '2 days')`, [i.sourceId, crypto.randomBytes(8).toString('hex')]);
  // stale shadow
  const s = await provisionShadow();
  await backdate(s.sourceId, 40);

  const res = await runRetentionSweep(adminPool, { abandonedTtlDays: 30 });
  assert.ok(res.grants >= 1, 'expired grant reaped');
  assert.ok(res.invites >= 1, 'expired invite revoked');
  assert.ok(res.shadows >= 1, 'stale shadow reaped');
});
