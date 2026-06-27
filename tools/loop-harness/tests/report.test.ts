import { test } from 'node:test';
import assert from 'node:assert/strict';
import { renderReport, computeHistory } from '../src/report.js';
import type { RunRecord, MetricsLine } from '../src/types.js';

const base: RunRecord = {
  loop: 'convergence', run_index: 42, outcome: 'green', breaker_reason: null,
  iter_from: 1, iter_to: 7, t_start: '2026-06-27T10:02:11Z', t_end: '2026-06-27T10:41:55Z', wall_s: 2384,
  goal: 'Bring checkout + i18n flows to green for all 3 roles (al/en).',
  what_done: 'CheckoutStepper validation fixed; i18n al/en verified. 4 files, +86/−31.',
  issues: ['429 retry-after path NOT verified', 'risk: menu_version drift'],
  patterns: ['RECURRING (3rd run): i18n contract stall → GRADUATING'],
  telemetry: {
    iterations: 7, tests_fail_start: 11, tests_fail_end: 0, edits: 9, loc_add: 86, loc_del: 31,
    slop_min: 92, fake_green_caught: 1, commits: 1, conflicts: 0, prs: 1, rss_peak_mb: 5600,
    agents: { generator: 6, reviewer: 7 }, skills_used: { 'playwright-mcp': 22 }, skills_ghost: ['unused-skill-x'],
    tokens_in: 312000, tokens_out: 61000, cache_read: 198000, cache_write: 44000, cost_usd: 2.74, per_resolved: 19100,
    eco: { kwh: 0.081, gco2: 23.6, water_ml: 96, method: 'ecologits', estimate: true },
  },
  carry_forward: { guards: ['[active] checkout-cash assertion'], watch: ['429 retry-after'] },
};

test('renderReport — includes all 7 sections + key telemetry, driven by the record', () => {
  const out = renderReport(base);
  assert.match(out, /LOOP REPORT · convergence · run #42 · iter 1–7 · GREEN ✓/);
  assert.match(out, /1\. INITIAL GOAL/);
  assert.match(out, /checkout \+ i18n/);
  assert.match(out, /2\. WHAT WAS DONE/);
  assert.match(out, /3\. ISSUES/);
  assert.match(out, /429 retry-after path NOT verified/);
  assert.match(out, /4\. PATTERNS/);
  assert.match(out, /GRADUATING/);
  assert.match(out, /5\. TELEMETRY/);
  assert.match(out, /tests 11→0/);
  assert.match(out, /cost \$2\.74/);
  assert.match(out, /23\.6 gCO₂ · 96 ml water/);
  assert.match(out, /6\. VS HISTORY/);
  assert.match(out, /7\. CARRY FORWARD → run #43/);
});

test('renderReport — stall outcome shows the breaker reason', () => {
  const out = renderReport({ ...base, outcome: 'stall', breaker_reason: 'stall' });
  assert.match(out, /STALL ✗/);
  assert.match(out, /breaker: stall/);
});

test('computeHistory — averages prior runs and surfaces recurring flags', () => {
  const prior: MetricsLine[] = [
    { loop: 'convergence', run_index: 40, ts: 'a', outcome: 'green', iters: 4, wall_s: 1, tokens_in: 0, tokens_out: 0, cost_usd: 3.0, kwh: 0, gco2: 0, water_ml: 0, fail_start: 10, fail_end: 0, per_resolved: 22000, slop_min: 90, conflicts: 0, recurring_flags: ['i18n-stall'] },
    { loop: 'convergence', run_index: 41, ts: 'b', outcome: 'green', iters: 8, wall_s: 1, tokens_in: 0, tokens_out: 0, cost_usd: 3.2, kwh: 0, gco2: 0, water_ml: 0, fail_start: 9, fail_end: 0, per_resolved: 21600, slop_min: 88, conflicts: 0, recurring_flags: ['i18n-stall'] },
  ];
  const h = computeHistory(prior, base);
  assert.equal(h.prior_runs, 2);
  assert.equal(h.iters_to_green.this, 7);
  assert.equal(h.iters_to_green.avg, 6); // (4+8)/2
  assert.equal(h.iters_to_green.best, 4);
  assert.equal(h.cost_usd.avg, 3.1);
  assert.deepEqual(h.recurring, [{ tag: 'i18n-stall', count: 2 }]);
});

test('computeHistory — first run has no prior history', () => {
  const h = computeHistory([], base);
  assert.equal(h.prior_runs, 0);
  const out = renderReport({ ...base, history: h });
  assert.match(out, /first run — no prior history/);
});
