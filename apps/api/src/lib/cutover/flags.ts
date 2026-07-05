/**
 * Cutover flag store — the runtime half of ADR-0022 §2 (Postgres `cutover_flags` as the
 * single flip authority), REV-C3-compliant propagation, REV-C5-compliant degradation.
 *
 * PROPAGATION (REV-C3): LISTEN/NOTIFY is documented blocked on the transaction pooler
 * (server.ts operational pool → Supavisor :6543), so this store uses the council's
 * sanctioned fallback: a bounded-TTL poll (default 2s) "with the window treated as real".
 * The TTL window is REAL split-brain time — acceptable for read-only surfaces; money
 * surfaces additionally require the quiesce procedure ratified in the S2 amendment
 * (atomic+quiesce), which is a FLIP-TIME runbook step, not a property of this cache.
 *
 * FAIL-SAFE: any error — table missing (migration 089 not yet operator-placed), pool
 * exhausted, network — collapses the cache to EMPTY, and an empty cache answers 'node'
 * for every surface. The harness can therefore deploy dark before the migration exists.
 *
 * DEGRADE DIRECTION INVARIANT (REV-C5): the machine may only ever move traffic TOWARD
 * Node (the incumbent stack), never toward Rust, and never automatically for the
 * money/irreversible surfaces (S5/S7/S9 — human go/no-go only). Enforced twice: here
 * (NO_AUTO_DEGRADE check) and in the DB (`cutover_auto_degrade` DEFINER fn refuses
 * S5/S7/S9 and can only write target='node'; the app role has no direct UPDATE grant).
 * Auto-degrade is a GLOBAL consensus action — one UPDATE all instances converge on
 * within one TTL — never a per-instance local override (that manufactures split-brain).
 */

import type { Pool } from 'pg';
import type { SurfaceId } from './matcher.js';

export type CutoverTarget = 'node' | 'rust';

/** REV-C5: money/irreversible surfaces never auto-degrade — human go/no-go only. */
export const NO_AUTO_DEGRADE: ReadonlySet<string> = new Set(['S5', 'S7', 'S9']);

interface FlagEntry {
  readonly target: CutoverTarget;
  readonly readinessOk: boolean;
}

export interface FlagsStoreOptions {
  ttlMs: number;
  log: {
    warn: (obj: object, msg: string) => void;
    error: (obj: object, msg: string) => void;
    debug: (obj: object, msg: string) => void;
  };
}

const ERROR_LOG_INTERVAL_MS = 60_000; // rate-limit store-failure logs to 1/min
const DEGRADE_DEBOUNCE_MS = 30_000; // one auto-degrade attempt per surface per 30s

export class CutoverFlagsStore {
  private cache = new Map<string, FlagEntry>();
  private timer: NodeJS.Timeout | null = null;
  private lastErrorLogAt = 0;
  private lastDegradeAttemptAt = new Map<string, number>();
  /** Surfaces whose rust-but-not-ready state was already logged (log on transition only). */
  private refusalLogged = new Set<string>();

  constructor(
    private readonly pool: Pool,
    private readonly opts: FlagsStoreOptions,
  ) {}

  start(): void {
    if (this.timer) return;
    // Fire one immediate refresh (don't await — boot must not block on the flag store),
    // then poll on the TTL. unref() so the poll never holds the process open.
    void this.refresh();
    this.timer = setInterval(() => void this.refresh(), this.opts.ttlMs);
    this.timer.unref();
  }

  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  /**
   * The single routing answer. Pure cache read — never touches the DB on the request
   * path. Missing row / empty cache / not-ready all answer 'node' (fail-safe).
   * The readiness_ok gate is ADR-0022 §3's machine-gate enforced at READ time: a flip
   * flag without its recorded-green DoD is refused, loudly, without serving rust.
   */
  targetFor(surface: SurfaceId): CutoverTarget {
    const entry = this.cache.get(surface);
    if (!entry) return 'node';
    if (entry.target === 'rust' && !entry.readinessOk) {
      if (!this.refusalLogged.has(surface)) {
        this.refusalLogged.add(surface);
        this.opts.log.error(
          { surface },
          '[cutover] FLIP REFUSED: target=rust but readiness_ok=false — serving Node. ' +
            'Record the surface DoD green (readiness_ok=true) before flipping.',
        );
      }
      return 'node';
    }
    if (entry.target !== 'rust' || entry.readinessOk) this.refusalLogged.delete(surface);
    return entry.target;
  }

  /** Test/ops introspection — never used for routing decisions. */
  snapshot(): ReadonlyMap<string, FlagEntry> {
    return this.cache;
  }

  async refresh(): Promise<void> {
    try {
      const res = await this.pool.query<{ surface: string; target: string; readiness_ok: boolean }>(
        'SELECT surface, target, readiness_ok FROM cutover_flags',
      );
      const next = new Map<string, FlagEntry>();
      for (const row of res.rows) {
        const target: CutoverTarget = row.target === 'rust' ? 'rust' : 'node';
        next.set(row.surface, { target, readinessOk: row.readiness_ok === true });
      }
      this.cache = next;
    } catch (err) {
      // Fail-safe: an unreadable flag store means ALL-NODE, not "keep last known".
      // Keeping stale state could keep serving rust after an operator emergency
      // rollback UPDATE that we can no longer read — the wrong failure mode.
      this.cache = new Map();
      const now = Date.now();
      if (now - this.lastErrorLogAt > ERROR_LOG_INTERVAL_MS) {
        this.lastErrorLogAt = now;
        this.opts.log.warn(
          { err: err instanceof Error ? err.message : String(err) },
          '[cutover] flag store unreadable — failing safe to all-Node (is migration 089 placed?)',
        );
      }
    }
  }

  /**
   * REV-C5 auto-degrade: ONE global consensus action via the constrained DEFINER fn.
   * Returns true iff a degrade was attempted (not necessarily that a row changed).
   * Money/irreversible surfaces are refused HERE and again in the DB function.
   */
  async autoDegrade(surface: SurfaceId, reason: string): Promise<boolean> {
    if (NO_AUTO_DEGRADE.has(surface)) {
      this.opts.log.error(
        { surface, reason },
        '[cutover] auto-degrade REFUSED for money/irreversible surface — human go/no-go required (REV-C5)',
      );
      return false;
    }
    const now = Date.now();
    const last = this.lastDegradeAttemptAt.get(surface) ?? 0;
    if (now - last < DEGRADE_DEBOUNCE_MS) return false;
    this.lastDegradeAttemptAt.set(surface, now);
    try {
      await this.pool.query('SELECT cutover_auto_degrade($1, $2)', [surface, reason.slice(0, 200)]);
      this.opts.log.error(
        { surface, reason },
        '[cutover] AUTO-DEGRADED surface to Node (global flag UPDATE) — operator attention required',
      );
      await this.refresh();
      return true;
    } catch (err) {
      this.opts.log.error(
        { surface, reason, err: err instanceof Error ? err.message : String(err) },
        '[cutover] auto-degrade FAILED — surface may still be flagged rust; requests fail safe per-decision',
      );
      return true;
    }
  }
}
