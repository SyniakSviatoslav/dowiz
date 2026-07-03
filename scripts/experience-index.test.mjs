#!/usr/bin/env node
// Hermetic tests for scripts/experience-index.mjs (plan §6.1 — the red→green gate).
// Fully self-contained: fixtures live in an OS temp dir addressed via the
// EXPERIENCE_INDEX_ROOT override. NEVER touches the real loops/runs or docs.
//
// Run: node --test scripts/experience-index.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SCRIPT = path.join(HERE, 'experience-index.mjs');

// ── fixture helpers ──────────────────────────────────────────────────────────

function mkRoot() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'expidx-'));
  fs.mkdirSync(path.join(root, 'loops', 'runs'), { recursive: true });
  return root;
}

function writeMetrics(root, rows) {
  fs.writeFileSync(
    path.join(root, 'loops', 'runs', 'metrics.jsonl'),
    rows.map((r) => JSON.stringify(r)).join('\n') + '\n',
  );
}

/** N metrics lines for one signature+arm with a given outcome (+ optional extras). */
function lines(n, { sig, arm, outcome, fake = 0, cost = 1, per = null }) {
  return Array.from({ length: n }, (_, i) => ({
    loop: arm, run_index: i + 1, ts: '2026-07-02T00:00:00Z', outcome,
    signature: sig, arm, fake_green_caught: fake, cost_usd: cost,
    fail_start: outcome === 'green' ? 3 : 3, fail_end: outcome === 'green' ? 0 : 3,
    per_resolved: per,
  }));
}

function runCli(root, args) {
  return spawnSync('node', [SCRIPT, ...args], {
    env: { ...process.env, EXPERIENCE_INDEX_ROOT: root },
    encoding: 'utf8',
  });
}

/** recursive snapshot of every file path under a dir (for the no-writes assertion). */
function snapshot(dir) {
  const out = [];
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p); else out.push(p);
    }
  };
  walk(dir);
  return out.sort();
}

function suggestJson(root, task) {
  const r = runCli(root, ['--suggest', task, '--json']);
  assert.equal(r.status, 0, `exit 0 (got ${r.status}); stderr=${r.stderr}`);
  return JSON.parse(r.stdout);
}

// ── (a) the loop is closed: ranking flips when the fixture outcome flips ───────

test('(a) ranking flips when the fixture outcome flips (closed loop)', () => {
  const root = mkRoot();

  // Arm A wins 8/10, arm B wins 2/10 on signature S → A ranks first.
  writeMetrics(root, [
    ...lines(8, { sig: 'S', arm: 'armA', outcome: 'green' }),
    ...lines(2, { sig: 'S', arm: 'armA', outcome: 'stall' }),
    ...lines(2, { sig: 'S', arm: 'armB', outcome: 'green' }),
    ...lines(8, { sig: 'S', arm: 'armB', outcome: 'stall' }),
  ]);
  let digest = JSON.parse(runCli(root, ['digest', '--json']).stdout);
  let bucket = digest.signatures.find((s) => s.key === 'S');
  assert.ok(bucket, 'signature S present');
  assert.equal(bucket.arms[0].arm, 'armA', 'armA ranks first when it wins 8/10');

  // FLIP the outcomes → armB should now rank first.
  writeMetrics(root, [
    ...lines(2, { sig: 'S', arm: 'armA', outcome: 'green' }),
    ...lines(8, { sig: 'S', arm: 'armA', outcome: 'stall' }),
    ...lines(8, { sig: 'S', arm: 'armB', outcome: 'green' }),
    ...lines(2, { sig: 'S', arm: 'armB', outcome: 'stall' }),
  ]);
  digest = JSON.parse(runCli(root, ['digest', '--json']).stdout);
  bucket = digest.signatures.find((s) => s.key === 'S');
  assert.equal(bucket.arms[0].arm, 'armB', 'armB ranks first after the flip — output tracks recorded outcome');
});

// ── (b) anti-cheat: a cheapest-but-fake-green arm does NOT rank first ──────────

test('(b) cheapest arm with fake_green_caught>0 does NOT rank first (quality gate)', () => {
  const root = mkRoot();
  writeMetrics(root, [
    // CHEAP: nominally "green" but fake-green caught → every trial is a LOSS.
    //        cheapest cost — would win if cost were the reward. It must not.
    ...lines(10, { sig: 'S', arm: 'cheapFake', outcome: 'green', fake: 1, cost: 0.01 }),
    // SOLID: genuine wins, higher cost.
    ...lines(7, { sig: 'S', arm: 'solid', outcome: 'green', cost: 5 }),
    ...lines(3, { sig: 'S', arm: 'solid', outcome: 'stall', cost: 5 }),
  ]);
  const digest = JSON.parse(runCli(root, ['digest', '--json']).stdout);
  const bucket = digest.signatures.find((s) => s.key === 'S');
  assert.notEqual(bucket.arms[0].arm, 'cheapFake', 'fake-green arm must not rank first');
  assert.equal(bucket.arms[0].arm, 'solid', 'the genuinely-winning (pricier) arm ranks first');
  const cheap = bucket.arms.find((a) => a.arm === 'cheapFake');
  assert.equal(cheap.wins, 0, 'fake-green green counts as 0 wins');
  assert.equal(cheap.losses, 10, 'fake-green green counts as losses');
});

// ── (c) exit 0 + zero writes to gate state / predictions ──────────────────────

test('(c) exit 0 and zero writes (no gate state, no predictions mutation)', () => {
  const root = mkRoot();
  writeMetrics(root, [...lines(6, { sig: 'S', arm: 'armA', outcome: 'green' })]);
  // seed a predictions file to prove it is never mutated.
  const predPath = path.join(root, 'loops', 'runs', 'predictions.jsonl');
  fs.writeFileSync(predPath, JSON.stringify({ target: 'x', confidence: 0.5, gap: 'hit' }) + '\n');
  const before = snapshot(root);
  const predBefore = fs.readFileSync(predPath, 'utf8');

  for (const argv of [['digest'], ['digest', '--json'], ['--suggest', 'fix all tests to green'], ['--suggest', 'x', '--json']]) {
    const r = runCli(root, argv);
    assert.equal(r.status, 0, `exit 0 for [${argv.join(' ')}] (got ${r.status})`);
  }

  const after = snapshot(root);
  assert.deepEqual(after, before, 'no files created or removed under the root');
  assert.equal(fs.readFileSync(predPath, 'utf8'), predBefore, 'predictions.jsonl byte-identical');
  assert.ok(!fs.existsSync(path.join(root, '.claude')), 'no gate-state dir created');
});

// ── (d) --json shape is stable ────────────────────────────────────────────────

test('(d) --json shape stable (digest + suggest)', () => {
  const root = mkRoot();
  writeMetrics(root, [...lines(6, { sig: 'S', arm: 'armA', outcome: 'green' })]);

  const digest = JSON.parse(runCli(root, ['digest', '--json']).stdout);
  assert.equal(digest.advisory, true);
  assert.match(digest.reward_source, /deterministic/i);
  assert.ok(Array.isArray(digest.signatures));
  const arm = digest.signatures[0].arms[0];
  for (const k of ['arm', 'wins', 'losses', 'trials', 'win_rate', 'wilson_lower', 'insufficient_data']) {
    assert.ok(k in arm, `arm has key ${k}`);
  }

  const sug = suggestJson(root, 'fix all tests to green');
  assert.equal(sug.advisory, true, 'suggest stamps advisory:true (router does not consume)');
  assert.match(sug.reward_source, /deterministic/i);
  for (const k of ['task', 'signature', 'recommendation', 'ranked_arms', 'advisory_pack']) {
    assert.ok(k in sug, `suggest has key ${k}`);
  }
  assert.ok('action' in sug.recommendation);
  assert.ok('key' in sug.signature);
});

// ── (e) insufficient-data path doesn't crash ──────────────────────────────────

test('(e) insufficient / empty data does not crash and yields a no-recommendation', () => {
  const root = mkRoot(); // no metrics.jsonl at all
  const d = runCli(root, ['digest']);
  assert.equal(d.status, 0);
  const dj = JSON.parse(runCli(root, ['digest', '--json']).stdout);
  assert.equal(dj.metrics_rows, 0);
  assert.deepEqual(dj.signatures, []);

  const sug = suggestJson(root, 'polish the storefront menu ui'); // non-redline, no data
  assert.equal(sug.recommendation.action, 'no-recommendation', 'no data → falls back, does not fabricate a pick');

  // a single line (below MIN_TRIALS) must be flagged insufficient, not crash.
  writeMetrics(root, [...lines(1, { sig: 'S', arm: 'armA', outcome: 'green' })]);
  const dj2 = JSON.parse(runCli(root, ['digest', '--json']).stdout);
  assert.equal(dj2.signatures[0].arms[0].insufficient_data, true, 'n<MIN_TRIALS flagged insufficient');
});

// ── extra invariant: red-line signature forces escalate (plan §4) ─────────────

test('(f) red-line signature forces escalate regardless of history', () => {
  const root = mkRoot();
  const sug = suggestJson(root, 'fix the auth token refund money migration'); // auth+money+migration
  assert.equal(sug.signature.redline, true, 'signature flags red-line');
  assert.equal(sug.recommendation.action, 'escalate', 'red-line → escalate; history never buys a shortcut');
});
