import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { Pool } from 'pg';
import {
  createIsolatedPool, resetSchema, applyMoneyFix, seedLocation, seedOrder, seedPaidPayment, refundDueCount,
} from './money-spine-fixture.js';

// LC6 four-layer refund-obligation spine proofs (ADR-audit-fix-money §3, migration drafts
// 1790000000086 M-1 trigger + 1790000000087 M-3 reconciler + the L-A fold in orderStatusService):
//   P14 raw-UPDATE writer → trigger records; P6 sweep (byte-identical fn) cancels AND records;
//   P6b poison-row liveness (insert failure cannot wedge the fleet-wide sweep); P15 reconciler
//   backstop + persistent-failure surfacing; P16 mismatch surfaced-never-obligated; P4 funnel
//   fold via the REAL updateOrderStatus (L-A + L-C in one tx → exactly one row); N5 NULL
//   provider_payment_id cannot triple-insert; N6 GUC read-back semantics pinned.
// All expectations are DB state counts — never a mirror of the implementation.
// RED on pre-fix code: no trigger/reconciler exist → P14/P6/P15 count 0 where 1 is pinned.
// Skips without MONEY_TEST_DATABASE_URL (throwaway DB — see money-spine-fixture.ts header).

const url = process.env.MONEY_TEST_DATABASE_URL;
const maybe = url ? test : test.skip;

let pool: Pool;
let dbUrl: string;
let teardown: (() => Promise<void>) | undefined;
before(async () => {
  if (!url) return;
  // updateOrderStatus transitively loads config-consuming modules — stub the env like orders-guards.
  const d: Record<string, string> = {
    NODE_ENV: 'test', APP_BASE_URL: 'http://localhost:3000',
    DATABASE_URL_OPERATIONAL: url, DATABASE_URL_SESSION: url, DATABASE_URL_MIGRATIONS: url,
    REDIS_URL: 'redis://localhost:6379', JWT_PRIVATE_KEY: 'test-priv', JWT_PUBLIC_KEY: 'test-pub',
    JWT_KID: 'test', GOOGLE_CLIENT_ID: 'test', GOOGLE_CLIENT_SECRET: 'test',
    VAPID_PUBLIC_KEY: 'test', VAPID_PRIVATE_KEY: 'test', IP_HASH_SALT: 'test',
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
  ({ pool, dbUrl, teardown } = await createIsolatedPool(url)); // per-process DB — parallel test files can't race
  await resetSchema(pool);
  await applyMoneyFix(pool);
  // P6b harness: a switchable poison that makes every payment_events INSERT throw.
  await pool.query(`
    CREATE OR REPLACE FUNCTION test_poison_payment_events() RETURNS trigger LANGUAGE plpgsql AS $$
    BEGIN
      IF current_setting('test.poison_events', true) = 'true' THEN
        RAISE EXCEPTION 'poisoned payment_events insert (test)';
      END IF;
      RETURN NEW;
    END $$;
    CREATE TRIGGER trg_test_poison BEFORE INSERT ON payment_events
      FOR EACH ROW EXECUTE FUNCTION test_poison_payment_events();
  `);
});
after(async () => { if (teardown) await teardown(); });

maybe('P14: raw `UPDATE orders SET status=CANCELLED` (funnel bypass) → L-C trigger records refund_due, idempotently', async () => {
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'PENDING', paymentStatus: 'paid' });
  await seedPaidPayment(pool, loc, orderId, 'inv-p14', 1000);

  await pool.query(`UPDATE orders SET status = 'CANCELLED' WHERE id = $1`, [orderId]); // deliberately bypasses the funnel
  assert.equal(await refundDueCount(pool, orderId), 1, 'pre-fix code has no trigger → RED (0)');

  // re-fire the edge → still exactly one obligation
  await pool.query(`UPDATE orders SET status = 'PENDING' WHERE id = $1`, [orderId]);
  await pool.query(`UPDATE orders SET status = 'CANCELLED' WHERE id = $1`, [orderId]);
  assert.equal(await refundDueCount(pool, orderId), 1, 'ON CONFLICT DO NOTHING: at most one refund_due per payment');
});

maybe('N3 behavioral: a cash order (no payments row) cancels with NO obligation and no trigger side effects', async () => {
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'PENDING' });
  await pool.query(`UPDATE orders SET status = 'CANCELLED' WHERE id = $1`, [orderId]);
  assert.equal(await refundDueCount(pool, orderId), 0);
});

maybe('P6: timeout sweep (fn byte-identical to 078) cancels AND the trigger records the obligation in the same statement', async () => {
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'PENDING', timeoutAt: '2020-01-01 00:00:00+00', paymentStatus: 'paid' });
  await seedPaidPayment(pool, loc, orderId, 'inv-p6', 1000);

  const swept = await pool.query(`SELECT * FROM app_sweep_timeout_orders()`);
  assert.ok(swept.rows.some((r: any) => r.id === orderId), 'sweep recovered the overdue order');
  const st = await pool.query(`SELECT status FROM orders WHERE id = $1`, [orderId]);
  assert.equal(st.rows[0].status, 'CANCELLED');
  assert.equal(await refundDueCount(pool, orderId), 1, 'obligation recorded transactionally via L-C — pre-fix RED (0)');
});

maybe('P6b: poison-row liveness — a failing refund insert CANNOT wedge the fleet-wide sweep; reconciler surfaces it as failed', async () => {
  const loc = await seedLocation(pool);
  const paidOrder = await seedOrder(pool, loc, { status: 'PENDING', timeoutAt: '2020-01-01 00:00:00+00', paymentStatus: 'paid' });
  await seedPaidPayment(pool, loc, paidOrder, 'inv-p6b', 1000);
  const cashOrder = await seedOrder(pool, loc, { status: 'PENDING', timeoutAt: '2020-01-01 00:00:00+00' });
  const otherLoc = await seedLocation(pool);
  const otherTenantOrder = await seedOrder(pool, otherLoc, { status: 'PENDING', timeoutAt: '2020-01-01 00:00:00+00' });

  const session = await pool.connect();
  try {
    await session.query(`SELECT set_config('test.poison_events', 'true', false)`); // session-scoped poison
    const swept = await session.query(`SELECT * FROM app_sweep_timeout_orders()`);
    const sweptIds = swept.rows.map((r: any) => r.id);
    for (const id of [paidOrder, cashOrder, otherTenantOrder]) {
      assert.ok(sweptIds.includes(id), 'sweep cancels EVERY overdue order across tenants despite the poisoned insert (C1 fix)');
    }
    assert.equal(await refundDueCount(pool, paidOrder), 0, 'obligation could not be recorded (swallowed, not thrown)');

    // reconciler under the same poison: surfaces the failure, never throws
    const recon = await session.query(`SELECT * FROM app_reconcile_refund_due()`);
    const failed = recon.rows.filter((r: any) => r.o_action === 'failed' && r.o_order_id === paidOrder);
    assert.equal(failed.length, 1, 'reconciler reports the row as failed (P15 alarm arm)');
    assert.match(failed[0].o_detail, /poisoned/, 'failure detail carried for the operator alert');

    // poison off → next tick records it
    await session.query(`SELECT set_config('test.poison_events', 'false', false)`);
    const recon2 = await session.query(`SELECT * FROM app_reconcile_refund_due()`);
    assert.ok(recon2.rows.some((r: any) => r.o_action === 'inserted' && r.o_order_id === paidOrder), 'reconciler records the missed obligation ≤1 tick after the fault clears');
    assert.equal(await refundDueCount(pool, paidOrder), 1);
  } finally {
    session.release();
  }
});

maybe('P15: L-C disabled (simulated healthy-layer miss) → one reconciler run inserts; second run is a no-op', async () => {
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'PENDING', paymentStatus: 'paid' });
  await seedPaidPayment(pool, loc, orderId, 'inv-p15', 2500);
  await pool.query(`ALTER TABLE orders DISABLE TRIGGER trg_orders_refund_due_on_terminal`);
  try {
    await pool.query(`UPDATE orders SET status = 'CANCELLED' WHERE id = $1`, [orderId]);
    assert.equal(await refundDueCount(pool, orderId), 0, 'layer miss seeded');
  } finally {
    await pool.query(`ALTER TABLE orders ENABLE TRIGGER trg_orders_refund_due_on_terminal`);
  }
  const recon = await pool.query(`SELECT * FROM app_reconcile_refund_due()`);
  assert.ok(recon.rows.some((r: any) => r.o_action === 'inserted' && r.o_order_id === orderId), 'reconciler records the miss');
  assert.equal(await refundDueCount(pool, orderId), 1);
  const again = await pool.query(`SELECT * FROM app_reconcile_refund_due()`);
  assert.ok(!again.rows.some((r: any) => r.o_order_id === orderId), 'recorded obligation is not re-reported');
});

maybe('P16: mismatch-class terminal order is SURFACED by the reconciler and NEVER auto-obligated', async () => {
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'CANCELLED' });
  // under/over-paid: payment never flipped 'paid'; webhook recorded a 'mismatch' event
  const payRes = await pool.query(
    `INSERT INTO payments (location_id, order_id, provider, provider_payment_id, status, amount_minor, currency_code)
     VALUES ($1, $2, 'plisio', 'inv-p16', 'pending', 1250, 'ALL') RETURNING id`,
    [loc, orderId],
  );
  await pool.query(
    `INSERT INTO payment_events (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified)
     VALUES ($1, $2, 'plisio', 'inv-p16', 'mismatch', 900, 'ALL', true)`,
    [payRes.rows[0].id, loc],
  );
  const recon = await pool.query(`SELECT * FROM app_reconcile_refund_due()`);
  const mm = recon.rows.filter((r: any) => r.o_action === 'mismatch' && r.o_order_id === orderId);
  assert.equal(mm.length, 1, 'mismatch surfaced in the operator alert listing');
  assert.equal(await refundDueCount(pool, orderId), 0, 'NO refund_due auto-created — amount is ambiguous, human disposes (M3 scope)');
});

maybe('P4: funnel cancel via the REAL updateOrderStatus → L-A fold + L-C trigger land EXACTLY one obligation', async () => {
  const { updateOrderStatus } = await import('../src/lib/orderStatusService.js');
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'PENDING', paymentStatus: 'paid' });
  await seedPaidPayment(pool, loc, orderId, 'inv-p4', 3000);

  const client = await pool.connect();
  const published: string[] = [];
  try {
    await client.query('BEGIN');
    await updateOrderStatus(client as any, orderId, loc, 'CANCELLED', {
      messageBus: { publish: async (ch: string) => { published.push(ch); } } as any,
      comment: 'p4-test',
    });
    await client.query('COMMIT');
  } catch (e) {
    await client.query('ROLLBACK').catch(() => {});
    throw e;
  } finally {
    client.release();
  }
  const st = await pool.query(`SELECT status FROM orders WHERE id = $1`, [orderId]);
  assert.equal(st.rows[0].status, 'CANCELLED');
  assert.equal(await refundDueCount(pool, orderId), 1, 'both layers fired in one tx → exactly one row (unique-deduped)');
  assert.ok(published.length >= 1, 'WS deltas still published');
});

maybe('P4 fail-closed arm: with the ledger poisoned, the funnel cancel ABORTS (per-order) and the order stays PENDING', async () => {
  const { updateOrderStatus } = await import('../src/lib/orderStatusService.js');
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'PENDING', paymentStatus: 'paid' });
  await seedPaidPayment(pool, loc, orderId, 'inv-p4b-fc', 3000);

  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    await client.query(`SELECT set_config('test.poison_events', 'true', true)`); // tx-scoped poison
    await assert.rejects(
      // L-C swallows (non-throwing), but the L-A fold is FAIL-CLOSED per order → typed throw
      updateOrderStatus(client as any, orderId, loc, 'CANCELLED', {
        messageBus: { publish: async () => {} } as any,
      }),
      (e: any) => e?.code === 'REFUND_DUE_RECORD_FAILED' && e?.statusCode === 500,
      'fold failure must abort THIS cancel with the typed ESC-2 error',
    );
    await client.query('ROLLBACK');
  } finally {
    client.release();
  }
  const st = await pool.query(`SELECT status FROM orders WHERE id = $1`, [orderId]);
  assert.equal(st.rows[0].status, 'PENDING', 'order not cancelled — obligation-or-abort, single-order blast radius');
  assert.equal(await refundDueCount(pool, orderId), 0);
});

maybe('N5: NULL provider_payment_id cannot multi-insert — partial unique holds across L-A/L-C/L-D shapes', async () => {
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'PENDING', paymentStatus: 'paid' });
  await seedPaidPayment(pool, loc, orderId, null, 4000); // NULL ref — the idem_unique treats NULLs as distinct

  await pool.query(`UPDATE orders SET status = 'CANCELLED' WHERE id = $1`, [orderId]); // L-C fires
  assert.equal(await refundDueCount(pool, orderId), 1);

  // L-A statement shape, executed directly (as the fold would) — must conflict on the partial unique
  await pool.query(
    `INSERT INTO payment_events (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified)
     SELECT p.id, p.location_id, p.provider, p.provider_payment_id, 'refund_due', p.amount_minor, p.currency_code, true
       FROM payments p WHERE p.order_id = $1 AND p.status = 'paid'
     ON CONFLICT DO NOTHING`,
    [orderId],
  );
  // L-D pass
  await pool.query(`SELECT * FROM app_reconcile_refund_due()`);
  assert.equal(await refundDueCount(pool, orderId), 1, 'exactly one obligation despite NULL ref + three writer shapes');
});

maybe('N6: GUC read-back after a trigger fire on the unset path is EMPTY STRING (≡ unset for every consumer), pinned', async () => {
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'PENDING', paymentStatus: 'paid' });
  await seedPaidPayment(pool, loc, orderId, 'inv-n6', 500);

  // Fresh session (dedicated pool): a POOLED connection that ever ran ANY tx-local set_config
  // reads '' afterwards (the placeholder persists post-commit as empty — PG16-verified), so the
  // NULL precondition only holds on a genuinely fresh connection.
  const fresh = new Pool({ connectionString: dbUrl, max: 1 });
  const client = await fresh.connect();
  try {
    const preState = await client.query(`SELECT current_setting('app.current_tenant', true) AS v`);
    assert.equal(preState.rows[0].v, null, 'unset path: fresh session reads NULL before the trigger');
    await client.query('BEGIN');
    await client.query(`UPDATE orders SET status = 'CANCELLED' WHERE id = $1`, [orderId]); // trigger success path
    const inTx = await client.query(
      `SELECT current_setting('app.current_tenant', true) AS raw,
              nullif(current_setting('app.current_tenant', true), '')::uuid AS policy_view`,
    );
    // PG16-verified: set_config(NULL) coerces to '' — a true-NULL restore is impossible. '' is
    // semantically unset for every consumer: the dual policies nullif('','') → NULL (arm inert).
    assert.equal(inTx.rows[0].raw, '', 'in-tx read-back after restore: empty string, NOT the leaked tenant uuid');
    assert.equal(inTx.rows[0].policy_view, null, 'policy GUC arm sees it as unset — no tenant contamination');
    await client.query('COMMIT');
    const post = await client.query(
      `SELECT current_setting('app.current_tenant', true) AS raw,
              nullif(current_setting('app.current_tenant', true), '')::uuid AS policy_view`,
    );
    assert.equal(post.rows[0].raw, '', 'post-commit session residue is the empty placeholder — the tenant value never escapes the tx');
    assert.equal(post.rows[0].policy_view, null);
  } finally {
    client.release();
    await fresh.end();
  }
  assert.equal(await refundDueCount(pool, orderId), 1);
});

maybe('L-B shape: pay-after-cancel — the webhook fold records the obligation for an already-terminal order, replay-safe', async () => {
  const loc = await seedLocation(pool);
  const orderId = await seedOrder(pool, loc, { status: 'CANCELLED', paymentStatus: 'paid' });
  await seedPaidPayment(pool, loc, orderId, 'inv-lb', 1750);
  const LB_SQL = `
    INSERT INTO payment_events
      (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified)
    SELECT p.id, p.location_id, p.provider, p.provider_payment_id, 'refund_due', p.amount_minor, p.currency_code, true
      FROM payments p JOIN orders o ON o.id = p.order_id
     WHERE p.provider = 'plisio' AND p.provider_payment_id = $1 AND p.status = 'paid'
       AND o.status IN ('CANCELLED','REJECTED')
    ON CONFLICT DO NOTHING`;
  await pool.query(LB_SQL, ['inv-lb']);
  assert.equal(await refundDueCount(pool, orderId), 1, 'pre-fix webhook records nothing → RED (0)');
  await pool.query(LB_SQL, ['inv-lb']); // Plisio replay
  assert.equal(await refundDueCount(pool, orderId), 1, 'replay-safe');
});
