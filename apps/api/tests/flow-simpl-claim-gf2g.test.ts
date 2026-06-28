import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { acceptClaim, ClaimError } from '../src/modules/acquisition/claim.js';

// flow-simplification §6 / G-F2g — the WEB claim-accept path REFUSES a token-only (NULL invited_contact_hash)
// invite. Such an invite would let a leaked token bind ownership to ANY authenticated account (council R3-1
// theft vector). The bound (non-NULL hash) invite passes the gate (and is then subject to claim_transfer's
// own recipient/contact checks). Proven vs the real head schema (claim_invites + claim_transfer).
const url = process.env.DV2_TEST_DATABASE_URL;
const maybe = url ? test : test.skip;

let pool: Pool;
before(() => { if (url) pool = new Pool({ connectionString: url }); });
after(async () => { if (pool) await pool.end(); });

async function seedInvite(contactHash: string | null): Promise<string> {
  const token = crypto.randomBytes(24).toString('hex'); // ≥16 chars
  const c = await pool.connect();
  try {
    const src = (await c.query(
      `INSERT INTO acquisition_sources (place_id) VALUES ($1) RETURNING id`,
      ['place-' + crypto.randomBytes(6).toString('hex')],
    )).rows[0].id;
    await c.query(
      `INSERT INTO claim_invites (acquisition_source_id, token_hash, invited_contact_hash, expires_at)
       VALUES ($1, encode(sha256($2::bytea),'hex'), $3, now() + interval '1 hour')`,
      [src, token, contactHash],
    );
    return token;
  } finally { c.release(); }
}

maybe('G-F2g: a token-only (NULL contact-hash) invite is REFUSED on web accept → CONTACT_REQUIRED', async () => {
  const token = await seedInvite(null);
  await assert.rejects(
    () => acceptClaim(pool, token, crypto.randomUUID()),
    (e: unknown) => e instanceof ClaimError && (e as ClaimError).code === 'CONTACT_REQUIRED',
  );
});

maybe('a bound (non-NULL contact-hash) invite PASSES the G-F2g gate (fails later for another reason, not CONTACT_REQUIRED)', async () => {
  const token = await seedInvite(crypto.createHash('sha256').update('owner@example.test').digest('hex'));
  await assert.rejects(
    () => acceptClaim(pool, token, crypto.randomUUID()),
    (e: unknown) => e instanceof ClaimError && (e as ClaimError).code !== 'CONTACT_REQUIRED',
  );
});
