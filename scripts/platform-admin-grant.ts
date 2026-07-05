#!/usr/bin/env tsx
/**
 * ADR-admin-platform-authz (B4) §8 — ops-only provisioning for platform_admins.
 *
 * The ONLY writer of the platform_admins allowlist (no API write path → self-serve escalation is
 * structurally impossible). Requires DB credentials (DATABASE_URL_MIGRATIONS) — already god-mode, so
 * this adds no escalation beyond what DB creds grant. KEEP this off any CI/API path with ambient prod
 * creds; its safety property is "no ambient creds".
 *
 *   pnpm tsx scripts/platform-admin-grant.ts <userId>            # grant (idempotent)
 *   pnpm tsx scripts/platform-admin-grant.ts <userId> --revoke   # revoke (insider-removal, immediate)
 *   pnpm tsx scripts/platform-admin-grant.ts --list
 *
 * Provision >=2 admins at bootstrap (bus-factor R3). Revoke is the offboarding runbook step.
 */
import { Client } from 'pg';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

async function main() {
  const args = process.argv.slice(2);
  const connectionString = process.env.DATABASE_URL_MIGRATIONS || process.env.DATABASE_URL;
  if (!connectionString) {
    console.error('FATAL: DATABASE_URL_MIGRATIONS (or DATABASE_URL) is required (DB-creds only).');
    process.exit(1);
  }
  const client = new Client({ connectionString });
  await client.connect();
  try {
    if (args[0] === '--list') {
      const r = await client.query(
        `SELECT user_id, granted_at, revoked_at FROM platform_admins ORDER BY granted_at`,
      );
      console.log(`platform_admins (${r.rowCount}):`);
      for (const row of r.rows) {
        console.log(`  ${row.user_id}  granted=${row.granted_at.toISOString()}  ${row.revoked_at ? 'REVOKED ' + row.revoked_at.toISOString() : 'active'}`);
      }
      return;
    }

    const userId = args[0];
    const revoke = args.includes('--revoke');
    if (!userId || !UUID_RE.test(userId)) {
      console.error('Usage: platform-admin-grant.ts <userId-uuid> [--revoke] | --list');
      process.exit(1);
    }

    if (revoke) {
      const r = await client.query(
        `UPDATE platform_admins SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL`,
        [userId],
      );
      console.log(r.rowCount ? `REVOKED platform-admin ${userId} (denied at next request).` : `No active platform-admin ${userId}.`);
      return;
    }

    // Grant: verify the user row exists FIRST (clean error, never an FK crash), then idempotent upsert.
    const u = await client.query(`SELECT 1 FROM users WHERE id = $1`, [userId]);
    if (u.rowCount === 0) {
      console.error(`FATAL: no users row for ${userId} — cannot grant platform-admin to a non-existent user.`);
      process.exit(1);
    }
    await client.query(
      `INSERT INTO platform_admins (user_id, granted_at, revoked_at)
       VALUES ($1, now(), NULL)
       ON CONFLICT (user_id) DO UPDATE SET revoked_at = NULL`,
      [userId],
    );
    console.log(`GRANTED platform-admin ${userId} (active immediately). Provision >=2 for bus-factor.`);
  } finally {
    await client.end();
  }
}

main().catch((err) => { console.error(err); process.exit(1); });
