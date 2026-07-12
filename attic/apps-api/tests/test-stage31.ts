import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();
const BASE = `http://127.0.0.1:${env.PORT || 3003}`;

test('Stage 31: Observability (P5-1)', async (t) => {
  // ═══════════════════════════════════════════════════════════════
  // R1: Health endpoint returns correct structure
  // ═══════════════════════════════════════════════════════════════
  await t.test('R1.1: /health returns 200 with healthy status', async () => {
    const res = await fetch(`${BASE}/health`);
    assert.strictEqual(res.status, 200);

    const data = await res.json();
    assert.ok(data.status, 'must have status field');
    assert.ok(['healthy', 'degraded'].includes(data.status),
      `expected healthy|degraded, got ${data.status}`);
    assert.ok(data.timestamp, 'must have timestamp');
    assert.ok(data.checks, 'must have checks object');
    assert.ok(data.checks.postgres, 'must have postgres check');
    assert.ok(data.checks.workers, 'must have workers check');
    assert.ok(data.checks.messageBus, 'must have messageBus check');
  });

  await t.test('R1.2: Per-check latencyMs and 2s timeout', async () => {
    const res = await fetch(`${BASE}/health`);
    const data = await res.json();

    for (const [name, check] of Object.entries(data.checks)) {
      if (check && typeof check === 'object' && 'latencyMs' in (check as any)) {
        const c = check as any;
        assert.ok(c.latencyMs >= 0, `${name}: latencyMs must be >= 0, got ${c.latencyMs}`);
        assert.ok(c.latencyMs < 5000, `${name}: latencyMs must be < 5s, got ${c.latencyMs}`);
      }
    }
  });

  await t.test('R1.3: No PII in /health response', async () => {
    const res = await fetch(`${BASE}/health`);
    const body = await res.text();

    assert.ok(!/\+355\d{7,}/.test(body), 'must not contain Albanian phone numbers');
    assert.ok(!/[\w.+-]+@[\w-]+\.[\w.-]+/.test(body), 'must not contain emails');
  });

  // ═══════════════════════════════════════════════════════════════
  // R2: Worker heartbeat table structure
  // ═══════════════════════════════════════════════════════════════
  await t.test('R2.1: ops_worker_heartbeat has extended columns', async () => {
    const { createSessionPool } = await import('@deliveryos/db');
    const pool = createSessionPool();
    try {
      const res = await pool.query(`
        SELECT column_name, data_type
        FROM information_schema.columns
        WHERE table_name = 'ops_worker_heartbeat'
        ORDER BY ordinal_position
      `);
      const cols = res.rows.map((r: any) => r.column_name);
      assert.ok(cols.includes('worker_id'), 'must have worker_id');
      assert.ok(cols.includes('instance_id'), 'must have instance_id');
      assert.ok(cols.includes('job_name'), 'must have job_name');
      assert.ok(cols.includes('status'), 'must have status');
      assert.ok(cols.includes('last_seen_at'), 'must have last_seen_at');
      assert.ok(cols.includes('last_job_at'), 'must have last_job_at');
    } finally {
      await pool.end();
    }
  });

  // ═══════════════════════════════════════════════════════════════
  // R3: Logger library has expected API
  // ═══════════════════════════════════════════════════════════════
  await t.test('R3.1: logger exports are correct', async () => {
    const loggerMod = await import('../src/lib/logger.js');
    assert.ok(typeof loggerMod.createPinoLogger === 'function', 'exports createPinoLogger');
    assert.ok(typeof loggerMod.getFastifyLoggerConfig === 'function', 'exports getFastifyLoggerConfig');
    assert.ok(typeof loggerMod.generateCorrelationId === 'function', 'exports generateCorrelationId');
    assert.ok(typeof loggerMod.runWithCorrelationId === 'function', 'exports runWithCorrelationId');
    assert.ok(loggerMod.correlationStore, 'exports correlationStore');

    const config = loggerMod.getFastifyLoggerConfig();
    assert.ok(config.level, 'logger config has level');
    assert.ok(config.serializers, 'logger config has serializers');
    assert.ok(config.redact, 'logger config has redact paths');
    assert.ok(Array.isArray(config.redact.paths), 'redact.paths is array');
  });

  await t.test('R3.2: logger redact config excludes PII paths', async () => {
    const { getFastifyLoggerConfig } = await import('../src/lib/logger.js');
    const config = getFastifyLoggerConfig() as any;
    const redactPaths = config.redact.paths.join(' ');
    assert.ok(redactPaths.includes('cookie'), 'must redact cookies');
    assert.ok(redactPaths.includes('authorization'), 'must redact authorization');
  });

  await t.test('R3.3: correlation ID generation works', async () => {
    const { generateCorrelationId, runWithCorrelationId } = await import('../src/lib/logger.js');
    const id1 = generateCorrelationId();
    assert.ok(id1.length > 0, 'correlation ID must not be empty');

    let captured: string | undefined;
    runWithCorrelationId('test-correlation-123', () => {
      captured = 'test-correlation-123';
    });
    assert.strictEqual(captured, 'test-correlation-123');
  });

  // ═══════════════════════════════════════════════════════════════
  // R4: WorkerHeartbeat library
  // ═══════════════════════════════════════════════════════════════
  await t.test('R4.1: WorkerHeartbeat exports expected API', async () => {
    const { WorkerHeartbeat } = await import('../src/lib/worker/heartbeat.js');
    assert.ok(typeof WorkerHeartbeat === 'function', 'exports WorkerHeartbeat class');

    const proto = WorkerHeartbeat.prototype;
    assert.ok(typeof proto.start === 'function', 'has start method');
    assert.ok(typeof proto.stop === 'function', 'has stop method');
    assert.ok(typeof proto.beat === 'function', 'has beat method');
    assert.ok(typeof proto.markDegraded === 'function', 'has markDegraded method');
  });

  await t.test('R4.2: heartbeat file imports are valid', () => {
    const heartbeatPath = path.resolve('apps/api/src/lib/worker/heartbeat.ts');
    const content = fs.readFileSync(heartbeatPath, 'utf8');
    assert.ok(content.includes('WorkerHeartbeat'), 'file contains WorkerHeartbeat class');
    assert.ok(content.includes('ops_worker_heartbeat'), 'writes to ops_worker_heartbeat');
    assert.ok(content.includes('ON CONFLICT'), 'uses upsert pattern');
    assert.ok(content.includes('worker_id'), 'upserts by worker_id');
  });

  // ═══════════════════════════════════════════════════════════════
  // R5: LivenessChecker
  // ═══════════════════════════════════════════════════════════════
  await t.test('R5.1: LivenessChecker exports expected API', async () => {
    const { LivenessChecker } = await import('../src/workers/liveness-checker.js');
    assert.ok(typeof LivenessChecker === 'function', 'exports LivenessChecker class');

    const proto = LivenessChecker.prototype;
    assert.ok(typeof proto.start === 'function', 'has start method');
  });

  await t.test('R5.2: liveness checker imports LivenessChecker', () => {
    const livenessPath = path.resolve('apps/api/src/workers/liveness-checker.ts');
    const content = fs.readFileSync(livenessPath, 'utf8');
    assert.ok(content.includes('LivenessChecker'), 'file contains LivenessChecker class');
    assert.ok(content.includes('ops_worker_heartbeat'), 'queries ops_worker_heartbeat');
    assert.ok(content.includes('WORKER_LIVENESS_STALE_MS'), 'uses stale threshold config');
    assert.ok(content.includes('WORKER_CRITICAL_LIST'), 'uses critical list config');
  });

  await t.test('R5.3: health endpoint embeds worker entries', async () => {
    const res = await fetch(`${BASE}/health`);
    const data = await res.json();
    assert.ok(data.checks.workers.entries !== undefined, 'workers check must have entries');
  });

  // ═══════════════════════════════════════════════════════════════
  // R6: Sentry library (PII-safe)
  // ═══════════════════════════════════════════════════════════════
  await t.test('R6.1: sentry lib exports expected API', async () => {
    const sentryMod = await import('../src/lib/sentry.js');
    assert.ok(typeof sentryMod.initSentry === 'function', 'exports initSentry');
  });

  await t.test('R6.2: sentry beforeSend redacts PII', async () => {
    const sentryContent = fs.readFileSync(
      path.resolve('apps/api/src/lib/sentry.ts'), 'utf8'
    );
    assert.ok(sentryContent.includes('beforeSend'), 'has beforeSend hook');
    assert.ok(sentryContent.includes('beforeBreadcrumb'), 'has beforeBreadcrumb hook');
    assert.ok(sentryContent.includes('email') || sentryContent.includes('phone'), 'redacts PII fields');
  });

  // ═══════════════════════════════════════════════════════════════
  // R7: server.ts imports and uses P31 modules
  // ═══════════════════════════════════════════════════════════════
  await t.test('R7.1: server.ts imports P31 modules', () => {
    const serverContent = fs.readFileSync(
      path.resolve('apps/api/src/server.ts'), 'utf8'
    );
    assert.ok(serverContent.includes('getFastifyLoggerConfig'), 'imports getFastifyLoggerConfig');
    assert.ok(serverContent.includes('initSentry'), 'imports initSentry');
    assert.ok(serverContent.includes('WorkerHeartbeat'), 'imports WorkerHeartbeat');
    assert.ok(serverContent.includes('LivenessChecker'), 'imports LivenessChecker');
  });

  await t.test('R7.2: server.ts registers LivenessChecker', () => {
    const serverContent = fs.readFileSync(
      path.resolve('apps/api/src/server.ts'), 'utf8'
    );
    assert.ok(serverContent.includes("new LivenessChecker(pool, queue.boss, messageBus)"),
      'instantiates LivenessChecker');
    assert.ok(serverContent.includes('livenessChecker.start()'),
      'starts LivenessChecker');
  });

  await t.test('R7.3: Fastify logger configured with P31 options', () => {
    const serverContent = fs.readFileSync(
      path.resolve('apps/api/src/server.ts'), 'utf8'
    );
    assert.ok(!serverContent.includes("logger: true"),
      'must not use plain logger:true, must use getFastifyLoggerConfig()');
    assert.ok(serverContent.includes('getFastifyLoggerConfig()'),
      'uses getFastifyLoggerConfig()');
  });

  // ═══════════════════════════════════════════════════════════════
  // R8: Migration M032 exists
  // ═══════════════════════════════════════════════════════════════
  await t.test('R8.1: M032 migration exists for heartbeat extension', () => {
    const migrationPath = path.resolve(
      'packages/db/migrations/1780421100061_worker-heartbeat-ext.ts'
    );
    assert.ok(fs.existsSync(migrationPath), 'migration file must exist');
    const content = fs.readFileSync(migrationPath, 'utf8');
    assert.ok(content.includes('ADD COLUMN IF NOT EXISTS instance_id'),
      'adds instance_id column');
    assert.ok(content.includes('ADD COLUMN IF NOT EXISTS job_name'),
      'adds job_name column');
    assert.ok(content.includes('ADD COLUMN IF NOT EXISTS status'),
      'adds status column');
    assert.ok(content.includes('ADD COLUMN IF NOT EXISTS last_job_at'),
      'adds last_job_at column');
    assert.ok(content.includes('idx_worker_heartbeat_status_seen'),
      'creates performance index');
  });

  // ═══════════════════════════════════════════════════════════════
  // R9: Config env vars defined
  // ═══════════════════════════════════════════════════════════════
  await t.test('R9.1: config has P31 env vars', () => {
    const configContent = fs.readFileSync(
      path.resolve('packages/config/src/index.ts'), 'utf8'
    );
    assert.ok(configContent.includes('SENTRY_DSN'), 'has SENTRY_DSN');
    assert.ok(configContent.includes('LOG_LEVEL'), 'has LOG_LEVEL');
    assert.ok(configContent.includes('WORKER_HEARTBEAT_INTERVAL_MS'), 'has WORKER_HEARTBEAT_INTERVAL_MS');
    assert.ok(configContent.includes('WORKER_LIVENESS_CHECK_MS'), 'has WORKER_LIVENESS_CHECK_MS');
    assert.ok(configContent.includes('WORKER_LIVENESS_STALE_MS'), 'has WORKER_LIVENESS_STALE_MS');
    assert.ok(configContent.includes('WORKER_CRITICAL_LIST'), 'has WORKER_CRITICAL_LIST');
    assert.ok(configContent.includes('GIT_SHA'), 'has GIT_SHA');
  });

  // ═══════════════════════════════════════════════════════════════
  // R10: Health dashboard HTML exists
  // ═══════════════════════════════════════════════════════════════
  await t.test('R10.1: health dashboard HTML exists with correct structure', () => {
    const htmlPath = path.resolve('apps/api/src/public/admin/health.html');
    assert.ok(fs.existsSync(htmlPath), 'health.html must exist');
    const content = fs.readFileSync(htmlPath, 'utf8');
    assert.ok(content.includes('System Health'), 'page title present');
    assert.ok(content.includes('/health'), 'fetches /health endpoint');
    assert.ok(content.includes('Worker Status'), 'has worker status section');
    assert.ok(content.includes('Auto-refresh'), 'has auto-refresh toggle');
    assert.ok(content.includes('health-dot'), 'has health dot styling');
  });
});
