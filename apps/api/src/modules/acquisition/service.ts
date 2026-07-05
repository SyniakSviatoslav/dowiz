import { assertTransition, REQUIRES_REASON } from './state-machine.js';
import { acquisitionSourceSchema, type AcquisitionSource, type AcquisitionState } from './types.js';

// P6-1 — acquisition service. Pure data layer over a Postgres queryable (Pool or PoolClient),
// so it composes inside a tx and is unit-testable against a throwaway PG.

export interface Queryable {
  query(text: string, values?: unknown[]): Promise<{ rows: unknown[] }>;
}

const COLS = `id, place_id, state, place_raw, website_url, menu_kind, menu_draft,
  confidence, org_id, location_id, failure_reason, claimed_at, created_at, updated_at`;

function parse(row: unknown): AcquisitionSource {
  return acquisitionSourceSchema.parse(row);
}

/**
 * Idempotent dedup anchor: one row per place_id. Concurrent/repeat calls return the
 * SAME row (single-statement ON CONFLICT — race-safe; never a second shadow lifecycle).
 */
export async function createSource(db: Queryable, placeId: string): Promise<AcquisitionSource> {
  const res = await db.query(
    `INSERT INTO acquisition_sources (place_id)
       VALUES ($1)
     ON CONFLICT (place_id) DO UPDATE SET updated_at = now()
     RETURNING ${COLS}`,
    [placeId],
  );
  return parse(res.rows[0]);
}

export async function getById(db: Queryable, id: string): Promise<AcquisitionSource | null> {
  const res = await db.query(`SELECT ${COLS} FROM acquisition_sources WHERE id = $1`, [id]);
  return res.rows[0] ? parse(res.rows[0]) : null;
}

interface AdvancePatch {
  place_raw?: unknown;
  website_url?: string;
  menu_kind?: string;
  menu_draft?: unknown;
  confidence?: number;
  org_id?: string;
  location_id?: string;
  failure_reason?: string;
  claimed_at?: Date;
}

const PATCHABLE = [
  'place_raw',
  'website_url',
  'menu_kind',
  'menu_draft',
  'confidence',
  'org_id',
  'location_id',
  'failure_reason',
  'claimed_at',
] as const;

/**
 * Move a source to `to`, validating the transition + the failure_reason invariant.
 * Throws AcquisitionTransitionError on an illegal edge; throws if a REQUIRES_REASON
 * state is entered without a non-empty failure_reason. Single UPDATE guarded by the
 * current state (optimistic — the WHERE state pins the read-modify-write).
 */
export async function advance(
  db: Queryable,
  id: string,
  to: AcquisitionState,
  patch: AdvancePatch = {},
): Promise<AcquisitionSource> {
  const current = await getById(db, id);
  if (!current) throw new Error(`acquisition source not found: ${id}`);
  assertTransition(current.state, to);

  const reason = patch.failure_reason ?? current.failure_reason ?? '';
  if (REQUIRES_REASON.has(to) && reason.trim().length === 0) {
    throw new Error(`transition to ${to} requires a non-empty failure_reason`);
  }

  const sets: string[] = ['state = $2', 'updated_at = now()'];
  const values: unknown[] = [id, to];
  for (const key of PATCHABLE) {
    if (patch[key] !== undefined) {
      values.push(patch[key]);
      sets.push(`${key} = $${values.length}`);
    }
  }
  values.push(current.state);
  const res = await db.query(
    `UPDATE acquisition_sources SET ${sets.join(', ')}
       WHERE id = $1 AND state = $${values.length}
     RETURNING ${COLS}`,
    values,
  );
  if (!res.rows[0]) throw new Error(`acquisition source ${id} changed under us (state != ${current.state})`);
  return parse(res.rows[0]);
}

/** Divert to MANUAL_REVIEW with a required reason (no silent drop). */
export function flagManualReview(db: Queryable, id: string, reason: string): Promise<AcquisitionSource> {
  return advance(db, id, 'MANUAL_REVIEW', { failure_reason: reason });
}

/** Enter a terminal/exit state with a required reason. */
export function flagTerminal(
  db: Queryable,
  id: string,
  state: AcquisitionState,
  reason: string,
): Promise<AcquisitionSource> {
  return advance(db, id, state, { failure_reason: reason });
}
