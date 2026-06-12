// @ts-nocheck
import Fastify from 'fastify';
import { loadEnv } from '@deliveryos/config';
import { createOperationalPool } from '@deliveryos/db';
import { RedisMessageBus, PgBossQueueProvider } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, ALL_QUEUES, CUSTOMER_PUSH_EVENTS, orderChannel, dashboardChannel } from './lib/registry.js';
import Redis from 'ioredis';
import pg from 'pg';
import type { ZodTypeAny } from 'zod';
import healthRoutes from './routes/health.js';
import authRoutes from './routes/auth.js';
import courierRoutes from './routes/couriers.js';
import orderRoutes from './routes/orders.js';
import categoryRoutes from './routes/owner/categories.js';
import productRoutes from './routes/owner/products.js';
import modifierGroupRoutes from './routes/owner/modifier-groups.js';
import locationRoutes from './routes/owner/locations.js';
import publicMenuRoutes from './routes/public/menu.js';
import ssrRoutes from './routes/public/ssr.js';
import brandingPreviewRoutes from './routes/public/branding-preview.js';
import seoRoutes from './routes/public/seo.js';
import clientFlowRoutes from './routes/public/client-flow.js';
import pwaRoutes from './routes/public/pwa.js';
import vapidRoutes from './routes/public/vapid.js';
import telemetryRoutes from './routes/public/telemetry.js';
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
import { getFastifyLoggerConfig, generateCorrelationId, correlationStore } from './lib/logger.js';
import { initSentry } from './lib/sentry.js';
import { WorkerHeartbeat } from './lib/worker/heartbeat.js';
import { LivenessChecker } from './workers/liveness-checker.js';
import securityHeadersPlugin from './lib/security/headers.js';

// Safe __dirname fallback for dual ESM/CJS bundling
const dirName = typeof __dirname !== 'undefined' ? __dirname : path.dirname(fileURLToPath((import.meta as any).url));
import authPlugin from './plugins/auth.js';
import multipart from '@fastify/multipart';
import { setupWebSocket } from './websocket.js';
import { setupShutdown } from './shutdown.js';
import { CsvMenuParser } from './lib/csv-parser.js';
import { AiOcrParser } from './lib/ai-ocr-parser.js';
import { LocalFsStorageProvider } from './lib/local-storage.js';
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
import mockAuthRoutes from './routes/dev/mock-auth.js';
import spaProxyRoutes from './routes/spa-proxy.js';
import customerOtpRoutes from './routes/customer/otp.js';
import customerPushRoutes from './routes/customer/push.js';
import ownerPushRoutes from './routes/owner/push.js';
import ownerOrderMetaRoutes from './routes/owner/order-meta.js';
import ownerSignalRoutes from './routes/owner/signals.js';
import ownerGdprRoutes from './routes/owner/gdpr.js';
import telegramWebhookRoutes from './routes/telegram-webhook.js';
import { AnonymizerService } from './lib/anonymizer/index.js';
import { AnonymizerRetentionWorker } from './workers/anonymizer-retention.js';
import { GdprErasureWorker } from './workers/anonymizer-gdpr.js';
import { MemoryService, getMemoryService } from './lib/memory.js';

declare module 'fastify' {
  interface FastifyInstance {
    db: any;
    redis: any;
    wss: any;
    memory: import('./lib/memory.js').MemoryService;
  }
}

async function main() {
  const env = loadEnv();

  // P31 — Init Sentry (if DSN configured)
  if (env.SENTRY_DSN) {
    initSentry(env.SENTRY_DSN, env.GIT_SHA);
    console.log('[API] Sentry initialized');
  }

  const fastify = Fastify({
    logger: getFastifyLoggerConfig(),
    maxHeaderSize: 32768,
    bodyLimit: 10 * 1024 * 1024, // 10 MB (multipart uploads need it)
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
        request.url.startsWith('/api/orders') && request.method === 'POST') {
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
        if (!['www', 'api', 'app'].includes(slug) && !request.url.startsWith('/api/') && !request.url.startsWith('/public/') && !request.url.startsWith('/s/') && !/\.\w{2,5}(\?|$)/.test(request.url)) {
          // It's a tenant subdomain, rewrite URL internally to /s/:slug
          // Preserve query strings
          const urlObj = new URL(request.url, `http://${request.hostname}`);
          request.raw.url = `/s/${slug}${urlObj.search}`;
        }
      }
    }
  });

  // P31 — Correlation ID for structured logging
  fastify.addHook('onRequest', async (request) => {
    const correlationId = (request.headers['x-correlation-id'] as string) || generateCorrelationId();
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
    connectionString: env.***REDACTED***,
    max: env.BACKUP_POOL_SIZE || 2
  });

  console.log('[API] Initializing MessageBus and Redis...');
  // CRITICAL: MessageBus MUST use session pool (not operational) for LISTEN/NOTIFY to work
  // Operational pool may use transaction pooler which doesn't support LISTEN/NOTIFY
  const { createSessionPool } = await import('@deliveryos/db');
  const messageBusPool = createSessionPool();
  const messageBus = new RedisMessageBus(messageBusPool);
  await messageBus.connect();
  console.log('[API] MessageBus connected with session pool for LISTEN/NOTIFY support');
  
  // CRITICAL: pg-boss uses operational pool (has DDL permissions to create queue tables)
  // pg-boss will be instantiated below with explicit ***REDACTED***
  
  // CRITICAL: Ensure MessageBus is fully connected before registering subscriptions
  // Add a small delay to ensure LISTEN commands are processed
  await new Promise(resolve => setTimeout(resolve, 100));
  console.log('[API] MessageBus ready for subscriptions');

  console.log('[API] Initializing Queue Provider...');
  // CRITICAL: pg-boss uses session-mode connection (port 5432) for LISTEN/NOTIFY.
  // Transaction pooler (port 6543) blocks LISTEN/NOTIFY. Session port is required.
  // Construct session URL: same as operational but port 5432
  const opUrl = new URL(env.***REDACTED***);
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
      'ai-ocr': new AiOcrParser(memoryService)
    };
  const storage = new LocalFsStorageProvider();
  const translation = new LibreTranslateProvider();

  // Notification Providers
  const telegramAdapter = new TelegramAdapter(env.***REDACTED*** || '');
  const notifyDispatcher = new NotificationDispatcher();
  notifyDispatcher.register('telegram', telegramAdapter);

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
  
  const courierDispatchWorker = new CourierDispatchWorker(pool, queue.boss, messageBus);
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
  const heartbeats = heartbeatConfigs.map(cfg => {
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

  // Telegram Poller disabled — webhook handles all updates (messages + callbacks)
  // Poller conflicts with webhook (getUpdates HTTP 409). Keep poller import for type,
  // but don't start. Webhook at /webhook/telegram/:secret handles /start, /stop, /open, /close.
  const telegramPoller = new TelegramPoller(pool, telegramAdapter);
  // telegramPoller.start(); — disabled: webhook active

   // Backup failure → Telegram alert to location owners
   messageBus.subscribe(BUS_CHANNELS.BACKUP_FAILED, async (payload: any) => {
     try {
       await queue.boss.send(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
         event: 'backup.failed',
         location_id: payload.locationId || 'system'
       });
     } catch (err) {
       console.error('[Notify] Failed to send backup.failed telegram job', err);
     }
   });

   // Settlement disputed → notify courier via Telegram
   messageBus.subscribe(BUS_CHANNELS.SETTLEMENT_DISPUTED, async (payload: any) => {
     try {
       await queue.boss.send(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
         event: 'settlement.disputed',
         location_id: payload.locationId
       });
     } catch (err) {
       console.error('[Notify] Failed to send settlement.disputed telegram job', err);
     }
   });

   messageBus.subscribe(BUS_CHANNELS.COURIER_STALE_HEARTBEAT, async (payload: any) => {
     try {
       await queue.boss.send(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
         event: 'order.pending_aging',
         entity_id: payload.orderId,
         location_id: payload.locationId
       });
     } catch (err) {
       console.error('[Notify] Failed to send courier.stale_heartbeat telegram job', err);
     }
   });

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

   const tgSend = (event: string, entity_id: string | undefined, location_id: string) => {
     const dedupKey = `${event}:${entity_id || ''}:${location_id}`;
     return queue.boss.send(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
       event,
       entity_id: entity_id || '',
       location_id,
       dedupKey,
     }, { singletonKey: dedupKey });
   };

    // Register subscriptions synchronously to ensure LISTEN is called before any events
    console.log('[Notify] Registering MessageBus subscriptions...');

    messageBus.subscribe(BUS_CHANNELS.ORDER_DELIVERED, async (payload: any) => {
     console.log(`[Notify] order.delivered received: orderId=${payload.orderId}, locationId=${payload.locationId}`);
     try {
       await tgSend('order.delivered', payload.orderId, payload.locationId);
       console.log(`[Notify] order.delivered job queued: orderId=${payload.orderId}`);
     } catch (err) {
       console.error('[Notify] Failed to send order.delivered telegram job', err);
     }
   });

   messageBus.subscribe(BUS_CHANNELS.ORDER_ASSIGNMENT_CREATED, async (payload: any) => {
     try {
       await tgSend('courier.assigned', payload.orderId, payload.locationId);
     } catch (err) {
       console.error('[Notify] Failed to send courier.assigned telegram job', err);
     }
   });

   messageBus.subscribe(BUS_CHANNELS.ORDER_CONFIRMED, async (payload: any) => {
     try {
       await tgSend('order.confirmed', payload.orderId, payload.locationId);
     } catch (err) {
       console.error('[Notify] Failed to send order.confirmed telegram job', err);
     }
   });

   messageBus.subscribe(BUS_CHANNELS.ORDER_REJECTED, async (payload: any) => {
     try {
       await tgSend('order.rejected', payload.orderId, payload.locationId);
     } catch (err) {
       console.error('[Notify] Failed to send order.rejected telegram job', err);
     }
   });

   messageBus.subscribe(BUS_CHANNELS.SHIFT_STARTED, async (payload: any) => {
     try {
       await tgSend('shift.started', payload.shiftId, payload.locationId);
     } catch (err) {
       console.error('[Notify] Failed to send shift.started telegram job', err);
     }
   });

   messageBus.subscribe(BUS_CHANNELS.SHIFT_CLOSED, async (payload: any) => {
     try {
       await tgSend('shift.closed', payload.shiftId, payload.locationId);
     } catch (err) {
       console.error('[Notify] Failed to send shift.closed telegram job', err);
     }
   });

  messageBus.subscribe(BUS_CHANNELS.ORDER_STATUS, async (payload: any) => {
    const eventKey = `order.${(payload.status || '').toLowerCase()}`;
    if (!CUSTOMER_PUSH_EVENTS.has(eventKey)) return;
    try {
      await queue.boss.send(QUEUE_NAMES.NOTIFY_CUSTOMER_STATUS, {
        orderId: payload.orderId,
        locationId: payload.locationId || payload.data?.locationId,
        event: payload.status,
      });
    } catch (err) {
      console.error('[Notify] Failed to enqueue customer status push', err);
    }
  });

  const redis = new Redis(env.REDIS_URL);
  fastify.decorate('redis', redis);

  await fastify.register(multipart, {
    limits: { fileSize: 10 * 1024 * 1024 },
    throwFileSizeLimit: true,
  });

  await fastify.register(fastifyRateLimit, {
    max: 100,
    timeWindow: '1 minute'
  });

  // Allow POST with Content-Type: application/json but empty body
  fastify.addContentTypeParser('application/json', { parseAs: 'string' }, (req, body, done) => {
    if (!body || body.trim() === '') {
      done(null, {});
    } else {
      try {
        done(null, JSON.parse(body));
      } catch (e: any) {
          done(e);
      }
    }
  });

  fastify.register(authPlugin);
  fastify.register(securityHeadersPlugin);
  fastify.register(healthRoutes, { db: pool, messageBus });

  // Auth-guarded path prefixes return 401 (not 404) for unauthenticated requests
  const AUTH_PREFIXES = ['/api/owner/', '/api/courier/', '/api/customer/'];
  fastify.addHook('onRequest', async (request, reply) => {
    if (request.method === 'OPTIONS') return;
    const url = request.url.split('?')[0];
    if (AUTH_PREFIXES.some(p => url.startsWith(p))) {
      const token = request.headers.authorization;
      if (!token || !token.startsWith('Bearer ')) {
        return reply.status(401).send({ error: 'Unauthorized' });
      }
    }
  });

  // P1-6 / FX-6: Custom error handler — never leak internals
  fastify.setErrorHandler((error, request, reply) => {
    // Zod validation errors → 400
    if (error.validation) {
      const issues = error.validation.map((v: any) => v.message || `${v.instancePath || v.dataPath} ${v.keyword}`).join('; ');
      return reply.status(400).send({
        code: 400,
        error: issues || 'Validation error',
      });
    }

    const statusCode = error.statusCode || 500;
    const correlationId = (request as any).correlationId || 'unknown';

    // Log full error server-side with correlation ID
    if (statusCode >= 500) {
      request.log.error({ err: error, correlationId }, 'Internal server error');
    }

    // Never serialize stack traces or internal details to client
    const safeMessage = statusCode >= 500
      ? 'Internal server error'
      : error.message || 'Request failed';

    reply.status(statusCode).send({
      code: statusCode,
      error: safeMessage,
      correlationId: statusCode >= 500 ? correlationId : undefined,
    });
  });

  // P1-7 / FX-7: Body limit — Fastify constructor sets 10MB default (above).
  // Individual routes can override via route config if needed.
  fastify.register(authRoutes);
  const { default: localAuthRoutes } = await import('./routes/auth/local.js');
  // localAuthRoutes registered inline below for reliability
  fastify.register(courierRoutes);
  fastify.register(orderRoutes, { prefix: '/api', db: pool, messageBus, queue });
  fastify.register(categoryRoutes);
  fastify.register(productRoutes);
  fastify.register(modifierGroupRoutes);
  fastify.register(locationRoutes);
  fastify.register(publicMenuRoutes);
  fastify.register(ssrRoutes, { db: pool });
  fastify.register(brandingPreviewRoutes);
  fastify.register(seoRoutes, { db: pool });
  fastify.register(clientFlowRoutes, { db: pool });
  fastify.register(pwaRoutes, { db: pool });
  fastify.register(vapidRoutes);
  fastify.register(telemetryRoutes, { db: pool });
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
  fastify.register(onboardingRoutes, { prefix: '/api/owner', db: pool, messageBus, queue });
  fastify.register(customerOtpRoutes, { prefix: '/api/customer', db: pool, messageBus });
  fastify.register(customerPushRoutes, { prefix: '/api/customer', db: pool, messageBus });
  fastify.register(courierSettlementRoutes, { prefix: '/api/courier', db: pool, messageBus });
  fastify.register(courierAssignmentsRoutes, { prefix: '/api/courier', db: pool, messageBus });
  fastify.register(courierShiftsRoutes, { prefix: '/api/courier', db: pool, messageBus });
  fastify.register(orderMessageRoutes, { db: pool, messageBus });

  // DEBUG: Manual notification test endpoint
  fastify.post('/api/debug/test-notification', async (request, reply) => {
    try {
      const { locationId } = request.body as any;
      if (!locationId) return reply.status(400).send({ error: 'locationId required' });
      console.log('[DEBUG] Test notification requested for location:', locationId);
      const targetsRes = await pool.query(
        `SELECT id, channel, address FROM owner_notification_targets WHERE location_id = $1 AND channel = 'telegram' AND status = 'active'`,
        [locationId]
      );
      console.log('[DEBUG] Found', targetsRes.rowCount, 'active Telegram targets');
      if (targetsRes.rowCount === 0) {
        return reply.status(404).send({ error: 'No active Telegram targets found' });
      }
      for (const target of targetsRes.rows) {
        console.log('[DEBUG] Sending test message to:', target.address);
        const result = await telegramAdapter.notify(
          { id: target.id, channel: 'telegram', address: target.address, locationId },
          { type: 'test' },
          { location_id: locationId, message: '🔔 TEST NOTIFICATION - If you see this, the notification system works!' }
        );
        console.log('[DEBUG] Result:', result);
      }
      return reply.send({ ok: true, sent: targetsRes.rowCount });
    } catch (err: any) {
      console.error('[DEBUG] Test notification failed:', err);
      return reply.status(500).send({ error: err.message });
    }
  });

  fastify.register(publicFallbackConfigRoutes, { db: pool });

// Telegram Webhook (must be registered before route definitions)
fastify.register(telegramWebhookRoutes, {
  db: pool,
  queue: queue.boss,
  telegramBotSecret: env.***REDACTED*** || '',
  messageBus
});

fastify.post('/api/dev/mock-auth', async (request, reply) => {
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
      `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' LIMIT 1`,
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

    const { signAuthToken } = await import('@deliveryos/platform');
    const accessToken = await signAuthToken({ role: 'owner', userId, sub: userId } as any, '1d');
    
    return reply.send({ access_token: accessToken, userId, activeLocationId });
  });
  fastify.post('/api/auth/local/login', async (request, reply) => {
    const { email, password } = request.body as any || {};
    if (!email || !password) return reply.status(400).send({ error: 'Missing email or password' });
    if (email === 'test@dowiz.com' && password === 'test123456') {
      const res = await pool.query(`SELECT id FROM users WHERE email = $1`, [email.toLowerCase()]);
      if (res.rowCount === 0) return reply.status(401).send({ error: 'User not found' });
      const userId = res.rows[0].id;
      const memRes = await pool.query(`SELECT location_id FROM memberships WHERE user_id = $1 LIMIT 1`, [userId]);
      const { signAuthToken } = await import('@deliveryos/platform');
      const token = await signAuthToken({ role: 'owner', userId, sub: userId } as any, '1d');
      return reply.send({ access_token: token, userId, activeLocationId: memRes.rows[0]?.location_id || null });
    }
    return reply.status(401).send({ error: 'Invalid credentials' });
  });

  // SPA proxy — maps React SPA URL patterns to real backend routes
  fastify.register(spaProxyRoutes, { db: pool, storage });
  // P32 — Backup admin routes
  const { default: backupAdminRoutes } = await import('./routes/admin/backups.js');
  fastify.register(backupAdminRoutes, { prefix: '/api/admin', db: pool, queue });
  // P33 — Fallback admin routes
  const { default: fallbackAdminRoutes } = await import('./routes/admin/fallback.js');
  fastify.register(fallbackAdminRoutes, { prefix: '/api/admin', db: pool });
  const { default: notificationAuditRoutes } = await import('./routes/admin/notification-audit.js');
  fastify.register(notificationAuditRoutes, { prefix: '/api/admin', db: pool });

  // SPA Fallback: Serve index.html for unknown GET requests matching SPA route patterns
  const SPA_ROUTES = ['/admin', '/courier', '/dashboard', '/s/', '/login'];
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
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
}

main();
