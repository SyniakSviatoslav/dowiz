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
  let linkedLoc = '';

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
    // a SECOND location the chat IS linked to (target + membership) — used to exercise the
    // store.confirm nonce-confusion path at the service layer (auth passes, nonce must still
    // be rejected because it was minted for a different location_id).
    const org3 = await c.query(`INSERT INTO organizations(name) VALUES ('Linked Org') RETURNING id`);
    const loc3 = await c.query(`INSERT INTO locations(org_id, slug, name, phone) VALUES ($1,'linked-loc','Linked','+355') RETURNING id`, [org3.rows[0].id]);
    linkedLoc = loc3.rows[0].id;
    await c.query(`INSERT INTO memberships(user_id, location_id, role, status) VALUES ($1,$2,'owner','active')`, [userId, linkedLoc]);
    await c.query(
      `INSERT INTO owner_notification_targets(location_id, channel, address, status, user_id)
       VALUES ($1,'telegram',$2,'active',$3)`,
      [linkedLoc, chatId, userId],
    );
  } finally {
    c.release();
  }

  // Recording wrappers (not silent no-ops) so realtime/queue side-effects are observable.
  const busPublishes: Array<{ channel: string; payload: any }> = [];
  const bossSends: Array<{ queue: string; payload: any }> = [];

  const app = Fastify();
  await app.register(webhookRoutes, {
    db: pool,
    queue: { boss: { send: async (q: string, payload: any) => { bossSends.push({ queue: q, payload }); return ''; } } },
    telegramBotSecret: 'sek',
    messageBus: { publish: async (channel: string, payload: any) => { busPublishes.push({ channel, payload }); } },
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

  await t.test('wrong webhook secret in URL is rejected (no route, no mutation)', async () => {
    // The bot secret IS the route path segment — a wrong secret matches no registered
    // route, so Fastify returns 404 and the handler (and every DB mutation) is never reached.
    await pool.query(`UPDATE locations SET delivery_paused=true WHERE id=$1`, [locationId]);
    const r = await app.inject({
      method: 'POST',
      url: '/webhook/telegram/WRONG-SECRET',
      payload: cb(`store.open:${locationId}`),
    });
    assert.equal(r.statusCode, 404);
    assert.equal(await paused(locationId), true, 'wrong-secret request must not toggle the storefront');
  });

  await t.test('BR-3: store.close on a foreign location is denied (no nonce, no mutation)', async () => {
    await pool.query(`UPDATE locations SET delivery_paused=false WHERE id=$1`, [otherLoc]);
    const r = await post(cb(`store.close:${otherLoc}`)); // chat has no target@otherLoc
    assert.equal(r.statusCode, 200); // handler answers gracefully
    assert.equal(await paused(otherLoc), false, 'foreign location must stay unchanged');
    const nonceCount = (await pool.query(
      `SELECT count(*)::int AS n FROM telegram_action_nonces WHERE location_id=$1 AND action='store.close'`,
      [otherLoc],
    )).rows[0].n;
    assert.equal(nonceCount, 0, 'no close nonce may be minted for a foreign location');
  });

  await t.test('BR-3: pref.set on a foreign location is denied (no target, no audit)', async () => {
    const r = await post(cb(`pref.set:${otherLoc}:operational:0`)); // chat has no target@otherLoc
    assert.equal(r.statusCode, 200); // handler answers gracefully
    const targetCount = (await pool.query(
      `SELECT count(*)::int AS n FROM owner_notification_targets WHERE location_id=$1 AND channel='telegram'`,
      [otherLoc],
    )).rows[0].n;
    assert.equal(targetCount, 0, 'pref.set must not create/mutate a target on a foreign location');
    const auditCount = (await pool.query(
      `SELECT count(*)::int AS n FROM notification_prefs_audit a
        JOIN owner_notification_targets t ON t.id = a.target_id
       WHERE t.location_id=$1`,
      [otherLoc],
    )).rows[0].n;
    assert.equal(auditCount, 0, 'no consent-audit row may be written for a foreign location');
  });

  await t.test('store.confirm nonce confusion: a nonce minted for locationId cannot close linkedLoc', async () => {
    // Mint a REAL close nonce for locationId (chat is linked there).
    await pool.query(`UPDATE locations SET delivery_paused=false WHERE id=$1`, [locationId]);
    await pool.query(`UPDATE locations SET delivery_paused=false WHERE id=$1`, [linkedLoc]);
    const r0 = await post(cb(`store.close:${locationId}`));
    assert.equal(r0.statusCode, 200);
    const nonce = (await pool.query(
      `SELECT nonce FROM telegram_action_nonces WHERE location_id=$1 AND action='store.close' ORDER BY created_at DESC LIMIT 1`,
      [locationId],
    )).rows[0]?.nonce;
    assert.match(String(nonce), /^[0-9a-f]{12}$/, 'a valid 12-hex close nonce was minted for locationId');
    // Replay it against linkedLoc — auth passes (chat IS linked to linkedLoc) so the ONLY
    // thing that can stop this is the service-layer nonce↔location binding.
    const r = await post(cb(`store.confirm:${linkedLoc}:${nonce}`));
    assert.equal(r.statusCode, 200);
    assert.equal(await paused(linkedLoc), false, 'cross-location nonce must NOT toggle linkedLoc');
    // The nonce belongs to locationId and must remain unconsumed by the cross-location replay.
    const stillThere = (await pool.query(
      `SELECT count(*)::int AS n FROM telegram_action_nonces WHERE nonce=$1 AND location_id=$2`,
      [nonce, locationId],
    )).rows[0].n;
    assert.equal(stillThere, 1, "locationId's nonce must survive a foreign-location confirm attempt");
  });

  await t.test('storefront/pref toggles emit NO realtime broadcast (current contract)', async () => {
    // Characterization: store.open/close/confirm + pref.set mutate the DB but do NOT call
    // messageBus.publish / queue.boss.send (unlike order.* and shift paths). Locking this
    // makes adding realtime a deliberate, test-reviewed change rather than a silent drift.
    // TODO(realtime): when storefront toggles should live-update dashboards, exercise the
    // order.confirm path (needs an orders fixture) and assert publish(channel, {locationId}).
    assert.equal(busPublishes.length, 0, 'no messageBus.publish on storefront/pref Telegram paths');
    assert.equal(bossSends.length, 0, 'no queue.boss.send on storefront/pref Telegram paths');
  });

  await app.close();
  await pool.end();
  globalThis.fetch = origFetch;
});
