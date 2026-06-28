import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { createSource, advance } from '../src/modules/acquisition/service.js';
import { mintProvisionToken, provisionShadowSpine } from '../src/modules/acquisition/provisioning.js';
import { markVerified, mintClaimInvite, acceptClaim, declineAndErase, ClaimError } from '../src/modules/acquisition/claim.js';

// P6 CLAIM PHASE — proves the ownership transfer THROUGH RLS under a real NOBYPASSRLS role (the live
// operational role bypasses RLS → would mask the claim_accept policy). Skips without both env URLs.
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

const RICH_DRAFT = { categories: [{ name: 'Pizza', products: [{ name: 'Margherita', price: 850 }] }] };

// Provision a shadow + advance to CLAIM_OFFERED. Returns ids + the plaintext claim token.
async function offeredShadow(): Promise<{ sourceId: string; orgId: string; locationId: string; slug: string; token: string }> {
  const src = await createSource(pool, 'ChIJ_claim_' + crypto.randomBytes(6).toString('hex'));
  await advance(pool, src.id, 'PLACE_INGESTED', { website_url: 'https://x.test' });
  await advance(pool, src.id, 'MENU_EXTRACTED', { menu_draft: RICH_DRAFT });
  await advance(pool, src.id, 'ENRICHED');
  const { token: provTok } = await mintProvisionToken(pool, src.id);
  const slug = 'claim-' + crypto.randomBytes(4).toString('hex');
  const { orgId, locationId } = await provisionShadowSpine(pool, { acquisitionSourceId: src.id, token: provTok, name: 'Claimco', slug });
  await markVerified(pool, src.id); // PROVISIONED→VERIFIED (cheap floor: preview renders)
  const { token } = await mintClaimInvite(pool, src.id, 'owner@claimco.test'); // VERIFIED→CLAIM_OFFERED
  return { sourceId: src.id, orgId, locationId, slug, token };
}

async function newUser(): Promise<string> {
  const r = await adminPool.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['u-' + crypto.randomBytes(4).toString('hex') + '@t.test']);
  return r.rows[0].id;
}

maybe('(a) happy path: accept transfers ownership, no auto-publish, raw blob erased', async () => {
  const { sourceId, orgId, locationId, token } = await offeredShadow();
  await adminPool.query(`UPDATE acquisition_sources SET place_raw = '{"phone":"x"}'::jsonb WHERE id = $1`, [sourceId]);
  const userId = await newUser();

  const res = await acceptClaim(pool, token, userId);
  assert.equal(res.orgId, orgId);

  const org = await adminPool.query('SELECT owner_id FROM organizations WHERE id = $1', [orgId]);
  assert.equal(org.rows[0].owner_id, userId, 'ownership transferred to the claimer');
  const mem = await adminPool.query(`SELECT count(*)::int n FROM memberships WHERE location_id = $1 AND user_id = $2 AND role = 'owner' AND status = 'active'`, [locationId, userId]);
  assert.equal(mem.rows[0].n, 1, 'active owner membership created');
  const loc = await adminPool.query('SELECT status, published_at FROM locations WHERE id = $1', [locationId]);
  assert.equal(loc.rows[0].status, 'closed', 'B3: NOT auto-published — status stays closed');
  assert.equal(loc.rows[0].published_at, null, 'B3: published_at stays NULL after claim');
  const s = await adminPool.query('SELECT state, place_raw, menu_draft FROM acquisition_sources WHERE id = $1', [sourceId]);
  assert.equal(s.rows[0].state, 'CLAIMED');
  assert.equal(s.rows[0].place_raw, null, 'H-erase: raw scraped blob cleared on claim');
  assert.equal(s.rows[0].menu_draft, null, 'H-erase: menu_draft cleared on claim');
});

maybe('(b) IDOR: bogus / expired / used token cannot claim — owner_id stays NULL', async () => {
  const { orgId } = await offeredShadow();
  const userId = await newUser();
  await assert.rejects(
    () => acceptClaim(pool, 'deadbeef'.repeat(8), userId),
    (e: unknown) => e instanceof ClaimError && (e as ClaimError).code === 'INVALID_OR_EXPIRED_TOKEN',
  );
  const org = await adminPool.query('SELECT owner_id FROM organizations WHERE id = $1', [orgId]);
  assert.equal(org.rows[0].owner_id, null, 'no token → no transfer');
});

maybe('(c) double-claim race: second accept of the same source is rejected', async () => {
  const { token, orgId } = await offeredShadow();
  const u1 = await newUser();
  await acceptClaim(pool, token, u1);
  // the token is now used; a second accept with it → INVALID
  const u2 = await newUser();
  await assert.rejects(() => acceptClaim(pool, token, u2), (e: unknown) => e instanceof ClaimError);
  const org = await adminPool.query('SELECT owner_id FROM organizations WHERE id = $1', [orgId]);
  assert.equal(org.rows[0].owner_id, u1, 'first claimer wins; no second owner');
});

maybe('(d) one active invite per source (mint race guard)', async () => {
  const { sourceId } = await offeredShadow();
  await assert.rejects(
    () => mintClaimInvite(pool, sourceId, 'again@t.test'),
    (e: unknown) => e instanceof ClaimError && (e as ClaimError).code === 'ACTIVE_INVITE_EXISTS',
  );
});

maybe('(e) RLS: a direct UPDATE of a shadow org owner_id (not via claim_transfer) cannot touch it', async () => {
  const { orgId } = await offeredShadow();
  // claim_transfer (SECURITY DEFINER, token-gated) is the ONLY path. A direct provtest UPDATE is
  // blocked by RLS (no SELECT/UPDATE policy admits the shadow org → 0 rows, owner_id stays NULL).
  const upd = await pool.query(`UPDATE organizations SET owner_id = gen_random_uuid() WHERE id = $1 AND owner_id IS NULL`, [orgId]);
  assert.equal(upd.rowCount, 0, 'no policy lets the claim role directly transfer ownership');
  const org = await adminPool.query('SELECT owner_id FROM organizations WHERE id = $1', [orgId]);
  assert.equal(org.rows[0].owner_id, null, 'owner_id stays NULL — only claim_transfer can set it');
});

maybe('(f) tenant flip: after claim, read_preview_menu no longer serves the (now real) tenant', async () => {
  const { slug, token } = await offeredShadow();
  const before = await adminPool.query('SELECT read_preview_menu($1) AS m', [slug]);
  assert.ok(before.rows[0].m, 'preview served pre-claim');
  await acceptClaim(pool, token, await newUser());
  const afterRes = await adminPool.query('SELECT read_preview_menu($1) AS m', [slug]);
  assert.equal(afterRes.rows[0].m, null, 'owner_id set → no longer a shadow → preview returns null');
});

maybe('(g) decline + erase: token-only erases the shadow tenant', async () => {
  const { token, orgId, locationId, sourceId } = await offeredShadow();
  // decline/erase is an ops operation (hard-deletes shadow rows) → runs on the bypassing operational
  // pool, exactly like hardDeleteShadow, since no tenant policy admits deleting a shadow row.
  await declineAndErase(adminPool, token);
  const org = await adminPool.query('SELECT count(*)::int n FROM organizations WHERE id = $1', [orgId]);
  const loc = await adminPool.query('SELECT count(*)::int n FROM locations WHERE id = $1', [locationId]);
  const s = await adminPool.query('SELECT state FROM acquisition_sources WHERE id = $1', [sourceId]);
  assert.equal(org.rows[0].n, 0, 'org erased');
  assert.equal(loc.rows[0].n, 0, 'location erased');
  assert.equal(s.rows[0].state, 'ABANDONED', 'source abandoned');
});
