#!/usr/bin/env node
/**
 * analytics/mine-failures.mjs
 *
 * Cluster failures from run-history.jsonl and produce:
 *   1. mempalace-reflections.jsonl — one entry per recurring failure cluster
 *      (ingest via MemPalace MCP write/diary tool)
 *   2. held-out-candidates.json   — representative failing run_ids per cluster
 *      (for adding to Phase C eval set)
 *   3. mine-report.json           — raw cluster data
 *
 * Usage:
 *   node analytics/mine-failures.mjs [--days N]
 *
 * Graphify hook (optional): for each cluster's affected_files, call the
 * Graphify MCP tool to find impacted communities/modules and annotate
 * the reflection. Look for `graphify query "..."` in the reflection body.
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

if (!existsSync(historyPath)) {
  console.error('[mine-failures] no run-history.jsonl found');
  process.exit(0);
}

const lines = readFileSync(historyPath, 'utf-8').trim().split('\n').filter(Boolean);
const runs = lines.map(l => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean);

const cutoff = Date.now() - DAYS * 86400_000;
const recent = runs.filter(r => new Date(r.timestamp).getTime() > cutoff);
const failed = recent.filter(r => r.core && !r.core.passed);

console.log(`[mine-failures] ${runs.length} total, ${recent.length} recent, ${failed.length} failed`);

if (failed.length === 0) {
  console.log('[mine-failures] no failures to mine');
  writeFileSync(resolve(__dirname, 'mine-report.json'), JSON.stringify({ clusters: [] }, null, 2));
  writeFileSync(resolve(__dirname, 'mempalace-reflections.jsonl'), '', 'utf-8');
  writeFileSync(resolve(__dirname, 'held-out-candidates.json'), JSON.stringify([], null, 2));
  process.exit(0);
}

// ── Cluster by (gating_failed × config_version × category) ────
const clusters = {};
for (const r of failed) {
  const key = (r.core.gating_failed || []).sort().join('+') || 'unknown';
  if (!clusters[key]) clusters[key] = [];
  clusters[key].push(r);
}

// ── Build cluster data ─────────────────────────────────────────
const clusterData = Object.entries(clusters).map(([key, members]) => {
  const gatingSet = key.split('+');
  const categories = [...new Set(members.map(r => r.meta?.category || '').filter(Boolean))];
  const subagents = [...new Set(members.map(r => r.meta?.subagent || '').filter(Boolean))];
  const configs = [...new Set(members.map(r => r.config_version || '0'))];
  const failingChecks = members.flatMap(r => (r.core.checks || []).filter(c => !c.passed));

  // Most common failing check
  const checkCounts = {};
  for (const c of failingChecks) {
    checkCounts[c.name] = (checkCounts[c.name] || 0) + 1;
  }
  const topChecks = Object.entries(checkCounts)
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count);

  // Representative run (first member with all meta fields)
  const rep = members.find(r => r.meta?.category && r.meta?.subagent) || members[0];

  // Held-out candidates: up to 3 runs per cluster, spread across configs
  const seenConfigs = new Set();
  const candidates = [];
  for (const r of members) {
    const cv = r.config_version || '0';
    if (!seenConfigs.has(cv) && candidates.length < 3) {
      seenConfigs.add(cv);
      candidates.push({
        run_id: r.run_id,
        config_version: cv,
        timestamp: r.timestamp,
        gating_failed: r.core.gating_failed || [],
      });
    }
  }

  return {
    id: `cluster-${Buffer.from(key).toString('base64').slice(0, 8)}`,
    gating: gatingSet,
    count: members.length,
    categories,
    subagents,
    config_versions: configs,
    top_failing_checks: topChecks.slice(0, 3),
    representative: {
      run_id: rep.run_id,
      timestamp: rep.timestamp,
      config_version: rep.config_version || '0',
      category: rep.meta?.category || '',
      subagent: rep.meta?.subagent || '',
      model: rep.meta?.model || '',
      task: (rep.meta?.task || '').slice(0, 200),
    },
    held_out_candidates: candidates,
  };
}).sort((a, b) => b.count - a.count);

// ── Write mine-report.json ─────────────────────────────────────
const report = {
  generated_at: new Date().toISOString(),
  period_days: DAYS,
  total_failed: failed.length,
  clusters_count: clusterData.length,
  clusters: clusterData,
};

const reportPath = resolve(__dirname, 'mine-report.json');
writeFileSync(reportPath, JSON.stringify(report, null, 2), 'utf-8');
console.log(`[mine-failures] report written to ${reportPath}`);

// ── Write mempalace-reflections.jsonl ──────────────────────────
const reflections = clusterData.map(c => ({
  type: 'failure_cluster',
  id: c.id,
  generated_at: report.generated_at,
  gating_signature: c.gating,
  occurrence_count: c.count,
  affected_categories: c.categories,
  affected_subagents: c.subagents,
  top_failing_checks: c.top_failing_checks,
  representative_run: c.representative,
  reflection: (
    `Failure cluster ${c.id}: ${c.gating.join(', ')} failed ${c.count} time(s) ` +
    `across config(s) ${c.config_versions.join(', ')}. ` +
    (c.categories.length ? `Categories: ${c.categories.join(', ')}. ` : '') +
    (c.subagents.length ? `Subagents: ${c.subagents.join(', ')}. ` : '') +
    `Top failing checks: ${c.top_failing_checks.map(ch => `${ch.name}(${ch.count})`).join(', ')}. ` +
    `Representative run: ${c.representative.run_id} (${c.representative.task || 'no task'}).`
  ),
  suggested_action: (
    c.gating.includes('playwright-smoke') ? 'Check E2E_BASE_URL and test environment setup.' :
    c.gating.includes('check-money') ? 'Run check-money.mjs locally to find money-contract violations.' :
    c.gating.includes('check-rls') ? 'Run check-rls.mjs locally to find RLS violations.' :
    c.gating.includes('tsc') ? 'Run pnpm typecheck locally and fix all errors.' :
    c.gating.includes('lint') ? 'Run pnpm lint and fix introduced warnings.' :
    c.gating.includes('verify-env') ? 'Check .env.example for missing env vars.' :
    'Investigate gating failures — no specific action mapped.'
  ),
  // Graphify hook: for deeper analysis, query:
  //   graphify query "what modules relate to [gating_signature] failures?"
  graphify_query: `graphify query "${c.gating.join(' ')} failures in ${c.categories.join(',')}"`,
}));

const refPath = resolve(__dirname, 'mempalace-reflections.jsonl');
writeFileSync(refPath, reflections.map(r => JSON.stringify(r)).join('\n') + '\n', 'utf-8');
console.log(`[mine-failures] ${reflections.length} reflection(s) written to ${refPath}`);

// ── Write held-out-candidates.json ─────────────────────────────
const allCandidates = clusterData.flatMap(c => c.held_out_candidates);
const candPath = resolve(__dirname, 'held-out-candidates.json');
writeFileSync(candPath, JSON.stringify(allCandidates, null, 2), 'utf-8');
console.log(`[mine-failures] ${allCandidates.length} candidate(s) written to ${candPath}`);

// ── Summary ────────────────────────────────────────────────────
console.log(`\n── Clusters ──`);
for (const c of clusterData) {
  console.log(`  ${c.id}: ${c.gating.join(', ')} — ${c.count} failure(s), ${c.categories.join(', ')}`);
}
