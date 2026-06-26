// @ts-nocheck
import crypto from 'node:crypto';
import Fastify from 'fastify';
import { loadEnv } from '@deliveryos/config';
import { createOperationalPool } from '@deliveryos/db';
import { RedisMessageBus, PgBossQueueProvider } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, ALL_QUEUES, CUSTOMER_PUSH_EVENTS, orderChannel, dashboardChannel } from './lib/registry.js';
import { assertSchemaCurrent } from './lib/schema-guard.js';
import { SYNTHETIC_COURIER_EMAIL_HASH } from './lib/synthetic-courier.js';
import Redis from 'ioredis';
import pg from 'pg';
import { z, type ZodTypeAny } from 'zod';
import healthRoutes from './routes/health.js';
import authRoutes from './routes/auth.js';
import courierRoutes from './routes/couriers.js';
import orderRoutes from './routes/orders.js';
import categoryRoutes from './routes/owner/categories.js';
import productRoutes from './routes/owner/products.js';
import modifierGroupRoutes from './routes/owner/modifier-groups.js';
import menuAvailabilityRoutes from './routes/owner/menu-availability.js';
import locationRoutes from './routes/owner/locations.js';
import publicMenuRoutes from './routes/public/menu.js';
import ssrRoutes from './routes/public/ssr.js';
import brandingPreviewRoutes from './routes/public/branding-preview.js';
import seoRoutes from './routes/public/seo.js';
import clientFlowRoutes from './routes/public/client-flow.js';
import pwaRoutes from './routes/public/pwa.js';
import vapidRoutes from './routes/public/vapid.js';
import telemetryRoutes from './routes/public/telemetry.js';
import accessRequestRoutes from './routes/public/access-requests.js';
import funnelRoutes from './routes/public/funnel.js';
import { AccessRequestNotifyWorker } from './workers/access-request-notify.js';
import { AccessRequestRetentionWorker, assertAccessRequestSchedules } from './workers/access-request-retention.js';
import ownerThemeRoutes from './routes/owner/themes.js';
import publicThemeRoutes from './routes/public/theme.js';
import ownerNotificationRoutes from './routes/owner/notifications.js';
import menuImportRoutes from './routes/owner/menu-import.js';
import menuTranslateRoutes from './routes/owner/menu-translate.js';
import courierAuthRoutes from './routes/courier/auth.js';
import courierMeRoutes from './routes/courier/me.js';
import ownerCourierRoutes from './routes/owner/couriers.js';
import ownerCourierInvitesRoutes from './routes/owner/courier-invites.js';
import onboardingRoutes from './routes/owner/onboarding.js';
import activationRoutes from './routes/owner/activation.js';
import orderMessageRoutes from './routes/order-messages.js';
import customerOrderRoutes from './routes/customer/orders.js';
import ownerSettlementRoutes from './routes/owner/settlements.js';
import ownerDashboardRoutes from './routes/owner/dashboard.js';
import courierSettlementRoutes from './routes/courier/settlements.js';
import courierAssignmentsRoutes from './routes/courier/assignments.js';
import courierShiftsRoutes from './routes/courier/shifts.js';
import ownerAlertRoutes from './routes/owner/alerts.js';
import ownerDwellSettingsRoutes from './routes/owner/dwell-settings.js';
import fastifyMultipart from '@fastify/multipart';
import fastifyRateLimit from '@fastify/rate-limit';
import fastifyStatic from '@fastify/static';
import fastifyCors from '@fastify/cors';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getFastifyLoggerConfig, correlationStore } from './lib/logger.js';
import { ApiError, isContractCode, rateLimitError, buildErrorEnvelope } from './lib/api-error.js';
import { initSentry, getSentry } from './lib/sentry.js';
import { WorkerHeartbeat } from './lib/worker/heartbeat.js';
import { LivenessChecker } from './workers/liveness-checker.js';
import securityHeadersPlugin from './lib/security/headers.js';

// Safe __dirname fallback for dual ESM/CJS bundling
const dirName = typeof __dirname !== 'undefined' ? __dirname : path.dirname(fileURLToPath((import.meta as any).url));
import authPlugin from './plugins/auth.js';
import { isDevPath, isDevRequestAuthorized } from './plugins/dev-guard.js';
import multipart from '@fastify/multipart';
import { setupWebSocket } from './websocket.js';
import { setupShutdown } from './shutdown.js';
import { CsvMenuParser } from './lib/csv-parser.js';
import { AiOcrParser } from './lib/ai-ocr-parser.js';
import { LocalFsStorageProvider } from './lib/local-storage.js';
import { R2StorageProvider } from './lib/r2-storage.js';
import { LibreTranslateProvider } from './lib/libretranslate-provider.js';
import { TelegramAdapter } from './notifications/adapters/telegram.js';
import { WebPushAdapter } from './notifications/adapters/webpush.js';
import { NotificationDispatcher } from './notifications/provider.js';
import { RetryPolicy } from './notifications/retry.js';
import { NotificationWorker } from './notifications/workers/index.js';
import { TelegramPoller } from './notifications/workers/telegram.poll.js';
import { DwellMonitorWorker } from './workers/dwell-monitor.js';
import { LifecycleHandlers } from './workers/lifecycle-handlers.js';
import { SignalRaiserWorker } from './workers/signal-raiser.js';
import { VelocityIncrementer } from './lib/signals/velocity-increment.js';
import ownerFallbackRoutes from './routes/owner/fallback.js';
import ownerRevealContactRoutes from './routes/owner/reveal-contact.js';
import publicFallbackConfigRoutes from './routes/public/fallback-config.js';
import ratesRoutes from './routes/public/rates.js';
import mockAuthRoutes from './routes/dev/mock-auth.js';
import spaProxyRoutes from './routes/spa-proxy.js';
import customerOtpRoutes from './routes/customer/otp.js';
import customerTrackRoutes from './routes/customer/track.js';
import customerPushRoutes from './routes/customer/push.js';
import ownerPushRoutes from './routes/owner/push.js';
import ownerOrderMetaRoutes from './routes/owner/order-meta.js';
import ownerSignalRoutes from './routes/owner/signals.js';
import ownerGdprRoutes from './routes/owner/gdpr.js';
import ownerPromotionRoutes from './routes/owner/promotions.js';
import telegramWebhookRoutes from './routes/telegram-webhook.js';
import { AnonymizerService } from './lib/anonymizer/index.js';
import { AnonymizerRetentionWorker } from './workers/anonymizer-retention.js';
import { GdprErasureWorker } from './workers/anonymizer-gdpr.js';
import { MemoryService, getMemoryService } from './lib/memory.js';
import { registerNotifySubscriptions } from './bootstrap/messaging.js';

declare module 'fastify' {
  interface FastifyInstance {
    db: any;
    redis: any;
    wss: any;
    memory: import('./lib/memory.js').MemoryService;
  }
  interface FastifyReply {
    /**
     * A2 (ADR-0010): emit the structured error envelope for a RETURN-based ad-hoc site (the
     * drop-in for `reply.status(n).send({ error })`). Same envelope as setErrorHandler (shared
     * builder), incl. server correlationId + x-correlation-id echo. `code` must be SCREAMING_SNAKE.
     * Use this for sites that return mid-handler; THROW `new ApiError(...)` where a throw is cleaner.
     */
    sendError(
      status: number,
      code: string,
      message: string,
      opts?: import('./lib/api-error.js').ErrorEnvelopeOpts,
    ): FastifyReply;
  }
}

async function main() {
  const env = loadEnv();

  // P31 — Init Sentry (if DSN configured)
  if (env.SENTRY_DSN) {
    initSentry(env.SENTRY_DSN, env.GIT_SHA);
    console.log('[API] Sentry initialized');
  }

  // Last-resort process guards. A single floating promise rejection or uncaught
  // error must NOT take the whole web process down — that drops every live
  // WebSocket (owner dashboard, courier, customer tracking) with it. Sentry only
  // installs its own handlers when a DSN is set, so register ours unconditionally:
  // log, forward to Sentry if present, and keep serving. Registering an
  // 'uncaughtException' listener also suppresses Node's default crash-and-exit.
  // Genuinely wedged states are still caught by Fly's liveness probe (/livez).
  process.on('unhandledRejection', (reason: unknown) => {
    console.error('[API] unhandledRejection (kept alive):', reason);
    getSentry()?.captureException(reason);
  });
  process.on('uncaughtException', (err: Error) => {
    console.error('[API] uncaughtException (kept alive):', err);
    getSentry()?.captureException(err);
  });

  const fastify = Fastify({
    logger: getFastifyLoggerConfig(),
    maxHeaderSize: 32768,
    bodyLimit: 10 * 1024 * 1024, // 10 MB (multipart uploads need it)
    // ADR-0010 (A1/B6): the correlation id is SERVER-AUTHORITATIVE — always generated
    // with crypto.randomUUID (governance), never read from an inbound header (that would
    // let a client forge a victim's "support code" / poison logs). `requestIdHeader` is
    // intentionally NOT set to a header name so Fastify cannot trust inbound; the inbound
    // x-correlation-id is captured separately as a sanitized, identity-free clientTraceId
    // in the onRequest hook below.
    genReqId: () => crypto.randomUUID(),
    requestIdHeader: false,
    requestIdLogLabel: 'correlationId',
  });

  // Custom validator/serializer compilers for Zod v3 compat.
  // fastify-type-provider-zod@6.x requires Zod v4 (peerDep zod>=4.1.5),
  // but we use Zod v3.25.x. Use Zod's native safeParse instead.
  fastify.setValidatorCompiler(({ schema }) => {
    return (data) => {
      const result = (schema as ZodTypeAny).safeParse(data);
      if (!result.success) {
        return { error: result.error };
      }
      return { value: result.data };
    };
  });
  fastify.setSerializerCompiler(({ schema }) => {
    const zodSchema = schema as ZodTypeAny;
    return (data) => {
      const result = zodSchema.safeParse(data);
      if (!result.success) {
        throw new Error(String(result.error));
      }
      return JSON.stringify(result.data);
    };
  });

  fastify.addHook('onRequest', async (request, reply) => {
    if (process.env.NODE_ENV === 'production') {
      reply.header('Strict-Transport-Security', 'max-age=31536000; includeSubDomains');
    }
    reply.header('X-Content-Type-Options', 'nosniff');
    // Allow iframe embedding when embed=true (widget embeds on restaurant websites)
    if (!request.url.includes('embed=true')) {
      reply.header('X-Frame-Options', 'SAMEORIGIN');
    }
    reply.header('Referrer-Policy', 'strict-origin-when-cross-origin');
  });

  
  // P34: Strict CORS — restrictive default; public routes override via hook
  fastify.register(fastifyCors, {
    origin: (origin: string, cb: any) => {
      if (!origin) return cb(null, true);
      cb(null, false);
    },
    credentials: false,
  });
  // Override CORS for public routes (menu read + order POST)
  fastify.addHook('onRequest', async (request, reply) => {
    if (request.url.startsWith('/public/locations/') ||
        request.url.startsWith('/s/') ||
        (request.url.startsWith('/api/orders') && request.method === 'POST')) {
      reply.header('Access-Control-Allow-Origin', '*');
    }
  });

  fastify.register(fastifyStatic, {
    root: path.join(dirName, '..', 'public'),
    prefix: '/',
    cacheControl: true,
    maxAge: '365d',
  });

  // Override cache headers after fastify-static's send() applies maxAge.
  // Note: for directly-served static files, fastify-static may pipe to res directly
  // bypassing onSend. The SPA fallback (setNotFoundHandler) uses reply.sendFile()
  // which DOES trigger onSend for HTML routes.
  fastify.addHook('onSend', async (_request, reply, payload) => {
    const ct = reply.getHeader('content-type');
    if (!ct) return;
    const type = String(ct);
    if (type.startsWith('text/html')) {
      reply.header('Cache-Control', 'no-cache, no-store, must-revalidate');
    } else if (type.startsWith('text/css') || type.startsWith('application/javascript') || type.startsWith('text/javascript')) {
      reply.header('Cache-Control', 'public, max-age=31536000, immutable');
    }
  });

  // Subdomain routing middleware
  fastify.addHook('onRequest', async (request, reply) => {
    const host = request.hostname.split(':')[0]; // remove port
    // e.g. margherita.dowiz.org
    if (host.endsWith('dowiz.org')) {
      const parts = host.split('.');
      if (parts.length >= 3) {
        const slug = parts[0];
        if (!['www', 'api', 'app'].includes(slug) && !request.url.startsWith('/api/') && !request.url.startsWith('/public/') && !request.url.startsWith('/s/') && !request.url.startsWith('/admin') && !request.url.startsWith('/courier') && !request.url.startsWith('/dashboard') && !/\.\w{2,5}(\?|$)/.test(request.url)) {
          // It's a tenant subdomain, rewrite URL internally to /s/:slug
          // Preserve query strings
          const urlObj = new URL(request.url, `http://${request.hostname}`);
          request.raw.url = `/s/${slug}${urlObj.search}`;
        }
      }
    }
  });

  // P31 — Correlation ID for structured logging (ADR-0010 A1/B6).
  // The id is the SERVER-generated request id (crypto.randomUUID via genReqId) — never the
  // inbound header. The inbound x-correlation-id is demoted to a sanitized, identity-free
  // `clientTraceId` (widget/WS stitching only): bounded charset + length so it can't inject
  // newlines/control chars into Pino, and never used as req.id, the user-facing code, or
  // joined to user identity.
  fastify.addHook('onRequest', async (request) => {
    const correlationId = String(request.id); // server-authoritative
    const rawInbound = request.headers['x-correlation-id'];
    const clientTraceId =
      typeof rawInbound === 'string' && /^[A-Za-z0-9._-]{1,128}$/.test(rawInbound)
        ? rawInbound
        : undefined;
    (request as any).clientTraceId = clientTraceId;
    // Overwrite the inbound header so nothing downstream can raw-trust it as the id.
    request.headers['x-correlation-id'] = correlationId;
    correlationStore.enterWith(correlationId);
  });

  fastify.decorate('wss', null);

  console.log('[API] Initializing Operational Pool...');
  const pool = createOperationalPool();
  fastify.decorate('db', pool);

  // Dedicated backup pool to avoid starving operational queries
  const { Pool } = pg;
  const backupPool = new Pool({
    connectionString: env.DATABASE_URL_MIGRATIONS,
    max: env.BACKUP_POOL_SIZE || 2
  });

  console.log('[API] Initializing MessageBus and Redis...');
  // CRITICAL: MessageBus MUST use session pool (not operational) for LISTEN/NOTIFY to work
  // Operational pool may use transaction pooler which doesn't support LISTEN/NOTIFY
  const { createSessionPool } = await import('@deliveryos/db');
  const messageBusPool = createSessionPool();
  // Fail fast if this build's schema head is missing from the DB (i.e. migrations
  // did not run before boot). No-op in dev/unbundled. See lib/schema-guard.ts.
  await assertSchemaCurrent(messageBusPool);
  const messageBus = new RedisMessageBus(messageBusPool);
  await messageBus.connect();
  console.log('[API] MessageBus connected with session pool for LISTEN/NOTIFY support');
  
  // CRITICAL: pg-boss uses operational pool (has DDL permissions to create queue tables)
  // pg-boss will be instantiated below with explicit DATABASE_URL_OPERATIONAL
  
  // CRITICAL: Ensure MessageBus is fully connected before registering subscriptions
  // Add a small delay to ensure LISTEN commands are processed
  await new Promise(resolve => setTimeout(resolve, 100));
  console.log('[API] MessageBus ready for subscriptions');

  console.log('[API] Initializing Queue Provider...');
  // CRITICAL: pg-boss uses session-mode connection (port 5432) for LISTEN/NOTIFY.
  // Transaction pooler (port 6543) blocks LISTEN/NOTIFY. Session port is required.
  // Construct session URL: same as operational but port 5432
  const opUrl = new URL(env.DATABASE_URL_OPERATIONAL);
  opUrl.port = '5432';
  const queue = new PgBossQueueProvider(opUrl.toString());
  await queue.start();
  
  // Attempt to verify/create queues. Some queues require DDL (CREATE TABLE) on pgboss schema
  // which the runtime role may not have. Pre-created queues succeed silently; new ones may warn.
  console.log('[API] Verifying notification queues...');
  for (const qName of ALL_QUEUES) {
    await queue.boss.createQueue(qName).catch((err: any) => {
      console.warn(`[API] Queue "${qName}" not pre-created: ${err.message}`);
    });
  }
  // Verify queue table existence in pgboss schema
  try {
    const queueCheck = await messageBusPool.query(
      `SELECT COUNT(*) AS cnt FROM information_schema.tables WHERE table_schema = 'pgboss' AND table_name = 'job'`
    );
    if (queueCheck.rows[0]?.cnt === '0') {
      console.warn('[API] ⚠️  pgboss.job table not found — queue operations may fail. Run migrations first.');
    } else {
      console.log('[API] ✅ pgboss.job table verified');
    }
  } catch (err) {
    console.warn('[API] ⚠️  Could not verify pgboss schema:', (err as Error).message);
  }
  console.log('[API] Notification queues created');

  console.log('[API] Initializing Providers...');

  // Memory Service (mem0 — persistent agent memory via Ollama)
  const memoryService = getMemoryService();
  memoryService.initialize().catch((err) => {
    console.warn('[API] MemoryService init failed, continuing without memory:', (err as Error).message);
  });
  fastify.decorate('memory', memoryService);

  const parsers = {
      'csv': new CsvMenuParser(),
      'ai-ocr': new AiOcrParser(memoryService),
    };
  // Durable object storage for product images. Cloudflare R2 when configured
  // (survives redeploys, shared across machines); otherwise the local fs
  // (STORAGE_DIR for a mounted volume, or the ephemeral default in dev).
  const storage = process.env.R2_BUCKET && process.env.R2_ENDPOINT
    ? new R2StorageProvider()
    : new LocalFsStorageProvider(process.env.STORAGE_DIR || 'tmp/imports');
  const translation = new LibreTranslateProvider();

  // Notification Providers
  const telegramAdapter = new TelegramAdapter(env.TELEGRAM_BOT_TOKEN || '');
  const notifyDispatcher = new NotificationDispatcher();
  notifyDispatcher.register('telegram', telegramAdapter);
  // P0-2 (ADR-p0-privacy-hardening): WhatsApp/Baileys channel removed — it streamed
  // customer PII to Meta via an unofficial client. Telegram + push + email remain.

  if (env.VAPID_PUBLIC_KEY && env.VAPID_PRIVATE_KEY) {
    const subject = env.VAPID_SUBJECT ? (env.VAPID_SUBJECT.startsWith('mailto:') ? env.VAPID_SUBJECT : `mailto:${env.VAPID_SUBJECT}`) : 'mailto:admin@deliveryos.local';
    const webPushAdapter = new WebPushAdapter(env.VAPID_PUBLIC_KEY, env.VAPID_PRIVATE_KEY, subject);
    notifyDispatcher.register('push', webPushAdapter);
  }

const retryPolicy = new RetryPolicy();
  const notifyWorker = new NotificationWorker(pool, queue.boss, notifyDispatcher, retryPolicy, memoryService);
  
   // Register pg-boss workers
   // NOTE: queue.work() wraps pg-boss v10 array-of-jobs callback, extracting job.data per job
   // Direct queue.boss.work() would receive [job] not job
   //
   // BOOT RESILIENCE (incident 2026-06-21): a pg-boss createQueue permission error
   // wedged boot BEFORE fastify.listen, taking the whole storefront down. Bound the
   // ENTIRE worker/queue startup in a budget: if it hangs past WORKER_BOOT_BUDGET_MS
   // or throws, log and proceed to listen anyway — the menu must serve even with
   // degraded background workers. Workers keep starting in the background.
   let heartbeats: any[] = [];
   const WORKER_BOOT_BUDGET_MS = 25_000;
   await Promise.race([
    (async () => {
   await queue.work(QUEUE_NAMES.NOTIFY_DISPATCH, async (data: any) => notifyWorker.handleDispatch({ data }));
   await queue.work(QUEUE_NAMES.NOTIFY_CUSTOMER_STATUS, async (data: any) => notifyWorker.handleCustomerStatus({ data }));
   await queue.work(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, async (data: any) => notifyWorker.handleTelegramSend({ data }));
   console.log('[API] pg-boss workers registered');

  const { CourierDispatchWorker } = await import('./workers/courier-dispatch.js');
  const { CourierCronWorker } = await import('./workers/courier-cron.js');
  const { CourierEventsWorker } = await import('./workers/courier-events.js');
  const { SettlementCronWorker } = await import('./workers/settlement-cron.js');
  const { BackupCronWorker } = await import('./workers/backup/index.js');
  const { BackupVerifyWorker } = await import('./workers/backup/backup-verify-scheduled.js');
  
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

  // Nightly Reconciliation Worker — temporarily removed (esbuild bundle issue). Re-add in separate deploy.

  // Order Timeout Sweep — standalone 1-min reconciliation + detection. Safety net
  // for the per-order order.timeout handler (apps/worker): recovers overdue PENDING
  // orders whose per-order job was lost, and counts overdue-but-undrained
  // order.timeout jobs (the lost-consumer signal). Lives here (not on the removed
  // ReconciliationWorker) so detection cannot lose its host; apps/api does not auto-stop.
  const { OrderTimeoutSweepWorker } = await import('./workers/order-timeout-sweep.js');
  const orderTimeoutSweepWorker = new OrderTimeoutSweepWorker(pool, queue.boss, messageBus);
  await orderTimeoutSweepWorker.start();

  // Lifecycle Handlers (auto-resolve alerts on order transitions)
  const lifecycleHandlers = new LifecycleHandlers(pool, queue.boss, messageBus);
  await lifecycleHandlers.start();

  // P5-0 Anonymizer Service + Workers
  const anonymizerService = new AnonymizerService(pool, messageBus);
  const anonymizerRetentionWorker = new AnonymizerRetentionWorker(pool, queue.boss, messageBus, anonymizerService);
  const gdprErasureWorker = new GdprErasureWorker(pool, queue.boss, messageBus, anonymizerService);
  await anonymizerRetentionWorker.start();
  await gdprErasureWorker.start();

  // P31 — Worker Heartbeats (critical workers)
  const heartbeatConfigs = [
    { workerId: 'dispatcher', jobName: QUEUE_NAMES.COURIER_DISPATCH },
    { workerId: 'settlement-cron', jobName: QUEUE_NAMES.SETTLEMENT_CRON },
    { workerId: 'dwell-monitor', jobName: QUEUE_NAMES.DWELL_MONITOR },
    { workerId: 'anonymizer-retention', jobName: QUEUE_NAMES.ANONYMIZER_RETENTION },
  ];
  heartbeats = heartbeatConfigs.map(cfg => {
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
  const { RatesRefreshWorker } = await import('./workers/rates-refresh.js');
  const ratesRefreshWorker = new RatesRefreshWorker(pool, queue.boss);
  await ratesRefreshWorker.start();

  // Soft access gate (ADR-soft-access-gate) — notify + retention/reconcile crons. These
  // run unconditionally (data hygiene): retention auto-erase + notify-gap recovery must
  // work the moment the public route is enabled. The public POST route itself is gated
  // separately by ACCESS_GATE_PUBLIC_ENABLED (below).
  const accessRequestNotifyWorker = new AccessRequestNotifyWorker(pool, queue.boss, messageBus);
  await accessRequestNotifyWorker.start();
  const accessRequestRetentionWorker = new AccessRequestRetentionWorker(pool, queue.boss, messageBus);
  await accessRequestRetentionWorker.start();

  // Telegram Poller disabled — webhook handles all updates (messages + callbacks)
  // Poller conflicts with webhook (getUpdates HTTP 409). Keep poller import for type,
  // but don't start. Webhook at /webhook/telegram/:secret handles /start, /stop, /open, /close.
  const telegramPoller = new TelegramPoller(pool, telegramAdapter);
  // telegramPoller.start(); — disabled: webhook active

  registerNotifySubscriptions(messageBus, queue.boss);

  // P31 — Worker Liveness Checker (singleton cron, 60s)
  const livenessChecker = new LivenessChecker(pool, queue.boss, messageBus);
  await livenessChecker.start();

  // P5-5 — Free-tier watch (hourly, monitors Free tier limits)
  const { collectFreeTierMetrics } = await import('./workers/free-tier-watch.js');
  await queue.boss.work(QUEUE_NAMES.FREE_TIER_WATCH, async () => {
    try {
      const metrics = await collectFreeTierMetrics(pool);
      console.log(`[FreeTier] Watch complete: ${metrics.status} (DB: ${metrics.dbPct}%)`);
    } catch (err: any) {
      console.error('[FreeTier] Watch failed:', err.message);
    }
  });
  await queue.boss.schedule(QUEUE_NAMES.FREE_TIER_WATCH, '0 * * * *');
    })().catch((err: any) => console.error('[API] worker startup error (continuing to listen):', err?.message || err)),
    new Promise<void>((res) => setTimeout(() => { console.warn(`[API] worker startup exceeded ${WORKER_BOOT_BUDGET_MS}ms — listening anyway; workers continue in background`); res(); }, WORKER_BOOT_BUDGET_MS)),
  ]);

  const redis = new Redis(env.REDIS_URL);
  fastify.decorate('redis', redis);

  await fastify.register(multipart, {
    limits: { fileSize: 10 * 1024 * 1024 },
    throwFileSizeLimit: true,
  });

  await fastify.register(fastifyRateLimit, {
    max: 100,
    timeWindow: '1 minute',
    // A3 (ADR-0010): @fastify/rate-limit THROWS this return value (index.js:333), so it must
    // be a throwable ApiError — returning a plain body made setErrorHandler read `.statusCode`
    // as undefined → 500. Throwing an ApiError routes the 429 through the ONE envelope source
    // (setErrorHandler) → `{code:'RATE_LIMIT', retryAfterMs, correlationId, status:429, …}`. The
    // plugin already set `Retry-After`/`x-ratelimit-*` before throwing. `context.ttl` is ms left.
    errorResponseBuilder: (_request, context) => rateLimitError(context.statusCode, context.ttl),
  });

  // Allow POST with Content-Type: application/json but empty body
  fastify.addContentTypeParser('application/json', { parseAs: 'string' }, (req, body, done) => {
    if (!body || body.trim() === '') {
      done(null, {});
    } else {
      try {
        done(null, JSON.parse(body));
      } catch (e: any) {
          // Malformed JSON is a client error (400), not a 500.
          e.statusCode = 400;
          done(e);
      }
    }
  });

  fastify.register(authPlugin);
  fastify.register(securityHeadersPlugin);
  fastify.register(healthRoutes, { db: pool, messageBus });

  // Auth-guarded path prefixes return 401 (not 404) for unauthenticated requests
  const AUTH_PREFIXES = ['/api/owner/', '/api/courier/', '/api/customer/'];
  const NO_AUTH_PATHS = [
    '/api/courier/auth/',            // public endpoints under auth prefix
    '/api/customer/track/exchange',  // pre-auth: trades opaque ?t= code for a customer JWT
  ];
  fastify.addHook('onRequest', async (request, reply) => {
    if (request.method === 'OPTIONS') return;
    const url = request.url.split('?')[0];
    // Test-only /dev + /api/dev endpoints (mock-auth, create-assignment, seed-data)
    // require BOTH the ALLOW_DEV_LOGIN flag AND the shared DEV_AUTH_SECRET (ADR-0003).
    // Fails closed: in production (flag off), they 404 as if they do not exist — never
    // leak their presence, and never honor the secret alone if it leaks.
    if (isDevPath(url)) {
      if (!isDevRequestAuthorized(url, request.headers['x-dev-auth-secret'], env)) {
        return reply.status(404).send({ error: 'Not found' });
      }
    }
    if (NO_AUTH_PATHS.some(p => url.startsWith(p))) return;
    // Public pre-auth customer phone OTP: a diner verifies their phone at checkout
    // before any customer token exists, so send/verify must be reachable unauthenticated.
    if (/^\/api\/customer\/locations\/[^/]+\/otp\/(send|verify)$/.test(url)) return;
    if (AUTH_PREFIXES.some(p => url.startsWith(p))) {
      const token = request.headers.authorization;
      if (!token || !token.startsWith('Bearer ')) {
        return reply.status(401).send({ error: 'Unauthorized' });
      }
    }
  });

  // A2 (ADR-0010): `reply.sendError` — the return-based drop-in for ad-hoc `reply.status(n)
  // .send({ error })` sites. Emits the SAME envelope as setErrorHandler (shared builder) with
  // the server correlationId + x-correlation-id echo, so a migrated site is byte-compatible plus
  // a machine `code` the FE can branch on.
  fastify.decorateReply(
    'sendError',
    function (this: any, status: number, code: string, message: string, opts?: any) {
      const correlationId = String(this.request.id);
      this.header('x-correlation-id', correlationId);
      if (opts?.retryAfterMs) this.header('retry-after', Math.ceil(opts.retryAfterMs / 1000));
      return this.status(status).send(buildErrorEnvelope(status, code, message, correlationId, opts));
    },
  );

  // P1-6 / FX-6 / ADR-0010 A1: ONE structured error envelope — never leak internals.
  // Shape: { code:<SCREAMING_SNAKE string>, message, fields?, correlationId, retryAfterMs?,
  //          status:<number,legacy>, error:<string,legacy> }
  // `code` is the STRING machine code (preserved verbatim from a thrown ApiError or an
  // error carrying a contract-shaped string code — B1, never normalize/drop); the numeric
  // HTTP status lives in `status`. `error` (legacy string) is retained so the not-yet-
  // migrated FE keeps working (code-preserving rollout). Business outcomes (soft_confirm/
  // hard_block `{outcome,reasons}`) never reach here — they are reply.send success-path
  // payloads, not thrown errors (regression trap / B2).
  fastify.setErrorHandler((error, request, reply) => {
    const correlationId = String(request.id); // server-authoritative; echoed for support
    reply.header('x-correlation-id', correlationId);

    // Fastify/AJV validation → VALIDATION_FAILED. Status is PRESERVED at 400 (the pre-A1
    // behavior): ~10 e2e contract tests + a FE branch (MediaManager.tsx:128) assert 400 here,
    // and A1 is a code-preserving rollout — moving validation to 422 is a separate, deliberate
    // breaking change with FE+test lockstep (A2), not this step. `fields` carries PATHS +
    // keyword only — never the submitted value (B4: no PII/secret echo).
    // Two validation mechanisms reach here: AJV (`error.validation` is an ARRAY) and our Zod
    // compiler (setValidatorCompiler returns {error: ZodError} → `error.validation` is a ZodError
    // OBJECT with `.issues`, `error.code`==='FST_ERR_VALIDATION'). Handle both. `fields` carries
    // PATHS + a code only — never the submitted value (B4); the message is generic (the raw
    // Zod/AJV dump is not serialized).
    const ajvIssues = Array.isArray((error as any).validation) ? (error as any).validation : null;
    const zodIssues = (error as any).validation?.issues ?? (error as any).issues ?? null;
    if (ajvIssues || zodIssues || (error as any).code === 'FST_ERR_VALIDATION') {
      const fields = ajvIssues
        ? ajvIssues.map((v: any) => ({
            path: v.instancePath || v.dataPath || '',
            code: String(v.keyword || 'INVALID').toUpperCase(),
          }))
        : Array.isArray(zodIssues)
          ? zodIssues.map((i: any) => ({
              // field PATH/NAME only (the API contract — not a submitted value, B4-safe);
              // unrecognized_keys carries the rejected key in `i.keys`, not `i.path`.
              path:
                Array.isArray(i.path) && i.path.length
                  ? i.path.join('.')
                  : Array.isArray(i.keys)
                    ? i.keys.join(',')
                    : String(i.path ?? ''),
              code: String(i.code || 'INVALID').toUpperCase(),
            }))
          : [];
      // generic message (raw validation dump no longer leaked); fields = paths only (B4)
      return reply.status(400).send(
        buildErrorEnvelope(400, 'VALIDATION_FAILED', 'Invalid request', correlationId, { fields }),
      );
    }

    const apiErr = error instanceof ApiError ? error : null;
    const status = apiErr?.status ?? (error as any).statusCode ?? 500;

    // Preserve a contract-shaped string `code` verbatim (B1). A PG/driver code like '23505'
    // is NOT contract-shaped → never surfaced (B4 leak guard); derive a generic code instead.
    const code =
      apiErr?.code ??
      (isContractCode((error as any).code) ? (error as any).code : undefined) ??
      (status >= 500 ? 'INTERNAL' : 'ERROR');

    // 5xx → generic message; no stack, no err.detail / PG internals ever serialized (B4).
    const message =
      status >= 500 ? 'Internal server error' : apiErr?.message || error.message || 'Request failed';
    const retryAfterMs = apiErr?.retryAfterMs;
    const fields = apiErr?.fields;

    if (status >= 500) {
      request.log.error({ err: error, correlationId, code }, 'request_failed');
      // Tag the captured event so an on-screen support code greps straight to the trace.
      // (correlationId must be in the Sentry tag allowlist — sentry.ts — or it is dropped.)
      const sentry = getSentry();
      sentry?.withScope?.((scope: any) => {
        scope.setTag('correlationId', correlationId);
        scope.setTag('error_code', code);
        sentry.captureException(error);
      });
    }

    if (retryAfterMs) reply.header('retry-after', Math.ceil(retryAfterMs / 1000));

    reply
      .status(status)
      .send(buildErrorEnvelope(status, code, message, correlationId, { fields, retryAfterMs }));
  });

  // P1-7 / FX-7: Body limit — Fastify constructor sets 10MB default (above).
  // Individual routes can override via route config if needed.
  // authRoutes define /auth/* paths; mount under /api so they resolve at /api/auth/*
  // — matching the frontend, apiClient, the inline local-login, and the OAuth
  // redirect_uri (APP_BASE_URL/api/auth/google/callback). Without the prefix the
  // Google button + callback 404'd.
  fastify.register(authRoutes, { prefix: '/api' });
  // Real email+password login (argon2) + flag-gated dev bypass, both in routes/auth/local.ts.
  // Registered here (prefix /api → /api/auth/local/login). Previously this plugin was imported
  // but never registered; an inline dev-only handler shadowed the route, so DB-password login
  // was dead code (test@dowiz.com could only 401 on prod). The inline handler is now removed.
  const { default: localAuthRoutes } = await import('./routes/auth/local.js');
  fastify.register(localAuthRoutes, { prefix: '/api', db: pool });
  fastify.register(courierRoutes);
  fastify.register(orderRoutes, { prefix: '/api', db: pool, messageBus, queue });
  fastify.register(categoryRoutes);
  fastify.register(productRoutes);
  fastify.register(modifierGroupRoutes);
  fastify.register(menuAvailabilityRoutes);
  fastify.register(locationRoutes);
  fastify.register(publicMenuRoutes);
  fastify.register(ssrRoutes, { db: pool });
  fastify.register(brandingPreviewRoutes);
  fastify.register(seoRoutes, { db: pool });
  fastify.register(clientFlowRoutes, { db: pool });
  fastify.register(pwaRoutes, { db: pool });
  fastify.register(vapidRoutes);
  fastify.register(telemetryRoutes, { db: pool });
  // SENSOR-BUS §1.3: anonymous storefront-funnel ingest. Always mounted; the FUNNEL_INGEST_ENABLED
  // kill-switch is enforced inside (returns a uniform 204 when off) so it can be silenced without a deploy.
  fastify.register(funnelRoutes, { db: pool });
  // R3-4 (STOP-1 reachable-surface gate): the access-request capture route is mounted
  // ONLY when the flag is on. While off, POST /api/access-requests 404s via
  // setNotFoundHandler — it is NOT publicly POST-able before invite-gating ships. The
  // migrations (table + 3 queues) still ship; only the route is gated.
  if (env.ACCESS_GATE_PUBLIC_ENABLED === 'true') {
    fastify.register(accessRequestRoutes, { db: pool, queue });
  }
  fastify.register(ownerThemeRoutes, { db: pool, storage });
  fastify.register(publicThemeRoutes, { db: pool });
  fastify.register(ownerNotificationRoutes, { db: pool, queue });
  fastify.register(menuImportRoutes, { prefix: '/api/owner', db: pool, messageBus, parsers, storage, translation });
  fastify.register(menuTranslateRoutes, { prefix: '/api/owner', db: pool, messageBus, translation });
  fastify.register(courierAuthRoutes, { prefix: '/api/courier/auth', db: pool });
  fastify.register(courierMeRoutes, { prefix: '/api/courier', db: pool });
  fastify.register(ownerCourierRoutes, { db: pool });
  fastify.register(ownerCourierInvitesRoutes, { db: pool });
  fastify.register(customerOrderRoutes, { prefix: '/api/customer', db: pool, messageBus });
  fastify.register(ownerSettlementRoutes, { prefix: '/api/owner/locations', db: pool, messageBus });
  fastify.register(ownerDashboardRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerAlertRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerDwellSettingsRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerSignalRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerPushRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerOrderMetaRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerFallbackRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerRevealContactRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerGdprRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerPromotionRoutes, { db: pool });
  fastify.register(onboardingRoutes, { prefix: '/api/owner', db: pool, messageBus, queue });
  fastify.register(activationRoutes, { prefix: '/api/owner', db: pool });
  fastify.register(customerOtpRoutes, { prefix: '/api/customer', db: pool, messageBus });
  fastify.register(customerTrackRoutes, { prefix: '/api/customer', db: pool });
  fastify.register(customerPushRoutes, { prefix: '/api/customer', db: pool, messageBus });
  fastify.register(courierSettlementRoutes, { prefix: '/api/courier', db: pool, messageBus });
  fastify.register(courierAssignmentsRoutes, { prefix: '/api/courier', db: pool, messageBus });
  fastify.register(courierShiftsRoutes, { prefix: '/api/courier', db: pool, messageBus });
  fastify.register(orderMessageRoutes, { db: pool, messageBus });

  fastify.register(ratesRoutes, { db: pool });
  fastify.register(publicFallbackConfigRoutes, { db: pool });

// Telegram Webhook (must be registered before route definitions)
fastify.register(telegramWebhookRoutes, {
  db: pool,
  queue: queue.boss,
  telegramBotSecret: env.TELEGRAM_BOT_SECRET || '',
  messageBus
});

fastify.register(mockAuthRoutes, { db: pool });

  fastify.post('/api/dev/mock-auth', async (request, reply) => {
    const body = (request.body || {}) as Record<string, unknown>;
    // Dev-kid signing (ADR-0003): mock tokens are signed under the dev keypair so a prod
    // verifier rejects them. This path is already flag-gated by isDevRequestAuthorized.
    const { signDevToken } = await import('@deliveryos/platform');

    // Courier role: simple JWT with real location UUID
    if (body.role === 'courier') {
      // SYNTHETIC-ONLY RE-DERIVED MINT (constraint #1 / resolution NEW-M1, L1): mint for the ONE
      // seeded synthetic courier ONLY on an explicit synthetic:true. The id is RE-DERIVED by
      // SELECTing on the sentinel email_hash — NO caller-supplied courierId is read or echoed, so
      // even on an open (staging) gate the capability is "impersonate the one synthetic fixture",
      // never "impersonate any courier" (the dev-login-backdoor class). Any other input keeps the
      // existing random-uuid behaviour below.
      if (body.synthetic === true) {
        const cRes = await pool.query(
          `SELECT c.id, cl.location_id
             FROM couriers c
             JOIN courier_locations cl ON cl.courier_id = c.id
            WHERE c.email_hash = $1
            ORDER BY cl.added_at ASC
            LIMIT 1`,
          [SYNTHETIC_COURIER_EMAIL_HASH],
        );
        if (cRes.rowCount === 0) {
          return reply.status(409).send({
            error: 'synthetic courier not seeded — run /dev/seed-visual-state first',
            code: 'SYNTHETIC_COURIER_MISSING',
          });
        }
        const syntheticId = cRes.rows[0].id as string;
        const syntheticLocationId = cRes.rows[0].location_id as string;
        const accessToken = await signDevToken({ role: 'courier', sub: syntheticId, activeLocationId: syntheticLocationId } as any, '1d');
        return reply.send({ access_token: accessToken, userId: syntheticId, activeLocationId: syntheticLocationId, synthetic: true });
      }

      const courierId = crypto.randomUUID();
      const locRes = await pool.query(`SELECT id FROM locations WHERE slug = 'demo' LIMIT 1`);
      const locationId = locRes.rowCount > 0 ? locRes.rows[0].id : '1f609add-062a-4bb5-89bf-d695f963ede6';
      const accessToken = await signDevToken({ role: 'courier', sub: courierId, activeLocationId: locationId } as any, '1d');
      return reply.send({ access_token: accessToken, userId: courierId, activeLocationId: locationId });
    }

    // Fresh-owner E2E mode: mint a brand-new owner with NO location membership so
    // the admin flow lands on the onboarding wizard (/admin/onboarding) instead of
    // the demo dashboard. Each call is a distinct throwaway user.
    if (body.fresh === true) {
      const suffix = crypto.randomUUID().slice(0, 8);
      const fRes = await pool.query(
        `INSERT INTO users (email, google_sub, display_name) VALUES ($1, $2, 'E2E Fresh Owner') RETURNING id`,
        [`fresh-${suffix}@e2e.dowiz`, `mock-fresh-${suffix}`],
      );
      const fUserId = fRes.rows[0].id;
      const fToken = await signDevToken({ role: 'owner', userId: fUserId, sub: fUserId } as any, '1d');
      return reply.send({ access_token: fToken, userId: fUserId, activeLocationId: undefined });
    }

    // Owner role: create/upsert dev user and return owner token
    const email = 'dev@deliveryos.com';
    const googleSub = 'mock-google-12345';
    const name = 'Dev Owner';

    let userId;
    try {
      const res = await pool.query(
        `INSERT INTO users (email, google_sub, display_name) 
         VALUES ($1, $2, $3)
         ON CONFLICT (google_sub) DO UPDATE SET email = EXCLUDED.email, display_name = EXCLUDED.display_name
         RETURNING id`,
        [email, googleSub, name]
      );
      userId = res.rows[0].id;
    } catch (e) {
      const updateRes = await pool.query(
        `UPDATE users SET google_sub = $2, display_name = COALESCE(users.display_name, $3) WHERE email = $1 RETURNING id`,
        [email, googleSub, name]
      );
      if (updateRes.rowCount === 0) {
        throw new Error('Failed to upsert dev user');
      }
      userId = updateRes.rows[0].id;
    }

    const memberRes = await pool.query(
      `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' AND status = 'active' LIMIT 1`, // P-d (ADR-0004)
      [userId]
    );
    let activeLocationId = memberRes.rowCount > 0 ? memberRes.rows[0].location_id : undefined;

    if (!activeLocationId) {
      const locRes = await pool.query(`SELECT id FROM locations WHERE slug = 'demo' LIMIT 1`);
      if (locRes.rowCount > 0) {
        await pool.query(
          `INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING`,
          [userId, locRes.rows[0].id]
        );
        activeLocationId = locRes.rows[0].id;
      }
    }

    const accessToken = await signDevToken({ role: 'owner', userId, sub: userId } as any, '1d');

    return reply.send({ access_token: accessToken, userId, activeLocationId });
  });
  fastify.post('/api/dev/create-assignment', async (request, reply) => {
    const { orderId, courierId, locationId } = (request.body || {}) as Record<string, string>;
    if (!orderId || !courierId || !locationId) {
      return reply.status(400).send({ error: 'orderId, courierId, locationId required' });
    }
    try {
      const emailHash = crypto.createHash('sha256').update(courierId).digest('hex');
      await pool.query(
        `INSERT INTO couriers (id, email_encrypted, email_hash, full_name_encrypted, password_hash)
         VALUES ($1, $2, $3, $2, 'mock')
         ON CONFLICT (id) DO NOTHING`,
        [courierId, Buffer.alloc(0), emailHash]
      );
      await pool.query(
        `INSERT INTO courier_locations (courier_id, location_id, role)
         VALUES ($1, $2, 'courier')
         ON CONFLICT (courier_id, location_id) DO NOTHING`,
        [courierId, locationId]
      );
      await pool.query(
        `INSERT INTO courier_shifts (courier_id, location_id, status, started_at, last_heartbeat_at)
         VALUES ($1, $2, 'available', now(), now())
         ON CONFLICT DO NOTHING`,
        [courierId, locationId]
      );
      const asgn = await pool.query(
        `INSERT INTO courier_assignments (order_id, courier_id, location_id, status, assigned_at)
         VALUES ($1, $2, $3, 'assigned', now())
         ON CONFLICT (order_id) DO UPDATE SET courier_id = EXCLUDED.courier_id, status = 'assigned'
         RETURNING id`,
        [orderId, courierId, locationId]
      );
      const courierChannel = `courier:${courierId}`;
      await messageBus.publish(`courier:${courierId}`, {
        type: 'task_assigned',
        payload: { id: orderId, order_id: orderId, orderId, status: 'assigned', courierId }
      });
      await messageBus.publish(dashboardChannel(locationId), {
        type: 'assignment.created',
        orderId,
        courierId
      });
      return reply.send({ assignmentId: asgn.rows[0]?.id });
    } catch (e: any) {
      return reply.status(500).send({ error: 'Assignment failed: ' + (e?.message || '') });
    }
  });
  fastify.post('/api/dev/seed-data', async (request, reply) => {
    const body = (request.body || {}) as Record<string, unknown>;
    const slug = (body.slug as string) || 'demo';
    try {
      const locRes = await pool.query(`SELECT id FROM locations WHERE slug = $1 LIMIT 1`, [slug]);
      let locationId: string;
      if (locRes.rowCount && locRes.rowCount > 0) {
        locationId = locRes.rows[0].id;
      } else {
        const devUser = await pool.query(
          `INSERT INTO users (email, display_name) VALUES ($1, $2) ON CONFLICT (email) DO NOTHING RETURNING id`,
          ['dev@deliveryos.com', 'Dev Owner']
        );
        let ownerId = devUser.rows[0]?.id;
        if (!ownerId) {
          const existing = await pool.query(`SELECT id FROM users WHERE email = 'dev@deliveryos.com' LIMIT 1`);
          ownerId = existing.rows[0]?.id;
        }
        if (!ownerId) ownerId = '00000000-0000-0000-0000-000000000000';

        const orgRes = await pool.query(
          `INSERT INTO organizations (name, owner_id) VALUES ($1, $2) ON CONFLICT DO NOTHING RETURNING id`,
          ['Dev Org', ownerId]
        );
        let orgId = orgRes.rows[0]?.id;
        if (!orgId) {
          const existing = await pool.query(`SELECT id FROM organizations WHERE name = 'Dev Org' LIMIT 1`);
          orgId = existing.rows[0]?.id;
        }
        if (!orgId) {
          const org2 = await pool.query(
            `INSERT INTO organizations (name, owner_id) VALUES ($1, $2) RETURNING id`,
            ['Dev Org 2', ownerId]
          );
          orgId = org2.rows[0]?.id;
        }
        const newLoc = await pool.query(
          `INSERT INTO locations (org_id, slug, name, phone, status, default_locale, supported_locales, currency_code, currency_minor_unit, delivery_fee_flat, min_order_value, free_delivery_threshold)
           VALUES ($1, $2, $3, $4, 'active', 'sq', ARRAY['sq','en','uk'], 'ALL', 0, 200, 500, 2000) RETURNING id`,
          [orgId, slug, body.name || 'Demo Store', body.phone || '+355690000000']
        );
        locationId = newLoc.rows[0].id;
      }

      await pool.query(`INSERT INTO menu_versions (location_id, version) VALUES ($1, 1) ON CONFLICT (location_id) DO NOTHING`, [locationId]);

      const catNames = ['Pizzas', 'Pastas', 'Salads', 'Beverages'];
      const catIds: string[] = [];
      for (const name of catNames) {
        const existing = await pool.query(`SELECT id FROM categories WHERE location_id = $1 AND name = $2`, [locationId, name]);
        if (existing.rowCount && existing.rowCount > 0) {
          catIds.push(existing.rows[0].id);
        } else {
          const c = await pool.query(`INSERT INTO categories (location_id, name) VALUES ($1, $2) RETURNING id`, [locationId, name]);
          catIds.push(c.rows[0].id);
        }
      }

      const products = [
        { name: 'Margherita', price: 1200, cat: 0, taste: { spicy: 0, sweet: 1, salty: 2, richness: 2, sour: 1 } },
        { name: 'Pepperoni', price: 1500, cat: 0, taste: { spicy: 2, sweet: 0, salty: 2, richness: 2, sour: 0 } },
        { name: 'Carbonara', price: 1300, cat: 1, taste: { spicy: 0, sweet: 0, salty: 2, richness: 3, sour: 1 } },
        { name: 'Caesar Salad', price: 800, cat: 2, taste: { spicy: 0, sweet: 1, salty: 2, richness: 1, sour: 2 } },
        { name: 'Cola', price: 200, cat: 3, taste: { spicy: 0, sweet: 3, salty: 0, richness: 0, sour: 0 } },
      ];
      const prodIds: string[] = [];
      for (const p of products) {
        const existing = await pool.query(`SELECT id FROM products WHERE location_id = $1 AND name = $2`, [locationId, p.name]);
        if (existing.rowCount && existing.rowCount > 0) {
          prodIds.push(existing.rows[0].id);
        } else {
          const r = await pool.query(
            `INSERT INTO products (location_id, category_id, name, price, is_available, attributes)
             VALUES ($1, $2, $3, $4, true, $5) RETURNING id`,
            [locationId, catIds[p.cat], p.name, p.price, JSON.stringify({ taste: p.taste })]
          );
          prodIds.push(r.rows[0].id);
        }
      }

      const themeCheck = await pool.query(`SELECT 1 FROM location_themes WHERE location_id = $1`, [locationId]);
      if (themeCheck.rowCount === 0) {
        await pool.query(`INSERT INTO location_themes (location_id, frame_ancestors) VALUES ($1, ARRAY['*'])`, [locationId]);
      }
      const tvCheck = await pool.query(`SELECT 1 FROM theme_versions WHERE location_id = $1`, [locationId]);
      if (tvCheck.rowCount === 0) {
        await pool.query(
          `INSERT INTO theme_versions (location_id, version, css_hash, css_body)
           VALUES ($1, 1, md5(random()::text),
           E':root{--brand-primary:#ea4f16;--brand-bg:#121212;--brand-surface:#1e1e1e;--brand-text:#ffffff}')`,
          [locationId]
        );
      }

      return reply.send({
        success: true,
        locationId,
        slug,
        categories: catIds.length,
        products: prodIds.length,
        names: products.map(p => p.name),
      });
    } catch (e: any) {
      return reply.status(500).send({ error: 'Seed failed: ' + (e?.message || '') });
    }
  });
  // (Local email+password login — real argon2 + flag-gated dev bypass — is served by the
  //  registered routes/auth/local.ts plugin above, not an inline handler.)

  // SPA proxy — maps React SPA URL patterns to real backend routes
  fastify.register(spaProxyRoutes, { db: pool, storage });
  // Owner rich-media CRUD (cinematic product-media seam, ADR-0002). Dark behind
  // MEDIA_RICH_ENABLED at the storefront; the owner endpoints are always wired so
  // an owner can stage media before launch. Operational pool (RLS) via withTenant.
  const { default: ownerProductMediaRoutes } = await import('./routes/owner/product-media.js');
  fastify.register(ownerProductMediaRoutes, { prefix: '/api/owner', db: pool, storage });
  // P32 — Backup admin routes
  const { default: backupAdminRoutes } = await import('./routes/admin/backups.js');
  fastify.register(backupAdminRoutes, { prefix: '/api/admin', db: pool, queue });
  // P33 — Fallback admin routes
  const { default: fallbackAdminRoutes } = await import('./routes/admin/fallback.js');
  fastify.register(fallbackAdminRoutes, { prefix: '/api/admin', db: pool });
  const { default: notificationAuditRoutes } = await import('./routes/admin/notification-audit.js');
  fastify.register(notificationAuditRoutes, { prefix: '/api/admin', db: pool });

  // SPA Fallback: Serve index.html for unknown GET requests matching SPA route patterns
  const SPA_ROUTES = ['/admin', '/courier', '/dashboard', '/s/', '/login', '/branding-preview', '/privacy'];
  fastify.setNotFoundHandler((request, reply) => {
    if (
      request.method === 'GET' &&
      (request.headers.accept?.includes('text/html') ||
        SPA_ROUTES.some(prefix => request.url === prefix || request.url.startsWith(prefix + '/')))
    ) {
      return reply.sendFile('index.html');
    }
    reply.status(404).send({ error: 'Not found', path: request.url });
  });

  fastify.ready(err => {
    if (err) throw err;
    console.log(fastify.printRoutes());
    setupWebSocket(fastify, messageBus);
  });

  fastify.addHook('onClose', async () => {
    if (fastify.wss) {
      console.log('[API] Closing WebSocket connections...');
      for (const client of fastify.wss.clients) {
        client.close(1012, 'Server restarting');
      }
      fastify.wss.close();
    }
    telegramPoller.stop();
    heartbeats.forEach(hb => hb.stop());
    await queue.stop();
  });

  setupShutdown(fastify, pool, messageBus, queue);

  try {
    const port = env.PORT || 8080;
    await fastify.listen({ port, host: '0.0.0.0' });
    console.log(`[API] Server listening on port ${port}`);
    // R3-1 fail-fast: verify both access-request cron schedules landed; a miss is a
    // VISIBLE deploy failure in prod (process.exit 1), not a silent HTTP-dead zombie.
    await assertAccessRequestSchedules(pool);
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
}

main();
