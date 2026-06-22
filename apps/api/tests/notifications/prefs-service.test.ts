import test from 'node:test';
import assert from 'node:assert/strict';
import pg from 'pg';
import { setCategoryPref } from '../../src/lib/notificationPrefsService.js';

// Integration test for the category-pref writer (BR-4 atomic jsonb_set, BR-16 in-txn
// consent audit). Requires the migration chain applied:
//   TEST_DATABASE_URL=postgresql://… node --test --import tsx tests/notifications/prefs-service.test.ts
const url = process.env.TEST_DATABASE_URL;

test('notification prefs service (DB integration)', { skip: url ? false : 'set TEST_DATABASE_URL to run' }, async (t) => {
  const pool = new pg.Pool({ connectionString: url });
  const userId = '22222222-2222-2222-2222-222222222222';
  let locationId = '';
  let targetId = '';

  const seed = await pool.connect();
  try {
    const org = await seed.query(`INSERT INTO organizations(name) VALUES ('P Org') RETURNING id`);
    const loc = await seed.query(
      `INSERT INTO locations(org_id, slug, name, phone) VALUES ($1, 'p-loc', 'P Loc', '+355601') RETURNING id`,
      [org.rows[0].id],
    );
    locationId = loc.rows[0].id;
    // New target inherits the category default prefs (operational:true, quality:false).
    const tgt = await seed.query(
      `INSERT INTO owner_notification_targets(location_id, channel, address, user_id)
       VALUES ($1, 'telegram', 'p-tg', NULL) RETURNING id, prefs`,
      [locationId],
    );
    targetId = tgt.rows[0].id;
    assert.equal(tgt.rows[0].prefs.operational, true);
    assert.equal(tgt.rows[0].prefs.quality, false);
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
    const c = await pool.connect();
    try {
      const r = await setCategoryPref(c, { targetId: '00000000-0000-0000-0000-000000000000', locationId, userId, category: 'operational', value: true, changedVia: 'web' });
      assert.equal(r.ok, false);
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
