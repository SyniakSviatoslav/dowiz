// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { QUEUE_NAMES, dashboardChannel, orderChannel } from '../lib/registry.js';
import { loadEnv } from '@deliveryos/config';

// deliver v2 §A + ADR-dispatch-recovery (B2): the durable courier-recovery sweep. Four passes
// under ONE advisory lock, one connection, 1-min cron — "data + sweep, no live timer to lose"
// (same machinery shape as OrderTimeoutSweep):
//   1. offer-expiry — an 'offered' assignment past offered_expires_at flips to 'offered_expired'
//      and the order is re-enqueued to the journal (inert unless COURIER_OFFER_HANDSHAKE_ENABLED).
//   2. accept-timeout — an auto-assigned 'assigned' binding never accepted within
//      COURIER_ASSIGN_ACCEPT_TIMEOUT_MS expires to 'cancelled'/'assign_accept_timeout', the shift
//      is freed and the order re-enqueued. Applies in BOTH flag states. Standing constraint
//      (R-ACC-5): carries NO courier reliability penalty — keep it scoring-free.
//   3. drain (Option C pump — the ONLY courier_dispatch_queue consumer): every resident journal
//      row is pumped into a COURIER_DISPATCH job, deduped by pg-boss singletonKey=orderId with the
//      courier_assignments_order_active_uniq partial unique as the hard DB backstop. One attempt
//      per order per tick — the 60s pump IS the dispatch retry cadence (ADR: the in-worker 30s
//      self-retry was deleted). DoD-1 pins this fold-in as a standing regression (R-ACC-4).
//   4. grace-window auto-cancel — 🔴 FLAG-OFF dark (DISPATCH_OWNER_GRACE_ENABLED, default false,
//      R-NEEDS-HUMAN-1): pending STOP-ETHICS ratification, an exhausted order the owner ignored
//      for DISPATCH_OWNER_GRACE_MS auto-transitions to the customer-honest terminal CANCELLED.
// 🔴 Pass 1-3 never touch the customer order; only the human-gated pass 4 (dark) transitions it.
const SWEEP_QUEUE = 'courier.offer_sweep';
const SWEEP_CRON = '* * * * *';
const SWEEP_LOCK_ID = 9; // distinct: 5 order-timeout, 7 acquisition, 8 delivery-trace

const TERMINAL_ORDER_STATUSES = ['DELIVERED', 'CANCELLED', 'REJECTED', 'PICKED_UP'];

export class CourierOfferSweepWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.work(SWEEP_QUEUE, { singletonKey: SWEEP_QUEUE }, async () => this.run());
    await this.boss.createQueue(SWEEP_QUEUE);
    // Defensive: the drain sends COURIER_DISPATCH jobs — make sure the queue exists even if
    // the dispatch worker's registration order ever changes (operability note, ADR §9).
    await this.boss
      .createQueue(QUEUE_NAMES.COURIER_DISPATCH)
      .catch((err: any) => console.warn(`[CourierOfferSweep] createQueue(${QUEUE_NAMES.COURIER_DISPATCH}) failed: ${err?.message}`));
    await this.boss
      .schedule(SWEEP_QUEUE, SWEEP_CRON, null, { singletonKey: SWEEP_QUEUE })
      .catch((err: any) => console.warn(`[CourierOfferSweep] schedule failed: ${err?.message}`));
    console.log('[CourierOfferSweep] scheduled (1-min offer-expiry + accept-timeout + dispatch-drain sweep)');
  }

  private async run() {
    const client = await this.pool.connect();
    try {
      const lock = await client.query('SELECT pg_try_advisory_lock($1) AS locked', [SWEEP_LOCK_ID]);
      if (!lock.rows[0]?.locked) return;
      try {
        await this.expireUnansweredOffers(client);
        await this.expireStaleAssignments(client);
        await this.drainDispatchQueue(client);
        await this.graceCancelExhausted(client);
      } finally {
        await client.query('SELECT pg_advisory_unlock($1)', [SWEEP_LOCK_ID]);
      }
    } catch (err) {
      console.error('[CourierOfferSweep] Error:', err);
    } finally {
      client.release();
    }
  }

  // ── Pass 1 — offer-expiry (deliver v2 §A, unchanged behavior, honest log) ──
  private async expireUnansweredOffers(client: any) {
    // Guarded transition: status='offered' AND past deadline → 'offered_expired'. Cross-tenant (one pass).
    // The order row is deliberately not touched.
    // B3: this UPDATE spans courier_assignments across ALL tenants in a single pass, so there
    // is no single app.current_tenant to set. Encapsulated in a SECURITY DEFINER fn
    // (app_sweep_expired_offers) that mirrors the original UPDATE … RETURNING and runs above RLS.
    const res = await client.query(`SELECT order_id, location_id FROM app_sweep_expired_offers()`);
    if (!res.rowCount) return;
    // Honest signal (ADR-dispatch-recovery Q4): the journal write is a RE-ENQUEUE, not a re-offer.
    // The genuine "assignment created / re-offered" claim is emitted only by handleDispatch on a
    // successful new binding (ORDER_ASSIGNMENT_CREATED).
    console.log(`[CourierOfferSweep] expired ${res.rowCount} unanswered offer(s) → re-enqueued for dispatch`);
    for (const row of res.rows) {
      try {
        // The re-enqueue is single-tenant (this expired offer → its location). courier_dispatch_queue
        // is keyed on app.current_tenant (Phase-1 isolate policy), so pin the GUC per row inside a
        // txn. Session-level advisory lock above survives these inner transactions.
        await client.query('BEGIN');
        await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [row.location_id]);
        await client.query(
          `INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at) VALUES ($1,$2,now())
           ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1`,
          [row.order_id, row.location_id],
        );
        await client.query('COMMIT');
        await this.messageBus.publish(dashboardChannel(row.location_id), { type: 'offer_expired', orderId: row.order_id });
      } catch (e) {
        await client.query('ROLLBACK').catch(() => {});
        console.error(`[CourierOfferSweep] re-enqueue failed for ${row.order_id}:`, e);
      }
    }
  }

  // ── Pass 2 — 'assigned' acceptance timeout (ADR-dispatch-recovery Q3) ──
  // An auto-assigned binding a courier never accepts (nor rejects) previously never expired —
  // stuck shift + stuck order. Reuses 'cancelled' (already in the mig-073 status CHECK — no enum
  // migration); the order_active_uniq then frees the order for a fresh binding via the journal.
  private async expireStaleAssignments(client: any) {
    const env = loadEnv();
    // Default 5 min — comfortably above the FE accept window (COURIER_ACCEPT_WINDOW_MS, 30s),
    // per R-OPEN-1, so the boundary accept-vs-sweep race is rare (and row-guarded anyway).
    const timeoutMs = parseInt(env.COURIER_ASSIGN_ACCEPT_TIMEOUT_MS || '300000', 10);
    // B3: cross-tenant SELECT on the BYPASSRLS operational pool (same posture as the drain below);
    // a NOBYPASSRLS flip needs a SECURITY DEFINER wrapper like app_sweep_expired_offers (R-FLAG follow-up).
    const res = await client.query(
      `SELECT id, order_id, shift_id, location_id FROM courier_assignments
       WHERE status = 'assigned' AND assigned_at < now() - ($1::int * interval '1 millisecond')`,
      [timeoutMs],
    );
    if (!res.rowCount) return;
    for (const row of res.rows) {
      try {
        await client.query('BEGIN');
        await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [row.location_id]);
        // Guarded: only a still-'assigned' row expires — a boundary accept wins the race cleanly.
        // No courier reliability penalty is recorded (R-ACC-5 standing constraint).
        const upd = await client.query(
          `UPDATE courier_assignments SET status = 'cancelled', cancelled_at = now(),
                  cancellation_reason = 'assign_accept_timeout'
           WHERE id = $1 AND status = 'assigned'`,
          [row.id],
        );
        if (upd.rowCount) {
          if (row.shift_id) {
            await client.query(`UPDATE courier_shifts SET status = 'available' WHERE id = $1`, [row.shift_id]);
          }
          await client.query(
            `INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at) VALUES ($1,$2,now())
             ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1`,
            [row.order_id, row.location_id],
          );
        }
        await client.query('COMMIT');
        if (upd.rowCount) {
          console.log(`[CourierOfferSweep] expired stale 'assigned' ${row.id} (accept-timeout) → re-enqueued for dispatch`);
          await this.messageBus.publish(dashboardChannel(row.location_id), { type: 'assignment_expired', orderId: row.order_id });
        }
      } catch (e) {
        await client.query('ROLLBACK').catch(() => {});
        console.error(`[CourierOfferSweep] accept-timeout expiry failed for assignment ${row.id}:`, e);
      }
    }
  }

  // ── Pass 3 — the drain / pump (ADR-dispatch-recovery Option C; DoD-1 standing regression) ──
  // The ONLY consumer of courier_dispatch_queue: pumps every resident journal row into a
  // COURIER_DISPATCH job. Idempotency layers: pg-boss singletonKey (one in-flight job per order)
  // + courier_assignments_order_active_uniq (≤1 active binding per order, hard DB backstop) +
  // handleDispatch's pre-check. A job lost by pg-boss self-heals: the row survives, the key frees
  // on terminal job state, the next tick re-pumps.
  private async drainDispatchQueue(client: any) {
    // B3: deliberate cross-tenant read on the BYPASSRLS operational pool; each pumped job carries
    // its own location_id so handleDispatch operates within one tenant (isolation preserved).
    const due = await client.query(`SELECT order_id, location_id FROM courier_dispatch_queue`);
    if (!due.rowCount) return;
    let sent = 0;
    for (const row of due.rows) {
      try {
        await this.boss.send(
          QUEUE_NAMES.COURIER_DISPATCH,
          { orderId: row.order_id, locationId: row.location_id },
          { singletonKey: row.order_id },
        );
        sent++;
      } catch (e) {
        console.error(`[CourierOfferSweep] dispatch pump failed for ${row.order_id}:`, e);
      }
    }
    console.log(`[CourierOfferSweep] pumped ${sent}/${due.rowCount} journal row(s) → ${QUEUE_NAMES.COURIER_DISPATCH}`);
  }

  // ── Pass 4 — grace-window auto-cancel — 🔴 FLAG-OFF dark (R-NEEDS-HUMAN-1) ──
  // After exhaustion set orders.dispatch_exhausted_at + alerted the owner, owner inaction must not
  // equal permanent customer silence: past DISPATCH_OWNER_GRACE_MS the order auto-transitions to
  // the customer-honest terminal CANCELLED (+ honest terminal push). Ships DISPATCH_OWNER_GRACE_ENABLED
  // =false until the operator ratifies at STOP-ETHICS. NOTE for that ratification: the order state
  // machine has no CONFIRMED/PREPARING/READY → CANCELLED edge today, so this pass uses a
  // status-guarded direct UPDATE (a deliberate widened terminal edge, mirroring the
  // updateOrderStatus guard + history shape) — the human gate covers this edge too.
  private async graceCancelExhausted(client: any) {
    const env = loadEnv();
    if (env.DISPATCH_OWNER_GRACE_ENABLED !== 'true') return;
    const graceMs = parseInt(env.DISPATCH_OWNER_GRACE_MS || '900000', 10);
    const res = await client.query(
      `SELECT o.id, o.location_id FROM orders o
       WHERE o.dispatch_exhausted_at IS NOT NULL
         AND o.dispatch_exhausted_at < now() - ($1::int * interval '1 millisecond')
         AND o.status NOT IN ('DELIVERED','CANCELLED','REJECTED','PICKED_UP')
         AND NOT EXISTS (
           SELECT 1 FROM courier_assignments ca
           WHERE ca.order_id = o.id AND ca.status IN ('offered','assigned','accepted','picked_up')
         )`,
      [graceMs],
    );
    if (!res.rowCount) return;
    for (const row of res.rows) {
      try {
        await client.query('BEGIN');
        await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [row.location_id]);
        const cur = await client.query(`SELECT status FROM orders WHERE id = $1 FOR UPDATE`, [row.id]);
        const st = cur.rows[0]?.status;
        if (!st || TERMINAL_ORDER_STATUSES.includes(st)) {
          await client.query('ROLLBACK');
          continue;
        }
        // Status-guarded terminal write (anti-race, same shape as updateOrderStatus).
        const upd = await client.query(
          `UPDATE orders SET status = 'CANCELLED', timeout_at = NULL WHERE id = $1 AND status = $2`,
          [row.id, st],
        );
        if (!upd.rowCount) {
          await client.query('ROLLBACK');
          continue;
        }
        // Audit trail (best-effort, savepoint-guarded like updateOrderStatus).
        try {
          await client.query('SAVEPOINT grace_history_ins');
          await client.query(
            `INSERT INTO order_status_history (order_id, location_id, from_status, to_status, actor, comment)
             VALUES ($1, $2, $3, 'CANCELLED', 'system:dispatch_grace', $4)`,
            [row.id, row.location_id, st, 'dispatch_exhausted'],
          );
          await client.query('RELEASE SAVEPOINT grace_history_ins');
        } catch {
          try { await client.query('ROLLBACK TO SAVEPOINT grace_history_ins'); } catch { /* no tx */ }
        }
        await client.query('COMMIT');
        // Post-commit: live deltas + the honest customer TERMINAL push (never a false "on its way").
        const nowIso = new Date().toISOString();
        await this.messageBus.publish(orderChannel(row.id), {
          type: 'order.status', orderId: row.id, status: 'CANCELLED', locationId: row.location_id,
          timestamp: nowIso, statusAtField: null, statusAt: null,
        });
        await this.messageBus.publish(dashboardChannel(row.location_id), {
          type: 'order.status', data: { orderId: row.id, status: 'CANCELLED', statusUpdatedAt: nowIso },
        });
        await this.boss.send(QUEUE_NAMES.NOTIFY_CUSTOMER_STATUS, {
          orderId: row.id, locationId: row.location_id, event: 'CANCELLED',
        });
        console.log(`[CourierOfferSweep] grace-window expired for ${row.id} → CANCELLED (dispatch_exhausted)`);
      } catch (e) {
        await client.query('ROLLBACK').catch(() => {});
        console.error(`[CourierOfferSweep] grace-cancel failed for order ${row.id}:`, e);
      }
    }
  }
}
