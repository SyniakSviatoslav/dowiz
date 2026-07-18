import test from 'node:test';
import assert from 'node:assert/strict';
import pg from 'pg';
import Fastify from 'fastify';

// Integration test for the telegram-webhook callback routing (the glue the service-level
// tests don't cover: callback parse → location-scoped authority (BR-3) → flag gate →
// switch → service). Requires the migration chain applied:
//   TEST_DATABASE_URL=postgresql://… node --test --import tsx tests/notifications/telegram-webhook-storefront.test.ts
const url = process.env.TEST_DATABASE_URL;

test('telegram webhook storefront + pref routing (DB integration)', { skip: url ? false : 'set TEST_DATABASE_URL to run' }, async (t) => {
  // Flags + token must be set BEFORE the plugin module loads (it reads them at import).
  process.env.TG_STOREFRONT_ACTION = 'true';
  process.env.TG_CATEGORY_GATING = 'true';
  process.env.TELEGRAM_BOT_TOKEN = 'test-token';

  // Mock the Telegram Bot API so answerCallbackQuery/sendMessage resolve instantly
  // (the handler answers the callback BEFORE the mutation, so this must succeed).
  const origFetch = globalThis.fetch;
  globalThis.fetch = (async (u: any, init?: any) => {
    if (String(u).includes('api.telegram.org')) {
      return { ok: true, status: 200, json: async () => ({ ok: true, result: {} }), text: async () => '{}' } as any;
    }
    return origFetch(u, init);
  }) as any;

  const webhookRoutes = (await import('../../src/routes/telegram-webhook.js')).default;
  const pool = new pg.Pool({ connectionString: url });

  const chatId = '987654321';
  let locationId = '';
  let userId = '';
  let targetId = '';
  let otherLoc = '';

  const c = await pool.connect();
  try {
    const u = await c.query(
      `INSERT INTO users(email, google_sub, display_name) VALUES ('wh@dev.com','wh-sub','WH')
       ON CONFLICT (google_sub) DO UPDATE SET email = EXCLUDED.email RETURNING id`,
    );
    userId = u.rows[0].id;
    const org = await c.query(`INSERT INTO organizations(name) VALUES ('WH Org') RETURNING id`);
    const loc = await c.query(`INSERT INTO locations(org_id, slug, name, phone) VALUES ($1,'wh-loc','WH Loc','+355') RETURNING id`, [org.rows[0].id]);
    locationId = loc.rows[0].id;
    await c.query(`INSERT INTO memberships(user_id, location_id, role, status) VALUES ($1,$2,'owner','active')`, [userId, locationId]);
    const tgt = await c.query(
      `INSERT INTO owner_notification_targets(location_id, channel, address, status, user_id)
       VALUES ($1,'telegram',$2,'active',$3) RETURNING id`,
      [locationId, chatId, userId],
    );
    targetId = tgt.rows[0].id;
    // a foreign location the chat is NOT linked to (BR-3 target)
    const org2 = await c.query(`INSERT INTO organizations(name) VALUES ('Other Org') RETURNING id`);
    const loc2 = await c.query(`INSERT INTO locations(org_id, slug, name, phone) VALUES ($1,'other-loc','Other','+355') RETURNING id`, [org2.rows[0].id]);
    otherLoc = loc2.rows[0].id;
  } finally {
    c.release();
  }

  const app = Fastify();
  await app.register(webhookRoutes, {
    db: pool,
    queue: { boss: { send: async () => {} } },
    telegramBotSecret: 'sek',
    messageBus: { publish: async () => {} },
  });

  const cb = (data: string) => ({
    update_id: 1,
    callback_query: { id: 'cbid', from: { id: Number(chatId) }, data, message: { chat: { id: Number(chatId) }, message_id: 10, text: 'x' } },
  });
  const post = (update: any) => app.inject({ method: 'POST', url: '/webhook/telegram/sek', payload: update });
  const paused = async (id: string) => (await pool.query(`SELECT delivery_paused FROM locations WHERE id=$1`, [id])).rows[0].delivery_paused;

  await t.test('store.open toggles delivery_paused → false', async () => {
    await pool.query(`UPDATE locations SET delivery_paused=true WHERE id=$1`, [locationId]);
    const r = await post(cb(`store.open:${locationId}`));
    assert.equal(r.statusCode, 200);
    assert.equal(await paused(locationId), false);
  });

  await t.test('store.close creates a confirm nonce but does NOT toggle yet', async () => {
    const r = await post(cb(`store.close:${locationId}`));
    assert.equal(r.statusCode, 200);
    const nonce = (await pool.query(
      `SELECT nonce FROM telegram_action_nonces WHERE location_id=$1 AND action='store.close' ORDER BY created_at DESC LIMIT 1`,
      [locationId],
    )).rows[0]?.nonce;
    assert.ok(nonce, 'confirm nonce created');
    assert.equal(await paused(locationId), false, 'close not applied until confirmed');
    // confirm → toggles true
    const r2 = await post(cb(`store.confirm:${locationId}:${nonce}`));
    assert.equal(r2.statusCode, 200);
    assert.equal(await paused(locationId), true);
  });

  await t.test('pref.set toggles category + writes consent audit (changed_via=telegram)', async () => {
    const r = await post(cb(`pref.set:${locationId}:operational:0`));
    assert.equal(r.statusCode, 200);
    const prefs = (await pool.query(`SELECT prefs FROM owner_notification_targets WHERE id=$1`, [targetId])).rows[0].prefs;
    assert.equal(prefs.operational, false);
    const n = (await pool.query(
      `SELECT count(*)::int AS n FROM notification_prefs_audit WHERE target_id=$1 AND category='operational' AND changed_via='telegram'`,
      [targetId],
    )).rows[0].n;
    assert.ok(n >= 1, 'consent audit row written');
  });

  await t.test('BR-3: store.open on a foreign location is denied (no mutation)', async () => {
    await pool.query(`UPDATE locations SET delivery_paused=true WHERE id=$1`, [otherLoc]);
    const r = await post(cb(`store.open:${otherLoc}`)); // chat has no target@otherLoc
    assert.equal(r.statusCode, 200); // handler answers gracefully
    assert.equal(await paused(otherLoc), true, 'foreign location must stay unchanged');
  });

  await app.close();
  await pool.end();
  globalThis.fetch = origFetch;
});
