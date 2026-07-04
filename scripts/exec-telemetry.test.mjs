// exec-telemetry.test.mjs — red-green DoD test for scripts/exec-telemetry.mjs (emitter) and
// scripts/telemetry-analyze.mjs (analyzer), SYSTEMS-MAP.md backlog item 3.
//
// Run: node --test scripts/exec-telemetry.test.mjs
//
// All fs work happens in per-test temp dirs (EXEC_TELEMETRY_ROOT override). The real
// loops/runs/ is never touched.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { buildEvent, eventsPath, ACTION_KINDS, OUTCOMES } from './exec-telemetry.mjs';
import { analyze } from './telemetry-analyze.mjs';

const CLI = join(import.meta.dirname, 'exec-telemetry.mjs');

function tmp(t) {
  const dir = mkdtempSync(join(tmpdir(), 'exectel-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

function cli(args, root) {
  return spawnSync(process.execPath, [CLI, ...args], {
    encoding: 'utf8', shell: false,
    env: { ...process.env, EXEC_TELEMETRY_ROOT: root },
  });
}

test('emit rejects an invalid layer (not kebab-case)', () => {
  assert.throws(() => buildEvent({
    layer: 'Not Kebab!', actionKind: 'loop-run', name: 'x', outcome: 'pass', durationMs: 10,
  }), /layer must match/);
});

test('emit rejects an unknown action_kind', () => {
  assert.throws(() => buildEvent({
    layer: 'ssg', actionKind: 'not-a-kind', name: 'x', outcome: 'pass', durationMs: 10,
  }), /action_kind must be one of/);
});

test('emit rejects an unknown outcome', () => {
  assert.throws(() => buildEvent({
    layer: 'ssg', actionKind: 'loop-run', name: 'x', outcome: 'maybe', durationMs: 10,
  }), /outcome must be one of/);
});

test('emit rejects a negative duration_ms', () => {
  assert.throws(() => buildEvent({
    layer: 'ssg', actionKind: 'loop-run', name: 'x', outcome: 'pass', durationMs: -5,
  }), /duration_ms must be a non-negative number/);
});

test('emit rejects meta that is not a JSON object', () => {
  assert.throws(() => buildEvent({
    layer: 'ssg', actionKind: 'loop-run', name: 'x', outcome: 'pass', durationMs: 10, meta: '[1,2,3]',
  }), /meta must be a JSON object/);
});

test('every declared ACTION_KINDS/OUTCOMES value round-trips through buildEvent', () => {
  for (const actionKind of ACTION_KINDS) {
    for (const outcome of OUTCOMES) {
      const e = buildEvent({ layer: 'loop-registry', actionKind, name: 'n', outcome, durationMs: 1 });
      assert.equal(e.action_kind, actionKind);
      assert.equal(e.outcome, outcome);
    }
  }
});

test('CLI emit appends a schema-v1 event with the exact documented field set', (t) => {
  const root = tmp(t);
  const r = cli(['emit', '--layer', 'ssg', '--action-kind', 'gate-pass', '--name', 'lane-merge', '--outcome', 'pass', '--duration-ms', '1234', '--tokens', '500', '--meta', '{"lane":"a"}'], root);
  assert.equal(r.status, 0, r.stderr);
  const month = new Date().toISOString().slice(0, 7);
  const rows = readFileSync(eventsPath(month, root), 'utf8').trim().split('\n').map((l) => JSON.parse(l));
  assert.equal(rows.length, 1);
  const e = rows[0];
  assert.equal(e.layer, 'ssg');
  assert.equal(e.action_kind, 'gate-pass');
  assert.equal(e.name, 'lane-merge');
  assert.equal(e.outcome, 'pass');
  assert.equal(e.duration_ms, 1234);
  assert.equal(e.tokens, 500);
  assert.deepEqual(e.meta, { lane: 'a' });
  assert.ok(e.ts);
});

test('CLI emit fails LOUDLY (nonzero exit, no file written) on an invalid outcome', (t) => {
  const root = tmp(t);
  const r = cli(['emit', '--layer', 'ssg', '--action-kind', 'gate-pass', '--name', 'x', '--outcome', 'bogus', '--duration-ms', '1'], root);
  assert.notEqual(r.status, 0);
  const month = new Date().toISOString().slice(0, 7);
  assert.throws(() => readFileSync(eventsPath(month, root), 'utf8'));
});

test('CLI query filters by layer/action-kind/outcome', (t) => {
  const root = tmp(t);
  cli(['emit', '--layer', 'ssg', '--action-kind', 'gate-pass', '--name', 'a', '--outcome', 'pass', '--duration-ms', '10'], root);
  cli(['emit', '--layer', 'skill-evolution', '--action-kind', 'gate-fail', '--name', 'b', '--outcome', 'fail', '--duration-ms', '20'], root);
  const r = cli(['query', '--layer', 'ssg', '--json'], root);
  assert.equal(r.status, 0, r.stderr);
  const out = JSON.parse(r.stdout);
  assert.equal(out.count, 1);
  assert.equal(out.events[0].layer, 'ssg');
});

// ---------------------------------------------------------------------------
// telemetry-analyze — pure-function tests over a fixture (no CLI/fs needed)
// ---------------------------------------------------------------------------
function fixtureEvents() {
  return [
    { layer: 'ssg', action_kind: 'gate-pass', name: 'lane-1', outcome: 'pass', duration_ms: 1000, ts: '2026-01-01T00:00:00.000Z' },
    { layer: 'ssg', action_kind: 'gate-fail', name: 'lane-2', outcome: 'fail', duration_ms: 3000, ts: '2026-01-01T00:01:00.000Z' },
    { layer: 'ssg', action_kind: 'gate-fail', name: 'lane-2', outcome: 'fail', duration_ms: 2000, ts: '2026-01-01T00:02:00.000Z' },
    { layer: 'ssg', action_kind: 'gate-fail', name: 'lane-2', outcome: 'fail', duration_ms: 2500, ts: '2026-01-01T00:03:00.000Z' },
    { layer: 'skill-evolution', action_kind: 'loop-run', name: 'draft-skill', outcome: 'pass', duration_ms: 500, ts: '2026-01-01T00:04:00.000Z' },
  ];
}

test('analyze: recurring-failure detection surfaces (layer,name) pairs at/above threshold', () => {
  const report = analyze(fixtureEvents(), { repeatThreshold: 3 });
  assert.equal(report.recurring_failures.length, 1);
  assert.equal(report.recurring_failures[0].layer, 'ssg');
  assert.equal(report.recurring_failures[0].name, 'lane-2');
  assert.equal(report.recurring_failures[0].count, 3);
});

test('analyze: recurring-failure detection does NOT flag below-threshold repeats (RED-proof canary)', () => {
  const report = analyze(fixtureEvents(), { repeatThreshold: 4 });
  assert.equal(report.recurring_failures.length, 0, 'threshold of 4 must exclude a 3x failure — proves the >= comparison is not silently off-by-one');
});

test('analyze: by-layer fail rate and bottleneck ordering', () => {
  const report = analyze(fixtureEvents(), { top: 2 });
  const ssg = report.by_layer.find((s) => s.layer === 'ssg');
  assert.equal(ssg.count, 4);
  assert.equal(ssg.fails, 3);
  assert.equal(ssg.failRate, 0.75);
  assert.equal(ssg.totalDurationMs, 8500);
  // ssg (8500ms total) must rank above skill-evolution (500ms total) as the top bottleneck.
  assert.equal(report.bottlenecks.layers[0].layer, 'ssg');
  assert.equal(report.bottlenecks.layers.length, 2);
});

test('analyze: empty input never throws and reports zero events', () => {
  const report = analyze([]);
  assert.equal(report.total_events, 0);
  assert.deepEqual(report.by_layer, []);
  assert.deepEqual(report.recurring_failures, []);
});
