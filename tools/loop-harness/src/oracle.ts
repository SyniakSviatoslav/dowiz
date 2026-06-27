// §2 — the machine oracle. Replaces the human gate for Class A. A change is KEPT
// iff ALL hold; otherwise ATOMIC ROLLBACK (revert + record). No human in any step;
// evidence + reversibility decide. A change that passes tests but shows no speedup
// is rolled back (added risk for nothing). The oracle is deliberately strict — its
// job is to REJECT anything it can't PROVE safe, faster, and reversible.
//
// Gate logic is pure-orchestration over pluggable hooks (apply/revert/green/
// security/measure) so it is fully unit-testable and works for repo-code changes
// (isolated git worktree) and config changes (recorded inverse command) alike.

export interface OracleThresholds {
  /** Minimum proven speedup to KEEP (§2.3). Default 5%. */
  minSpeedupPct: number;
}
export const DEFAULT_THRESHOLDS: OracleThresholds = { minSpeedupPct: 5 };

export interface OracleHooks {
  /** A recorded, tested revert exists (§2.4). If false, the oracle won't even apply. */
  reversible: boolean;
  /** The target metric (lower = better): tokens-per-resolved / wall-clock / RSS / cost. Called before + after. */
  measure(): Promise<number> | number;
  /** Mutate — in an isolated worktree (repo) or via a command (config). */
  apply(): Promise<void> | void;
  /** Inverse of apply — atomic rollback (drop worktree) or recorded inverse command. */
  revert(): Promise<void> | void;
  /** Full test suite passes (unit + integration + E2E, all roles) (§2.1). */
  green(): Promise<boolean> | boolean;
  /** Standing RLS/auth/tenant-isolation/secret-scrub assertions still pass (§2.2). */
  security(): Promise<boolean> | boolean;
}

export type OracleDecision = 'kept' | 'rolled_back';
export interface OracleVerdict {
  decision: OracleDecision;
  kept: boolean;
  green: boolean | null;
  security_ok: boolean | null;
  before: number | null;
  after: number | null;
  speedup_pct: number | null;
  reason: string;
}

/**
 * Evaluate a candidate through all four gates. KEEP iff reversible AND green AND
 * no-security-regression AND speedup ≥ threshold; otherwise revert + rolled_back.
 * The first failing gate decides; revert() runs on every non-keep path that applied.
 */
export async function evaluate(hooks: OracleHooks, thr: OracleThresholds = DEFAULT_THRESHOLDS): Promise<OracleVerdict> {
  const base = { kept: false, green: null, security_ok: null, before: null, after: null, speedup_pct: null } as const;

  // §2.4 — never apply something without a recorded, tested revert.
  if (!hooks.reversible) {
    return { ...base, decision: 'rolled_back', reason: 'NOT reversible — no recorded revert; refused to apply' };
  }

  const before = await hooks.measure();
  await hooks.apply();

  // §2.1 — green
  const green = await hooks.green();
  if (!green) {
    await hooks.revert();
    return { ...base, decision: 'rolled_back', green: false, before, reason: 'tests RED after apply → atomic rollback' };
  }
  // §2.2 — no security regression
  const security_ok = await hooks.security();
  if (!security_ok) {
    await hooks.revert();
    return { ...base, decision: 'rolled_back', green: true, security_ok: false, before, reason: 'security/RLS assertion regressed → atomic rollback' };
  }
  // §2.3 — measurably faster
  const after = await hooks.measure();
  const speedup_pct = before > 0 ? ((before - after) / before) * 100 : 0;
  if (speedup_pct < thr.minSpeedupPct) {
    await hooks.revert();
    return {
      ...base, decision: 'rolled_back', green: true, security_ok: true, before, after,
      speedup_pct: Math.round(speedup_pct * 10) / 10,
      reason: `no proven speedup (${speedup_pct.toFixed(1)}% < ${thr.minSpeedupPct}%) → rolled back (added risk for nothing)`,
    };
  }

  // ALL gates pass → KEEP.
  return {
    decision: 'kept', kept: true, green: true, security_ok: true, before, after,
    speedup_pct: Math.round(speedup_pct * 10) / 10,
    reason: `KEPT — green · security-ok · ${speedup_pct.toFixed(1)}% faster · reversible`,
  };
}
