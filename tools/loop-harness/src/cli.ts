#!/usr/bin/env -S npx tsx
// Wiring seam: `finalize` is the standard end-of-loop step. An agent-run loop
// (audit-gate, convergence, triage, ...) hands the harness a PARTIAL record (the
// qualitative parts it knows: goal/what_done/issues/patterns + code/test deltas);
// finalize MEASURES the rest (git + session tokens + eco), renders + prints the
// full §5 report (always), and persists permanently + losslessly. This is how
// "both existing loops adopt the harness" (§10 step 1) without a TS program
// driving the LLM — the loop is the agent, the harness is the instrument.
//
//   npx tsx tools/loop-harness/src/cli.ts finalize \
//     --record run.json --base loops/runs --session <session.jsonl> [--since <gitRef>] [--repo .]

import fs from 'node:fs';
import type { RunRecord, AggregateTelemetry, IterationTelemetry, RunOutcome, BreakerReason } from './types.js';
import { collectGitMem, collectSessionTelemetry, collectWorkflowTelemetry, mergeTelemetry } from './collect.js';
import { computeEco } from './eco.js';
import { renderReport, computeHistory } from './report.js';
import { buildPropagation, renderPropagation } from './propagate.js';
import { nextRunIndex, writeRunRecord, appendIter, appendMetricsLine, readMetrics } from './storage.js';
import path from 'node:path';

interface FinalizeInput {
  loop: string;
  goal: string;
  what_done: string;
  outcome: RunOutcome;
  breaker_reason?: BreakerReason | null;
  t_start: string;
  t_end: string;
  iter_from?: number;
  iter_to?: number;
  issues?: string[];
  patterns?: string[];
  carry_forward?: { guards: string[]; watch: string[] };
  code?: Partial<Pick<AggregateTelemetry, 'tests_fail_start' | 'tests_fail_end' | 'edits' | 'loc_add' | 'loc_del' | 'slop_min' | 'fake_green_caught'>>;
  skills_used?: Record<string, number>;
  iters?: IterationTelemetry[];
}

function arg(flag: string, fallback?: string): string | undefined {
  const i = process.argv.indexOf(flag);
  return i >= 0 ? process.argv[i + 1] : fallback;
}

export function buildRecord(input: FinalizeInput, baseDir: string, opts: { session?: string; repo: string; since?: string; workflow?: string }): RunRecord {
  const run_index = nextRunIndex(baseDir, input.loop);
  const git = collectGitMem(opts.repo, opts.since);
  // Merge the main-session telemetry with any background-Workflow subagent transcripts
  // (a workflow loop's agents live in separate transcripts the session JSONL can't see).
  const sessRaw = opts.session ? collectSessionTelemetry(opts.session, input.t_start, input.t_end) : null;
  const wf = opts.workflow ? collectWorkflowTelemetry(opts.workflow) : null;
  const sess = (sessRaw || wf) ? mergeTelemetry(sessRaw, wf) : null;
  const eco = computeEco(sess?.tokensByModel ?? {});

  const c = input.code ?? {};
  const failStart = c.tests_fail_start ?? 0;
  const failEnd = c.tests_fail_end ?? 0;
  const resolved = failStart - failEnd;
  const tokensTotal = (sess?.tokens.in ?? 0) + (sess?.tokens.out ?? 0);

  const telemetry: AggregateTelemetry = {
    iterations: input.iters?.length ?? (input.iter_to ?? 1) - (input.iter_from ?? 1) + 1,
    tests_fail_start: failStart, tests_fail_end: failEnd,
    edits: c.edits ?? 0, loc_add: c.loc_add ?? 0, loc_del: c.loc_del ?? 0,
    slop_min: c.slop_min ?? null, fake_green_caught: c.fake_green_caught ?? 0,
    commits: git.commits, conflicts: 0, prs: 0, rss_peak_mb: git.rss_peak_mb,
    agents: sess?.agents ?? {},
    skills_used: { ...(sess?.skills_used ?? {}), ...(input.skills_used ?? {}) },
    skills_ghost: [],
    tokens_in: sess?.tokens.in ?? 0, tokens_out: sess?.tokens.out ?? 0,
    cache_read: sess?.tokens.cache_read ?? 0, cache_write: sess?.tokens.cache_write ?? 0,
    cost_usd: sess?.tokens.cost_usd ?? 0,
    per_resolved: resolved > 0 ? Math.round(tokensTotal / resolved) : null,
    eco,
  };

  const record: RunRecord = {
    loop: input.loop, run_index, outcome: input.outcome, breaker_reason: input.breaker_reason ?? null,
    iter_from: input.iter_from ?? 1, iter_to: input.iter_to ?? telemetry.iterations,
    t_start: input.t_start, t_end: input.t_end,
    wall_s: Math.max(0, Math.round((Date.parse(input.t_end) - Date.parse(input.t_start)) / 1000)),
    goal: input.goal, what_done: input.what_done,
    issues: input.issues ?? [], patterns: input.patterns ?? [],
    telemetry,
    carry_forward: input.carry_forward ?? { guards: [], watch: [] },
  };
  record.history = computeHistory(readMetrics(baseDir, input.loop), record);
  return record;
}

function main(): void {
  if (process.argv[2] !== 'finalize') {
    console.error('usage: cli.ts finalize --record <json> --base <dir> [--session <jsonl>] [--workflow <transcriptDir>] [--since <ref>] [--repo <dir>]');
    process.exit(2);
  }
  const recordPath = arg('--record');
  const baseDir = arg('--base', 'loops/runs')!;
  if (!recordPath) { console.error('--record required'); process.exit(2); }
  const input: FinalizeInput = JSON.parse(fs.readFileSync(recordPath, 'utf8'));
  const repo = arg('--repo', '.')!;
  const record = buildRecord(input, baseDir, { session: arg('--session'), repo, since: arg('--since'), workflow: arg('--workflow') });

  // §5 — ALWAYS print the full report.
  console.log(renderReport(record));

  // §8 — ALWAYS emit loop-end propagation (memory + reflection + cross-surface directives).
  const prop = buildPropagation(record);
  console.log('\n' + renderPropagation(prop));
  try {
    const inbox = path.join(repo, 'docs/reflections/INBOX');
    fs.mkdirSync(inbox, { recursive: true });
    const rf = path.join(inbox, `${input.loop}-${record.run_index}.md`);
    fs.writeFileSync(rf, prop.reflection + '\n');
    console.error(`[propagation] reflection → ${rf}`);
  } catch (err) { console.error('[propagation] reflection write skipped:', (err as Error).message); }

  // §7 — persist permanently + losslessly.
  for (const it of input.iters ?? []) appendIter(baseDir, input.loop, record.run_index, it);
  writeRunRecord(baseDir, input.loop, record.run_index, record);
  appendMetricsLine(baseDir, {
    loop: input.loop, run_index: record.run_index, ts: record.t_end, outcome: record.outcome,
    iters: record.telemetry.iterations, wall_s: record.wall_s,
    tokens_in: record.telemetry.tokens_in, tokens_out: record.telemetry.tokens_out, cost_usd: record.telemetry.cost_usd,
    kwh: record.telemetry.eco.kwh ?? 0, gco2: record.telemetry.eco.gco2 ?? 0, water_ml: record.telemetry.eco.water_ml ?? 0,
    fail_start: record.telemetry.tests_fail_start, fail_end: record.telemetry.tests_fail_end,
    per_resolved: record.telemetry.per_resolved, slop_min: record.telemetry.slop_min, conflicts: 0,
    recurring_flags: record.patterns.filter((p) => /recurring/i.test(p)),
  });
  console.error(`\n[persisted] ${baseDir}/${input.loop}/${record.run_index}.json.gz (+ metrics.jsonl)`);
}

// Run only when invoked directly (so tests can import buildRecord).
if (import.meta.url === `file://${process.argv[1]}`) main();
