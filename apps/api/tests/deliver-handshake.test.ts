import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { updateOrderStatus } from '../src/lib/orderStatusService.js';
import { CourierOfferSweepWorker } from '../src/workers/courier-offer-sweep.js';

// deliver v2 §A — the offer handshake state machine + the 🔴 no-trap red-line (decline/expiry leave the
// CUSTOMER order UNTOUCHED — only the binding rolls back). vs the REAL full schema (DV2_TEST_DATABASE_URL).
const url = process.env.DV2_TEST_DATABASE_URL;
const maybe = url ? test : test.skip;
const bus = { publish: async () => {} } as any;

let pool: Pool;
before(() => { if (url) pool = new Pool({ connectionString: url }); });
after(async () => { if (pool) await pool.end(); });

// Seed an order at READY + an 'offered' assignment (shift available). expiresInMin<0 → already overdue.
async function seedOffered(expiresInMin = 5): Promise<{ orderId: string; assignmentId: string; shiftId: string; courierId: string; locationId: string }> {
  const c = await pool.connect();
  try {
    const u = (await c.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['h-' + crypto.randomBytes(4).toString('hex') + '@t.test'])).rows[0].id;
    const org = (await c.query(`INSERT INTO organizations (name, owner_id) VALUES ('H',$1) RETURNING id`, [u])).rows[0].id;
    const loc = (await c.query(`INSERT INTO locations (org_id, slug, name, phone, status) VALUES ($1,$2,'H','','open') RETURNING id`, [org, 'h-' + crypto.randomBytes(4).toString('hex')])).rows[0].id;
    const ord = (await c.query(`INSERT INTO orders (location_id, subtotal, total, request_hash, status) VALUES ($1,500,500,$2,'READY') RETURNING id`, [loc, 'rh-' + crypto.randomBytes(8).toString('hex')])).rows[0].id;
    const cour = (await c.query(`INSERT INTO couriers (email_encrypted, email_hash, full_name_encrypted, password_hash) VALUES ('\\x00'::bytea,$1,'\\x00'::bytea,'x') RETURNING id`, ['h-' + crypto.randomBytes(6).toString('hex')])).rows[0].id;
    const shift = (await c.query(`INSERT INTO courier_shifts (courier_id, location_id, status) VALUES ($1,$2,'available') RETURNING id`, [cour, loc])).rows[0].id;
    const asg = (await c.query(
      `INSERT INTO courier_assignments (order_id, location_id, courier_id, shift_id, status, offered_at, offered_expires_at)
       VALUES ($1,$2,$3,$4,'offered', now(), now() + ($5 || ' minutes')::interval) RETURNING id`,
      [ord, loc, cour, shift, String(expiresInMin)],
    )).rows[0].id;
    return { orderId: ord, assignmentId: asg, shiftId: shift, courierId: cour, locationId: loc };
  } finally { c.release(); }
}

maybe('accept: offered → accepted advances the order to IN_DELIVERY', async () => {
  const s = await seedOffered();
  const c = await pool.connect();
  try {
    await c.query('BEGIN');
    await c.query(`UPDATE courier_assignments SET status='accepted', offered_expires_at=NULL WHERE id=$1`, [s.assignmentId]);
    await c.query(`UPDATE courier_shifts SET status='on_delivery' WHERE id=$1`, [s.shiftId]);
    await updateOrderStatus(c, s.orderId, s.locationId, 'IN_DELIVERY', { messageBus: bus });
    await c.query(`UPDATE orders SET courier_id=$1 WHERE id=$2`, [s.courierId, s.orderId]);
    await c.query('COMMIT');
  } finally { c.release(); }
  assert.equal((await pool.query(`SELECT status FROM courier_assignments WHERE id=$1`, [s.assignmentId])).rows[0].status, 'accepted');
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'IN_DELIVERY');
});

maybe('decline: offered → offered_expired, 🔴 the customer order is UNTOUCHED + re-enqueued', async () => {
  const s = await seedOffered();
  const c = await pool.connect();
  try {
    await c.query('BEGIN');
    await c.query(`UPDATE courier_assignments SET status='offered_expired', cancelled_at=now(), cancellation_reason='courier_declined' WHERE id=$1`, [s.assignmentId]);
    await c.query(`INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at) VALUES ($1,$2,now()) ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1`, [s.orderId, s.locationId]);
    await c.query('COMMIT');
  } finally { c.release(); }
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'READY', 'customer order UNTOUCHED by a decline');
  assert.equal((await pool.query(`SELECT status FROM courier_assignments WHERE id=$1`, [s.assignmentId])).rows[0].status, 'offered_expired');
  assert.equal((await pool.query(`SELECT count(*)::int n FROM courier_dispatch_queue WHERE order_id=$1`, [s.orderId])).rows[0].n, 1, 're-enqueued for another courier');
});

maybe('sweep: an expired offer flips to offered_expired + re-enqueues; 🔴 order UNTOUCHED (the durable timer)', async () => {
  const s = await seedOffered(-5); // already overdue
  const worker = new CourierOfferSweepWorker(pool, {} as any, bus);
  await (worker as any).run();
  assert.equal((await pool.query(`SELECT status FROM courier_assignments WHERE id=$1`, [s.assignmentId])).rows[0].status, 'offered_expired', 'expired offer swept');
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'READY', 'customer order UNTOUCHED by the sweep');
  assert.equal((await pool.query(`SELECT count(*)::int n FROM courier_dispatch_queue WHERE order_id=$1`, [s.orderId])).rows[0].n, 1, 're-enqueued');
});

maybe('the partial-unique allows a re-offer after a terminal row (C-1: terminal never blocks)', async () => {
  const s = await seedOffered(-5);
  await (worker_run(s)); // sweep → offered_expired (terminal)
  // a NEW offered row for the same order must now insert without colliding on the partial-unique.
  const c = await pool.connect();
  try {
    const ins = await c.query(
      `INSERT INTO courier_assignments (order_id, location_id, courier_id, shift_id, status, offered_at, offered_expires_at)
       VALUES ($1,$2,$3,$4,'offered', now(), now() + interval '5 minutes') RETURNING id`,
      [s.orderId, s.locationId, s.courierId, s.shiftId],
    );
    assert.ok(ins.rows[0].id, 're-offer inserts (terminal offered_expired row does not block — C-1 fix)');
  } finally { c.release(); }
});

async function worker_run(_s: any) {
  const worker = new CourierOfferSweepWorker(pool, {} as any, bus);
  await (worker as any).run();
}
