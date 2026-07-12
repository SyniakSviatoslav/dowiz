#!/usr/bin/env tsx
/**
 * erase-access-request.ts — day-one GDPR right-to-erasure / consent-withdrawal for the
 * access_requests PII store (ADR-soft-access-gate, STOP-2).
 *
 * Usage:
 *   cd apps/api && node --env-file=../../.env --import tsx scripts/erase-access-request.ts <email>
 *
 * Deletes the row by normalized email (the same trim+lower the handler stores). Idempotent:
 * a missing email reports 0 rows. Uses the operational pool (which holds the DELETE grant
 * from migration 1790000000041). PII never logged beyond the operator-supplied email.
 */
import { Pool } from 'pg';
import { loadEnv } from '@deliveryos/config';

async function main() {
  const email = process.argv[2]?.trim().toLowerCase();
  if (!email) {
    console.error('Usage: erase-access-request.ts <email>');
    process.exit(2);
  }

  const env = loadEnv();
  const url = env.***REDACTED***;
  // Local/non-TLS DBs reject an ssl handshake; Supabase poolers require one.
  const local = /localhost|127\.0\.0\.1|sslmode=disable/.test(url);
  const pool = new Pool({
    connectionString: url,
    ssl: local ? false : { rejectUnauthorized: false },
    max: 1,
  });

  try {
    const res = await pool.query(
      `DELETE FROM access_requests WHERE lower(email) = lower($1)`,
      [email],
    );
    console.log(`Erased ${res.rowCount} access_request row(s) for ${email}`);

    // Proof: confirm the row is gone.
    const check = await pool.query(
      `SELECT count(*)::int AS n FROM access_requests WHERE lower(email) = lower($1)`,
      [email],
    );
    const remaining = check.rows[0]?.n ?? 0;
    if (remaining !== 0) {
      console.error(`ERROR: ${remaining} row(s) still present after erasure`);
      process.exit(1);
    }
    console.log('Verified: 0 rows remain.');
  } finally {
    await pool.end();
  }
}

main().catch((err) => {
  console.error('erase-access-request failed:', err);
  process.exit(1);
});
