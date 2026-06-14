#!/usr/bin/env node
// analytics/analyze.mjs
// Reads run-history.jsonl and computes the DERIVED metrics that Langfuse's built-in charts don't do
// well: iterations-to-green (per task), flake (pass→fail oscillation under the same config),
// success-rate by dimension, cost per task, and regression when a config_version changes.
// Writes analytics-report.json + prints a summary. (Time-series success-rate/cost charts: build in
// the Langfuse UI over the spans/scores you emit — see README.)
//
// Usage: node analytics/analyze.mjs [run-history.jsonl]

import { readFileSync, writeFileSync } from "node:fs";

const runs = readFileSync(process.argv[2] || "run-history.jsonl", "utf8")
  .trim().split("\n").filter(Boolean).map((l) => JSON.parse(l));

const groupBy = (arr, key) => arr.reduce((m, x) => ((m[key(x)] ??= []).push(x), m), {});
const rate = (arr, pred) => (arr.length ? arr.filter(pred).length / arr.length : 0);
const passed = (r) => r.core?.passed === true;

// ── iterations-to-green: per task, runs (ordered) until first green ──
const byTask = groupBy(runs, (r) => r.task_id);
const iterationsToGreen = Object.entries(byTask).map(([task_id, rs]) => {
  const ordered = [...rs].sort((a, b) => a.ts.localeCompare(b.ts));
  const idx = ordered.findIndex(passed);
  return { task_id, iterations: idx === -1 ? null : idx + 1, green: idx !== -1, attempts: ordered.length };
});
const greenTasks = iterationsToGreen.filter((t) => t.green);
const meanIters = greenTasks.length ? greenTasks.reduce((s, t) => s + t.iterations, 0) / greenTasks.length : null;

// ── flake: a pass followed later by a fail under the SAME config_version (suspicious oscillation) ──
const flakeTasks = Object.entries(byTask).filter(([, rs]) => {
  const ordered = [...rs].sort((a, b) => a.ts.localeCompare(b.ts));
  for (let i = 1; i < ordered.length; i++) {
    if (passed(ordered[i - 1]) && !passed(ordered[i]) && ordered[i - 1].config_version === ordered[i].config_version) return true;
  }
  return false;
}).map(([task_id]) => task_id);

// ── success-rate by dimension ──
const byDim = (dim) => Object.entries(groupBy(runs.filter((r) => r[dim] != null), (r) => r[dim]))
  .map(([k, rs]) => ({ [dim]: k, runs: rs.length, success_rate: +rate(rs, passed).toFixed(3) }))
  .sort((a, b) => a.success_rate - b.success_rate); // worst first

// ── cost per task / model ──
const costPerTask = Object.entries(byTask).map(([task_id, rs]) => ({
  task_id,
  tokens: rs.reduce((s, r) => s + (r.usage?.tokens || 0), 0),
  cost_usd: +rs.reduce((s, r) => s + (r.usage?.cost_usd || 0), 0).toFixed(4),
})).sort((a, b) => b.cost_usd - a.cost_usd);

// ── regression on config_version change (ordered by first-seen) ──
const byVersion = groupBy(runs, (r) => r.config_version);
const versionsOrdered = Object.keys(byVersion).sort((a, b) =>
  Math.min(...byVersion[a].map((r) => Date.parse(r.ts))) - Math.min(...byVersion[b].map((r) => Date.parse(r.ts))));
const versionRates = versionsOrdered.map((v) => ({ config_version: v, runs: byVersion[v].length, success_rate: +rate(byVersion[v], passed).toFixed(3) }));
const regressions = [];
for (let i = 1; i < versionRates.length; i++) {
  const drop = versionRates[i - 1].success_rate - versionRates[i].success_rate;
  if (drop > 0.1) regressions.push({ from: versionRates[i - 1].config_version, to: versionRates[i].config_version, drop: +drop.toFixed(3) });
}

const report = {
  generated_at: new Date().toISOString(),
  total_runs: runs.length,
  overall_success_rate: +rate(runs, passed).toFixed(3),
  iterations_to_green: { mean: meanIters, per_task: iterationsToGreen },
  flake_suspect_tasks: flakeTasks,
  success_rate_by: { category: byDim("category"), subagent: byDim("subagent"), model: byDim("model"), provider: byDim("provider") },
  cost_per_task: costPerTask,
  version_success_rates: versionRates,
  regressions,
};

writeFileSync("analytics-report.json", JSON.stringify(report, null, 2));
console.log(`runs=${report.total_runs}  success=${(report.overall_success_rate * 100).toFixed(1)}%  mean-iters-to-green=${meanIters?.toFixed(2) ?? "n/a"}  flake-suspect=${flakeTasks.length}`);
if (regressions.length) console.log("⚠ regressions:", regressions);
console.log("worst by model:", report.success_rate_by.model.slice(0, 3));
