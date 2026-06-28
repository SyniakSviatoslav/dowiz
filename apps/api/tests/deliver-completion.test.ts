import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { completeDelivery, CompletionError } from '../src/lib/deliveryCompletion.js';
import { updateOrderStatus } from '../src/lib/orderStatusService.js';

// deliver v2 L2 — completeDelivery + the updateOrderStatus terminalize-fold, vs the REAL full schema
// (DV2_TEST_DATABASE_URL = a throwaway migrated to head 073). Skips cleanly when unset.
const url = process.env.DV2_TEST_DATABASE_URL;
const maybe = url ? test : test.skip;
const bus = { publish: async () => {} } as any;

let pool: Pool;
before(() => { if (url) pool = new Pool({ connectionString: url }); });
after(async () => { if (pool) await pool.end(); });

// Seed a location + a courier on shift + an order at IN_DELIVERY + a picked_up assignment. Returns ids.
async function seed(total = 850): Promise<{ orderId: string; assignmentId: string; shiftId: string; courierId: string; locationId: string }> {
  const c = await pool.connect();
  try {
    const u = (await c.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['c-' + crypto.randomBytes(4).toString('hex') + '@t.test'])).rows[0].id;
    const org = (await c.query(`INSERT INTO organizations (name, owner_id) VALUES ('DV2',$1) RETURNING id`, [u])).rows[0].id;
    const loc = (await c.query(`INSERT INTO locations (org_id, slug, name, phone, status) VALUES ($1,$2,'DV2','','open') RETURNING id`, [org, 'dv2-' + crypto.randomBytes(4).toString('hex')])).rows[0].id;
    const ord = (await c.query(
      `INSERT INTO orders (location_id, subtotal, total, request_hash, status) VALUES ($1,$2,$2,$3,'IN_DELIVERY') RETURNING id`,
      [loc, total, 'rh-' + crypto.randomBytes(8).toString('hex')],
    )).rows[0].id;
    const cour = (await c.query(
      `INSERT INTO couriers (email_encrypted, email_hash, full_name_encrypted, password_hash)
       VALUES ('\\x00'::bytea, $1, '\\x00'::bytea, 'x') RETURNING id`,
      ['h-' + crypto.randomBytes(6).toString('hex')],
    )).rows[0].id;
    const shift = (await c.query(`INSERT INTO courier_shifts (courier_id, location_id, status) VALUES ($1,$2,'on_delivery') RETURNING id`, [cour, loc])).rows[0].id;
    const asg = (await c.query(
      `INSERT INTO courier_assignments (order_id, location_id, courier_id, shift_id, status) VALUES ($1,$2,$3,$4,'picked_up') RETURNING id`,
      [ord, loc, cour, shift],
    )).rows[0].id;
    return { orderId: ord, assignmentId: asg, shiftId: shift, courierId: cour, locationId: loc };
  } finally {
    c.release();
  }
}

maybe('paid_full → assignment delivered + order DELIVERED + ledger hold + trace crumb + orders.payment_outcome', async () => {
  const s = await seed(850);
  const c = await pool.connect();
  try {
    await c.query('BEGIN');
    const r = await completeDelivery(c, { ...s, total: 850, paymentOutcome: 'paid_full', cashAmount: 850, gpsLat: 41.3, gpsLng: 19.8 }, { messageBus: bus });
    await c.query('COMMIT');
    assert.equal(r.orderStatus, 'DELIVERED');
  } finally { c.release(); }
  assert.equal((await pool.query(`SELECT status FROM courier_assignments WHERE id=$1`, [s.assignmentId])).rows[0].status, 'delivered');
  assert.equal((await pool.query(`SELECT status, payment_outcome FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'DELIVERED');
  assert.equal((await pool.query(`SELECT payment_outcome FROM orders WHERE id=$1`, [s.orderId])).rows[0].payment_outcome, 'paid_full');
  const hold = await pool.query(`SELECT amount FROM courier_cash_ledger WHERE order_id=$1 AND type='hold'`, [s.orderId]);
  assert.equal(hold.rowCount, 1, 'cash-as-proof hold written');
  assert.equal(hold.rows[0].amount, 850);
  const tr = await pool.query(`SELECT payment_outcome, cash_amount, gps_lat FROM delivery_trace WHERE order_id=$1`, [s.orderId]);
  assert.equal(tr.rows[0].payment_outcome, 'paid_full');
  assert.equal(tr.rows[0].cash_amount, 850);
  assert.ok(tr.rows[0].gps_lat != null, 'gps crumb recorded passively');
  assert.equal((await pool.query(`SELECT status FROM courier_shifts WHERE id=$1`, [s.shiftId])).rows[0].status, 'available', 'shift freed');
});

maybe('no-cash tail (refused_goods) → assignment cancelled + order CANCELLED + NO hold + trace', async () => {
  const s = await seed(600);
  const c = await pool.connect();
  try {
    await c.query('BEGIN');
    const r = await completeDelivery(c, { ...s, total: 600, paymentOutcome: 'refused_goods' }, { messageBus: bus });
    await c.query('COMMIT');
    assert.equal(r.orderStatus, 'CANCELLED');
  } finally { c.release(); }
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'CANCELLED', 'customer never sees Delivered for refused food');
  assert.equal((await pool.query(`SELECT status, cancellation_reason FROM courier_assignments WHERE id=$1`, [s.assignmentId])).rows[0].cancellation_reason, 'refused_goods');
  assert.equal((await pool.query(`SELECT count(*)::int n FROM courier_cash_ledger WHERE order_id=$1`, [s.orderId])).rows[0].n, 0, 'no hold for refused');
  assert.equal((await pool.query(`SELECT payment_outcome FROM delivery_trace WHERE order_id=$1`, [s.orderId])).rows[0].payment_outcome, 'refused_goods', 'distinguishing crumb recorded');
});

maybe('paid_full with cash != total → CASH_AMOUNT_MISMATCH (422), no mutation', async () => {
  const s = await seed(700);
  const c = await pool.connect();
  try {
    await c.query('BEGIN');
    await assert.rejects(
      () => completeDelivery(c, { ...s, total: 700, paymentOutcome: 'paid_full', cashAmount: 500 }, { messageBus: bus }),
      (e: unknown) => e instanceof CompletionError && (e as CompletionError).code === 'CASH_AMOUNT_MISMATCH',
    );
    await c.query('ROLLBACK');
  } finally { c.release(); }
  assert.equal((await pool.query(`SELECT status FROM courier_assignments WHERE id=$1`, [s.assignmentId])).rows[0].status, 'picked_up', 'no partial completion');
});

maybe('R2-3 fold: updateOrderStatus(IN_DELIVERY→CANCELLED) terminalizes the active assignment (no strand)', async () => {
  const s = await seed(500); // order IN_DELIVERY + a picked_up assignment, NOT pre-terminalized
  const c = await pool.connect();
  try {
    await c.query('BEGIN');
    await updateOrderStatus(c, s.orderId, s.locationId, 'CANCELLED', { messageBus: bus, comment: 'owner_no_show' });
    await c.query('COMMIT');
  } finally { c.release(); }
  const a = await pool.query(`SELECT status FROM courier_assignments WHERE id=$1`, [s.assignmentId]);
  assert.equal(a.rows[0].status, 'cancelled', 'active assignment terminalized by the central fold — no dangling row');
  assert.equal((await pool.query(`SELECT status FROM courier_shifts WHERE id=$1`, [s.shiftId])).rows[0].status, 'available', 'shift freed by the fold');
});
