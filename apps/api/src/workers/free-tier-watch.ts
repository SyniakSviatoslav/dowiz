const FREE_TIER_LIMITS = {
  db: { maxBytes: 500 * 1024 * 1024, warnPct: 0.80, critPct: 0.95 },
  storage: { maxBytes: 1 * 1024 * 1024 * 1024, warnPct: 0.80, critPct: 0.95 },
  connections: { maxCount: 15, warnPct: 0.80, critPct: 0.95 },
  egress: { maxBytes: 2 * 1024 * 1024 * 1024, warnPct: 0.80, critPct: 0.95 },
};

export interface FreeTierMetrics {
  dbSizeBytes: number;
  storageBytes: number;
  activeConnections: number;
  egressEstimateBytes: number;
  dbPct: number;
  storagePct: number;
  connectionsPct: number;
  egressPct: number;
  status: 'ok' | 'warning' | 'critical';
}

function computeStatus(pcts: number[]): FreeTierMetrics['status'] {
  if (pcts.some(p => p >= 95)) return 'critical';
  if (pcts.some(p => p >= 80)) return 'warning';
  return 'ok';
}

export async function collectFreeTierMetrics(
  pool?: any,
  makePool?: () => any,
): Promise<FreeTierMetrics> {
  let owned = false;
  let p: any;

  if (pool) {
    p = pool;
  } else if (makePool) {
    p = makePool();
    owned = true;
  } else {
    const { createSessionPool } = await import('@deliveryos/db');
    p = createSessionPool();
    owned = true;
  }

  try {
    const pg = await p.connect();
    try {
      const dbSizeRes = await pg.query(
        `SELECT pg_database_size(current_database()) AS db_size_bytes`,
      );
      const dbSizeBytes = parseInt(dbSizeRes.rows[0].db_size_bytes, 10);

      const storageRes = await pg.query(
        `SELECT COALESCE(SUM(file_size_bytes), 0)::bigint AS storage_bytes FROM upload_audit WHERE status = 'accepted'`,
      );
      const storageBytes = parseInt(storageRes.rows[0].storage_bytes, 10);

      const connRes = await pg.query(
        `SELECT count(*)::int AS cnt FROM pg_stat_activity WHERE state = 'active'`,
      );
      const activeConnections = connRes.rows[0].cnt;

      const egressRes = await pg.query(
        `SELECT COALESCE(SUM(COALESCE((metadata->>'egress_bytes')::bigint, 0)), 0)::bigint AS egress_bytes
         FROM backup_metadata WHERE created_at >= now() - interval '30 days'`,
      );
      const egressEstimateBytes = parseInt(egressRes.rows[0].egress_bytes, 10);

      const dbPct = Number(((dbSizeBytes / FREE_TIER_LIMITS.db.maxBytes) * 100).toFixed(2));
      const storagePct = Number(((storageBytes / FREE_TIER_LIMITS.storage.maxBytes) * 100).toFixed(2));
      const connectionsPct = Number(((activeConnections / FREE_TIER_LIMITS.connections.maxCount) * 100).toFixed(2));
      const egressPct = Number(((egressEstimateBytes / FREE_TIER_LIMITS.egress.maxBytes) * 100).toFixed(2));

      const metrics: FreeTierMetrics = {
        dbSizeBytes, storageBytes, activeConnections, egressEstimateBytes,
        dbPct, storagePct, connectionsPct, egressPct,
        status: computeStatus([dbPct, storagePct, connectionsPct, egressPct]),
      };

      await pg.query(
        `INSERT INTO free_tier_snapshots
         (db_size_bytes, storage_bytes, active_connections, egress_estimate_bytes,
          db_pct, storage_pct, connections_pct, egress_pct, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
        [
          dbSizeBytes, storageBytes, activeConnections, egressEstimateBytes,
          dbPct, storagePct, connectionsPct, egressPct, metrics.status,
        ],
      );

      return metrics;
    } finally {
      pg.release();
    }
  } finally {
    if (owned) await p.end();
  }
}
