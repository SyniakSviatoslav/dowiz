// metric-reflection.test.mjs — red-green DoD test for scripts/metric-reflection.mjs
// (SYSTEMS-MAP.md backlog item 4: metric-reflection loop).
//
// Run: node --test scripts/metric-reflection.test.mjs
//
// Pure-function tests need no fs/git; the one round-trip test uses a scratch dir
// (METRIC_REFLECTION_ROOT override). The real loops/runs/ and docs/governance/ are never touched.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { analyze } from './telemetry-analyze.mjs';
import {
  foldGitHistory, findCrossPatterns, buildSnapshot, compareHistory, buildReport, formatMarkdown,
  loadHistory, appendHistory,
} from './metric-reflection.mjs';

function tmp(t) {
  const dir = mkdtempSync(join(tmpdir(), 'metricrefl-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

function fixtureCommits() {
  return [
    { hash: 'a1', ts: '2026-07-01T00:00:00Z', subject: 'x', files: ['scripts/exec-telemetry.mjs', 'docs/governance/foo.md'] },
    { hash: 'a2', ts: '2026-07-02T00:00:00Z', subject: 'y', files: ['scripts/exec-telemetry.mjs'] },
    { hash: 'a3', ts: '2026-07-03T00:00:00Z', subject: 'z', files: ['scripts/exec-telemetry.mjs', 'apps/api/src/routes/orders.ts'] },
  ];
}

function fixtureRecurringFailures() {
  return [
    { layer: 'exec-telemetry', name: 'emit', count: 3, lastTs: '2026-07-03T00:00:00Z' },
    { layer: 'skill-evolution', name: 'emit', count: 3, lastTs: '2026-07-03T00:00:00Z' },
    { layer: 'ssg', name: 'lane-merge', count: 4, lastTs: '2026-07-03T00:00:00Z' },
  ];
}

test('foldGitHistory: aggregates per-file commit counts, sorted desc then alphabetically', () => {
  const fold = foldGitHistory(fixtureCommits());
  assert.equal(fold.totalCommits, 3);
  assert.equal(fold.byFile[0].file, 'scripts/exec-telemetry.mjs');
  assert.equal(fold.byFile[0].count, 3);
  assert.ok(fold.byFile.find((f) => f.file === 'apps/api/src/routes/orders.ts'));
});

test('foldGitHistory: empty commit list yields an empty, non-throwing fold', () => {
  const fold = foldGitHistory([]);
  assert.equal(fold.totalCommits, 0);
  assert.deepEqual(fold.byFile, []);
});

test('findCrossPatterns: detects a cross-layer pattern when the same name recurs across >=2 layers', () => {
  const patterns = findCrossPatterns(fixtureRecurringFailures(), foldGitHistory([]));
  const crossLayer = patterns.filter((p) => p.type === 'cross-layer');
  assert.equal(crossLayer.length, 1);
  assert.equal(crossLayer[0].name, 'emit');
  assert.deepEqual(crossLayer[0].layers, ['exec-telemetry', 'skill-evolution']);
});

test('findCrossPatterns: RED-proof — no cross-layer pattern when every name is single-layer', () => {
  const singleLayer = [{ layer: 'ssg', name: 'lane-merge', count: 4, lastTs: 't' }];
  const patterns = findCrossPatterns(singleLayer, foldGitHistory([]));
  assert.deepEqual(patterns.filter((p) => p.type === 'cross-layer'), []);
});

test('findCrossPatterns: detects a churn-correlated pattern when layer/name substring-matches a heavily churned file', () => {
  const fold = foldGitHistory(fixtureCommits()); // scripts/exec-telemetry.mjs churned 3x
  const patterns = findCrossPatterns(fixtureRecurringFailures(), fold, { churnThreshold: 3 });
  const churn = patterns.filter((p) => p.type === 'churn-correlated');
  assert.equal(churn.length, 1);
  assert.equal(churn[0].layer, 'exec-telemetry');
  assert.equal(churn[0].file, 'scripts/exec-telemetry.mjs');
});

test('findCrossPatterns: RED-proof — raising churnThreshold above the real count suppresses the churn-correlated match', () => {
  const fold = foldGitHistory(fixtureCommits());
  const patterns = findCrossPatterns(fixtureRecurringFailures(), fold, { churnThreshold: 4 });
  assert.deepEqual(patterns.filter((p) => p.type === 'churn-correlated'), []);
});

test('buildSnapshot + compareHistory: a first run has no previous snapshot to compare against', () => {
  const analyzed = analyze([], {});
  const snap = buildSnapshot(analyzed, 't1');
  const cmp = compareHistory(snap, null);
  assert.equal(cmp.isFirstRun, true);
  assert.deepEqual(cmp.newRecurringFailures, []);
  assert.equal(cmp.totalEventsDelta, null);
});

test('compareHistory: surfaces a NEW recurring failure absent from the previous snapshot', () => {
  const prev = { schema_version: 1, ts: 't0', total_events: 5, by_layer_fail_rate: [{ layer: 'ssg', failRate: 0.5 }], recurring_failure_keys: ['ssg|lane-merge'] };
  const cur = { schema_version: 1, ts: 't1', total_events: 8, by_layer_fail_rate: [{ layer: 'ssg', failRate: 0.5 }], recurring_failure_keys: ['ssg|lane-merge', 'exec-telemetry|emit'] };
  const cmp = compareHistory(cur, prev);
  assert.deepEqual(cmp.newRecurringFailures, ['exec-telemetry|emit']);
  assert.deepEqual(cmp.resolvedRecurringFailures, []);
  assert.equal(cmp.totalEventsDelta, 3);
});

test('compareHistory: surfaces a RESOLVED recurring failure present only in the previous snapshot', () => {
  const prev = { ts: 't0', total_events: 5, by_layer_fail_rate: [], recurring_failure_keys: ['ssg|lane-merge'] };
  const cur = { ts: 't1', total_events: 5, by_layer_fail_rate: [], recurring_failure_keys: [] };
  const cmp = compareHistory(cur, prev);
  assert.deepEqual(cmp.newRecurringFailures, []);
  assert.deepEqual(cmp.resolvedRecurringFailures, ['ssg|lane-merge']);
});

test('compareHistory: computes a nonzero fail-rate delta only for layers present in both snapshots', () => {
  const prev = { ts: 't0', total_events: 1, by_layer_fail_rate: [{ layer: 'ssg', failRate: 0.25 }], recurring_failure_keys: [] };
  const cur = { ts: 't1', total_events: 1, by_layer_fail_rate: [{ layer: 'ssg', failRate: 0.75 }, { layer: 'new-layer', failRate: 1 }], recurring_failure_keys: [] };
  const cmp = compareHistory(cur, prev);
  assert.equal(cmp.failRateDeltas.length, 1);
  assert.equal(cmp.failRateDeltas[0].layer, 'ssg');
  assert.equal(Math.round(cmp.failRateDeltas[0].delta * 100), 50);
});

test('buildReport: combines patterns + cross-patterns + git-fold + history-comparison into one report', () => {
  const events = [
    { layer: 'exec-telemetry', action_kind: 'gate-fail', name: 'emit', outcome: 'fail', duration_ms: 10, ts: '2026-07-01T00:00:00Z' },
    { layer: 'exec-telemetry', action_kind: 'gate-fail', name: 'emit', outcome: 'fail', duration_ms: 10, ts: '2026-07-02T00:00:00Z' },
    { layer: 'exec-telemetry', action_kind: 'gate-fail', name: 'emit', outcome: 'fail', duration_ms: 10, ts: '2026-07-03T00:00:00Z' },
  ];
  const report = buildReport({ events, commits: fixtureCommits(), previousSnapshot: null, ts: 't1', since: '30d', repeatThreshold: 3, churnThreshold: 3 });
  assert.equal(report.advisory, true);
  assert.equal(report.patterns.recurring_failures.length, 1);
  assert.ok(report.cross_patterns.some((p) => p.type === 'churn-correlated'));
  assert.equal(report.git_fold.totalCommits, 3);
  assert.equal(report.history_comparison.isFirstRun, true);
  assert.equal(report.snapshot.recurring_failure_keys.length, 1);
});

test('formatMarkdown: renders every section and stays advisory-labelled (never gate language)', () => {
  const report = buildReport({ events: [], commits: [], previousSnapshot: null, ts: 't1', since: '30d' });
  const md = formatMarkdown(report);
  assert.match(md, /# Metric-Reflection Report \(advisory\)/);
  assert.match(md, /## Patterns/);
  assert.match(md, /## Cross-Patterns/);
  assert.match(md, /## Git Churn/);
  assert.match(md, /## Historical Comparison/);
  assert.match(md, /human\/council\/librarian decision/);
  assert.doesNotMatch(md, /\bgate\b/i);
});

test('appendHistory + loadHistory: round-trips a snapshot through the real filesystem in a scratch root', (t) => {
  const root = tmp(t);
  assert.deepEqual(loadHistory(root), []);
  const analyzed = analyze([], {});
  const snap = buildSnapshot(analyzed, 't1');
  appendHistory(snap, root);
  const history = loadHistory(root);
  assert.equal(history.length, 1);
  assert.equal(history[0].ts, 't1');
  const raw = readFileSync(join(root, 'loops', 'runs', 'metric-reflection-history.jsonl'), 'utf8');
  assert.equal(raw.trim().split('\n').length, 1);
});
