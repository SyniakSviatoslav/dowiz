// §1 — the harness: the only way a loop runs. Drives iterate(), feeds the
// breaker (§3), captures per-iteration telemetry (§2), builds the canonical
// run-record (§7), ALWAYS renders + prints the full report (§5), and persists
// permanently + losslessly. Deferred (steps 4–7): eco (§6), recall/distill/
// graduate (§8), fresh-context reviewer (§4) — wired in later, same seam.

import type {
  Loop, IterationTelemetry, RunRecord, AggregateTelemetry, MetricsLine, BreakerConfig, RunOutcome,
} from './types.js';
import { DEFAULT_BREAKER, initBreaker, stepBreaker } from './breaker.js';
import { appendIter, writeRunRecord, appendMetricsLine, nextRunIndex, readMetrics } from './storage.js';
import { renderReport, computeHistory } from './report.js';

export interface HarnessOptions<C> {
  /** Root runs/ directory (permanent store). */
  baseDir: string;
  ctx: C;
  breaker?: Partial<BreakerConfig>;
  /** Injectable clock (ms) + ISO timestamp — deterministic in tests. */
  clockMs?: () => number;
  nowIso?: () => string;
  /** Defaults to console.log. The report is ALWAYS printed. */
  print?: (s: string) => void;
  /** Hard safety cap independent of the breaker (prevents infinite loops in tests). */
  hardIterCap?: number;
  /** Optional loop-supplied narrative for the record (else derived from reflections). */
  summarize?: (iters: IterationTelemetry[], finalState: unknown) => Partial<Pick<RunRecord, 'what_done' | 'issues' | 'patterns' | 'carry_forward'>>;
}

function aggregate(iters: IterationTelemetry[], initialMetric: number): AggregateTelemetry {
  const agg: AggregateTelemetry = {
    iterations: iters.length,
    tests_fail_start: 0, tests_fail_end: 0, edits: 0, loc_add: 0, loc_del: 0,
    slop_min: null, fake_green_caught: 0, commits: 0, conflicts: 0, prs: 0, rss_peak_mb: 0,
    agents: {}, skills_used: {}, skills_ghost: [], tokens_in: 0, tokens_out: 0, cache_read: 0, cache_write: 0,
    cost_usd: 0, per_resolved: null, eco: { kwh: 0, gco2: 0, water_ml: 0, method: 'deferred', estimate: true },
  };
  agg.tests_fail_start = initialMetric;
  if (!iters.length) { agg.tests_fail_end = initialMetric; return agg; }

  agg.tests_fail_end = iters[iters.length - 1]!.code?.tests_fail_after ?? iters[iters.length - 1]!.progress_metric;

  const ghost = new Set<string>();
  for (const it of iters) {
    agg.edits += it.code?.edits ?? 0;
    agg.loc_add += it.code?.loc_add ?? 0;
    agg.loc_del += it.code?.loc_del ?? 0;
    agg.fake_green_caught += it.code?.fake_green_caught ?? 0;
    if (it.code?.slop_score != null) agg.slop_min = agg.slop_min == null ? it.code.slop_score : Math.min(agg.slop_min, it.code.slop_score);
    agg.commits += it.git_mem?.commits ?? 0;
    agg.conflicts += it.git_mem?.conflicts ?? 0;
    agg.prs += it.git_mem?.prs ?? 0;
    agg.rss_peak_mb = Math.max(agg.rss_peak_mb, it.git_mem?.rss_peak_mb ?? 0);
    for (const [k, v] of Object.entries(it.agents ?? {})) agg.agents[k] = (agg.agents[k] ?? 0) + v;
    for (const [k, v] of Object.entries(it.skills?.used ?? {})) agg.skills_used[k] = (agg.skills_used[k] ?? 0) + v;
    for (const g of it.skills?.ghost ?? []) ghost.add(g);
    agg.tokens_in += it.tokens?.in ?? 0;
    agg.tokens_out += it.tokens?.out ?? 0;
    agg.cache_read += it.tokens?.cache_read ?? 0;
    agg.cost_usd += it.tokens?.cost_usd ?? 0;
    agg.eco.kwh = (agg.eco.kwh ?? 0) + (it.eco?.kwh ?? 0);
    agg.eco.gco2 = (agg.eco.gco2 ?? 0) + (it.eco?.gco2 ?? 0);
    agg.eco.water_ml = (agg.eco.water_ml ?? 0) + (it.eco?.water_ml ?? 0);
  }
  agg.skills_ghost = [...ghost];
  agg.cost_usd = Math.round(agg.cost_usd * 100) / 100;
  const resolved = agg.tests_fail_start - agg.tests_fail_end;
  agg.per_resolved = resolved > 0 ? Math.round((agg.tokens_in + agg.tokens_out) / resolved) : null;
  return agg;
}

export async function runLoop<S, C>(loop: Loop<S, C>, initialState: S, opts: HarnessOptions<C>): Promise<RunRecord> {
  const clockMs = opts.clockMs ?? (() => Date.now());
  const nowIso = opts.nowIso ?? (() => new Date(clockMs()).toISOString());
  const print = opts.print ?? ((s: string) => console.log(s));
  const cfg: BreakerConfig = { ...DEFAULT_BREAKER, ...opts.breaker };
  const hardCap = opts.hardIterCap ?? Math.max(cfg.maxIter * 2, 1000);

  const runIndex = nextRunIndex(opts.baseDir, loop.id);
  const goal = loop.goal(opts.ctx);
  const t0 = clockMs();
  const tStartIso = nowIso();

  let state = initialState;
  const initialMetric = loop.progressMetric(state);
  let prevMetric = initialMetric;
  let breaker = initBreaker();
  let cumulativeCost = 0;
  let iteration = 0;
  const iters: IterationTelemetry[] = [];

  while (!loop.isTerminal(state) && !breaker.tripped && iteration < hardCap) {
    iteration += 1;
    const itStartMs = clockMs();
    const itStartIso = nowIso();
    const outcome = await loop.iterate(opts.ctx, state);
    state = outcome.state;

    const metric = loop.progressMetric(state);
    const delta = metric - prevMetric;
    prevMetric = metric;
    cumulativeCost += outcome.telemetry?.tokens?.cost_usd ?? 0;
    breaker = stepBreaker(breaker, { delta, iteration, cumulativeCostUsd: cumulativeCost, elapsedMs: clockMs() - t0 }, cfg);

    const itEndMs = clockMs();
    const reflection = outcome.reflection ?? (loop.reflect ? loop.reflect(opts.ctx, state) : undefined);
    const iter: IterationTelemetry = {
      loop: loop.id, run_index: runIndex, iteration,
      t_start: itStartIso, t_end: nowIso(), dur_s: Math.round((itEndMs - itStartMs) / 1000),
      ...outcome.telemetry,
      reflection,
      breaker: { state: breaker.tripped ? 'tripped' : 'running', stall_count: breaker.stallCount, reason: breaker.reason },
      progress_metric: metric,
      progress_delta: delta,
    };
    appendIter(opts.baseDir, loop.id, runIndex, iter);
    iters.push(iter);
  }

  let outcome: RunOutcome;
  if (breaker.tripped) outcome = breaker.reason === 'stall' ? 'stall' : 'abort';
  else if (loop.isTerminal(state)) outcome = 'green';
  else outcome = 'natural_stop';

  const telemetry = aggregate(iters, initialMetric);
  const lastRefl = iters[iters.length - 1]?.reflection;
  const derived = opts.summarize?.(iters, state) ?? {};
  const record: RunRecord = {
    loop: loop.id, run_index: runIndex, outcome, breaker_reason: breaker.reason,
    iter_from: iters.length ? 1 : 0, iter_to: iteration,
    t_start: tStartIso, t_end: nowIso(), wall_s: Math.round((clockMs() - t0) / 1000),
    goal,
    what_done: derived.what_done ?? (lastRefl?.changed.join('; ') || '(no changes recorded)'),
    issues: derived.issues ?? [...(lastRefl?.not_verified.map((x) => `NOT verified: ${x}`) ?? []), ...(lastRefl?.risks.map((x) => `risk: ${x}`) ?? [])],
    patterns: derived.patterns ?? [],
    telemetry,
    carry_forward: derived.carry_forward ?? { guards: [], watch: lastRefl?.not_verified ?? [] },
  };
  record.history = computeHistory(readMetrics(opts.baseDir, loop.id), record);

  // §5 — ALWAYS print the full report, every time, no flag.
  print(renderReport(record));

  // §7 — persist permanently + losslessly.
  writeRunRecord(opts.baseDir, loop.id, runIndex, record);
  const metricsLine: MetricsLine = {
    loop: loop.id, run_index: runIndex, ts: record.t_end, outcome,
    iters: iteration, wall_s: record.wall_s,
    tokens_in: telemetry.tokens_in, tokens_out: telemetry.tokens_out, cost_usd: telemetry.cost_usd,
    kwh: telemetry.eco.kwh ?? 0, gco2: telemetry.eco.gco2 ?? 0, water_ml: telemetry.eco.water_ml ?? 0,
    fail_start: telemetry.tests_fail_start, fail_end: telemetry.tests_fail_end,
    per_resolved: telemetry.per_resolved, slop_min: telemetry.slop_min, conflicts: telemetry.conflicts,
    recurring_flags: record.patterns.filter((p) => /recurring/i.test(p)),
    edits: telemetry.edits,
  };
  appendMetricsLine(opts.baseDir, metricsLine);

  return record;
}
