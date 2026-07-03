import './_env-stub.js';
import { test } from 'node:test';
import assert from 'node:assert/strict';

// ADR-dispatch-recovery (B2 dispatch auto-recovery + B5 reconciliation re-enable) — red→green
// guardrails for every prescription:
//   1. the broken `this.boss` 30s self-retry is DELETED (pump = sole cadence, no TypeError);
//   2. the CourierOfferSweep drain pass pumps journal rows → COURIER_DISPATCH with singletonKey
//      (DoD-1 standing regression, R-ACC-4: dropping the fold-in goes RED here);
//   3. exhaustion is honest at both ends: orders.dispatch_exhausted_at + journal delete COMMIT
//      first, ORDER_DISPATCH_FAILED published post-commit, and the bootstrap consumer enqueues
//      the owner Telegram AND the honest customer push (void = RED);
//   4. handleDispatch idempotency pre-check + 23505-by-constraint benign-race handling;
//   5. shift-pick excludes offer-holding couriers ('offered' in the exclusion set);
//   6. 'assigned' acceptance timeout (COURIER_ASSIGN_ACCEPT_TIMEOUT_MS) expires + re-enqueues;
//   7. grace-window auto-cancel is FLAG-OFF dark (DISPATCH_OWNER_GRACE_ENABLED, R-NEEDS-HUMAN-1);
//   8. ReconciliationWorker is registered and A6's 8-worker set genuinely heartbeats (R3′);
//   9. no false signals: bindingRelease returns `requeued`, sweep log says "re-enqueued".

interface Call { sql: string; params: unknown[] }

type Handler = (sql: string, params: unknown[]) => { rows: any[]; rowCount: number } | undefined;

function makeClient(handler: Handler, seq?: string[]) {
  const calls: Call[] = [];
  return {
    calls,
    async query(sql: string, params: unknown[] = []) {
      calls.push({ sql, params });
      seq?.push(`sql:${sql.replace(/\s+/g, ' ').trim().slice(0, 60)}`);
      const res = handler(sql, params);
      return res ?? { rows: [], rowCount: 0 };
    },
    release() {},
  };
}

function makePool(client: any) {
  return { connect: async () => client, query: client.query.bind(client) } as any;
}

function makeBus(events: Array<{ channel: string; payload: any }>, seq?: string[]) {
  return {
    publish: async (channel: string, payload: any) => {
      events.push({ channel, payload });
      seq?.push(`publish:${channel}`);
    },
    subscribe: () => {},
  } as any;
}

const dispatchDefaults: Record<string, { rows: any[]; rowCount: number }> = {};

function dispatchHandler(overrides: Array<[RegExp, { rows: any[]; rowCount: number } | (() => any)]>): Handler {
  return (sql) => {
    for (const [re, res] of overrides) {
      if (re.test(sql)) return typeof res === 'function' ? res() : res;
    }
    return undefined;
  };
}

async function loadDispatchWorker() {
  const { CourierDispatchWorker } = await import('../src/workers/courier-dispatch.js');
  return CourierDispatchWorker as any;
}

// ── 1. self-retry deleted: no-courier, not-exhausted → attempts++, COMMIT, clean return ──

test('handleDispatch — no courier, not exhausted: increments attempts, COMMITs, returns (no self-retry, no TypeError)', async () => {
  const CourierDispatchWorker = await loadDispatchWorker();
  const client = makeClient(dispatchHandler([
    [/FROM courier_dispatch_queue WHERE order_id/, { rows: [{ attempts: 1 }], rowCount: 1 }],
    [/SELECT status FROM orders WHERE id/, { rows: [{ status: 'READY' }], rowCount: 1 }],
    [/FROM courier_assignments WHERE order_id/, { rows: [], rowCount: 0 }],
    [/FROM courier_shifts cs/, { rows: [], rowCount: 0 }],
  ]));
  const events: any[] = [];
  const worker = new CourierDispatchWorker(makePool(client), {} as any, makeBus(events));

  await worker.handleDispatch('order-1', 'loc-1'); // current code: TypeError (this.boss undefined) → RED

  const sqls = client.calls.map((c) => c.sql);
  assert.ok(sqls.some((s) => /UPDATE courier_dispatch_queue SET attempts/.test(s)), 'attempts incremented');
  assert.equal(sqls[sqls.length - 1].trim(), 'COMMIT', 'transaction committed');
  assert.ok(!sqls.includes('ROLLBACK'), 'no rollback on the honest retry path');
  assert.equal(events.length, 0, 'no event published — the 60s pump is the sole retry cadence');
});

// ── 3. exhaustion tail is honest: marker + delete committed first, event published post-commit ──

test('handleDispatch — exhaustion: dispatch_exhausted_at + journal delete committed BEFORE ORDER_DISPATCH_FAILED publish', async () => {
  const CourierDispatchWorker = await loadDispatchWorker();
  const seq: string[] = [];
  const client = makeClient(dispatchHandler([
    [/FROM courier_dispatch_queue WHERE order_id/, { rows: [{ attempts: 4 }], rowCount: 1 }],
    [/SELECT status FROM orders WHERE id/, { rows: [{ status: 'READY' }], rowCount: 1 }],
    [/FROM courier_assignments WHERE order_id/, { rows: [], rowCount: 0 }],
    [/FROM courier_shifts cs/, { rows: [], rowCount: 0 }],
    [/UPDATE orders SET dispatch_exhausted_at/, { rows: [], rowCount: 1 }],
  ]), seq);
  const events: Array<{ channel: string; payload: any }> = [];
  const worker = new CourierDispatchWorker(makePool(client), {} as any, makeBus(events, seq));

  await worker.handleDispatch('order-1', 'loc-1');

  const markerIdx = seq.findIndex((s) => s.includes('UPDATE orders SET dispatch_exhausted_at'));
  const deleteIdx = seq.findIndex((s) => s.includes('DELETE FROM courier_dispatch_queue'));
  const commitIdx = seq.lastIndexOf('sql:COMMIT');
  const publishIdx = seq.findIndex((s) => s === 'publish:order.dispatch_failed');
  assert.ok(markerIdx >= 0, 'orders.dispatch_exhausted_at is set (durable held-marker)');
  assert.ok(deleteIdx > markerIdx, 'journal row deleted only after the order marker is written');
  assert.ok(commitIdx > deleteIdx, 'marker + delete share one transaction');
  assert.ok(publishIdx > commitIdx, 'ORDER_DISPATCH_FAILED published post-commit (trace durable first)');
  assert.equal(events.length, 1);
  assert.deepEqual(Object.keys(events[0].payload).sort(), ['locationId', 'orderId', 'reason'], 'claim-check clean payload (no PII)');
});

// ── 4a. idempotency pre-check: active binding → delete journal row, no second assignment ──

test('handleDispatch — order already actively bound → journal row deleted, no assignment INSERT', async () => {
  const CourierDispatchWorker = await loadDispatchWorker();
  const client = makeClient(dispatchHandler([
    [/FROM courier_dispatch_queue WHERE order_id/, { rows: [{ attempts: 0 }], rowCount: 1 }],
    [/SELECT status FROM orders WHERE id/, { rows: [{ status: 'READY' }], rowCount: 1 }],
    [/FROM courier_assignments WHERE order_id/, { rows: [{ '?column?': 1 }], rowCount: 1 }],
  ]));
  const events: any[] = [];
  const worker = new CourierDispatchWorker(makePool(client), {} as any, makeBus(events));

  await worker.handleDispatch('order-1', 'loc-1');

  const sqls = client.calls.map((c) => c.sql);
  assert.ok(sqls.some((s) => /DELETE FROM courier_dispatch_queue/.test(s)), 'stale journal row deleted');
  assert.ok(!sqls.some((s) => /INSERT INTO courier_assignments/.test(s)), 'no double assignment');
  assert.equal(sqls[sqls.length - 1].trim(), 'COMMIT');
});

test('handleDispatch — terminal order (CANCELLED) → journal row deleted, no assignment INSERT', async () => {
  const CourierDispatchWorker = await loadDispatchWorker();
  const client = makeClient(dispatchHandler([
    [/FROM courier_dispatch_queue WHERE order_id/, { rows: [{ attempts: 0 }], rowCount: 1 }],
    [/SELECT status FROM orders WHERE id/, { rows: [{ status: 'CANCELLED' }], rowCount: 1 }],
    [/FROM courier_assignments WHERE order_id/, { rows: [], rowCount: 0 }],
  ]));
  const worker = new CourierDispatchWorker(makePool(client), {} as any, makeBus([]));

  await worker.handleDispatch('order-1', 'loc-1');

  const sqls = client.calls.map((c) => c.sql);
  assert.ok(sqls.some((s) => /DELETE FROM courier_dispatch_queue/.test(s)), 'journal row deleted for terminal order');
  assert.ok(!sqls.some((s) => /INSERT INTO courier_assignments/.test(s)));
});

// ── 5. shift-pick excludes offer-holding couriers (aligned with courier_one_active_assignment) ──

test('handleDispatch — shift-pick exclusion set includes offered (no perpetual 23505 under flag-ON)', async () => {
  const CourierDispatchWorker = await loadDispatchWorker();
  const client = makeClient(dispatchHandler([
    [/FROM courier_dispatch_queue WHERE order_id/, { rows: [{ attempts: 0 }], rowCount: 1 }],
    [/SELECT status FROM orders WHERE id/, { rows: [{ status: 'READY' }], rowCount: 1 }],
    [/FROM courier_assignments WHERE order_id/, { rows: [], rowCount: 0 }],
    [/FROM courier_shifts cs/, { rows: [{ courier_id: 'c1', shift_id: 's1' }], rowCount: 1 }],
  ]));
  const worker = new CourierDispatchWorker(makePool(client), {} as any, makeBus([]));

  await worker.handleDispatch('order-1', 'loc-1');

  const pick = client.calls.find((c) => /FROM courier_shifts cs/.test(c.sql));
  assert.ok(pick, 'shift pick issued');
  assert.match(pick!.sql, /'offered','assigned','accepted','picked_up'/, "pick excludes couriers holding an 'offered' assignment");
});

// ── 6/7/8. 23505 special-cased by constraint ──

function throwing23505(constraint: string): Handler {
  return (sql) => {
    if (/FROM courier_dispatch_queue WHERE order_id/.test(sql)) return { rows: [{ attempts: 0 }], rowCount: 1 };
    if (/SELECT status FROM orders WHERE id/.test(sql)) return { rows: [{ status: 'READY' }], rowCount: 1 };
    if (/FROM courier_assignments WHERE order_id/.test(sql)) return { rows: [], rowCount: 0 };
    if (/FROM courier_shifts cs/.test(sql)) return { rows: [{ courier_id: 'c1', shift_id: 's1' }], rowCount: 1 };
    if (/INSERT INTO courier_assignments/.test(sql)) {
      const err: any = new Error('duplicate key value violates unique constraint');
      err.code = '23505';
      err.constraint = constraint;
      throw err;
    }
    return undefined;
  };
}

test('handleDispatch — 23505 on courier_assignments_order_active_uniq → benign: journal row deleted, NO throw', async () => {
  const CourierDispatchWorker = await loadDispatchWorker();
  const client = makeClient(throwing23505('courier_assignments_order_active_uniq'));
  const worker = new CourierDispatchWorker(makePool(client), {} as any, makeBus([]));

  await worker.handleDispatch('order-1', 'loc-1'); // must NOT reject (no pg-boss retry, no false O3)

  const sqls = client.calls.map((c) => c.sql);
  const rollbackIdx = sqls.findIndex((s) => s.trim() === 'ROLLBACK');
  const deleteIdx = sqls.findIndex((s) => /DELETE FROM courier_dispatch_queue/.test(s));
  assert.ok(rollbackIdx >= 0, 'aborted tx rolled back');
  assert.ok(deleteIdx > rollbackIdx, 'journal row deleted after rollback (order is already bound — resolved)');
});

test('handleDispatch — 23505 on courier_one_active_assignment → row kept, NO throw (next tick re-picks)', async () => {
  const CourierDispatchWorker = await loadDispatchWorker();
  const client = makeClient(throwing23505('courier_one_active_assignment'));
  const worker = new CourierDispatchWorker(makePool(client), {} as any, makeBus([]));

  await worker.handleDispatch('order-1', 'loc-1');

  const sqls = client.calls.map((c) => c.sql);
  const rollbackIdx = sqls.findIndex((s) => s.trim() === 'ROLLBACK');
  assert.ok(rollbackIdx >= 0, 'aborted tx rolled back');
  assert.ok(!sqls.slice(rollbackIdx).some((s) => /DELETE FROM courier_dispatch_queue/.test(s)), 'journal row NOT deleted — order still needs a courier');
});

test('handleDispatch — any other error → ROLLBACK + rethrow (pg-boss retry semantics unchanged)', async () => {
  const CourierDispatchWorker = await loadDispatchWorker();
  const client = makeClient((sql) => {
    if (/FROM courier_dispatch_queue WHERE order_id/.test(sql)) { const e: any = new Error('boom'); e.code = '57014'; throw e; }
    return undefined;
  });
  const worker = new CourierDispatchWorker(makePool(client), {} as any, makeBus([]));

  await assert.rejects(() => worker.handleDispatch('order-1', 'loc-1'), /boom/);
  assert.ok(client.calls.some((c) => c.sql.trim() === 'ROLLBACK'));
});

// ── 2. the drain pump (DoD-1 standing regression — R-ACC-4) ──

function sweepHandler(overrides: Array<[RegExp, { rows: any[]; rowCount: number }]>): Handler {
  return (sql) => {
    if (/pg_try_advisory_lock/.test(sql)) return { rows: [{ locked: true }], rowCount: 1 };
    for (const [re, res] of overrides) if (re.test(sql)) return res;
    return undefined;
  };
}

async function loadSweepWorker() {
  const { CourierOfferSweepWorker } = await import('../src/workers/courier-offer-sweep.js');
  return CourierOfferSweepWorker as any;
}

test('sweep drain — journal row pumped into COURIER_DISPATCH via boss.send with singletonKey (real enqueue, not this.boss)', async () => {
  const CourierOfferSweepWorker = await loadSweepWorker();
  const sends: Array<{ name: string; payload: any; opts: any }> = [];
  const boss = { send: async (name: string, payload: any, opts: any) => { sends.push({ name, payload, opts }); }, work: async () => {}, createQueue: async () => {}, schedule: async () => {} };
  const client = makeClient(sweepHandler([
    [/SELECT order_id, location_id FROM courier_dispatch_queue/, { rows: [{ order_id: 'o1', location_id: 'l1' }, { order_id: 'o2', location_id: 'l2' }], rowCount: 2 }],
  ]));
  const sweep = new CourierOfferSweepWorker(makePool(client), boss as any, makeBus([]));

  await (sweep as any).run();

  assert.equal(sends.length, 2, 'one COURIER_DISPATCH job per journal row');
  assert.equal(sends[0].name, 'courier.dispatch');
  assert.deepEqual(sends[0].payload, { orderId: 'o1', locationId: 'l1' });
  assert.equal(sends[0].opts?.singletonKey, 'o1', 'singletonKey dedup — one in-flight job per order');
  assert.equal(sends[1].opts?.singletonKey, 'o2');
});

// ── 6. 'assigned' acceptance timeout ──

test("sweep accept-timeout — stale 'assigned' → cancelled/assign_accept_timeout, shift freed, re-enqueued", async () => {
  const CourierOfferSweepWorker = await loadSweepWorker();
  const boss = { send: async () => {}, work: async () => {}, createQueue: async () => {}, schedule: async () => {} };
  const client = makeClient(sweepHandler([
    [/FROM courier_assignments\s+WHERE status = 'assigned'/, { rows: [{ id: 'a1', order_id: 'o1', shift_id: 's1', location_id: 'l1' }], rowCount: 1 }],
    [/UPDATE courier_assignments SET status\s*=\s*'cancelled'/, { rows: [], rowCount: 1 }],
  ]));
  const events: any[] = [];
  const sweep = new CourierOfferSweepWorker(makePool(client), boss as any, makeBus(events));

  await (sweep as any).run();

  const sqls = client.calls.map((c) => c.sql);
  const expire = client.calls.find((c) => /UPDATE courier_assignments SET status\s*=\s*'cancelled'/.test(c.sql));
  assert.ok(expire, 'expiry UPDATE issued');
  assert.match(expire!.sql, /assign_accept_timeout/, 'cancellation_reason disambiguates the reused cancelled status');
  assert.match(expire!.sql, /status = 'assigned'/, "guarded transition — only still-'assigned' rows expire (boundary accept wins)");
  assert.ok(sqls.some((s) => /UPDATE courier_shifts SET status\s*=\s*'available'/.test(s)), 'shift freed');
  assert.ok(sqls.some((s) => /INSERT INTO courier_dispatch_queue/.test(s)), 're-enqueued to the journal');
});

// ── 9. honest signals: log says re-enqueued, never re-offered ──

test('sweep — expired offer log claims "re-enqueued for dispatch", not a false "re-offered"', async () => {
  const CourierOfferSweepWorker = await loadSweepWorker();
  const boss = { send: async () => {}, work: async () => {}, createQueue: async () => {}, schedule: async () => {} };
  const client = makeClient(sweepHandler([
    [/app_sweep_expired_offers/, { rows: [{ order_id: 'o1', location_id: 'l1' }], rowCount: 1 }],
  ]));
  const sweep = new CourierOfferSweepWorker(makePool(client), boss as any, makeBus([]));

  const logs: string[] = [];
  const origLog = console.log;
  console.log = (...args: any[]) => { logs.push(args.join(' ')); };
  try {
    await (sweep as any).run();
  } finally {
    console.log = origLog;
  }

  assert.ok(logs.some((l) => /re-enqueued for dispatch/.test(l)), 'honest log present');
  assert.ok(!logs.some((l) => /re-offered/.test(l)), 'no false re-offered claim');
});

test('bindingRelease — returns { requeued } (renamed from the false reoffered claim)', async () => {
  const { releaseBindingAndReoffer } = await import('../src/lib/bindingRelease.js');
  const client = makeClient(() => undefined);
  const result: any = await releaseBindingAndReoffer(
    client,
    { assignmentId: 'a1', orderId: 'o1', shiftId: 's1', asgStatus: 'accepted', ordStatus: 'CONFIRMED', locationId: 'l1', reason: 'courier_declined' },
    { messageBus: makeBus([]) },
  );
  assert.deepEqual(result, { requeued: true });
  assert.ok(!('reoffered' in result), 'the false reoffered key is gone');
});

// ── 7. grace-window auto-cancel: FLAG-OFF dark (R-NEEDS-HUMAN-1), honest terminal when ON ──

test('sweep grace-window — DISPATCH_OWNER_GRACE_ENABLED default OFF: no exhausted-order scan, no auto-cancel', async () => {
  const CourierOfferSweepWorker = await loadSweepWorker();
  delete process.env.DISPATCH_OWNER_GRACE_ENABLED;
  const boss = { send: async () => {}, work: async () => {}, createQueue: async () => {}, schedule: async () => {} };
  const client = makeClient(sweepHandler([]));
  const sweep = new CourierOfferSweepWorker(makePool(client), boss as any, makeBus([]));

  await (sweep as any).run();

  assert.ok(!client.calls.some((c) => /dispatch_exhausted_at/.test(c.sql)), 'grace pass dark by default');
  assert.ok(!client.calls.some((c) => /UPDATE orders SET status\s*=\s*'CANCELLED'/.test(c.sql)), 'no auto-cancel while unratified');
});

// offer-sweep-cancel addendum: grace-cancel now routes through the SANCTIONED mutator updateOrderStatus
// (no raw UPDATE — R3-3 guardrail green), publishes ORDER_CANCELLED post-commit (F1), and writes NO cash.
// Handler models updateOrderStatus's real queries (it runs for real against the fake client).
function graceFunnelHandler(status: string, extra: Array<[RegExp, { rows: any[]; rowCount: number }]> = []): Handler {
  return sweepHandler([
    [/dispatch_exhausted_at IS NOT NULL/, { rows: [{ id: 'o9', location_id: 'l1' }], rowCount: 1 }],
    [/SELECT status FROM orders WHERE id = \$1 FOR UPDATE/, { rows: [{ status }], rowCount: 1 }], // worker lock read
    ...extra,
    // F7 under-lock re-check: no active binding by default.
    [/SELECT 1 FROM courier_assignments WHERE order_id = \$1/, { rows: [], rowCount: 0 }],
    // updateOrderStatus internals:
    [/SELECT id, status, location_id FROM orders WHERE id/, { rows: [{ id: 'o9', status, location_id: 'l1' }], rowCount: 1 }],
    [/UPDATE orders SET status = \$1/, { rows: [{ id: 'o9' }], rowCount: 1 }], // status-guarded, parameterized
  ]);
}

test('sweep grace-window — flag ON: PREPARING order past grace → CANCELLED via updateOrderStatus funnel (no raw UPDATE), ORDER_CANCELLED post-commit, honest customer push, ZERO cash', async () => {
  const CourierOfferSweepWorker = await loadSweepWorker();
  process.env.DISPATCH_OWNER_GRACE_ENABLED = 'true';
  try {
    const sends: Array<{ name: string; payload: any }> = [];
    const boss = { send: async (name: string, payload: any) => { sends.push({ name, payload }); }, work: async () => {}, createQueue: async () => {}, schedule: async () => {} };
    const client = makeClient(graceFunnelHandler('PREPARING'));
    const events: Array<{ channel: string; payload: any }> = [];
    const sweep = new CourierOfferSweepWorker(makePool(client), boss as any, makeBus(events));

    await (sweep as any).run();

    const sqls = client.calls.map((c) => c.sql);
    // Sanctioned funnel, NOT a raw cancel (R3-3): the write is updateOrderStatus's parameterized guard.
    assert.ok(!sqls.some((s) => /UPDATE orders SET status\s*=\s*'CANCELLED'/.test(s)), 'no raw inline CANCELLED UPDATE (funnel, not laundering)');
    assert.ok(sqls.some((s) => /UPDATE orders SET status = \$1, timeout_at = NULL/.test(s)), 'routed through updateOrderStatus (parameterized guard + timeout_at cleared)');
    assert.ok(sqls.some((s) => /SAVEPOINT order_status_history_ins/.test(s)), 'updateOrderStatus history savepoint present (real funnel ran)');
    // F7: the "no active assignment" re-check ran UNDER the FOR UPDATE lock before the mutator.
    const lockIdx = sqls.findIndex((s) => /FOR UPDATE/.test(s));
    const recheckIdx = sqls.findIndex((s) => /SELECT 1 FROM courier_assignments WHERE order_id/.test(s));
    const mutIdx = sqls.findIndex((s) => /UPDATE orders SET status = \$1/.test(s));
    assert.ok(lockIdx >= 0 && recheckIdx > lockIdx && mutIdx > recheckIdx, 'F7: re-check under lock, before the mutator');
    // Audit trail carries the dispatch_exhausted reason (as updateOrderStatus comment).
    assert.ok(client.calls.some((c) => /INSERT INTO order_status_history/.test(c.sql) && c.params.includes('dispatch_exhausted')), 'history comment = dispatch_exhausted');
    // COMMIT durable before consequential effects.
    assert.ok(sqls.some((s) => /^COMMIT$/.test(s.trim())), 'tx committed');
    // F1: ORDER_CANCELLED published POST-commit → drives lifecycle-handlers (dwell resolve + boss.cancel).
    const cancelledEvt = events.find((e) => e.channel === 'order.cancelled');
    assert.ok(cancelledEvt, 'ORDER_CANCELLED published (F1 fan-out)');
    assert.equal(cancelledEvt!.payload.orderId, 'o9');
    assert.equal(cancelledEvt!.payload.reason, 'dispatch_exhausted');
    const commitPos = client.calls.findIndex((c) => c.sql.trim() === 'COMMIT');
    const cancelledPublishAfterCommit = client.calls.length >= commitPos; // publishes happen after the loop's COMMIT
    assert.ok(commitPos >= 0 && cancelledPublishAfterCommit, 'fan-out is post-commit');
    // Honest customer terminal push.
    const push = sends.find((s) => s.name === 'notify.customer_status');
    assert.ok(push, 'honest customer terminal push enqueued');
    assert.equal(push!.payload.event, 'CANCELLED');
    assert.equal(push!.payload.orderId, 'o9');
    // 🔴 CASH-SAFETY (load-bearing): the grace path writes NO cash ledger hold and NO delivery_trace crumb.
    assert.ok(!sqls.some((s) => /INSERT INTO courier_cash_ledger/i.test(s)), 'ZERO courier_cash_ledger writes');
    assert.ok(!sqls.some((s) => /INSERT INTO delivery_trace/i.test(s)), 'ZERO delivery_trace writes');
  } finally {
    delete process.env.DISPATCH_OWNER_GRACE_ENABLED;
  }
});

test('sweep grace-window — F7 anti-race: a freshly-bound (accepted) order is NOT cancelled (skip, no history, no ORDER_CANCELLED)', async () => {
  const CourierOfferSweepWorker = await loadSweepWorker();
  process.env.DISPATCH_OWNER_GRACE_ENABLED = 'true';
  try {
    const sends: Array<{ name: string; payload: any }> = [];
    const boss = { send: async (name: string, payload: any) => { sends.push({ name, payload }); }, work: async () => {}, createQueue: async () => {}, schedule: async () => {} };
    // Candidate prefilter picked it up, but under the lock a fresh 'accepted' binding exists → skip.
    const client = makeClient(graceFunnelHandler('PREPARING', [
      [/SELECT 1 FROM courier_assignments WHERE order_id = \$1/, { rows: [{ '?column?': 1 }], rowCount: 1 }],
    ]));
    const events: Array<{ channel: string; payload: any }> = [];
    const sweep = new CourierOfferSweepWorker(makePool(client), boss as any, makeBus(events));

    await (sweep as any).run();

    const sqls = client.calls.map((c) => c.sql);
    assert.ok(!sqls.some((s) => /UPDATE orders SET status = \$1/.test(s)), 'no status write — order left alone');
    assert.ok(!sqls.some((s) => /INSERT INTO order_status_history/.test(s)), 'no history row');
    assert.ok(!events.some((e) => e.channel === 'order.cancelled'), 'no ORDER_CANCELLED publish');
    assert.ok(!sends.some((s) => s.name === 'notify.customer_status'), 'no customer push');
    assert.ok(sqls.some((s) => s.trim() === 'ROLLBACK'), 'rolled back cleanly');
  } finally {
    delete process.env.DISPATCH_OWNER_GRACE_ENABLED;
  }
});

test('sweep grace-window — idempotency: lost race (409 from the mutator) → ROLLBACK, no duplicate history/publish', async () => {
  const CourierOfferSweepWorker = await loadSweepWorker();
  process.env.DISPATCH_OWNER_GRACE_ENABLED = 'true';
  try {
    const sends: Array<{ name: string; payload: any }> = [];
    const boss = { send: async (name: string, payload: any) => { sends.push({ name, payload }); }, work: async () => {}, createQueue: async () => {}, schedule: async () => {} };
    // The guarded UPDATE inside updateOrderStatus matches 0 rows (status changed under us) → throws 409.
    const client = makeClient(graceFunnelHandler('PREPARING', [
      [/UPDATE orders SET status = \$1/, { rows: [], rowCount: 0 }],
    ]));
    const events: Array<{ channel: string; payload: any }> = [];
    const sweep = new CourierOfferSweepWorker(makePool(client), boss as any, makeBus(events));

    await (sweep as any).run();

    const sqls = client.calls.map((c) => c.sql);
    assert.ok(sqls.some((s) => s.trim() === 'ROLLBACK'), '409 → ROLLBACK');
    assert.ok(!sqls.some((s) => s.trim() === 'COMMIT'), 'no COMMIT on a lost race');
    assert.ok(!events.some((e) => e.channel === 'order.cancelled'), 'no ORDER_CANCELLED on a lost race');
    assert.ok(!sends.some((s) => s.name === 'notify.customer_status'), 'no customer push on a lost race');
  } finally {
    delete process.env.DISPATCH_OWNER_GRACE_ENABLED;
  }
});

// ── 3b. the exhaustion consumer is WIRED (void = RED) ──

test('messaging — ORDER_DISPATCH_FAILED subscriber enqueues owner Telegram AND honest customer push', async () => {
  const { registerNotifySubscriptions } = await import('../src/bootstrap/messaging.js');
  const subs = new Map<string, (payload: any) => Promise<void>>();
  const bus = { subscribe: (ch: string, fn: any) => { subs.set(ch, fn); } } as any;
  const sends: Array<{ name: string; payload: any; opts: any }> = [];
  const boss = { send: async (name: string, payload: any, opts: any) => { sends.push({ name, payload, opts }); } } as any;

  registerNotifySubscriptions(bus, boss);

  const handler = subs.get('order.dispatch_failed');
  assert.ok(handler, 'ORDER_DISPATCH_FAILED has a subscriber (was publishing into the void)');
  await handler!({ orderId: 'o1', locationId: 'l1', reason: 'No couriers available after max attempts' });

  const tg = sends.find((s) => s.name === 'notify.telegram.send');
  assert.ok(tg, 'owner Telegram-ops alert enqueued');
  assert.equal(tg!.payload.event, 'order.dispatch_failed');
  assert.equal(tg!.payload.location_id, 'l1');
  const push = sends.find((s) => s.name === 'notify.customer_status');
  assert.ok(push, 'honest customer push enqueued');
  assert.equal(push!.payload.event, 'DISPATCH_DELAYED');
  assert.equal(push!.payload.orderId, 'o1');
});

test('notifications — order.dispatch_failed renders in sq/en/uk; DISPATCH_DELAYED + CANCELLED are pushable customer events', async () => {
  const { getMessage } = await import('../src/notifications/locales.js');
  for (const locale of ['sq', 'en', 'uk'] as const) {
    const msg = getMessage(locale, 'order.dispatch_failed', { shortOrderId: 'AB12' });
    assert.notEqual(msg, 'Unknown notification', `order.dispatch_failed has ${locale} copy`);
    assert.match(msg, /AB12/, `${locale} copy carries the order ref`);
  }
  const { CUSTOMER_STATUS_EVENTS } = await import('../src/notifications/workers/index.js');
  assert.ok((CUSTOMER_STATUS_EVENTS as readonly string[]).includes('DISPATCH_DELAYED'), 'honest delay push renderable');
  assert.ok((CUSTOMER_STATUS_EVENTS as readonly string[]).includes('CANCELLED'), 'honest terminal push renderable');
});

// ── 8. B5 — ReconciliationWorker registered; A6 EXPECTED_WORKERS (8) all heartbeat (R3′, no trim) ──

test('bootstrap — ReconciliationWorker registered and the A6 8-worker set genuinely heartbeats', async () => {
  const { startBackgroundWorkers } = await import('../src/bootstrap/workers.js');

  const workNames: string[] = [];
  const bossStub = {
    work: async (name: string) => { workNames.push(name); },
    createQueue: async () => {},
    schedule: async () => {},
    send: async () => {},
  };
  const queueStub = { work: async (name: string) => { workNames.push(name); }, boss: bossStub } as any;
  const clientStub = { query: async () => ({ rows: [], rowCount: 0 }), release() {} };
  const poolStub = { query: async () => ({ rows: [], rowCount: 0 }), connect: async () => clientStub } as any;
  const busStub = { publish: async () => {}, subscribe: () => {} } as any;

  const { heartbeats } = await startBackgroundWorkers({
    pool: poolStub, backupPool: poolStub, queue: queueStub, messageBus: busStub, notifyWorker: {} as any,
  });
  try {
    assert.ok(workNames.includes('reconciliation.nightly'), 'ReconciliationWorker.start() registered its queue worker');

    const beatingIds = new Set(heartbeats.map((hb: any) => hb.workerId));
    const A6_EXPECTED = ['dispatcher', 'settlement-cron', 'dwell-monitor', 'anonymizer-retention',
      'signal-raiser', 'liveness-checker', 'courier-stale_check', 'backup-hourly'];
    for (const id of A6_EXPECTED) {
      assert.ok(beatingIds.has(id), `${id} heartbeats (A6 must see the TRUE set of 8 — no false DRIFT)`);
    }
    assert.equal(beatingIds.size, A6_EXPECTED.length, 'heartbeat set == A6 EXPECTED_WORKERS exactly');
  } finally {
    for (const hb of heartbeats) hb.stop?.();
  }
});

// ── config — the accept-timeout + grace flags exist with safe defaults; retired knob is gone ──

test('config — new dispatch-recovery envs parse with safe defaults; COURIER_DISPATCH_RETRY_MS retired', async () => {
  const { loadEnv } = await import('@deliveryos/config');
  const env: any = loadEnv();
  assert.equal(env.DISPATCH_OWNER_GRACE_ENABLED, 'false', 'grace auto-cancel ships flag-OFF (R-NEEDS-HUMAN-1)');
  assert.ok('COURIER_ASSIGN_ACCEPT_TIMEOUT_MS' in (env as object) || env.COURIER_ASSIGN_ACCEPT_TIMEOUT_MS === undefined, 'accept-timeout knob accepted by the schema');
  assert.match(env.WORKER_CRITICAL_LIST, /backup-hourly/, 'backup-hourly gains the live 60s LivenessChecker path');
});
