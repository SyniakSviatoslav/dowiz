import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';

const FREE_TIER_LIMITS = {
  db: {
    maxBytes: 500 * 1024 * 1024,  // 500 MB
    warnPct: 0.80,
    critPct: 0.95,
  },
  storage: {
    maxBytes: 1 * 1024 * 1024 * 1024,  // 1 GB
    warnPct: 0.80,
    critPct: 0.95,
  },
  connections: {
    maxCount: 15,
    warnPct: 0.80,
    critPct: 0.95,
  },
  egress: {
    maxBytes: 2 * 1024 * 1024 * 1024,  // 2 GB/month
    warnPct: 0.80,
    critPct: 0.95,
  },
};

interface FreeTierMetrics {
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

function computeStatus(metrics: Omit<FreeTierMetrics, 'status'>): FreeTierMetrics['status'] {
  const pcts = [
    metrics.dbPct,
    metrics.storagePct,
    metrics.connectionsPct,
    metrics.egressPct,
  ];
  if (pcts.some(p => p >= 95)) return 'critical';
  if (pcts.some(p => p >= 80)) return 'warning';
  return 'ok';
}

export async function collectFreeTierMetrics(): Promise<FreeTierMetrics> {
  const pool = createSessionPool();

  try {
    const client = await pool.connect();
    try {
      // DB size
      const dbSizeRes = await client.query(
        `SELECT pg_database_size(current_database()) AS db_size_bytes`,
      );
      const dbSizeBytes = parseInt(dbSizeRes.rows[0].db_size_bytes, 10);

      // Storage - sum of file sizes from upload_audit
      const storageRes = await client.query(
        `SELECT COALESCE(SUM(file_size_bytes), 0)::bigint AS storage_bytes FROM upload_audit WHERE status = 'accepted'`,
      );
      const storageBytes = parseInt(storageRes.rows[0].storage_bytes, 10);

      // Active connections
      const connRes = await client.query(
        `SELECT count(*)::int AS cnt FROM pg_stat_activity WHERE state = 'active'`,
      );
      const activeConnections = connRes.rows[0].cnt;

      // Egress estimate - rough proxy via timestamp-since-last-month
      // This is the hardest to estimate without external metrics
      const egressRes = await client.query(
        `SELECT COALESCE(SUM(COALESCE((metadata->>'egress_bytes')::bigint, 0)), 0)::bigint AS egress_bytes
         FROM backup_metadata WHERE created_at >= now() - interval '30 days'`,
      );
      const egressEstimateBytes = parseInt(egressRes.rows[0].egress_bytes, 10);

      const dbPct = Number(((dbSizeBytes / FREE_TIER_LIMITS.db.maxBytes) * 100).toFixed(2));
      const storagePct = Number(((storageBytes / FREE_TIER_LIMITS.storage.maxBytes) * 100).toFixed(2));
      const connectionsPct = Number(((activeConnections / FREE_TIER_LIMITS.connections.maxCount) * 100).toFixed(2));
      const egressPct = Number(((egressEstimateBytes / FREE_TIER_LIMITS.egress.maxBytes) * 100).toFixed(2));

      const metrics: FreeTierMetrics = {
        dbSizeBytes,
        storageBytes,
        activeConnections,
        egressEstimateBytes,
        dbPct,
        storagePct,
        connectionsPct,
        egressPct,
        status: 'ok',
      };
      metrics.status = computeStatus(metrics);

      // Insert snapshot
      await client.query(
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
      client.release();
    }
  } finally {
    await pool.end();
  }
}

async function main() {
  console.log('\n=== Free Tier Watch ===\n');

  try {
    const metrics = await collectFreeTierMetrics();

    console.log(`DB size:        ${(metrics.dbSizeBytes / 1024 / 1024).toFixed(1)} MB (${metrics.dbPct}%)`);
    console.log(`Storage:        ${(metrics.storageBytes / 1024 / 1024).toFixed(1)} MB (${metrics.storagePct}%)`);
    console.log(`Connections:    ${metrics.activeConnections} (${metrics.connectionsPct}%)`);
    console.log(`Egress (est):   ${(metrics.egressEstimateBytes / 1024 / 1024).toFixed(1)} MB (${metrics.egressPct}%)`);
    console.log(`Status:         ${metrics.status}`);

    if (metrics.status === 'critical') {
      console.log('\n❌ CRITICAL: Free tier limit breached! Upgrade to Pro required.');
      process.exit(1);
    } else if (metrics.status === 'warning') {
      console.log('\n⚠ WARNING: Free tier usage above 80%. Plan upgrade.');
      process.exit(0);
    } else {
      console.log('\n✅ All Free tier metrics within safe range.');
      process.exit(0);
    }
  } catch (err: any) {
    console.error('Fatal:', err.message);
    process.exit(1);
  }
}

main();
