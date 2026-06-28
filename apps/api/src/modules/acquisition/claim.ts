import crypto from 'node:crypto';
import type { Pool, PoolClient } from 'pg';
import { advance, getById, flagTerminal, type Queryable } from './service.js';
import { hashToken, hardDeleteShadow } from './provisioning.js';

// P6 CLAIM PHASE — the ownership-transfer authority. A single-use, short-TTL, opaque-256-bit claim
// token transfers a SHADOW org (owner_id NULL) to an authenticated owner THROUGH RLS (claim_accept,
// migration 071), then the owner reviews/authors the menu and publishes via the existing gated path.
// Council verdict: docs/design/p6-claim-council-verdict.md.

const INVITE_TTL_MS = 72 * 60 * 60 * 1000; // 72h — a human round-trip, not minutes

export class ClaimError extends Error {
  constructor(readonly code: string, message?: string) {
    super(message ?? code);
    this.name = 'ClaimError';
  }
}

/** SECURITY DEFINER-style hash of the invited contact (email/phone) for audit + future email-match. */
export function hashContact(contact: string): string {
  return crypto.createHash('sha256').update(contact.trim().toLowerCase(), 'utf8').digest('hex');
}

/**
 * VERIFIED floor (council C7 / P-2) — the minimal legal PROVISIONED→VERIFIED without P6-6's Playwright
 * verifier: the spine FKs exist AND the public preview renders (read_preview_menu non-null). A shadow
 * that doesn't actually render must not be offered for claim.
 */
export async function markVerified(pool: Pool, acquisitionSourceId: string): Promise<void> {
  const src = await getById(pool, acquisitionSourceId);
  if (!src) throw new ClaimError('SOURCE_NOT_FOUND');
  if (!src.org_id || !src.location_id) throw new ClaimError('NOT_VERIFIABLE', 'spine not provisioned');
  const loc = await pool.query('SELECT slug FROM locations WHERE id = $1', [src.location_id]);
  const slug = (loc.rows[0] as { slug?: string } | undefined)?.slug;
  if (!slug) throw new ClaimError('NOT_VERIFIABLE', 'location missing');
  const preview = await pool.query('SELECT read_preview_menu($1) AS m', [slug]);
  if (!(preview.rows[0] as { m?: unknown }).m) throw new ClaimError('NOT_VERIFIABLE', 'preview does not render');
  await advance(pool, acquisitionSourceId, 'VERIFIED');
}

/**
 * Mint a single-use claim invite for a VERIFIED/CLAIM_OFFERED source. Returns the PLAINTEXT token ONCE
 * (only the hash is stored). The partial-unique index makes a second active invite fail (race guard).
 * Advances VERIFIED→CLAIM_OFFERED. `invitedContact` (if given) is hashed for audit / future email-match.
 */
export async function mintClaimInvite(
  pool: Pool,
  acquisitionSourceId: string,
  invitedContact?: string,
): Promise<{ token: string; expiresAt: Date }> {
  const src = await getById(pool, acquisitionSourceId);
  if (!src) throw new ClaimError('SOURCE_NOT_FOUND');
  if (src.state !== 'VERIFIED' && src.state !== 'CLAIM_OFFERED') {
    throw new ClaimError('NOT_OFFERABLE', `source state ${src.state} is not VERIFIED/CLAIM_OFFERED`);
  }
  const token = crypto.randomBytes(32).toString('hex');
  const expiresAt = new Date(Date.now() + INVITE_TTL_MS);
  try {
    await pool.query(
      `INSERT INTO claim_invites (acquisition_source_id, token_hash, invited_contact_hash, expires_at)
       VALUES ($1, $2, $3, $4)`,
      [acquisitionSourceId, hashToken(token), invitedContact ? hashContact(invitedContact) : null, expiresAt],
    );
  } catch (e) {
    if ((e as { code?: string }).code === '23505') throw new ClaimError('ACTIVE_INVITE_EXISTS');
    throw e;
  }
  if (src.state === 'VERIFIED') await advance(pool, acquisitionSourceId, 'CLAIM_OFFERED');
  return { token, expiresAt };
}

/** Resolve a valid, unconsumed invite to its source id (token is the SOLE selector — council K2). */
async function resolveActiveInvite(db: Queryable, token: string): Promise<string> {
  const res = await db.query(
    `SELECT acquisition_source_id FROM claim_invites
      WHERE token_hash = encode(sha256($1::bytea), 'hex')
        AND used_at IS NULL AND revoked_at IS NULL AND expires_at > now()
      FOR UPDATE`,
    [token],
  );
  const row = res.rows[0] as { acquisition_source_id?: string } | undefined;
  if (!row?.acquisition_source_id) throw new ClaimError('INVALID_OR_EXPIRED_TOKEN');
  return row.acquisition_source_id;
}

/**
 * Accept a claim: transfer ownership of the shadow to `userId` via the token-gated SECURITY DEFINER
 * `claim_transfer` carve-out (one atomic statement). The TOKEN is the sole authority — org/location are
 * derived from the matched invite inside the fn, never the request (no IDOR/enum). The fn leaves
 * status='closed' + published_at NULL (NO auto-publish — B3), erases the raw scraped blob (H-erase),
 * and voids outstanding provisioning grants (H-void). Its 'CLAIMERR:<code>' raises map to ClaimError.
 */
export async function acceptClaim(
  pool: Pool,
  token: string,
  userId: string,
): Promise<{ orgId: string; locationId: string }> {
  try {
    const res = await pool.query('SELECT org_id, location_id FROM claim_transfer($1, $2)', [token, userId]);
    const row = res.rows[0] as { org_id: string; location_id: string } | undefined;
    if (!row) throw new ClaimError('NOT_CLAIMABLE');
    return { orgId: row.org_id, locationId: row.location_id };
  } catch (e) {
    const m = /CLAIMERR:(\w+)/.exec((e as Error).message ?? '');
    if (m) throw new ClaimError(m[1]!);
    throw e;
  }
}

/**
 * Decline + erase (council H-decline / C2): TOKEN-ONLY, no registration. Anyone holding the invite (the
 * restaurant) can erase the unconsented shadow in one action. Hard-deletes the spine + raw blob, marks
 * the invite consumed, and abandons the source.
 */
export async function declineAndErase(pool: Pool, token: string): Promise<void> {
  // Validate + burn the invite first (so the token can't be replayed), then erase.
  const client = await pool.connect();
  let sourceId: string;
  try {
    await client.query('BEGIN');
    sourceId = await resolveActiveInvite(client, token);
    await client.query(
      `UPDATE claim_invites SET used_at = now(), revoked_at = now()
        WHERE token_hash = encode(sha256($1::bytea), 'hex') AND used_at IS NULL`,
      [token],
    );
    await client.query('COMMIT');
  } catch (e) {
    await client.query('ROLLBACK');
    throw e;
  } finally {
    client.release();
  }
  await hardDeleteShadow(pool, sourceId); // clears spine + place_raw + menu_draft + grants
  await flagTerminal(pool, sourceId, 'ABANDONED', 'owner declined the preview (erased)');
}

/** Reaper (council H-abandoned-TTL): expired/unused invites → revoked; SHORT TTL for a public shadow. */
export async function reapExpiredInvites(pool: Pool): Promise<number> {
  const res = await pool.query(
    `UPDATE claim_invites SET revoked_at = now()
      WHERE used_at IS NULL AND revoked_at IS NULL AND expires_at < now()`,
  );
  return res.rowCount ?? 0;
}
