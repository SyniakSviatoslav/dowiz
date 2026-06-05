import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { createOperationalPool, createSessionPool } from '@deliveryos/db';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();
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

    const locRes = await sessionPool.query(
      `SELECT id FROM locations ORDER BY created_at LIMIT 2`,
    );
    if (locRes.rowCount < 2) {
      console.error('❌ Need at least 2 seeded locations.');
      process.exit(1);
    }
    tenantALocationId = locRes.rows[0].id;
    tenantBLocationId = locRes.rows[1].id;
  } finally {
    await sessionPool.end();
  }
}

test('H1: Adversarial cross-tenant RLS audit', async (t) => {
  await setup();

  const pool = createOperationalPool();
  t.after(async () => { await pool.end(); });

  const testId = `test_${crypto.randomUUID().slice(0, 8)}`;

  for (const table of tenantTables) {
    const isReadOnlyPublic = READ_ONLY_PUBLIC.has(table);

    await t.test(`${table}: cross-tenant SELECT = 0 rows`, async () => {
      const client = await pool.connect();
      try {
        await client.query('BEGIN');
        await client.query('SET LOCAL app.user_id = $1', [ownerAId]);
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
        try {
          await client.query('BEGIN');
          await client.query('SET LOCAL app.user_id = $1', [ownerAId]);
          try {
            await client.query(
              `INSERT INTO ${table} (id, location_id) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
              [crypto.randomUUID(), tenantBLocationId],
            );
          } catch {
            // RLS policy violation is fine
          }
          // Verify nothing was inserted
          const check = await client.query(
            `SELECT COUNT(*)::int AS cnt FROM ${table} WHERE location_id = $1`,
            [tenantBLocationId],
          );
          assert.strictEqual(check.rows[0].cnt, 0,
            `${table}: Owner A inserted into tenant B`);
          await client.query('COMMIT');
        } finally {
          client.release();
        }
      });

      await t.test(`${table}: cross-tenant UPDATE = 0 rows`, async () => {
        const client = await pool.connect();
        try {
          await client.query('BEGIN');
          await client.query('SET LOCAL app.user_id = $1', [ownerAId]);
          const res = await client.query(
            `UPDATE ${table} SET updated_at = now() WHERE location_id = $1`,
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
          await client.query('SET LOCAL app.user_id = $1', [ownerAId]);
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
  } catch { }
  return results;
}
