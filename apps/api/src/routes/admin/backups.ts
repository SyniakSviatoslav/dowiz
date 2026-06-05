import type { FastifyPluginAsync } from 'fastify';
import { runRestoreVerify } from '../../workers/backup/backup-verify.js';

const backupAdminRoutes: FastifyPluginAsync = async (fastify, opts) => {
  const { db, queue } = opts as any;

  // Auth: require owner JWT for all backup operations
  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));

  // GET /api/admin/backups — list recent backups
  fastify.get('/backups', async (request, reply) => {
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

  // POST /api/admin/backups/verify — trigger manual restore-test
  fastify.post('/backups/verify', async (request, reply) => {
    const body = request.body as any;
    const backupId = body?.backupId;

    // Run async, return immediately with status
    const result = await runRestoreVerify(db, { backupId, fullHash: false });
    return result;
  });

  // GET /api/admin/backups/dr-report — generate DR drill report
  fastify.get('/backups/dr-report', async (request, reply) => {
    const result = await runRestoreVerify(db, { fullHash: true });
    return result;
  });
};

export default backupAdminRoutes;
