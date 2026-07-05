// Living Loop System (docs/operating-model/living-loop-system-v3.md) — §1 contract
// + §2 telemetry shapes. Foundation only: the deterministic core. Eco (§6),
// recall/distill/graduate (§8), and the fresh-context reviewer (§4) are deferred.

/** §1 — every loop implements these 5 hooks; the harness provides the rest. */
export interface Loop<S = unknown, C = unknown> {
  id: string;
  goal(ctx: C): string;
  /** One pass. MUST return an IterationRecord carrying the progress metric. */
  iterate(ctx: C, state: S): Promise<IterationOutcome<S>> | IterationOutcome<S>;
  /** Breaker watches this. Lower = better (failing tests, unresolved issues). */
  progressMetric(state: S): number;
  reflect?(ctx: C, state: S): Reflection;
  isTerminal(state: S): boolean;
}

export interface IterationOutcome<S = unknown> {
  state: S;
  /** Per-iteration telemetry; fields the loop can fill. Harness stamps the rest. */
  telemetry?: Partial<IterationTelemetry>;
  reflection?: Reflection;
}

export interface Reflection {
  changed: string[];
  verified: string[];
  not_verified: string[];
  risks: string[];
  confidence: number; // 0..1
}

/** §2 — one record per iteration (persisted as one JSONL line). */
export interface IterationTelemetry {
  loop: string;
  run_index: number;
  iteration: number;
  t_start: string;
  t_end: string;
  dur_s: number;
  code?: CodeBlock;
  git_mem?: GitMemBlock;
  agents?: Record<string, number>;
  skills?: { used?: Record<string, number>; ghost?: string[] };
  tokens?: TokensBlock;
  eco?: EcoBlock; // §6 — deferred; shape reserved
  reflection?: Reflection;
  breaker: { state: 'running' | 'tripped'; stall_count: number; reason?: BreakerReason | null };
  /** The progress metric for this iteration (breaker input). Lower = better. */
  progress_metric: number;
  progress_delta: number;
}

export interface CodeBlock {
  files?: number; loc_add?: number; loc_del?: number; edits?: number;
  tests_fail_before?: number; tests_fail_after?: number; delta?: number;
  slop_score?: number; lint?: 'pass' | 'fail'; typecheck?: 'pass' | 'fail';
  fake_green_caught?: number;
}
export interface GitMemBlock {
  commits?: number; branch?: string; conflicts?: number; prs?: number;
  ctx_util_pct?: number; compactions?: number; rss_peak_mb?: number; oom?: boolean;
}
export interface TokensBlock {
  in?: number; out?: number; cache_read?: number; cache_write?: number;
  by_model?: Record<string, number>; per_resolved?: number;
  read_edit_ratio?: number; cost_usd?: number;
}
export interface EcoBlock {
  kwh?: number; gco2?: number; water_ml?: number; method?: string; estimate?: boolean;
}

// §3 — breaker
export type BreakerReason = 'stall' | 'max_iter' | 'budget' | 'time_cap';
export interface BreakerConfig {
  /** Stall threshold: trip after K consecutive non-improving iterations. */
  K: number;
  maxIter: number;
  budgetUsd: number;
  timeCapMs: number;
}
export interface BreakerState {
  stallCount: number;
  tripped: boolean;
  reason: BreakerReason | null;
}

export type RunOutcome = 'green' | 'stall' | 'abort' | 'natural_stop';

/** §7 — the canonical run-record. The report (§5) is rendered FROM this; the
 *  prose is never the unit of storage. */
export interface RunRecord {
  loop: string;
  run_index: number;
  outcome: RunOutcome;
  breaker_reason?: BreakerReason | null;
  iter_from: number;
  iter_to: number;
  t_start: string;
  t_end: string;
  wall_s: number;
  goal: string;
  what_done: string;
  issues: string[];
  patterns: string[];
  telemetry: AggregateTelemetry;
  carry_forward: { guards: string[]; watch: string[] };
  history?: HistoryComparison;
}

export interface AggregateTelemetry {
  iterations: number;
  tests_fail_start: number;
  tests_fail_end: number;
  edits: number;
  loc_add: number;
  loc_del: number;
  slop_min: number | null;
  fake_green_caught: number;
  commits: number;
  conflicts: number;
  prs: number;
  rss_peak_mb: number;
  agents: Record<string, number>;
  skills_used: Record<string, number>;
  skills_ghost: string[];
  tokens_in: number;
  tokens_out: number;
  cache_read: number;
  cache_write: number;
  cost_usd: number;
  per_resolved: number | null;
  eco: EcoBlock;
}

/** §5 §6 — run-over-run comparison (computed from metrics.jsonl). */
export interface HistoryComparison {
  prior_runs: number;
  iters_to_green: { this: number; avg: number; best: number };
  per_resolved: { this: number | null; avg: number | null };
  cost_usd: { this: number; avg: number };
  recurring: { tag: string; count: number }[];
}

/** §7 — one compact line per run in runs/metrics.jsonl (trend + recall index). */
export interface MetricsLine {
  loop: string;
  run_index: number;
  ts: string;
  outcome: RunOutcome;
  iters: number;
  wall_s: number;
  tokens_in: number;
  tokens_out: number;
  cost_usd: number;
  kwh: number;
  gco2: number;
  water_ml: number;
  fail_start: number;
  fail_end: number;
  per_resolved: number | null;
  slop_min: number | null;
  conflicts: number;
  recurring_flags: string[];
  /** Files-changed proxy for the governor's churn/day ceiling (optional). */
  edits?: number;
}
