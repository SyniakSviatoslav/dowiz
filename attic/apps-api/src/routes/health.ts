import type { FastifyInstance } from 'fastify';
import type { MessageBus } from '@deliveryos/platform';
import type { Pool } from 'pg';

const CHECK_TIMEOUT_MS = 2000;

interface HealthCheckBase {
  status: 'ok' | 'degraded' | 'down';
  latencyMs?: number;
  detail?: string;
}

type HealthCheckResult<T = undefined> = HealthCheckBase & { data?: T };

async function withTimeout<T>(
  promise: Promise<T>,
  label: string,
  treatErrorAsDegraded = false,
  treatTimeoutAsDown = false,
): Promise<HealthCheckResult<T>> {
  const start = Date.now();
  try {
    const result = await Promise.race([
      promise,
      new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error('timeout')), CHECK_TIMEOUT_MS),
      ),
    ]);
    return { status: 'ok', latencyMs: Date.now() - start, data: result };
  } catch (err: any) {
    const latencyMs = Date.now() - start;
    const isTimeout = err.message === 'timeout';
    // A hung/unreachable Postgres surfaces here as a timeout. For the CRITICAL pg
    // check that must be 'down' (→ 503) so external uptime monitors page during an
    // outage, NOT 'degraded'+200 (which kept monitors green while the storefront 500'd).
    // Soft checks keep 'degraded' so a transient external blip never 503s the app.
    // detail carries NO err.message — the public /health is unauthenticated, raw
    // driver text (host/role/SQLSTATE) is a recon leak (and is dropped at serialization).
    if (isTimeout && treatTimeoutAsDown) {
      return { status: 'down', latencyMs, detail: `${label} check timed out` };
    }
    if (isTimeout || treatErrorAsDegraded) {
      return { status: 'degraded', latencyMs, detail: `${label} check timed out or failed` };
    }
    return { status: 'down', latencyMs, detail: `${label} check failed` };
  }
}

export default async function healthRoutes(
  fastify: FastifyInstance,
  opts: { db: Pool; messageBus: MessageBus },
) {
  // Liveness probe — the ONLY endpoint Fly's machine health check should hit.
  // It must answer well within Fly's check timeout (3s). The rich /health below
  // runs ~11 sequential DB queries plus external Telegram/R2 calls and can blow
  // past 3s under load, which made Fly kill the machine and drop every live
  // WebSocket ("service falling down"). Liveness only asks "is the event loop
  // responsive?" — it deliberately does NOT touch Postgres, because restarting
  // the machine can't fix an external DB blip and would just sever all sockets.
  // Use /health for DB-aware readiness/ops monitoring.
  fastify.get('/livez', { config: { rateLimit: false } }, async (_request, reply) => {
    return reply.status(200).send({ status: 'ok', timestamp: new Date().toISOString() });
  });

  fastify.get('/health', { config: { rateLimit: false } }, async (request, reply) => {
    // ── 1. Postgres (Critical) ──────────────────────────────────────
    const pgResult = await withTimeout<{ rows: Array<{ alive: number }> }>(
      opts.db.query('SELECT 1 AS alive'),
      'postgres',
      false, // not a soft check
      true,  // treat a pg timeout as DOWN, not degraded
    );
    if (pgResult.status === 'down') {
      return reply.status(503).send({
        status: 'unhealthy',
        timestamp: new Date().toISOString(),
        checks: { postgres: { status: pgResult.status, latencyMs: pgResult.latencyMs } },
      });
    }

    // ── 2. Worker heartbeat status ──────────────────────────────────
    let workerEntries: Record<
      string,
      { status: string; instanceId?: string; jobName?: string; staleSeconds?: number }
    > = {};
    const workerResult = await withTimeout<{
      rows: Array<{
        worker_id: string;
        instance_id: string | null;
        job_name: string | null;
        status: string;
        stale_seconds: number;
      }>;
    }>(
      opts.db.query(
        `SELECT worker_id, instance_id, job_name, status,
                EXTRACT(EPOCH FROM now() - last_seen_at)::int AS stale_seconds
         FROM ops_worker_heartbeat
         WHERE last_seen_at >= now() - interval '60 seconds'
         ORDER BY worker_id`,
      ),
      'workers',
    );
    if (workerResult.status === 'ok' && workerResult.data) {
      workerEntries = Object.fromEntries(
        workerResult.data.rows.map((r) => [
          r.worker_id,
          {
            status: r.status === 'healthy' && r.stale_seconds < 60 ? 'ok' : 'degraded',
            instanceId: r.instance_id || undefined,
            jobName: r.job_name || undefined,
            staleSeconds: r.stale_seconds,
          },
        ]),
      );
    }

    // ── 3. MessageBus / Redis (Degraded) ───────────────────────────
    const redisResult = await withTimeout(opts.messageBus.checkHealth(), 'messageBus');

    // ── 4. Telegram (Degraded) ─────────────────────────────────────
    const telegramEnabled = !!process.env.TELEGRAM_BOT_TOKEN;
    let telegramResult: HealthCheckBase = { status: 'ok' };
    if (telegramEnabled) {
      telegramResult = await withTimeout(
        (async () => {
          const resp = await fetch(
            `https://api.telegram.org/bot${process.env.TELEGRAM_BOT_TOKEN}/getMe`,
            { signal: AbortSignal.timeout(5000) },
          );
          if (!resp.ok) throw new Error(`Telegram API getMe returned ${resp.status}`);
          const body = await resp.json();
          if (!body.ok) throw new Error(body.description ?? 'getMe not ok');
          return body;
        })(),
        'telegram',
        true,
      );
    }

    // ── 5. R2 (Degraded) ──────────────────────────────────────────
    let r2Result: HealthCheckBase = { status: 'ok' };
    const backupEnabled = process.env.BACKUP_ENABLED === 'true';
    if (backupEnabled) {
      r2Result = await withTimeout(
        (async () => {
          const { HeadBucketCommand, S3Client } = await import('@aws-sdk/client-s3');
          const client = new S3Client({
            endpoint: process.env.R2_ENDPOINT,
            region: 'auto',
            credentials: {
              accessKeyId: process.env.R2_ACCESS_KEY_ID || '',
              secretAccessKey: process.env.R2_SECRET_ACCESS_KEY || '',
            },
          });
          await client.send(new HeadBucketCommand({ Bucket: process.env.R2_BUCKET }));
        })(),
        'r2',
      );
    }

    // ── 6. Settlement (Degraded) ────────────────────────────────────
    const settlementResult = await withTimeout(
      opts.db.query(
        `SELECT MAX(created_at) AS last_period_end
         FROM settlement_audit_log WHERE action = 'generated' LIMIT 1`,
      ),
      'settlement',
      true
    );

    // ── 7. Anonymizer (Degraded) ────────────────────────────────────
    const anonResult = await withTimeout(
      opts.db.query(
        `SELECT MAX(created_at) AS last_run
         FROM anonymization_audit_log WHERE scope = 'retention' LIMIT 1`,
      ),
      'anonymizer',
      true
    );

    // ── 8. Backup drift (Degraded) ──────────────────────────────────
    let backupResult: HealthCheckBase = { status: 'ok' };
    if (backupEnabled) {
      const backupCheck = await withTimeout<{
        rows: Array<{ type: string; last_completed_at: string }>;
      }>(
        opts.db.query(
          `SELECT type, MAX(completed_at) as last_completed_at
           FROM backup_metadata WHERE status = 'completed' GROUP BY type`,
        ),
        'backup',
        true
      );
      if (backupCheck.status === 'ok' && backupCheck.data) {
        const now = Date.now();
        const limits: Record<string, number> = {
          hourly: 90 * 60 * 1000,
          daily: 26 * 60 * 60 * 1000,
        };
        const hasDrift = backupCheck.data.rows.some((r) => {
          const limit = limits[r.type];
          if (!limit) return false;
          return now - new Date(r.last_completed_at).getTime() > limit;
        });
        backupResult = { status: hasDrift ? 'degraded' : 'ok' };
      }
    }

    // ── 9. Backup restore-test status (Degraded) ──────────────────────────────────
    let backupRestoreResult: any = { status: 'ok' };
    if (backupEnabled) {
      const restoreCheck = await withTimeout<{
        rows: Array<{ last_verified_at: string | null; result: boolean | null }>;
      }>(
        opts.db.query(
          `SELECT MAX(created_at) AS last_verified_at,
                  bool_and(metadata->>'result' = 'success') AS result
           FROM backup_audit_log
           WHERE action IN ('restore_drill_started', 'restore_drill_completed')
           LIMIT 1`,
        ),
        'backup_restore',
        true
      );
      if (restoreCheck.status === 'ok' && restoreCheck.data) {
        const row = restoreCheck.data.rows[0];
        const lastVerified = row?.last_verified_at;
        const lastResult = row?.result;
        const neverRun = !lastVerified;
        const isStale = !lastVerified || (Date.now() - new Date(lastVerified).getTime() > 25 * 60 * 60 * 1000);
        backupRestoreResult = {
          last_verified_at: lastVerified,
          last_result: neverRun ? 'never_run' : (lastResult === true ? 'success' : 'failed'),
          stale: isStale,
          status: neverRun ? 'ok' : (isStale ? 'degraded' : 'ok'),
        };
      }
    }

    // ── 10. Fallback config coverage (Degraded) ─────────────────────
    let fallbackResult: HealthCheckBase = { status: 'ok' };
    const fallbackCheck = await withTimeout<{
      rows: Array<{ total: number; with_phone: number }>;
    }>(
      opts.db.query(
        `SELECT COUNT(*)::int AS total,
                COUNT(*) FILTER (WHERE fallback_config->>'phone' IS NOT NULL AND fallback_config->>'phone' != '') AS with_phone
         FROM locations`,
      ),
      'fallback',
      true
    );
    if (fallbackCheck.status === 'ok' && fallbackCheck.data) {
      const row = fallbackCheck.data.rows[0];
      if (row) {
        const pct = row.total > 0 ? Math.round((row.with_phone / row.total) * 100) : 0;
        fallbackResult = {
          status: pct < 50 ? 'degraded' : 'ok',
          detail: `${row.with_phone}/${row.total} locations have fallback phone configured (${pct}%)`,
          data: { totalLocations: row.total, withFallbackPhone: row.with_phone, coveragePct: pct },
        } as any;
      }
    }

    // ── 11. Free-tier metrics (Degraded) ────────────────────────────
    let freeTierResult: HealthCheckBase = { status: 'ok' };
    const freeTierCheck = await withTimeout<{
      rows: Array<{ status: string; db_pct: number; storage_pct: number; connections_pct: number; egress_pct: number }>;
    }>(
      opts.db.query(
        `SELECT status, db_pct, storage_pct, connections_pct, egress_pct
         FROM free_tier_snapshots
         ORDER BY created_at DESC LIMIT 1`,
      ),
      'free_tier',
      true
    );
    if (freeTierCheck.status === 'ok' && freeTierCheck.data) {
      const row = freeTierCheck.data.rows[0];
      if (row) {
        freeTierResult = {
          status: row.status === 'critical' ? 'degraded' : 'ok',
          detail: `DB: ${row.db_pct}% | Storage: ${row.storage_pct}% | Conns: ${row.connections_pct}% | Egress: ${row.egress_pct}%`,
        } as any;
      }
    }

    // ── Determine overall status ────────────────────────────────────
    const allChecks = {
      postgres: pgResult,
      workers: { status: 'ok' as const, entries: workerEntries },
      messageBus: redisResult,
      telegram: telegramResult,
      r2: r2Result,
      settlement: settlementResult,
      anonymizer: anonResult,
      backup: backupResult,
      backup_restore: backupRestoreResult,
      fallback: fallbackResult,
      free_tier: freeTierResult,
    };

    const hasDown = Object.values(allChecks).some((c) => c.status === 'down');
    const hasDegraded = Object.values(allChecks).some((c) => c.status === 'degraded');

    let overallStatus: string;
    let httpStatus: number;
    if (hasDown) {
      overallStatus = 'unhealthy';
      httpStatus = 503;
    } else if (hasDegraded) {
      overallStatus = 'degraded';
      httpStatus = 200;
    } else {
      overallStatus = 'healthy';
      httpStatus = 200;
    }

    // Public payload is minimal by construction: only { status, latencyMs } per
    // check. /health is unauthenticated, so we never serialize node-postgres result
    // objects (data: rows incl. oid/dataTypeID/type-catalog) or raw driver detail —
    // both are recon surface. Overall status still reflects every check internally.
    const publicChecks = Object.fromEntries(
      Object.entries(allChecks).map(([name, c]: [string, any]) => [
        name,
        c.latencyMs != null ? { status: c.status, latencyMs: c.latencyMs } : { status: c.status },
      ]),
    );

    return reply.status(httpStatus).send({
      status: overallStatus,
      timestamp: new Date().toISOString(),
      checks: publicChecks,
    });
  });
}
