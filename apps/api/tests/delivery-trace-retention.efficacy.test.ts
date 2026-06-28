import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';

// deliver v2 R3-1 — OUTCOME-based retention efficacy (the council's mandatory proof; schedule-existence is NOT
// enough). delivery_trace is tenant-scoped FORCE, so the sweep MUST run through the SECURITY DEFINER fn. The
// R4-1 precondition: the operational caller MUST be NOBYPASSRLS — only then is the test discriminating (a
// context-free raw UPDATE sees 0 rows → RED; the DEFINER fn reaches them via its owner's privilege → GREEN).
// Harness (admin URL migrates + creates the `dtprov` NOBYPASSRLS role):
//   DV2_ADMIN_DATABASE_URL = postgres superuser, DV2_PROV_DATABASE_URL = the dtprov NOBYPASSRLS role.
const adminUrl = process.env.DV2_ADMIN_DATABASE_URL;
const provUrl = process.env.DV2_PROV_DATABASE_URL;
const maybe = adminUrl && provUrl ? test : test.skip;

let admin: Pool, prov: Pool;
before(() => { if (adminUrl) { admin = new Pool({ connectionString: adminUrl }); prov = new Pool({ connectionString: provUrl }); } });
after(async () => { if (admin) await admin.end(); if (prov) await prov.end(); });

// Seed an order + a stale delivery_trace row (delivered 30d ago) with GPS crumbs, for a fresh tenant.
async function seedStaleTrace(): Promise<{ orderId: string; locationId: string }> {
  const c = await admin.connect();
  try {
    const u = (await c.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['dt-' + crypto.randomBytes(4).toString('hex') + '@t.test'])).rows[0].id;
    const org = (await c.query(`INSERT INTO organizations (name, owner_id) VALUES ('DT',$1) RETURNING id`, [u])).rows[0].id;
    const loc = (await c.query(`INSERT INTO locations (org_id, slug, name, phone, status) VALUES ($1,$2,'DT','','open') RETURNING id`, [org, 'dt-' + crypto.randomBytes(4).toString('hex')])).rows[0].id;
    const ord = (await c.query(`INSERT INTO orders (location_id, subtotal, total, request_hash, status) VALUES ($1,500,500,$2,'DELIVERED') RETURNING id`, [loc, 'rh-' + crypto.randomBytes(8).toString('hex')])).rows[0].id;
    await c.query(
      `INSERT INTO delivery_trace (order_id, location_id, total, delivered_at, gps_lat, gps_lng, name_snapshot, price_snapshot)
       VALUES ($1,$2,500, now() - interval '30 days', 41.3, 19.8, '{"x":1}'::jsonb, 500)`,
      [ord, loc],
    );
    return { orderId: ord, locationId: loc };
  } finally { c.release(); }
}

maybe('DEFINER sweep anonymizes stale GPS crumbs across ≥2 tenants under a NOBYPASSRLS context-free caller', async () => {
  const a = await seedStaleTrace();
  const b = await seedStaleTrace(); // a second, distinct tenant

  // R4-1 discriminator: a context-free RAW UPDATE under the NOBYPASSRLS prov role sees 0 rows (FORCE + no
  // tenant context) — proving the round-2 "just UPDATE" approach would silently anonymize nothing.
  const raw = await prov.query(
    `UPDATE delivery_trace SET gps_lat = NULL WHERE delivered_at < now() - interval '14 days' AND gps_lat IS NOT NULL`,
  );
  assert.equal(raw.rowCount, 0, 'a context-free raw UPDATE under NOBYPASSRLS sees 0 rows (the round-2 false-green)');

  // The DEFINER fn, invoked by the SAME NOBYPASSRLS prov caller, reaches all-tenant rows via its OWNER's
  // privilege → anonymizes both tenants.
  const res = await prov.query(`SELECT anonymize_stale_delivery_trace($1::interval) AS n`, ['14 days']);
  assert.ok(res.rows[0].n >= 2, `DEFINER fn anonymized ≥2 cross-tenant rows (got ${res.rows[0].n})`);

  // Outcome assertion (admin read): zero stale rows retain GPS.
  const left = await admin.query(
    `SELECT count(*)::int AS n FROM delivery_trace WHERE delivered_at < now() - interval '14 days' AND gps_lat IS NOT NULL`,
  );
  assert.equal(left.rows[0].n, 0, 'no stale row retains GPS after the sweep');
  // And the non-PII facts survive (anonymize-not-delete).
  for (const s of [a, b]) {
    const row = (await admin.query(`SELECT gps_lat, name_snapshot, total, delivered_at FROM delivery_trace WHERE order_id=$1`, [s.orderId])).rows[0];
    assert.equal(row.gps_lat, null, 'gps nulled');
    assert.equal(row.name_snapshot, null, 'name_snapshot nulled');
    assert.equal(row.total, 500, 'non-PII total survives');
    assert.ok(row.delivered_at != null, 'delivered_at survives');
  }
});

maybe('the 7-day dispute-window floor (R4-2): a mis-set 1-day window never anonymizes inside the dispute window', async () => {
  const fresh = await (async () => {
    const c = await admin.connect();
    try {
      const u = (await c.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['dtf-' + crypto.randomBytes(4).toString('hex') + '@t.test'])).rows[0].id;
      const org = (await c.query(`INSERT INTO organizations (name, owner_id) VALUES ('DTF',$1) RETURNING id`, [u])).rows[0].id;
      const loc = (await c.query(`INSERT INTO locations (org_id, slug, name, phone, status) VALUES ($1,$2,'DTF','','open') RETURNING id`, [org, 'dtf-' + crypto.randomBytes(4).toString('hex')])).rows[0].id;
      const ord = (await c.query(`INSERT INTO orders (location_id, subtotal, total, request_hash, status) VALUES ($1,500,500,$2,'DELIVERED') RETURNING id`, [loc, 'rh-' + crypto.randomBytes(8).toString('hex')])).rows[0].id;
      // delivered 3 days ago — INSIDE the 7-day dispute window.
      await c.query(`INSERT INTO delivery_trace (order_id, location_id, total, delivered_at, gps_lat, gps_lng) VALUES ($1,$2,500, now() - interval '3 days', 41.3, 19.8)`, [ord, loc]);
      return ord;
    } finally { c.release(); }
  })();
  // Caller passes a too-small window; the fn clamps to 7 days → the 3-day-old row is NOT anonymized.
  await prov.query(`SELECT anonymize_stale_delivery_trace($1::interval) AS n`, ['1 day']);
  const row = (await admin.query(`SELECT gps_lat FROM delivery_trace WHERE order_id=$1`, [fresh])).rows[0];
  assert.ok(row.gps_lat != null, 'evidence inside the dispute window is protected by the floor');
});
