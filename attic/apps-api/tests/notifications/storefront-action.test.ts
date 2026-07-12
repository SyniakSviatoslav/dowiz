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
  let locationId = '';

  const seed = await pool.connect();
  try {
    const org = await seed.query(`INSERT INTO organizations(name) VALUES ('SF Org') RETURNING id`);
    const loc = await seed.query(
      `INSERT INTO locations(org_id, slug, name, phone) VALUES ($1, 'sf-loc', 'SF Loc', '+355600') RETURNING id`,
      [org.rows[0].id],
    );
    locationId = loc.rows[0].id;
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

  await pool.end();
});
