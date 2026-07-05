import type { FastifyPluginAsync } from 'fastify';
import { runRestoreVerify } from '../../workers/backup/backup-verify.js';
import { auditCtx, auditStart, auditFinish, auditCompleted } from '../../lib/platform-admin.js';

const backupAdminRoutes: FastifyPluginAsync = async (fastify, opts) => {
  const { db, queue } = opts as any;

  // ADR-admin-platform-authz (B4): auth is the platform-admin gate on the parent plane
  // (routes/admin/index.ts) + the root-instance gate in server.ts — NOT a per-file owner check.
  // Do NOT re-add verifyAuth/requireRole here (an owner check would 403 a legitimate platform-admin).

  // GET /api/admin/backups — list recent backups
  fastify.get('/backups', async (request: any, reply: any) => {
    const { type, status, limit } = request.query as any;
    const maxLimit = Math.min(parseInt(limit) || 50, 100);

    let query = `SELECT id, type, status, created_at, size_bytes, duration_ms,
                        checksum_sha256 IS NOT NULL AS has_checksum
                 FROM backup_metadata WHERE 1=1`;
    const params: any[] = [];
    let idx = 1;

    if (type) {
      query += ` AND type = $${idx++}`;
      params.push(type);
    }
    if (status) {
      query += ` AND status = $${idx++}`;
      params.push(status);
    }

    query += ` ORDER BY created_at DESC LIMIT $${idx++}`;
    params.push(maxLimit);

    const res = await db.query(query, params);

    // Get restore-test results for each backup from audit log
    const backupIds = res.rows.map((r: any) => r.id);
    let restoreResults: Record<string, string> = {};
    if (backupIds.length > 0) {
      const auditRes = await db.query(
        `SELECT DISTINCT ON (backup_id) backup_id, metadata
         FROM backup_audit_log
         WHERE backup_id = ANY($1) AND action = 'restore_drill_completed'
         ORDER BY backup_id, created_at DESC`,
        [backupIds],
      );
      for (const row of auditRes.rows) {
        const meta = typeof row.metadata === 'string' ? JSON.parse(row.metadata) : (row.metadata || {});
        restoreResults[row.backup_id] = meta?.result === true ? 'passed' : 'failed';
      }
    }

    await auditCompleted(db, auditCtx(request, 'backups.list'), request.log);
    return {
      backups: res.rows.map((r: any) => ({
        id: r.id,
        type: r.type,
        status: r.status,
        created_at: r.created_at,
        size_bytes: r.size_bytes,
        duration_ms: r.duration_ms,
        has_checksum: r.has_checksum,
        restore_test_result: restoreResults[r.id] || null,
      })),
    };
  });

  // POST /api/admin/backups/verify — trigger manual restore-test (DR-drill, weaponizable)
  // ADR-admin-platform-authz §4: platform-admin (parent gate) + Zod-uuid backupId + per-actor
  // rate-limit + single-flight (the advisory lock inside runRestoreVerify → 409 if a drill is in
  // flight). backupId is uuid-validated so an arbitrary/unbounded body can't drive the drill.
  fastify.post('/backups/verify', {
    config: { rateLimit: { max: 3, timeWindow: '5 minutes' } },
  }, async (request: any, reply: any) => {
    const body = request.body as any;
    const backupId = body?.backupId;
    if (backupId !== undefined && !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(String(backupId))) {
      return reply.status(400).send({ error: 'VALIDATION_FAILED', message: 'backupId must be a uuid' });
    }
    // Write-ahead audit: a 'started' row is durable BEFORE the (destructive) drill runs.
    const auditId = await auditStart(db, auditCtx(request, 'backups.verify', backupId ?? null));
    try {
      const result = await runRestoreVerify(db, { backupId, fullHash: false });
      // single-flight: the internal advisory lock returns this when a drill is already in flight.
      if (result?.error === 'Another verify in progress') {
        await auditFinish(db, auditId, 'failed');
        return reply.status(409).send({ error: 'drill_in_progress' });
      }
      await auditFinish(db, auditId, result?.success ? 'completed' : 'failed');
      return result;
    } catch (err) {
      await auditFinish(db, auditId, 'failed');
      throw err;
    }
  });

  // GET /api/admin/backups/dr-report — generate DR drill report (heavy, fleet-wide DR-drill)
  // ADR-admin-platform-authz §4: platform-admin (parent gate) + rate-limit + single-flight (409).
  fastify.get('/backups/dr-report', {
    config: { rateLimit: { max: 3, timeWindow: '5 minutes' } },
  }, async (request: any, reply: any) => {
    const auditId = await auditStart(db, auditCtx(request, 'backups.dr_report'));
    try {
      const result = await runRestoreVerify(db, { fullHash: true });
      if (result?.error === 'Another verify in progress') {
        await auditFinish(db, auditId, 'failed');
        return reply.status(409).send({ error: 'drill_in_progress' });
      }
      await auditFinish(db, auditId, result?.success ? 'completed' : 'failed');
      return result;
    } catch (err) {
      await auditFinish(db, auditId, 'failed');
      throw err;
    }
  });
};

export default backupAdminRoutes;
