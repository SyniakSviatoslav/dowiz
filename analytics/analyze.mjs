#!/usr/bin/env node
/**
 * analytics/analyze.mjs
 *
 * Derived metrics from run-history.jsonl:
 *   - Success rate (by config_version, category, subagent, model)
 *   - Iterations-to-green (runs until first PASS per task)
 *   - Flake detection (pass→fail oscillation under same config)
 *   - Regression (success-rate drop between config_versions)
 *   - Slowest checks (median duration by check name)
 *   - Worst-first breakdowns (which check fails most)
 *
 * Output: analytics-report.json
 *
 * Usage:
 *   node analytics/analyze.mjs [--days N]
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { resolve } from 'path';
import { fileURLToPath } from 'url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');
const REPO_ROOT = resolve(__dirname, '..');
const historyPath = resolve(REPO_ROOT, 'run-history.jsonl');

const DAYS = process.argv.includes('--days')
  ? parseInt(process.argv[process.argv.indexOf('--days') + 1], 10) || 7
  : 30;

// ── Load history ───────────────────────────────────────────────
if (!existsSync(historyPath)) {
  console.error('[analyze] no run-history.jsonl found');
  process.exit(0);
}

const lines = readFileSync(historyPath, 'utf-8').trim().split('\n').filter(Boolean);
const runs = lines.map(l => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean);

// Tolerant timestamp parser: the kernel's eval rows emit a unix-epoch with an
// explicit offset (`1700000000+00:00`), which JS `Date` cannot parse (NaN).
// Accept a leading-integer epoch, else fall back to standard Date parsing.
function parseTs(ts) {
  if (typeof ts === 'number') return ts * 1000;
  if (typeof ts === 'string') {
    const m = ts.match(/^(\d{10,13})/);
    if (m) {
      const n = Number(m[1]);
      return n < 1e12 ? n * 1000 : n; // 10-digit sec, 13-digit ms
    }
    const d = new Date(ts).getTime();
    if (!Number.isNaN(d)) return d;
  }
  return NaN;
}

const cutoff = Date.now() - DAYS * 86400_000;
const recent = runs.filter(r => parseTs(r.timestamp) > cutoff);

console.log(`[analyze] ${runs.length} total runs, ${recent.length} in last ${DAYS} day(s)`);

if (recent.length === 0) {
  console.log('[analyze] no recent runs to analyze');
  writeFileSync(resolve(__dirname, 'analytics-report.json'), JSON.stringify({ runs: 0 }, null, 2));
  process.exit(0);
}

// ── 1. Success rate ────────────────────────────────────────────
function successRate(list) {
  const pass = list.filter(r => r.core?.passed).length;
  return { total: list.length, passed: pass, rate: list.length > 0 ? Math.round(pass / list.length * 100) / 100 : 0 };
}

const byConfig = {};
const byCategory = {};
const bySubagent = {};
const byModel = {};
const byCheck = {};
const gatingCounts = {};

for (const r of recent) {
  if (!r.core) continue;
  const cv = r.config_version || '0';
  if (!byConfig[cv]) byConfig[cv] = [];
  byConfig[cv].push(r);

  const cat = r.meta?.category || 'unknown';
  if (!byCategory[cat]) byCategory[cat] = [];
  byCategory[cat].push(r);

  const sub = r.meta?.subagent || 'general';
  if (!bySubagent[sub]) bySubagent[sub] = [];
  bySubagent[sub].push(r);

  const m = r.meta?.model || 'unknown';
  if (!byModel[m]) byModel[m] = [];
  byModel[m].push(r);

  for (const f of (r.core.gating_failed || [])) {
    gatingCounts[f] = (gatingCounts[f] || 0) + 1;
  }
  for (const f of (r.core.soft_failed || [])) {
    gatingCounts[f] = (gatingCounts[f] || 0) + 1;
  }

  for (const c of (r.core.checks || [])) {
    if (!byCheck[c.name]) byCheck[c.name] = [];
    byCheck[c.name].push(c);
  }
}

const configSummary = Object.entries(byConfig).map(([cv, rs]) => ({
  config_version: cv, ...successRate(rs),
})).sort((a, b) => a.config_version.localeCompare(b.config_version));

// ── 2. Flake detection ─────────────────────────────────────────
const flake = [];
for (const [cv, rs] of Object.entries(byConfig)) {
  const seq = rs.map(r => r.core?.passed ?? false);
  let transitions = 0;
  for (let i = 1; i < seq.length; i++) {
    if (seq[i] !== seq[i - 1]) transitions++;
  }
  if (transitions > 2) {
    flake.push({
      config_version: cv,
      runs: seq.length,
      transitions,
      last_result: seq[seq.length - 1] ? 'PASS' : 'FAIL',
    });
  }
}

// ── 3. Regression detection ────────────────────────────────────
const regression = [];
const sortedConfigs = Object.entries(byConfig).sort((a, b) => a[0].localeCompare(b[0]));
for (let i = 1; i < sortedConfigs.length; i++) {
  const prev = sortedConfigs[i - 1];
  const curr = sortedConfigs[i];
  const prevRate = successRate(prev[1]).rate;
  const currRate = successRate(curr[1]).rate;
  const drop = prevRate - currRate;
  if (drop > 0.1) {
    regression.push({
      from_config: prev[0], from_rate: prevRate,
      to_config: curr[0], to_rate: currRate,
      drop: Math.round(drop * 100) / 100,
    });
  }
}

// ── 4. Slowest checks ──────────────────────────────────────────
function median(arr) {
  const s = [...arr].sort((a, b) => a - b);
  const mid = Math.floor(s.length / 2);
  return s.length % 2 ? s[mid] : (s[mid - 1] + s[mid]) / 2;
}

const checkStats = Object.entries(byCheck).map(([name, results]) => {
  const passed = results.filter(r => r.passed).length;
  const durations = results.map(r => r.durationMs).filter(Boolean);
  return {
    name,
    pass_rate: Math.round(passed / results.length * 100) / 100,
    median_duration_ms: durations.length > 0 ? Math.round(median(durations)) : null,
    max_duration_ms: durations.length > 0 ? Math.max(...durations) : null,
    runs: results.length,
  };
}).sort((a, b) => (b.median_duration_ms || 0) - (a.median_duration_ms || 0));

// ── 5. Category breakdown ──────────────────────────────────────
const categoryBreakdown = Object.entries(byCategory).map(([cat, rs]) => ({
  category: cat, ...successRate(rs),
})).sort((a, b) => a.rate - b.rate);

const subagentBreakdown = Object.entries(bySubagent).map(([sub, rs]) => ({
  subagent: sub, ...successRate(rs),
})).sort((a, b) => a.rate - b.rate);

// ── Assemble report ────────────────────────────────────────────
const report = {
  generated_at: new Date().toISOString(),
  period_days: DAYS,
  total_runs: recent.length,
  overall: successRate(recent),
  by_config_version: configSummary,
  flake_detected: flake,
  regression_detected: regression,
  check_performance: checkStats,
  gating_failure_counts: Object.entries(gatingCounts)
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count),
  category_breakdown: categoryBreakdown,
  subagent_breakdown: subagentBreakdown,
};

const outPath = resolve(__dirname, 'analytics-report.json');
writeFileSync(outPath, JSON.stringify(report, null, 2), 'utf-8');
console.log(`[analyze] report written to ${outPath}`);

// ── Print summary ──────────────────────────────────────────────
console.log(`\n── Summary ──`);
console.log(`  Overall: ${report.overall.passed}/${report.overall.total} = ${Math.round(report.overall.rate * 100)}%`);
if (report.regression_detected.length > 0) {
  console.log(`  Regression: ${report.regression_detected.map(r => `${r.to_config} (${r.to_rate} vs ${r.from_rate})`).join(', ')}`);
}
if (report.flake_detected.length > 0) {
  console.log(`  Flake: ${report.flake_detected.map(f => `${f.config_version} (${f.transitions} transitions)`).join(', ')}`);
}
console.log(`  Worst gate: ${report.gating_failure_counts[0]?.name || 'none'} (${report.gating_failure_counts[0]?.count || 0} failures)`);
console.log(`  Slowest check: ${report.check_performance[0]?.name || 'none'} (${report.check_performance[0]?.median_duration_ms || 0}ms median)`);
