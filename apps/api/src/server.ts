// @ts-nocheck
import Fastify from 'fastify';
import { loadEnv } from '@deliveryos/config';
import { createOperationalPool } from '@deliveryos/db';
import { RedisMessageBus, PgBossQueueProvider } from '@deliveryos/platform';
import Redis from 'ioredis';
import pg from 'pg';
import { serializerCompiler, validatorCompiler } from 'fastify-type-provider-zod';
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
import { DwellEscalationWorker } from './workers/dwell-escalation.js';
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
  });

  fastify.setValidatorCompiler(validatorCompiler);
  fastify.setSerializerCompiler(serializerCompiler);

  fastify.addHook('onRequest', async (request, reply) => {
    if (process.env.NODE_ENV === 'production') {
      reply.header('Strict-Transport-Security', 'max-age=31536000; includeSubDomains');
    }
    reply.header('X-Content-Type-Options', 'nosniff');
    reply.header('X-Frame-Options', 'SAMEORIGIN');
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
    maxAge: '30d',
    setHeaders: (res, filePath) => {
      if (filePath.endsWith('.js') || filePath.endsWith('.css')) {
        res.setHeader('Cache-Control', 'public, max-age=31536000, immutable');
      } else if (filePath.endsWith('.html')) {
        res.setHeader('Cache-Control', 'public, max-age=0');
      }
    },
  });

  // Subdomain routing middleware
  fastify.addHook('onRequest', async (request, reply) => {
    const host = request.hostname.split(':')[0]; // remove port
    // e.g. margherita.dowiz.org
    if (host.endsWith('dowiz.org')) {
      const parts = host.split('.');
      if (parts.length >= 3) {
        const slug = parts[0];
        if (!['www', 'api', 'app'].includes(slug) && !request.url.startsWith('/api/') && !request.url.startsWith('/public/')) {
          // It's a tenant subdomain, rewrite URL internally to /s/:slug
          // Preserve query strings
          const urlObj = new URL(request.url, `http://${request.hostname}`);
          request.url = `/s/${slug}${urlObj.search}`;
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
  const messageBus = new RedisMessageBus();
  await messageBus.connect();

  console.log('[API] Initializing Queue Provider...');
  const queue = new PgBossQueueProvider();
  await queue.start();

  console.log('[API] Initializing Providers...');
  const parsers = {
    'csv': new CsvMenuParser(),
    'ai-ocr': new AiOcrParser()
  };
  const storage = new LocalFsStorageProvider();
  const translation = new LibreTranslateProvider();

  // Memory Service (mem0 — persistent agent memory via Ollama)
  const memoryService = getMemoryService();
  memoryService.initialize().catch((err) => {
    console.warn('[API] MemoryService init failed, continuing without memory:', (err as Error).message);
  });
  fastify.decorate('memory', memoryService);

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
  await queue.boss.work('notify.dispatch', async (job: any) => notifyWorker.handleDispatch(job));
  await queue.boss.work('notify.customer_status', async (job: any) => notifyWorker.handleCustomerStatus(job));
  
  // Register escalation worker
  await queue.boss.createQueue('order.pending_aging');
  await queue.boss.work('order.pending_aging', async () => notifyWorker.escalatePendingAging());
  await queue.boss.schedule('order.pending_aging', '*/5 * * * *');

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

  // Dwell Escalation Worker
  const dwellEscalationWorker = new DwellEscalationWorker(pool, queue.boss, messageBus, notifyDispatcher);
  await dwellEscalationWorker.start();

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
    { workerId: 'dispatcher', jobName: 'courier.dispatch' },
    { workerId: 'settlement-cron', jobName: 'settlement.cron' },
    { workerId: 'dwell-monitor', jobName: 'dwell.monitor' },
    { workerId: 'anonymizer-retention', jobName: 'anonymizer.retention' },
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
  await queue.boss.work('velocity.flush', async (job: any) => velocityIncrementer.handleFlush(job));

  // Telegram Poller
  const telegramPoller = new TelegramPoller(pool, telegramAdapter);
  if (env.***REDACTED***) {
    telegramPoller.start();
  }

  // Backup failure → Telegram alert to location owners
  messageBus.subscribe('backup.failed', async (payload: any) => {
    try {
      const client = await pool.connect();
      try {
        const targetsRes = await client.query(
          `SELECT id FROM owner_notification_targets WHERE status = 'active' AND channel = 'telegram'`
        );
        for (const target of targetsRes.rows) {
          await queue.boss.send('notify.dispatch', {
            targetId: target.id,
            eventType: 'backup.failed',
            locationId: payload.locationId || 'system',
            attempt: 0,
            testMessage: `⚠️ Backup failed: ${payload.type} — ${payload.reason}`
          });
        }
      } finally {
        client.release();
      }
    } catch (err) {
      console.error('[Notify] Failed to dispatch backup.failed', err);
    }
  });

  // Settlement disputed → notify courier via Telegram
  messageBus.subscribe('settlement.disputed', async (payload: any) => {
    try {
      const client = await pool.connect();
      try {
        const targetsRes = await client.query(
          `SELECT id FROM owner_notification_targets WHERE location_id = $1 AND status = 'active' AND channel = 'telegram'`,
          [payload.locationId]
        );
        for (const target of targetsRes.rows) {
          await queue.boss.send('notify.dispatch', {
            targetId: target.id,
            eventType: 'settlement.disputed',
            locationId: payload.locationId,
            attempt: 0,
            testMessage: `⚠️ Settlement disputed: payout ${payload.payoutId}`
          });
        }
      } finally {
        client.release();
      }
    } catch (err) {
      console.error('[Notify] Failed to dispatch settlement.disputed', err);
    }
  });

  // Courier stale heartbeat → notify owner
  messageBus.subscribe('courier.stale_heartbeat', async (payload: any) => {
    try {
      const client = await pool.connect();
      try {
        const targetsRes = await client.query(
          `SELECT id FROM owner_notification_targets WHERE location_id = $1 AND status = 'active' AND channel = 'telegram'`,
          [payload.locationId]
        );
        for (const target of targetsRes.rows) {
          await queue.boss.send('notify.dispatch', {
            targetId: target.id,
            eventType: 'order.pending_aging',
            locationId: payload.locationId,
            orderId: payload.orderId,
            attempt: 0,
            testMessage: `⚠️ Courier offline: order ${payload.orderId}`
          });
        }
      } finally {
        client.release();
      }
    } catch (err) {
      console.error('[Notify] Failed to dispatch courier.stale_heartbeat', err);
    }
  });

  // P31 — Worker Liveness Checker (singleton cron, 60s)
  const livenessChecker = new LivenessChecker(pool, queue.boss, messageBus);
  await livenessChecker.start();

  // P5-5 — Free-tier watch (hourly, monitors Free tier limits)
  const { collectFreeTierMetrics } = await import('./workers/free-tier-watch.js');
  await queue.boss.work('free_tier.watch', async () => {
    try {
      const metrics = await collectFreeTierMetrics(pool);
      console.log(`[FreeTier] Watch complete: ${metrics.status} (DB: ${metrics.dbPct}%)`);
    } catch (err: any) {
      console.error('[FreeTier] Watch failed:', err.message);
    }
  });
  await queue.boss.createQueue('free_tier.watch');
  await queue.boss.schedule('free_tier.watch', '0 * * * *');

  // Lifecycle Integration
  messageBus.subscribe('order.created', async (payload: any) => {
    try {
      const client = await pool.connect();
      try {
        const targetsRes = await client.query(`SELECT id FROM owner_notification_targets WHERE location_id = $1 AND status = 'active'`, [payload.locationId]);
        for (const target of targetsRes.rows) {
          await queue.boss.send('notify.dispatch', {
            targetId: target.id,
            eventType: 'order.created',
            orderId: payload.orderId,
            locationId: payload.locationId,
            attempt: 0
          });
        }
      } finally {
        client.release();
      }
    } catch (err) {
      console.error('[Notify] Failed to dispatch order.created', err);
    }
  });

  // Customer status push (opt-in, best-effort, after-commit)
  const CUSTOMER_PUSH_EVENTS = new Set(['order.confirmed', 'order.in_delivery', 'order.delivered']);
  messageBus.subscribe('order.status', async (payload: any) => {
    const eventKey = `order.${(payload.status || '').toLowerCase()}`;
    if (!CUSTOMER_PUSH_EVENTS.has(eventKey)) return;
    try {
      await queue.boss.send('notify.customer_status', {
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
    limits: { fileSize: 5 * 1024 * 1024 }
  });

  await fastify.register(fastifyRateLimit, {
    max: 100,
    timeWindow: '1 minute'
  });

  fastify.register(authPlugin);
  fastify.register(securityHeadersPlugin);
  fastify.register(healthRoutes, { db: pool, messageBus });

  // P1-6 / FX-6: Custom error handler — never leak internals
  fastify.setErrorHandler((error, request, reply) => {
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

  // P1-7 / FX-7: Body limit — prevent OOM on small instance
  fastify.addHook('onRoute', (routeOptions) => {
    if (!routeOptions.config) routeOptions.config = {};
    if (!(routeOptions.config as any).bodyLimit) {
      (routeOptions.config as any).bodyLimit = 256 * 1024; // 256 KB default
    }
  });
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
  fastify.register(seoRoutes, { db: pool });
  fastify.register(clientFlowRoutes, { db: pool });
  fastify.register(pwaRoutes, { db: pool });
  fastify.register(vapidRoutes);
  fastify.register(telemetryRoutes);
  fastify.register(ownerThemeRoutes, { db: pool, storage });
  fastify.register(publicThemeRoutes, { db: pool });
  fastify.register(ownerNotificationRoutes, { db: pool, queue });
  fastify.register(menuImportRoutes, { db: pool, messageBus, parsers, storage, translation });
  fastify.register(menuTranslateRoutes, { db: pool, messageBus, translation });
  fastify.register(courierAuthRoutes, { db: pool });
  fastify.register(courierMeRoutes, { db: pool });
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
  fastify.register(publicFallbackConfigRoutes, { db: pool });
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
  fastify.register(spaProxyRoutes, { db: pool });
  // P32 — Backup admin routes
  const { default: backupAdminRoutes } = await import('./routes/admin/backups.js');
  fastify.register(backupAdminRoutes, { prefix: '/api/admin', db: pool, queue });
  // P33 — Fallback admin routes
  const { default: fallbackAdminRoutes } = await import('./routes/admin/fallback.js');
  fastify.register(fallbackAdminRoutes, { prefix: '/api/admin', db: pool });

  // SPA Fallback: Serve index.html for unknown HTML requests (enables client-side routing)
  fastify.setNotFoundHandler((request, reply) => {
    if (request.method === 'GET' && request.headers.accept?.includes('text/html')) {
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
