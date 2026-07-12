import test from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import pg from 'pg';
import { setCategoryPref } from '../../src/lib/notificationPrefsService.js';

// Integration test for the category-pref writer (BR-4 atomic jsonb_set, BR-16 in-txn
// consent audit). Requires the migration chain applied:
//   TEST_DATABASE_URL=postgresql://… node --test --import tsx tests/notifications/prefs-service.test.ts
const url = process.env.TEST_DATABASE_URL;

test('notification prefs service (DB integration)', { skip: url ? false : 'set TEST_DATABASE_URL to run' }, async (t) => {
  const pool = new pg.Pool({ connectionString: url });
  const userId = '22222222-2222-2222-2222-222222222222';
  // Unique seed identity per run so the slug/phone/address unique indexes never collide
  // across re-runs (no teardown otherwise) — Test Integrity #7 (seed fixtures, assert exact).
  const sfx = randomUUID().slice(0, 8);
  const phone = '+355' + Math.floor(1e8 + Math.random() * 8e8); // valid-shaped, unique
  let locationId = '';
  let targetId = '';
  // Cross-tenant (IDOR) fixtures: a SECOND real location so targetId∈A can be probed with
  // locationId∈B (Test Integrity #5 — never an all-zero id).
  let locationB = '';
  // userId=null fixture: a separate target so the Telegram-bot (unauthenticated) toggle does
  // not perturb the "exactly 2 audit rows" count on the primary target.
  let targetNull = '';

  const seed = await pool.connect();
  try {
    const org = await seed.query(`INSERT INTO organizations(name) VALUES ('P Org') RETURNING id`);
    const orgId = org.rows[0].id;
    const loc = await seed.query(
      `INSERT INTO locations(org_id, slug, name, phone) VALUES ($1, $2, 'P Loc', $3) RETURNING id`,
      [orgId, `p-loc-${sfx}`, phone],
    );
    locationId = loc.rows[0].id;
    const locB = await seed.query(
      `INSERT INTO locations(org_id, slug, name, phone) VALUES ($1, $2, 'P Loc B', $3) RETURNING id`,
      [orgId, `p-loc-b-${sfx}`, phone + '1'],
    );
    locationB = locB.rows[0].id;
    // New target inherits the category default prefs (operational:true, quality:false).
    const tgt = await seed.query(
      `INSERT INTO owner_notification_targets(location_id, channel, address, user_id)
       VALUES ($1, 'telegram', $2, NULL) RETURNING id, prefs`,
      [locationId, `p-tg-${sfx}`],
    );
    targetId = tgt.rows[0].id;
    assert.equal(tgt.rows[0].prefs.operational, true);
    assert.equal(tgt.rows[0].prefs.quality, false);
    const tgtNull = await seed.query(
      `INSERT INTO owner_notification_targets(location_id, channel, address, user_id)
       VALUES ($1, 'telegram', $2, NULL) RETURNING id`,
      [locationId, `p-tg-null-${sfx}`],
    );
    targetNull = tgtNull.rows[0].id;
  } finally {
    seed.release();
  }

  await t.test('disable operational → atomic write + audit captures old=true,new=false', async () => {
    const c = await pool.connect();
    try {
      const r = await setCategoryPref(c, { targetId, locationId, userId, category: 'operational', value: false, changedVia: 'telegram' });
      assert.deepEqual(r, { ok: true, oldValue: true, newValue: false });
      const prefs = (await c.query(`SELECT prefs FROM owner_notification_targets WHERE id=$1`, [targetId])).rows[0].prefs;
      assert.equal(prefs.operational, false);
      assert.equal(prefs.quality, false, 'other category untouched (per-cell write)');
      const audit = (await c.query(
        `SELECT old_value, new_value, changed_via, user_id FROM notification_prefs_audit WHERE target_id=$1 AND category='operational'`,
        [targetId],
      )).rows[0];
      assert.deepEqual(
        { old: audit.old_value, neu: audit.new_value, via: audit.changed_via, uid: audit.user_id },
        { old: true, neu: false, via: 'telegram', uid: userId },
      );
    } finally { c.release(); }
  });

  await t.test('enable quality → old=false,new=true; operational unchanged', async () => {
    const c = await pool.connect();
    try {
      const r = await setCategoryPref(c, { targetId, locationId, userId, category: 'quality', value: true, changedVia: 'web' });
      assert.deepEqual(r, { ok: true, oldValue: false, newValue: true });
      const prefs = (await c.query(`SELECT prefs FROM owner_notification_targets WHERE id=$1`, [targetId])).rows[0].prefs;
      assert.equal(prefs.quality, true);
      assert.equal(prefs.operational, false, 'operational from prior test untouched');
    } finally { c.release(); }
  });

  await t.test('non-existent target → ok:false, no audit row', async () => {
    const nilTarget = '00000000-0000-0000-0000-000000000000';
    const c = await pool.connect();
    try {
      const r = await setCategoryPref(c, { targetId: nilTarget, locationId, userId, category: 'operational', value: true, changedVia: 'web' });
      assert.equal(r.ok, false);
      // The rollback path must leave NO audit row behind (BR-16 atomicity).
      const n = (await c.query(`SELECT count(*)::int AS n FROM notification_prefs_audit WHERE target_id=$1`, [nilTarget])).rows[0].n;
      assert.equal(n, 0, 'no consent-audit row for a missing target');
    } finally { c.release(); }
  });

  await t.test('cross-tenant targetId+foreign locationId → ok:false, no write, no audit (IDOR)', async () => {
    const c = await pool.connect();
    try {
      // Real target in location A probed with a real, DIFFERENT location B → row filter misses.
      const r = await setCategoryPref(c, { targetId, locationId: locationB, userId, category: 'operational', value: true, changedVia: 'web' });
      assert.equal(r.ok, false);
      // Target A's prefs must be untouched and no audit row attributed to location B.
      const prefs = (await c.query(`SELECT prefs FROM owner_notification_targets WHERE id=$1`, [targetId])).rows[0].prefs;
      assert.equal(prefs.operational, false, 'target A operational unchanged by cross-tenant write');
      const n = (await c.query(`SELECT count(*)::int AS n FROM notification_prefs_audit WHERE location_id=$1`, [locationB])).rows[0].n;
      assert.equal(n, 0, 'no audit row leaked to the foreign tenant');
    } finally { c.release(); }
  });

  await t.test('userId=null (Telegram bot toggle, unauthenticated) → ok:true, audit user_id IS NULL', async () => {
    const c = await pool.connect();
    try {
      const r = await setCategoryPref(c, { targetId: targetNull, locationId, userId: null, category: 'operational', value: false, changedVia: 'telegram' });
      assert.deepEqual(r, { ok: true, oldValue: true, newValue: false });
      const audit = (await c.query(
        `SELECT old_value, new_value, changed_via, user_id FROM notification_prefs_audit WHERE target_id=$1 AND category='operational'`,
        [targetNull],
      )).rows[0];
      assert.equal(audit.user_id, null, 'unauthenticated bot toggle records a NULL actor');
      assert.deepEqual(
        { old: audit.old_value, neu: audit.new_value, via: audit.changed_via },
        { old: true, neu: false, via: 'telegram' },
      );
    } finally { c.release(); }
  });

  await t.test('exactly 2 consent-audit rows written across the successful toggles', async () => {
    const c = await pool.connect();
    try {
      const n = (await c.query(`SELECT count(*)::int AS n FROM notification_prefs_audit WHERE target_id=$1`, [targetId])).rows[0].n;
      assert.equal(n, 2);
    } finally { c.release(); }
  });

  await pool.end();
});
