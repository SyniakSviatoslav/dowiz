#!/usr/bin/env node
// metric-reflection — folds scripts/exec-telemetry.mjs's log + git commit history into cross-run
// insights (SYSTEMS-MAP.md backlog item 4, paired with the item-3 emitter/analyzer). Advisory only:
// it SURFACES candidates for a docs/reflections/INBOX/ write or a regression-ledger guardrail; it
// never writes one itself and is never a gate, exactly like scripts/telemetry-analyze.mjs.
//
// Usage: node scripts/metric-reflection.mjs [--since 30d] [--top 5] [--repeat-threshold 3]
//                                            [--churn-threshold 3] [--json] [--no-write]
//
// Report sections:
//   - patterns: scripts/telemetry-analyze.mjs's by-layer/bottleneck/recurring-failure output, reused
//   - cross-patterns: a recurring-failure (layer, name) that is either (a) cross-layer — the same
//     `name` also recurs as a failure under a DIFFERENT layer, or (b) churn-correlated — its layer or
//     name substring-matches a git-history-heavy-churned file path in the same window. Both are the
//     "several fixes trace back to one cause" systemic shape (docs/design/harness/SYSTEMS-MAP.md's
//     pattern-critic analogue), not a one-off bug a single reflection would already catch.
//   - historical comparison: this run's snapshot vs the LAST stored snapshot in
//     loops/runs/metric-reflection-history.jsonl — new/resolved recurring failures + fail-rate delta.
//
// Node stdlib only. No new deps.
import { execFileSync } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { readEvents, parseSince } from './exec-telemetry.mjs';
import { analyze } from './telemetry-analyze.mjs';

const ROOT = () => process.env.METRIC_REFLECTION_ROOT || process.cwd();
const RUNS_DIR = (root = ROOT()) => join(root, 'loops', 'runs');
const HISTORY_PATH = (root = ROOT()) => join(RUNS_DIR(root), 'metric-reflection-history.jsonl');
const REPORT_PATH = (root = ROOT()) => join(root, 'docs', 'governance', 'metric-reflection-report.md');
const SCHEMA_VERSION = 1;

// ---------------------------------------------------------------------------
// Pure analysis — every function here takes plain data in, returns plain data out.
// Kept separate from I/O so tests can feed fixtures with no git shell / no real log file.
// ---------------------------------------------------------------------------

/** Fold a git-log commit list into a churn ranking. commits: [{hash, ts, subject, files: [path,...]}] */
export function foldGitHistory(commits) {
  const byFile = new Map();
  for (const c of commits) {
    for (const f of c.files ?? []) {
      byFile.set(f, (byFile.get(f) ?? 0) + 1);
    }
  }
  const files = [...byFile.entries()]
    .map(([file, count]) => ({ file, count }))
    .sort((a, b) => b.count - a.count || a.file.localeCompare(b.file));
  return { totalCommits: commits.length, byFile: files };
}

/**
 * Find cross-patterns: recurring failures that span more than a single point.
 * - cross-layer: the same `name` recurs as a failure under >=2 distinct layers
 * - churn-correlated: a recurring failure's layer or name substring-matches a file
 *   churned >= churnThreshold times in the window
 */
export function findCrossPatterns(recurringFailures, gitFold, { churnThreshold = 3 } = {}) {
  const patterns = [];

  const byName = new Map();
  for (const f of recurringFailures) {
    const cur = byName.get(f.name) ?? new Set();
    cur.add(f.layer);
    byName.set(f.name, cur);
  }
  for (const [name, layers] of byName.entries()) {
    if (layers.size >= 2) {
      patterns.push({ type: 'cross-layer', name, layers: [...layers].sort(), evidence: `"${name}" recurs as a failure under ${layers.size} distinct layers` });
    }
  }

  const churnedFiles = (gitFold?.byFile ?? []).filter((f) => f.count >= churnThreshold);
  for (const f of recurringFailures) {
    const needles = [f.layer, f.name].filter((s) => typeof s === 'string' && s.length >= 4).map((s) => s.toLowerCase());
    for (const cf of churnedFiles) {
      const haystack = cf.file.toLowerCase();
      const hit = needles.find((n) => haystack.includes(n));
      if (hit) {
        patterns.push({ type: 'churn-correlated', layer: f.layer, name: f.name, file: cf.file, churnCount: cf.count, evidence: `${f.layer}/${f.name} recurring failure correlates with ${cf.file} (${cf.count} commits, matched on "${hit}")` });
      }
    }
  }
  return patterns;
}

/** Reduce an analyze() report to the small fingerprint stored for historical comparison. */
export function buildSnapshot(analyzed, ts) {
  return {
    schema_version: SCHEMA_VERSION,
    ts,
    total_events: analyzed.total_events,
    by_layer_fail_rate: analyzed.by_layer.map((s) => ({ layer: s.layer, failRate: s.failRate })),
    recurring_failure_keys: analyzed.recurring_failures.map((f) => `${f.layer}|${f.name}`).sort(),
  };
}

/** Compare this run's snapshot against the last stored one (or null on a first run). */
export function compareHistory(current, previous) {
  if (!previous) return { isFirstRun: true, newRecurringFailures: [], resolvedRecurringFailures: [], failRateDeltas: [], totalEventsDelta: null };

  const prevKeys = new Set(previous.recurring_failure_keys ?? []);
  const curKeys = new Set(current.recurring_failure_keys ?? []);
  const newRecurringFailures = [...curKeys].filter((k) => !prevKeys.has(k)).sort();
  const resolvedRecurringFailures = [...prevKeys].filter((k) => !curKeys.has(k)).sort();

  const prevRates = new Map((previous.by_layer_fail_rate ?? []).map((s) => [s.layer, s.failRate]));
  const failRateDeltas = current.by_layer_fail_rate
    .filter((s) => prevRates.has(s.layer))
    .map((s) => ({ layer: s.layer, from: prevRates.get(s.layer), to: s.failRate, delta: s.failRate - prevRates.get(s.layer) }))
    .filter((d) => d.delta !== 0)
    .sort((a, b) => Math.abs(b.delta) - Math.abs(a.delta));

  return {
    isFirstRun: false,
    previousTs: previous.ts,
    newRecurringFailures,
    resolvedRecurringFailures,
    failRateDeltas,
    totalEventsDelta: current.total_events - previous.total_events,
  };
}

/** Orchestrate the pure pieces into one report object. No I/O. */
export function buildReport({ events, commits, previousSnapshot, ts, since, top = 5, repeatThreshold = 3, churnThreshold = 3 }) {
  const analyzed = analyze(events, { top, repeatThreshold });
  const gitFold = foldGitHistory(commits);
  const crossPatterns = findCrossPatterns(analyzed.recurring_failures, gitFold, { churnThreshold });
  const snapshot = buildSnapshot(analyzed, ts);
  const historyComparison = compareHistory(snapshot, previousSnapshot);
  return { advisory: true, generated_at: ts, since: since ?? null, patterns: analyzed, git_fold: gitFold, cross_patterns: crossPatterns, history_comparison: historyComparison, snapshot };
}

export function formatMarkdown(report) {
  const p = report.patterns;
  const lines = [
    '# Metric-Reflection Report (advisory)',
    '',
    `> Generated ${report.generated_at}${report.since ? ` · window since ${report.since}` : ''}. Folds` +
      ' `scripts/exec-telemetry.mjs` events + git commit history into cross-run insights. Read-only,' +
      ' advisory — feeds the self-improvement ratchet (CLAUDE.md); promoting a finding to a' +
      ' `docs/reflections/INBOX/` reflection or a `docs/regressions/REGRESSION-LEDGER.md` guardrail' +
      ' is always a human/council/librarian decision, never this script\'s own call.',
    '',
    '## Patterns (telemetry-analyze)',
    `- total events: ${p.total_events}`,
    ...p.by_layer.map((s) => `- ${s.layer}: n=${s.count} fail_rate=${(s.failRate * 100).toFixed(0)}% total=${s.totalDurationMs}ms`),
    p.recurring_failures.length
      ? `- recurring failures: ${p.recurring_failures.map((f) => `${f.layer}/${f.name} (${f.count}x)`).join(', ')}`
      : '- recurring failures: none',
    '',
    '## Cross-Patterns',
    report.cross_patterns.length ? '' : '(none found in this window)',
    ...report.cross_patterns.map((c) => `- [${c.type}] ${c.evidence}`),
    '',
    '## Git Churn (window)',
    `- commits: ${report.git_fold.totalCommits}`,
    ...report.git_fold.byFile.slice(0, 10).map((f) => `- ${f.file}: ${f.count} commit(s)`),
    '',
    '## Historical Comparison',
  ];
  const h = report.history_comparison;
  if (h.isFirstRun) {
    lines.push('- first recorded run — no prior snapshot to compare against');
  } else {
    lines.push(`- compared against snapshot from ${h.previousTs}`);
    lines.push(h.newRecurringFailures.length ? `- NEW recurring failures: ${h.newRecurringFailures.join(', ')}` : '- no new recurring failures');
    lines.push(h.resolvedRecurringFailures.length ? `- RESOLVED recurring failures: ${h.resolvedRecurringFailures.join(', ')}` : '- no resolved recurring failures');
    lines.push(h.failRateDeltas.length ? `- fail-rate deltas: ${h.failRateDeltas.map((d) => `${d.layer} ${(d.from * 100).toFixed(0)}%→${(d.to * 100).toFixed(0)}%`).join(', ')}` : '- no fail-rate change');
  }
  return lines.join('\n');
}

// ---------------------------------------------------------------------------
// I/O — real git shell, real filesystem. Not exercised by the pure-function unit tests above;
// only the round-trip CLI/history test at the bottom of scripts/metric-reflection.test.mjs touches this.
// ---------------------------------------------------------------------------

function ensureRunsDir(root) { mkdirSync(RUNS_DIR(root), { recursive: true }); }

export function loadHistory(root = ROOT()) {
  const path = HISTORY_PATH(root);
  if (!existsSync(path)) return [];
  const out = [];
  for (const line of readFileSync(path, 'utf8').split('\n')) {
    const t = line.trim();
    if (!t) continue;
    try { out.push(JSON.parse(t)); } catch { /* crashed trailing line — skip (append-atomicity) */ }
  }
  return out;
}

export function lastSnapshot(root = ROOT()) {
  const h = loadHistory(root);
  return h.length ? h[h.length - 1] : null;
}

export function appendHistory(snapshot, root = ROOT()) {
  ensureRunsDir(root);
  appendFileSync(HISTORY_PATH(root), `${JSON.stringify(snapshot)}\n`);
}

export function writeReport(markdown, root = ROOT()) {
  writeFileSync(REPORT_PATH(root), `${markdown}\n`);
}

/** Real `git log` in the given root, folded into the {hash, ts, subject, files} shape foldGitHistory expects. */
export function readGitCommits(since, root = ROOT()) {
  const SEP = '';
  let out;
  try {
    out = execFileSync('git', ['log', `--since=${since || '30 days ago'}`, `--pretty=format:%H${SEP}%cI${SEP}%s`, '--name-only'], { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  } catch {
    return [];
  }
  const commits = [];
  let cur = null;
  for (const line of out.split('\n')) {
    if (line.includes(SEP)) {
      if (cur) commits.push(cur);
      const [hash, ts, subject] = line.split(SEP);
      cur = { hash, ts, subject, files: [] };
    } else if (cur && line.trim()) {
      cur.files.push(line.trim());
    }
  }
  if (cur) commits.push(cur);
  return commits;
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------
function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--')) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next === undefined || (next.startsWith('--') && next.length > 2)) args[key] = true;
      else { args[key] = next; i++; }
    } else args._.push(a);
  }
  return args;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const sinceArg = typeof args.since === 'string' ? args.since : '30d';
  let events = readEvents();
  const sinceIso = parseSince(sinceArg);
  if (sinceIso) events = events.filter((e) => e.ts >= sinceIso);
  const commits = readGitCommits(sinceArg.replace(/^(\d+)d$/, '$1 days ago').replace(/^(\d+)h$/, '$1 hours ago'));
  const top = args.top !== undefined ? Number(args.top) : 5;
  const repeatThreshold = args['repeat-threshold'] !== undefined ? Number(args['repeat-threshold']) : 3;
  const churnThreshold = args['churn-threshold'] !== undefined ? Number(args['churn-threshold']) : 3;
  const ts = new Date().toISOString();
  const previousSnapshot = lastSnapshot();
  const report = buildReport({ events, commits, previousSnapshot, ts, since: sinceArg, top, repeatThreshold, churnThreshold });

  if (args.json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(formatMarkdown(report));
  }
  if (!args['no-write']) {
    writeReport(formatMarkdown(report));
    appendHistory(report.snapshot);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
