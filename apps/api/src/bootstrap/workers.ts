// @ts-nocheck
// Background worker startup extracted from server.ts main(). This is the BODY of
// the boot-budget Promise.race IIFE — the race + WORKER_BOOT_BUDGET_MS timeout +
// the "continue to listen" catch REMAIN in main() (boot resilience, incident
// 2026-06-21: the menu must serve even when workers are degraded). This function
// only constructs + starts the workers in their original order and returns the
// heartbeat handles main() needs for shutdown. @ts-nocheck mirrors server.ts so
// no new type surface is introduced by the move.
import { QUEUE_NAMES } from '../lib/registry.js';
import { WorkerHeartbeat } from '../lib/worker/heartbeat.js';
import { LivenessChecker } from '../workers/liveness-checker.js';
import { DwellMonitorWorker } from '../workers/dwell-monitor.js';
import { LifecycleHandlers } from '../workers/lifecycle-handlers.js';
import { SignalRaiserWorker } from '../workers/signal-raiser.js';
import { VelocityIncrementer } from '../lib/signals/velocity-increment.js';
import { AnonymizerService } from '../lib/anonymizer/index.js';
import { AnonymizerRetentionWorker } from '../workers/anonymizer-retention.js';
import { GdprErasureWorker } from '../workers/anonymizer-gdpr.js';
import { AccessRequestNotifyWorker } from '../workers/access-request-notify.js';
import { AccessRequestRetentionWorker } from '../workers/access-request-retention.js';
import { AcquisitionRetentionWorker } from '../workers/acquisition-retention.js';
import { DeliveryTraceRetentionWorker } from '../workers/delivery-trace-retention.js';
import { registerNotifySubscriptions } from './messaging.js';

export interface BackgroundWorkerDeps {
  pool: any;
  backupPool: any;
  queue: any;
  messageBus: any;
  notifyWorker: any;
}

export interface BackgroundWorkerHandles {
  heartbeats: any[];
}

/**
 * Constructs and starts all apps/api background workers in their original
 * (unchanged) order. Caller wraps this in the boot-budget Promise.race so a
 * hung/throwing worker startup never blocks fastify.listen.
 */
export async function startBackgroundWorkers(deps: BackgroundWorkerDeps): Promise<BackgroundWorkerHandles> {
  const { pool, backupPool, queue, messageBus, notifyWorker } = deps;

  // Register pg-boss notify workers (queue.work wraps pg-boss v10 array-of-jobs).
  await queue.work(QUEUE_NAMES.NOTIFY_DISPATCH, async (data: any) => notifyWorker.handleDispatch({ data }));
  await queue.work(QUEUE_NAMES.NOTIFY_CUSTOMER_STATUS, async (data: any) => notifyWorker.handleCustomerStatus({ data }));
  await queue.work(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, async (data: any) => notifyWorker.handleTelegramSend({ data }));
  console.log('[API] pg-boss workers registered');

  const { CourierDispatchWorker } = await import('../workers/courier-dispatch.js');
  const { CourierCronWorker } = await import('../workers/courier-cron.js');
  const { CourierEventsWorker } = await import('../workers/courier-events.js');
  const { SettlementCronWorker } = await import('../workers/settlement-cron.js');
  const { BackupCronWorker } = await import('../workers/backup/index.js');
  const { BackupVerifyWorker } = await import('../workers/backup/backup-verify-scheduled.js');

  const courierDispatchWorker = new CourierDispatchWorker(pool, queue, messageBus);
  const courierCronWorker = new CourierCronWorker(pool, queue.boss, messageBus);
  const courierEventsWorker = new CourierEventsWorker(pool, messageBus);
  const settlementCronWorker = new SettlementCronWorker(pool, queue.boss);
  const backupCronWorker = new BackupCronWorker(pool, backupPool, queue.boss, messageBus);
  const backupVerifyWorker = new BackupVerifyWorker(pool, queue.boss);

  await courierDispatchWorker.start();
  await courierCronWorker.start();
  await courierEventsWorker.start();
  await settlementCronWorker.start();
  await backupCronWorker.start();
  await backupVerifyWorker.start();

  // Dwell Monitor Worker
  const dwellMonitorWorker = new DwellMonitorWorker(pool, queue.boss, messageBus);
  await dwellMonitorWorker.start();

  // Order Timeout Sweep — standalone 1-min reconciliation + detection. Safety net
  // for the per-order order.timeout handler (apps/worker): recovers overdue PENDING
  // orders whose per-order job was lost, and counts overdue-but-undrained jobs.
  const { OrderTimeoutSweepWorker } = await import('../workers/order-timeout-sweep.js');
  const orderTimeoutSweepWorker = new OrderTimeoutSweepWorker(pool, queue.boss, messageBus);
  await orderTimeoutSweepWorker.start();
  const { CourierOfferSweepWorker } = await import('../workers/courier-offer-sweep.js');
  const courierOfferSweepWorker = new CourierOfferSweepWorker(pool, queue.boss, messageBus);
  await courierOfferSweepWorker.start();

  // Lifecycle Handlers (auto-resolve alerts on order transitions)
  const lifecycleHandlers = new LifecycleHandlers(pool, queue.boss, messageBus);
  await lifecycleHandlers.start();

  // P5-0 Anonymizer Service + Workers
  const anonymizerService = new AnonymizerService(pool, messageBus);
  const anonymizerRetentionWorker = new AnonymizerRetentionWorker(pool, queue.boss, messageBus, anonymizerService);
  const gdprErasureWorker = new GdprErasureWorker(pool, queue.boss, messageBus, anonymizerService);
  await anonymizerRetentionWorker.start();
  await gdprErasureWorker.start();

  // P31 — Worker Heartbeats. ADR-dispatch-recovery R3′ (B5): the set below MUST equal
  // ReconciliationWorker A6.EXPECTED_WORKERS (8 ids) — the heartbeat is a cadence-independent
  // 15s timer proving the PROCESS is alive, so hourly/nightly workers beat too. Instrumenting
  // the missing 4 (instead of trimming A6) keeps backup-hourly (data-recovery red-line) and
  // liveness-checker (watcher-of-the-watcher, caught by nightly A6) monitored with no false DRIFT.
  const heartbeatConfigs = [
    { workerId: 'dispatcher', jobName: QUEUE_NAMES.COURIER_DISPATCH },
    { workerId: 'settlement-cron', jobName: QUEUE_NAMES.SETTLEMENT_CRON },
    { workerId: 'dwell-monitor', jobName: QUEUE_NAMES.DWELL_MONITOR },
    { workerId: 'anonymizer-retention', jobName: QUEUE_NAMES.ANONYMIZER_RETENTION },
    { workerId: 'backup-hourly', jobName: QUEUE_NAMES.BACKUP_HOURLY },
    { workerId: 'signal-raiser', jobName: QUEUE_NAMES.SIGNAL_RAISER },
    { workerId: 'courier-stale_check', jobName: QUEUE_NAMES.COURIER_STALE_CHECK },
    { workerId: 'liveness-checker', jobName: QUEUE_NAMES.LIVENESS_CHECK },
  ];
  const heartbeats = heartbeatConfigs.map((cfg) => {
    const hb = new WorkerHeartbeat(pool, cfg);
    hb.start();
    return hb;
  });

  // Signal Raiser Worker (P26 — anti-fake signals)
  const signalRaiserWorker = new SignalRaiserWorker(pool, queue.boss, messageBus);
  await signalRaiserWorker.start();

  // Velocity Incrementer (P26 — async velocity counter)
  const velocityIncrementer = new VelocityIncrementer(pool, queue.boss);
  await queue.work(QUEUE_NAMES.VELOCITY_FLUSH, async (data: any) => velocityIncrementer.handleFlush({ data }));

  // Currency Rates Refresh Worker (hourly, fetches ALL→EUR from fawazahmed0)
  const { RatesRefreshWorker } = await import('../workers/rates-refresh.js');
  const ratesRefreshWorker = new RatesRefreshWorker(pool, queue.boss);
  await ratesRefreshWorker.start();

  // Soft access gate — notify + retention/reconcile crons run unconditionally (data
  // hygiene); the public POST route itself is gated separately by ACCESS_GATE_PUBLIC_ENABLED.
  const accessRequestNotifyWorker = new AccessRequestNotifyWorker(pool, queue.boss, messageBus);
  await accessRequestNotifyWorker.start();
  const accessRequestRetentionWorker = new AccessRequestRetentionWorker(pool, queue.boss, messageBus);
  await accessRequestRetentionWorker.start();
  const acquisitionRetentionWorker = new AcquisitionRetentionWorker(pool, queue.boss, messageBus);
  await acquisitionRetentionWorker.start();
  const deliveryTraceRetentionWorker = new DeliveryTraceRetentionWorker(pool, queue.boss, messageBus);
  await deliveryTraceRetentionWorker.start();

  registerNotifySubscriptions(messageBus, queue.boss);

  // P31 — Worker Liveness Checker (singleton cron, 60s)
  const livenessChecker = new LivenessChecker(pool, queue.boss, messageBus);
  await livenessChecker.start();

  // B5 (ADR-dispatch-recovery, Option R3′ — closes ADR-golive R9): the full READ-ONLY nightly
  // ReconciliationWorker is re-registered. Its A6 worker-liveness check watches the true set of
  // 8 heartbeating ids (see heartbeatConfigs above) so it yields no false DRIFT; nightly
  // detection+alert complements the sweeps' sub-minute recovery. Zero mutations, one 03:00 UTC
  // read burst on the existing pool.
  const { ReconciliationWorker } = await import('../workers/reconciliation.js');
  const reconciliationWorker = new ReconciliationWorker(pool, queue.boss, messageBus);
  await reconciliationWorker.start();

  // P5-5 — Free-tier watch (hourly, monitors Free tier limits)
  const { collectFreeTierMetrics } = await import('../workers/free-tier-watch.js');
  await queue.boss.work(QUEUE_NAMES.FREE_TIER_WATCH, async () => {
    try {
      const metrics = await collectFreeTierMetrics(pool);
      console.log(`[FreeTier] Watch complete: ${metrics.status} (DB: ${metrics.dbPct}%)`);
    } catch (err: any) {
      console.error('[FreeTier] Watch failed:', err.message);
    }
  });
  await queue.boss.schedule(QUEUE_NAMES.FREE_TIER_WATCH, '0 * * * *');

  return { heartbeats };
}
