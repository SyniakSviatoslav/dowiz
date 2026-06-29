import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { attemptHonestDispatch } from '../src/lib/dispatch.js';

// flow-simplification §5 / R2-1 — HONEST DISPATCH (no-trap red-line F1), proven deterministically vs the real
// head schema (DV2_TEST_DATABASE_URL). No courier → the order is NEVER advanced to IN_DELIVERY (no orphan).
const url = process.env.DV2_TEST_DATABASE_URL;
const maybe = url ? test : test.skip;
const bus = { publish: async () => {} } as any;

let pool: Pool;
before(() => { if (url) pool = new Pool({ connectionString: url }); });
after(async () => { if (pool) await pool.end(); });

async function seedLocationAndConfirmedOrder() {
  const c = await pool.connect();
  try {
    const u = (await c.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['o-' + crypto.randomBytes(4).toString('hex') + '@t.test'])).rows[0].id;
    const org = (await c.query(`INSERT INTO organizations (name, owner_id) VALUES ('FS',$1) RETURNING id`, [u])).rows[0].id;
    const loc = (await c.query(`INSERT INTO locations (org_id, slug, name, phone, status) VALUES ($1,$2,'FS','','open') RETURNING id`, [org, 'fs-' + crypto.randomBytes(4).toString('hex')])).rows[0].id;
    const ord = (await c.query(
      `INSERT INTO orders (location_id, subtotal, total, request_hash, status, type) VALUES ($1,$2,$2,$3,'CONFIRMED','delivery') RETURNING id`,
      [loc, 700, 'rh-' + crypto.randomBytes(8).toString('hex')],
    )).rows[0].id;
    return { locationId: loc, orderId: ord };
  } finally { c.release(); }
}

async function seedAvailableCourier(locationId: string) {
  const c = await pool.connect();
  try {
    const cour = (await c.query(
      `INSERT INTO couriers (email_encrypted, email_hash, full_name_encrypted, password_hash, status)
       VALUES ('\\x00'::bytea, $1, '\\x00'::bytea, 'x', 'active') RETURNING id`,
      ['h-' + crypto.randomBytes(6).toString('hex')],
    )).rows[0].id;
    await c.query(`INSERT INTO courier_locations (courier_id, location_id, role) VALUES ($1,$2,'courier')`, [cour, locationId]);
    const shift = (await c.query(`INSERT INTO courier_shifts (courier_id, location_id, status) VALUES ($1,$2,'available') RETURNING id`, [cour, locationId])).rows[0].id;
    return { courierId: cour, shiftId: shift };
  } finally { c.release(); }
}

maybe('no courier available → NOT advanced (no orphan): dispatched:false, reason:no_courier, order stays CONFIRMED', async () => {
  const s = await seedLocationAndConfirmedOrder(); // no courier seeded
  const c = await pool.connect();
  let out: any;
  try {
    await c.query('BEGIN');
    out = await attemptHonestDispatch(c, { orderId: s.orderId, locationId: s.locationId, currentStatus: 'CONFIRMED' }, { messageBus: bus });
    await c.query('COMMIT');
  } finally { c.release(); }
  assert.equal(out.dispatched, false);
  assert.equal(out.reason, 'no_courier');
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'CONFIRMED', 'never orphaned IN_DELIVERY');
  assert.equal((await pool.query(`SELECT count(*)::int n FROM courier_assignments WHERE order_id=$1`, [s.orderId])).rows[0].n, 0, 'no binding created');
});

maybe('available courier → advanced atomically: dispatched:true, IN_DELIVERY + assignment + shift on_delivery', async () => {
  const s = await seedLocationAndConfirmedOrder();
  const cr = await seedAvailableCourier(s.locationId);
  const c = await pool.connect();
  let out: any;
  try {
    await c.query('BEGIN');
    out = await attemptHonestDispatch(c, { orderId: s.orderId, locationId: s.locationId, currentStatus: 'CONFIRMED' }, { messageBus: bus });
    await c.query('COMMIT');
  } finally { c.release(); }
  assert.equal(out.dispatched, true);
  assert.equal(out.status, 'IN_DELIVERY');
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'IN_DELIVERY');
  const a = await pool.query(`SELECT status, courier_id FROM courier_assignments WHERE order_id=$1`, [s.orderId]);
  assert.equal(a.rowCount, 1);
  assert.equal(a.rows[0].status, 'assigned');
  assert.equal(a.rows[0].courier_id, cr.courierId);
  assert.equal((await pool.query(`SELECT status FROM courier_shifts WHERE id=$1`, [cr.shiftId])).rows[0].status, 'on_delivery');
});

maybe('order already bound (offered) → already_assigned, no double-bind, no advance', async () => {
  const s = await seedLocationAndConfirmedOrder();
  const cr = await seedAvailableCourier(s.locationId);
  // pre-existing active binding (e.g. an owner offer-handshake 'offered' row)
  await pool.query(
    `INSERT INTO courier_assignments (order_id, location_id, courier_id, shift_id, status) VALUES ($1,$2,$3,$4,'offered')`,
    [s.orderId, s.locationId, cr.courierId, cr.shiftId],
  );
  const c = await pool.connect();
  let out: any;
  try {
    await c.query('BEGIN');
    out = await attemptHonestDispatch(c, { orderId: s.orderId, locationId: s.locationId, currentStatus: 'CONFIRMED' }, { messageBus: bus });
    await c.query('COMMIT');
  } finally { c.release(); }
  assert.equal(out.dispatched, false);
  assert.equal(out.reason, 'already_assigned');
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'CONFIRMED');
  assert.equal((await pool.query(`SELECT count(*)::int n FROM courier_assignments WHERE order_id=$1`, [s.orderId])).rows[0].n, 1, 'no second binding');
});
