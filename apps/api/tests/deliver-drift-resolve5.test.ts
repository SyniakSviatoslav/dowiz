import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { releaseBindingAndReoffer } from '../src/lib/bindingRelease.js';
import { completeDelivery } from '../src/lib/deliveryCompletion.js';
import { updateOrderStatus } from '../src/lib/orderStatusService.js';

// deliver v2 — RESOLVE round 5 drift fixes (D1 /cancel-converge, D2 machine-path revert, D5 M-3a authority).
// Runs vs the REAL schema (DV2_TEST_DATABASE_URL = throwaway migrated to head). Skips cleanly when unset.
const url = process.env.DV2_TEST_DATABASE_URL;
const maybe = url ? test : test.skip;

function capturingBus() {
  const calls: Array<{ ch: any; msg: any }> = [];
  return { calls, bus: { publish: async (ch: any, msg: any) => { calls.push({ ch, msg }); } } as any };
}

let pool: Pool;
before(() => { if (url) pool = new Pool({ connectionString: url }); });
after(async () => { if (pool) await pool.end(); });

async function seed(opts: { orderStatus?: string; asgStatus?: string; total?: number } = {}) {
  const orderStatus = opts.orderStatus ?? 'IN_DELIVERY';
  const asgStatus = opts.asgStatus ?? 'picked_up';
  const total = opts.total ?? 850;
  const c = await pool.connect();
  try {
    const u = (await c.query(`INSERT INTO users (email) VALUES ($1) RETURNING id`, ['c-' + crypto.randomBytes(4).toString('hex') + '@t.test'])).rows[0].id;
    const org = (await c.query(`INSERT INTO organizations (name, owner_id) VALUES ('DV2',$1) RETURNING id`, [u])).rows[0].id;
    const loc = (await c.query(`INSERT INTO locations (org_id, slug, name, phone, status) VALUES ($1,$2,'DV2','','open') RETURNING id`, [org, 'dv2-' + crypto.randomBytes(4).toString('hex')])).rows[0].id;
    const ord = (await c.query(
      `INSERT INTO orders (location_id, subtotal, total, request_hash, status) VALUES ($1,$2,$2,$3,$4) RETURNING id`,
      [loc, total, 'rh-' + crypto.randomBytes(8).toString('hex'), orderStatus],
    )).rows[0].id;
    const cour = (await c.query(
      `INSERT INTO couriers (email_encrypted, email_hash, full_name_encrypted, password_hash)
       VALUES ('\\x00'::bytea, $1, '\\x00'::bytea, 'x') RETURNING id`,
      ['h-' + crypto.randomBytes(6).toString('hex')],
    )).rows[0].id;
    const shift = (await c.query(`INSERT INTO courier_shifts (courier_id, location_id, status) VALUES ($1,$2,'on_delivery') RETURNING id`, [cour, loc])).rows[0].id;
    const asg = (await c.query(
      `INSERT INTO courier_assignments (order_id, location_id, courier_id, shift_id, status) VALUES ($1,$2,$3,$4,$5) RETURNING id`,
      [ord, loc, cour, shift, asgStatus],
    )).rows[0].id;
    // owner-direct mirror: an IN_DELIVERY order points at the courier (the C-2 setup).
    if (orderStatus === 'IN_DELIVERY') await c.query(`UPDATE orders SET courier_id=$1 WHERE id=$2`, [cour, ord]);
    return { orderId: ord, assignmentId: asg, shiftId: shift, courierId: cour, locationId: loc, total };
  } finally { c.release(); }
}

// ── D1 (C-2 / R2-2 / R2-5): the cancel/abort exit rail. ──────────────────────────────────────────────────

maybe('D1a · IN_DELIVERY + picked_up → CANCELLED (honest terminal, food is out) + binding freed', async () => {
  const s = await seed({ asgStatus: 'picked_up' });
  const { bus } = capturingBus();
  const c = await pool.connect();
  let reoffered: boolean;
  try {
    await c.query('BEGIN');
    ({ reoffered } = await releaseBindingAndReoffer(c, { assignmentId: s.assignmentId, orderId: s.orderId, shiftId: s.shiftId, asgStatus: 'picked_up', ordStatus: 'IN_DELIVERY', locationId: s.locationId, reason: 'courier_cancelled: x' }, { messageBus: bus }));
    await c.query('COMMIT');
  } finally { c.release(); }
  assert.equal(reoffered!, false);
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'CANCELLED');
  assert.equal((await pool.query(`SELECT status FROM courier_assignments WHERE id=$1`, [s.assignmentId])).rows[0].status, 'cancelled');
});

maybe('D1b · IN_DELIVERY + accepted → READY (NOT stuck IN_DELIVERY, NOT a false CANCELLED) + re-offered + mirror cleared', async () => {
  const s = await seed({ asgStatus: 'accepted' });
  const { bus, calls } = capturingBus();
  const c = await pool.connect();
  let reoffered: boolean;
  try {
    await c.query('BEGIN');
    ({ reoffered } = await releaseBindingAndReoffer(c, { assignmentId: s.assignmentId, orderId: s.orderId, shiftId: s.shiftId, asgStatus: 'accepted', ordStatus: 'IN_DELIVERY', locationId: s.locationId, reason: 'courier_cancelled: x' }, { messageBus: bus }));
    await c.query('COMMIT');
  } finally { c.release(); }
  // THE C-2 TRAP FIX: the order is back to assignable, not stranded IN_DELIVERY and not lyingly CANCELLED.
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'READY');
  assert.equal((await pool.query(`SELECT courier_id FROM orders WHERE id=$1`, [s.orderId])).rows[0].courier_id, null, 'mirror cleared');
  assert.equal(reoffered!, true);
  assert.equal((await pool.query(`SELECT count(*)::int n FROM courier_dispatch_queue WHERE order_id=$1`, [s.orderId])).rows[0].n, 1, 're-enqueued');
  // R2-5: NO false ORDER_CANCELLED for an order that actually reverted to READY.
  const sawCancelled = calls.some(k => JSON.stringify(k).includes('CANCELLED'));
  assert.equal(sawCancelled, false, 'no false ORDER_CANCELLED emitted on a READY revert');
});

maybe('D1c · flag-ON accept (order CONFIRMED, never advanced) → NO throw, status unchanged, re-offered', async () => {
  const s = await seed({ orderStatus: 'CONFIRMED', asgStatus: 'accepted' });
  const { bus } = capturingBus();
  const c = await pool.connect();
  let reoffered: boolean;
  try {
    await c.query('BEGIN');
    // R3-2: must NOT force a transition from CONFIRMED (would throw IllegalTransition and roll back the abort).
    ({ reoffered } = await releaseBindingAndReoffer(c, { assignmentId: s.assignmentId, orderId: s.orderId, shiftId: s.shiftId, asgStatus: 'accepted', ordStatus: 'CONFIRMED', locationId: s.locationId, reason: 'courier_cancelled: x' }, { messageBus: bus }));
    await c.query('COMMIT');
  } finally { c.release(); }
  assert.equal((await pool.query(`SELECT status FROM orders WHERE id=$1`, [s.orderId])).rows[0].status, 'CONFIRMED', 'pre-pickup order untouched');
  assert.equal((await pool.query(`SELECT status FROM courier_assignments WHERE id=$1`, [s.assignmentId])).rows[0].status, 'cancelled', 'binding always freed');
  assert.equal(reoffered!, true);
});

// ── D2 (R2-6): IN_DELIVERY→READY through the machine writes order_status_history (the raw UPDATE did not). ──

maybe('D2 · updateOrderStatus(IN_DELIVERY→READY) writes an order_status_history row (machine path, not a raw UPDATE)', async () => {
  const s = await seed({ asgStatus: 'picked_up' });
  const { bus } = capturingBus();
  const c = await pool.connect();
  try {
    await c.query('BEGIN');
    await updateOrderStatus(c, s.orderId, s.locationId, 'READY', { messageBus: bus, comment: 'owner_reassigned' });
    await c.query('COMMIT');
  } finally { c.release(); }
  const h = await pool.query(`SELECT to_status, comment FROM order_status_history WHERE order_id=$1 AND to_status='READY'`, [s.orderId]);
  assert.equal(h.rowCount, 1, 'history row written by the machine path (a raw UPDATE orders SET status would skip this)');
  assert.equal(h.rows[0].comment, 'owner_reassigned');
});

// ── D5 (M-3a — THE AUTHORITY): the completion outcome is a pure function of the courier tap + server total,
//    INDEPENDENT of any passive signal row (delivery_trace / order_sensor_events / customer_signals). ────────

maybe('D5 · M-3a — a contradictory pre-existing delivery_trace signal does NOT change the completion outcome', async () => {
  // Control: a clean order, paid_full → DELIVERED + hold.
  const a = await seed({ total: 700 });
  const { bus } = capturingBus();
  let ca = await pool.connect();
  try { await ca.query('BEGIN'); await completeDelivery(ca, { ...a, paymentOutcome: 'paid_full', cashAmount: 700 }, { messageBus: bus }); await ca.query('COMMIT'); } finally { ca.release(); }

  // Subject: identical order, but seed a CONTRADICTORY passive signal BEFORE completion — a delivery_trace row
  // claiming 'refused_goods' with garbage GPS. If completion read the signal to decide, the outcome would differ.
  const b = await seed({ total: 700 });
  await pool.query(
    `INSERT INTO delivery_trace (order_id, location_id, courier_id, total, delivered_at, payment_outcome, gps_lat, gps_lng)
     VALUES ($1,$2,$3,$4, now(), 'refused_goods', 999, 999)`,
    [b.orderId, b.locationId, b.courierId, b.total],
  );
  let cb = await pool.connect();
  try { await cb.query('BEGIN'); await completeDelivery(cb, { ...b, paymentOutcome: 'paid_full', cashAmount: 700 }, { messageBus: bus }); await cb.query('COMMIT'); } finally { cb.release(); }

  // Outcome follows the TAP, not the signal: both reach DELIVERED + hold=total; B's order outcome ignores the
  // pre-existing 'refused_goods' crumb entirely.
  const oa = (await pool.query(`SELECT status, payment_outcome FROM orders WHERE id=$1`, [a.orderId])).rows[0];
  const ob = (await pool.query(`SELECT status, payment_outcome FROM orders WHERE id=$1`, [b.orderId])).rows[0];
  assert.deepEqual({ s: ob.status, p: ob.payment_outcome }, { s: oa.status, p: oa.payment_outcome }, 'identical outcome despite the contradictory signal');
  assert.equal(ob.status, 'DELIVERED');
  assert.equal(ob.payment_outcome, 'paid_full', 'tap wins over the stale refused_goods signal');
  assert.equal((await pool.query(`SELECT amount FROM courier_cash_ledger WHERE order_id=$1 AND type='hold'`, [b.orderId])).rows[0].amount, 700);
});
