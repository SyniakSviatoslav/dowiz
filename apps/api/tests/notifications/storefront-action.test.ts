import test from 'node:test';
import assert from 'node:assert/strict';
import pg from 'pg';
import {
  setStorefrontPaused,
  createCloseNonce,
  getLocationStorefront,
} from '../../src/lib/storefrontService.js';

// Integration test for the Telegram storefront toggle (BR-23 changed/noop/denied,
// BR-19 one-shot nonce). Requires a Postgres URL with the migration chain applied:
//   TEST_DATABASE_URL=postgresql://… node --test --import tsx tests/notifications/storefront-action.test.ts
const url = process.env.TEST_DATABASE_URL;

test('storefront action (DB integration)', { skip: url ? false : 'set TEST_DATABASE_URL to run' }, async (t) => {
  const pool = new pg.Pool({ connectionString: url });
  const userId = '11111111-1111-1111-1111-111111111111';
  const userIdB = '22222222-2222-2222-2222-222222222222';
  let locationId = '';
  let locationIdB = '';

  const seed = await pool.connect();
  try {
    const org = await seed.query(`INSERT INTO organizations(name) VALUES ('SF Org') RETURNING id`);
    const loc = await seed.query(
      `INSERT INTO locations(org_id, slug, name, phone) VALUES ($1, 'sf-loc', 'SF Loc', '+355600') RETURNING id`,
      [org.rows[0].id],
    );
    locationId = loc.rows[0].id;
    // Second location (same org) for nonce cross-location portability test.
    const locB = await seed.query(
      `INSERT INTO locations(org_id, slug, name, phone) VALUES ($1, 'sf-loc-b', 'SF Loc B', '+355601') RETURNING id`,
      [org.rows[0].id],
    );
    locationIdB = locB.rows[0].id;
  } finally {
    seed.release();
  }

  await t.test('open initial state', async () => {
    const c = await pool.connect();
    try {
      const st = await getLocationStorefront(c, locationId);
      assert.equal(st?.paused, false);
      assert.equal(st?.name, 'SF Loc');
    } finally { c.release(); }
  });

  await t.test('close → changed, then double-tap → noop', async () => {
    const c = await pool.connect();
    try {
      const r1 = await setStorefrontPaused(c, locationId, userId, true);
      assert.deepEqual(r1, { result: 'changed', paused: true });
      const r2 = await setStorefrontPaused(c, locationId, userId, true);
      assert.deepEqual(r2, { result: 'noop', paused: true });
      assert.equal((await getLocationStorefront(c, locationId))?.paused, true);
    } finally { c.release(); }
  });

  await t.test('open → changed back', async () => {
    const c = await pool.connect();
    try {
      const r = await setStorefrontPaused(c, locationId, userId, false);
      assert.deepEqual(r, { result: 'changed', paused: false });
    } finally { c.release(); }
  });

  await t.test('non-existent location → denied (never reported as success)', async () => {
    const c = await pool.connect();
    try {
      const r = await setStorefrontPaused(c, '99999999-9999-9999-9999-999999999999', userId, true);
      assert.deepEqual(r, { result: 'denied' });
    } finally { c.release(); }
  });

  await t.test('nonce: valid consume → changed; replay → nonce_invalid (one-shot)', async () => {
    const c = await pool.connect();
    try {
      const nonce = await createCloseNonce(c, locationId, userId, 'chat-1');
      const r1 = await setStorefrontPaused(c, locationId, userId, true, { consumeNonce: nonce });
      assert.deepEqual(r1, { result: 'changed', paused: true });
      // replay the same nonce — already consumed
      const r2 = await setStorefrontPaused(c, locationId, userId, true, { consumeNonce: nonce });
      assert.deepEqual(r2, { result: 'nonce_invalid' });
      await setStorefrontPaused(c, locationId, userId, false); // reset
    } finally { c.release(); }
  });

  await t.test('expired nonce → nonce_invalid and storefront NOT toggled', async () => {
    const c = await pool.connect();
    try {
      const expiredNonce = 'expired-nonce-abc';
      await c.query(
        `INSERT INTO telegram_action_nonces (nonce, location_id, user_id, action, chat_id, expires_at)
         VALUES ($1, $2, $3, 'store.close', 'chat-1', now() - interval '1 minute')`,
        [expiredNonce, locationId, userId],
      );
      const before = (await getLocationStorefront(c, locationId))?.paused;
      const r = await setStorefrontPaused(c, locationId, userId, true, { consumeNonce: expiredNonce });
      assert.deepEqual(r, { result: 'nonce_invalid' });
      const after = (await getLocationStorefront(c, locationId))?.paused;
      assert.equal(after, before, 'storefront must be unchanged when nonce is expired');
    } finally { c.release(); }
  });

  await t.test('nonce bound to location-A cannot be replayed against location-B (storefrontService.ts:90 AND location_id)', async () => {
    const c = await pool.connect();
    try {
      const nonce = await createCloseNonce(c, locationId, userId, 'chat-1');
      const beforeA = (await getLocationStorefront(c, locationId))?.paused;
      const beforeB = (await getLocationStorefront(c, locationIdB))?.paused;
      // Consume against the WRONG location — DELETE's `AND location_id = $2` must reject it.
      const r = await setStorefrontPaused(c, locationIdB, userId, true, { consumeNonce: nonce });
      assert.deepEqual(r, { result: 'nonce_invalid' });
      assert.equal((await getLocationStorefront(c, locationId))?.paused, beforeA, 'location-A unchanged');
      assert.equal((await getLocationStorefront(c, locationIdB))?.paused, beforeB, 'location-B unchanged');
      // The nonce is still valid for its own location (it was not consumed by the rejected call).
      const ok = await setStorefrontPaused(c, locationId, userId, true, { consumeNonce: nonce });
      assert.deepEqual(ok, { result: 'changed', paused: true });
      await setStorefrontPaused(c, locationId, userId, false); // reset
    } finally { c.release(); }
  });

  await t.test('nonce created by user-A cannot be consumed by user-B (storefrontService.ts:90 AND user_id)', async () => {
    const c = await pool.connect();
    try {
      const nonce = await createCloseNonce(c, locationId, userId, 'chat-1');
      const beforeA = (await getLocationStorefront(c, locationId))?.paused;
      // Consume as the WRONG user — DELETE's `AND user_id = $3` must reject it.
      const r = await setStorefrontPaused(c, locationId, userIdB, true, { consumeNonce: nonce });
      assert.deepEqual(r, { result: 'nonce_invalid' });
      assert.equal((await getLocationStorefront(c, locationId))?.paused, beforeA, 'storefront unchanged for wrong-user nonce');
      // The nonce remains valid for its owner (was not consumed by the rejected call).
      const ok = await setStorefrontPaused(c, locationId, userId, true, { consumeNonce: nonce });
      assert.deepEqual(ok, { result: 'changed', paused: true });
      await setStorefrontPaused(c, locationId, userId, false); // reset
    } finally { c.release(); }
  });

  // TODO(needs_staging): cross-tenant RLS enforcement — a userId with NO org membership must get
  // { result: 'denied' }, not 'changed'. setStorefrontPaused relies on the locations FORCE-RLS
  // policy reading the `app.user_id` GUC (storefrontService.ts:85). This pool connects as a
  // migration/superuser role that BYPASSRLS, so the GUC has no effect here and the assertion would
  // falsely pass as 'changed'. A faithful test needs a non-BYPASSRLS app-role DB connection
  // (staging operational role), so it is not authored here rather than faked. See FINDING #1.

  await pool.end();
});
