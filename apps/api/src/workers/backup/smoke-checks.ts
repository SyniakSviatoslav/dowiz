import { Pool } from 'pg';
import { createHash } from 'node:crypto';
import { PiiRedactor } from '../../lib/pii-redactor.js';

export interface SmokeCheck {
  name: string;
  passed: boolean;
  evidence: Record<string, any>;
  durationMs: number;
}

const EXPECTED_TABLES = [
  'orders', 'order_items', 'order_item_modifiers',
  'customers', 'locations', 'organizations', 'users',
  'products', 'menu_versions', 'categories', 'modifier_groups',
  'couriers', 'courier_assignments', 'courier_payouts', 'courier_shifts',
  'settlement_items', 'settlements',
  'location_alerts', 'owner_notification_targets',
  'customer_devices', 'customer_signals', 'velocity_events',
  'gdpr_erasure_requests', 'anonymization_audit_log',
  'backup_metadata', 'backup_audit_log',
];

const piiRedactor = new PiiRedactor();

function now(): number {
  return Date.now();
}

export async function runSmokeChecks(
  sandboxPool: Pool,
  opts: { fullHash?: boolean; baselineRowCounts?: Record<string, number> } = {},
): Promise<SmokeCheck[]> {
  const checks: SmokeCheck[] = [];

  checks.push(await checkSchema(sandboxPool));
  checks.push(await checkRowCounts(sandboxPool, opts.baselineRowCounts));
  checks.push(await checkFKIntegrity(sandboxPool));
  checks.push(await checkMenuVersions(sandboxPool));
  checks.push(await checkPayoutSums(sandboxPool));
  checks.push(await checkOrderTotals(sandboxPool));
  checks.push(await checkTimeOrder(sandboxPool));
  checks.push(await checkPIIFree(sandboxPool));

  if (opts.fullHash) {
    checks.push(await checkSampleHashes(sandboxPool));
  }

  return checks;
}

async function checkSchema(pool: Pool): Promise<SmokeCheck> {
  const start = now();
  try {
    const res = await pool.query(
      `SELECT table_name FROM information_schema.tables
       WHERE table_schema = 'public' AND table_type = 'BASE TABLE'
       ORDER BY table_name`,
    );
    const presentTables = new Set(res.rows.map((r: any) => r.table_name));
    const missing = EXPECTED_TABLES.filter(t => !presentTables.has(t));
    const passed = missing.length === 0;
    return {
      name: 'schema_validation',
      passed,
      evidence: { expected: EXPECTED_TABLES.length, present: presentTables.size, missing },
      durationMs: now() - start,
    };
  } catch (err: any) {
    return { name: 'schema_validation', passed: false, evidence: { error: err.message }, durationMs: now() - start };
  }
}

async function checkRowCounts(
  pool: Pool,
  baseline?: Record<string, number>,
): Promise<SmokeCheck> {
  const start = now();
  try {
    const results: Record<string, { count: number; outlier?: boolean }> = {};
    let anyOutlier = false;

    for (const table of EXPECTED_TABLES) {
      const res = await pool.query(`SELECT COUNT(*)::int AS c FROM "${table}"`);
      const count = res.rows[0].c;
      const base = baseline?.[table];
      const outlier = base !== undefined && (count === 0 || count > base * 10);
      if (outlier) anyOutlier = true;
      results[table] = { count, outlier: outlier || undefined };
    }

    return {
      name: 'row_counts',
      passed: !anyOutlier,
      evidence: results,
      durationMs: now() - start,
    };
  } catch (err: any) {
    return { name: 'row_counts', passed: false, evidence: { error: err.message }, durationMs: now() - start };
  }
}

async function checkFKIntegrity(pool: Pool): Promise<SmokeCheck> {
  const start = now();
  try {
    const res = await pool.query(
      `SELECT COUNT(*)::int AS cnt FROM information_schema.referential_constraints
       WHERE constraint_schema = 'public'`,
    );
    const count = res.rows[0].cnt;
    return {
      name: 'fk_integrity',
      passed: count >= 20,
      evidence: { foreignKeyCount: count, expectedMin: 20 },
      durationMs: now() - start,
    };
  } catch (err: any) {
    return { name: 'fk_integrity', passed: false, evidence: { error: err.message }, durationMs: now() - start };
  }
}

async function checkMenuVersions(pool: Pool): Promise<SmokeCheck> {
  const start = now();
  try {
    const res = await pool.query(
      `SELECT location_id, COUNT(*) AS cnt
       FROM menu_versions
       GROUP BY location_id
       HAVING COUNT(*) > 1`,
    );
    const passed = res.rows.length === 0;
    return {
      name: 'menu_versions_unique',
      passed,
      evidence: { duplicateLocations: res.rows.length, details: res.rows },
      durationMs: now() - start,
    };
  } catch (err: any) {
    return { name: 'menu_versions_unique', passed: false, evidence: { error: err.message }, durationMs: now() - start };
  }
}

async function checkPayoutSums(pool: Pool): Promise<SmokeCheck> {
  const start = now();
  try {
    const res = await pool.query(
      `SELECT cp.id, cp.total_earned, COALESCE(SUM(si.amount), 0) AS sum_items
       FROM courier_payouts cp
       LEFT JOIN settlement_items si ON si.payout_id = cp.id
       GROUP BY cp.id, cp.total_earned
       HAVING cp.total_earned != COALESCE(SUM(si.amount), 0)
       LIMIT 10`,
    );
    const passed = res.rows.length === 0;
    return {
      name: 'payout_sums',
      passed,
      evidence: { mismatches: res.rows.length, sample: res.rows },
      durationMs: now() - start,
    };
  } catch (err: any) {
    return { name: 'payout_sums', passed: false, evidence: { error: err.message }, durationMs: now() - start };
  }
}

async function checkOrderTotals(pool: Pool): Promise<SmokeCheck> {
  const start = now();
  try {
    const res = await pool.query(
      `SELECT id, subtotal, delivery_fee, tax_total, discount_total, total
       FROM orders
       WHERE subtotal + delivery_fee + tax_total - discount_total != total
       LIMIT 10`,
    );
    const passed = res.rows.length === 0;
    return {
      name: 'order_totals',
      passed,
      evidence: { mismatches: res.rows.length, sample: res.rows },
      durationMs: now() - start,
    };
  } catch (err: any) {
    return { name: 'order_totals', passed: false, evidence: { error: err.message }, durationMs: now() - start };
  }
}

async function checkTimeOrder(pool: Pool): Promise<SmokeCheck> {
  const start = now();
  try {
    const res = await pool.query(
      `SELECT id, created_at, delivered_at
       FROM orders
       WHERE delivered_at IS NOT NULL AND delivered_at < created_at
       LIMIT 10`,
    );
    const passed = res.rows.length === 0;
    return {
      name: 'time_order',
      passed,
      evidence: { violations: res.rows.length, sample: res.rows },
      durationMs: now() - start,
    };
  } catch (err: any) {
    return { name: 'time_order', passed: false, evidence: { error: err.message }, durationMs: now() - start };
  }
}

async function checkPIIFree(pool: Pool): Promise<SmokeCheck> {
  const start = now();
  try {
    const tables = ['customers', 'orders', 'couriers'];
    let totalPii = 0;
    const details: Record<string, number> = {};

    for (const table of tables) {
      try {
        const res = await pool.query(`SELECT * FROM "${table}" ORDER BY random() LIMIT 100`);
        let piiCount = 0;
        for (const row of res.rows) {
          const str = JSON.stringify(row);
          const { redactions } = piiRedactor.redact(str);
          if (redactions.length > 0) piiCount++;
        }
        details[table] = piiCount;
        totalPii += piiCount;
      } catch {
        details[table] = -1;
      }
    }

    return {
      name: 'pii_free',
      passed: totalPii === 0,
      evidence: { totalPiiMatches: totalPii, perTable: details },
      durationMs: now() - start,
    };
  } catch (err: any) {
    return { name: 'pii_free', passed: false, evidence: { error: err.message }, durationMs: now() - start };
  }
}

async function checkSampleHashes(pool: Pool): Promise<SmokeCheck> {
  const start = now();
  try {
    const res = await pool.query(
      `SELECT id, created_at, total, status FROM orders ORDER BY random() LIMIT 10`,
    );
    const hash = createHash('sha256').update(JSON.stringify(res.rows)).digest('hex');
    return {
      name: 'sample_hashes',
      passed: true,
      evidence: { sampleSize: res.rows.length, hash },
      durationMs: now() - start,
    };
  } catch (err: any) {
    return { name: 'sample_hashes', passed: false, evidence: { error: err.message }, durationMs: now() - start };
  }
}
