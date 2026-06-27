import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { createOperationalPool, createSessionPool } from '@deliveryos/db';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();
void env; // loaded for side-effect parity with other phase5 specs

// Local UUID shape guard (this spec is under apps/api/tests, not e2e/ — inline regex).
const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
function expectUuid(v: unknown, label: string): asserts v is string {
  assert.ok(typeof v === 'string' && UUID_RE.test(v), `${label}: expected a UUID, got ${String(v)}`);
}
// Postgres RLS / policy violation error codes & messages (everything else must re-throw).
function isRlsViolation(err: unknown): boolean {
  const e = err as { code?: string; message?: string };
  return (
    e?.code === '42501' ||
    /row-level security|violates row-level|new row violates|policy/i.test(e?.message ?? '')
  );
}

// High-value tables addressable by primary key without a location_id filter (IDOR surface).
const IDOR_TABLES = ['orders', 'customers', 'courier_positions'] as const;

const tenantTables = [
  'locations', 'categories', 'products', 'modifier_groups', 'modifiers',
  'product_modifier_groups', 'order_item_modifiers', 'order_status_history',
  'delivery_tiers', 'memberships', 'orders', 'order_items', 'customers',
  'couriers', 'courier_positions', 'delivery_trace', 'delivery_flags',
  'courier_shifts', 'courier_cash_ledger', 'courier_assignments',
  'courier_locations', 'courier_invites', 'courier_audit_log',
  'courier_payouts', 'settlement_items', 'settlement_audit_log',
  'courier_dispatch_queue', 'customer_signals', 'velocity_events',
  'customer_otp_sessions', 'phone_otp', 'customer_devices',
  'location_themes', 'theme_versions', 'owner_notification_targets',
  'location_alerts', 'gdpr_erasure_requests', 'anonymization_audit_log',
  'customer_contact_reveals',
];

const READ_ONLY_PUBLIC = new Set([
  'categories', 'products', 'modifier_groups', 'modifiers',
  'product_modifier_groups', 'delivery_tiers',
]);

let ownerAId: string;
let ownerBId: string;
let tenantALocationId: string;
let tenantBLocationId: string;
// Real tenant-B primary keys (resolved via the bypass pool) for IDOR sub-tests.
const tenantBRowIds: Partial<Record<(typeof IDOR_TABLES)[number], string>> = {};

async function setup() {
  const sessionPool = createSessionPool();
  try {
    const resA = await sessionPool.query(`SELECT id FROM users WHERE email = 'ownera@demo.com'`);
    const resB = await sessionPool.query(`SELECT id FROM users WHERE email = 'ownerb@demo2.com'`);
    if (resA.rowCount === 0 || resB.rowCount === 0) {
      console.error('❌ Missing seeded users. Run `pnpm seed` first.');
      process.exit(1);
    }
    ownerAId = resA.rows[0].id;
    ownerBId = resB.rows[0].id;

    // Resolve each owner's OWN location via membership ownership — not created_at order,
    // which does not prove ownerA owns tenant A or ownerB owns tenant B.
    const ownLoc = async (userId: string) =>
      sessionPool.query(
        `SELECT l.id FROM locations l JOIN memberships m ON m.location_id = l.id
         WHERE m.user_id = $1 ORDER BY l.created_at LIMIT 1`,
        [userId],
      );
    const locA = await ownLoc(ownerAId);
    const locB = await ownLoc(ownerBId);
    if (locA.rowCount === 0 || locB.rowCount === 0) {
      console.error('❌ Each demo owner must own at least one location (memberships).');
      process.exit(1);
    }
    tenantALocationId = locA.rows[0].id;
    tenantBLocationId = locB.rows[0].id;
    assert.notStrictEqual(tenantALocationId, tenantBLocationId,
      'tenant A and tenant B must be distinct locations');

    // Resolve a real tenant-B row id per IDOR table (bypass pool sees all tenants).
    for (const table of IDOR_TABLES) {
      const r = await sessionPool.query(
        `SELECT id FROM ${table} WHERE location_id = $1 LIMIT 1`,
        [tenantBLocationId],
      );
      if (r.rowCount && r.rowCount > 0) tenantBRowIds[table] = r.rows[0].id;
    }
  } finally {
    await sessionPool.end();
  }
}

test('H1: Adversarial cross-tenant RLS audit', async (t) => {
  await setup();

  const pool = createOperationalPool();
  // Bypass pool: RLS-unfiltered ground truth to verify the ACTUAL DML outcome (not a
  // COUNT that owner A can never see anyway). Closing both in t.after.
  const verifyPool = createSessionPool();
  t.after(async () => { await pool.end(); await verifyPool.end(); });

  // Positive control: the gate must not be silently rejecting everyone — owner A MUST
  // be able to read their OWN tenant-A location through the RLS-enforced operational pool.
  await t.test('positive control: owner A reads own tenant-A location', async () => {
    const client = await pool.connect();
    try {
      await client.query('BEGIN');
      await client.query("SELECT set_config('app.user_id', $1, true)", [ownerAId]);
      const res = await client.query(`SELECT id FROM locations WHERE id = $1`, [tenantALocationId]);
      await client.query('COMMIT');
      assert.strictEqual(res.rowCount, 1, 'owner A cannot read their own location — RLS over-blocks');
    } finally {
      client.release();
    }
  });

  for (const table of tenantTables) {
    const isReadOnlyPublic = READ_ONLY_PUBLIC.has(table);

    await t.test(`${table}: cross-tenant SELECT = 0 rows`, async () => {
      const client = await pool.connect();
      try {
        await client.query('BEGIN');
        await client.query("SELECT set_config('app.user_id', $1, true)", [ownerAId]);
        const res = await client.query(
          `SELECT * FROM ${table} WHERE location_id = $1 LIMIT 1`,
          [tenantBLocationId],
        );
        await client.query('COMMIT');
        assert.strictEqual(res.rowCount, 0,
          `${table}: Owner A saw data for tenant B location`);
      } finally {
        client.release();
      }
    });

    if (!isReadOnlyPublic) {
      await t.test(`${table}: cross-tenant INSERT denied`, async () => {
        const client = await pool.connect();
        // Known id so we can verify ITS absence via the bypass pool (a COUNT that owner A
        // runs is RLS-filtered and therefore vacuous regardless of whether the insert landed).
        const maliciousId = crypto.randomUUID();
        try {
          await client.query('BEGIN');
          await client.query("SELECT set_config('app.user_id', $1, true)", [ownerAId]);
          try {
            await client.query(
              `INSERT INTO ${table} (id, location_id) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
              [maliciousId, tenantBLocationId],
            );
          } catch (err) {
            // ONLY an RLS/policy violation is an acceptable block here. NOT NULL / FK / type
            // errors mean the test never actually exercised RLS — re-throw so it cannot pass blind.
            // TODO(needs-staging): supply a minimal valid row per table so non-RLS tables also
            // reach the WITH CHECK clause instead of failing on a NOT NULL column first.
            if (!isRlsViolation(err)) throw err;
          }
          await client.query('COMMIT');
        } finally {
          client.release();
        }
        // Ground truth via bypass pool: the malicious row must NOT exist in any tenant.
        const check = await verifyPool.query(
          `SELECT 1 FROM ${table} WHERE id = $1`,
          [maliciousId],
        );
        assert.strictEqual(check.rowCount, 0,
          `${table}: Owner A inserted a row into tenant B (RLS WITH CHECK bypassed)`);
      });

      await t.test(`${table}: cross-tenant UPDATE = 0 rows`, async () => {
        const client = await pool.connect();
        try {
          await client.query('BEGIN');
          await client.query("SELECT set_config('app.user_id', $1, true)", [ownerAId]);
          // SET id = id is a no-op present on every table — avoids schema noise (a missing
          // updated_at column) masquerading as an RLS signal; 0 rows == RLS USING blocked.
          const res = await client.query(
            `UPDATE ${table} SET id = id WHERE location_id = $1`,
            [tenantBLocationId],
          );
          await client.query('COMMIT');
          assert.strictEqual(res.rowCount, 0,
            `${table}: Owner A updated tenant B rows`);
        } finally {
          client.release();
        }
      });

      await t.test(`${table}: cross-tenant DELETE = 0 rows`, async () => {
        const client = await pool.connect();
        try {
          await client.query('BEGIN');
          await client.query("SELECT set_config('app.user_id', $1, true)", [ownerAId]);
          const res = await client.query(
            `DELETE FROM ${table} WHERE location_id = $1`,
            [tenantBLocationId],
          );
          await client.query('COMMIT');
          assert.strictEqual(res.rowCount, 0,
            `${table}: Owner A deleted tenant B rows`);
        } finally {
          client.release();
        }
      });
    }
  }

  // Privileged pool sweep — each query must have WHERE location_id
  await t.test('privileged pool queries have WHERE location_id', async () => {
    const srcDir = 'apps/api/src/workers';
    const files = findTsFiles(srcDir);
    const violations: string[] = [];
    for (const file of files) {
      const content = require('fs').readFileSync(file, 'utf8');
      const lines = content.split('\n');
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (line.includes('.query(') && line.includes('SELECT') &&
            !line.includes('WHERE location_id') &&
            !line.includes('SET LOCAL') &&
            !line.includes('SELECT 1') &&
            !line.includes('SELECT COUNT') &&
            !line.includes('-- no-location-id')) {
          const nextLine = lines[i + 1] || '';
          if (!nextLine.includes('WHERE location_id') && !line.includes('WHERE location_id')) {
            const moreLines = lines.slice(i, Math.min(i + 4, lines.length)).join('\\n');
            if (!moreLines.includes('WHERE location_id')) {
              violations.push(`${file}:${i + 1}: ${line.trim().slice(0, 80)}`);
            }
          }
        }
      }
    }
    assert.strictEqual(violations.length, 0,
      `Privileged pool queries without WHERE location_id:\n${violations.slice(0, 10).join('\n')}`);
  });
});

function findTsFiles(dir: string): string[] {
  const fs = require('node:fs');
  const path = require('node:path');
  const results: string[] = [];
  try {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory() && !entry.name.startsWith('.') && entry.name !== 'node_modules') {
        results.push(...findTsFiles(full));
      } else if (entry.isFile() && (entry.name.endsWith('.ts') || entry.name.endsWith('.js'))) {
        results.push(full);
      }
    }
  } catch {
    // skip unreadable directories
    console.debug('[rls-adversarial] skipping unreadable dir');
  }
  return results;
}
