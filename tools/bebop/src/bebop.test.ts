// Bebop — Verified-by-Math tests. Every gate ships GREEN and a RED case that flips it.
// No live LLM / network required: the loop uses the deterministic stub.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkRedLine, checkScope, certifyGate, selfTest } from './guard.ts';
import { route, enforceRouting } from './router.ts';
import { runLoop } from './loop.ts';
import path from 'node:path';
import os from 'node:os';
import fs from 'node:fs';

const REPO = path.resolve(import.meta.dirname, '..', '..');

test('RED: red-line edit is denied', () => {
  const d = checkRedLine('packages/db/migrations/003_x.sql');
  assert.equal(d.ok, false);
  assert.equal(d.kind, 'redline');
});

test('GREEN: in-scope non-redline edit is allowed', () => {
  const d = checkRedLine('tools/bebop/src/loop.ts');
  assert.equal(d.ok, true);
});

test('RED: scope-block rejects out-of-scope file', () => {
  const d = checkScope('apps/api/src/server.ts');
  assert.equal(d.ok, false);
  assert.equal(d.kind, 'scope');
});

test('GREEN: scope allows agreed surface', () => {
  const d = checkScope('tools/bebop/src/theme.ts');
  assert.equal(d.ok, true);
});

test('falsifiable gate: rejects a guard that cannot go red (no false-green)', () => {
  const alwaysGreen = certifyGate({ name: 'fake', green: () => true, red: () => true });
  assert.equal(alwaysGreen.certified, false);
});

test('falsifiable gate: certifies a guard that fails on bad input', () => {
  const real = certifyGate({ name: 'real', green: () => true, red: () => false });
  assert.equal(real.certified, true);
});

test('selfTest: gates certify (green+red) — Bebop refuses to start broken', () => {
  const t = selfTest();
  assert.equal(t.ok, true, t.log.join('\n'));
});

test('token router: redline escalates to opus', () => {
  assert.equal(route('redline').model, 'opus');
});

test('token router: doer routes to haiku (cheapest adequate)', () => {
  assert.equal(route('doer').model, 'haiku');
});

test('RED: routing a redline to haiku is a violation', () => {
  const g = enforceRouting('redline', 'haiku');
  assert.equal(g.ok, false);
});

test('GREEN: routing a doer to haiku is allowed', () => {
  const g = enforceRouting('doer', 'haiku');
  assert.equal(g.ok, true);
});

test('loop: runs, terminates via done, makes no mutations with stub', async () => {
  const res = await runLoop({ cwd: REPO, taskClass: 'doer' });
  assert.ok(res.steps >= 1);
  assert.equal(res.denied, 0);
  assert.equal(res.ok, true);
});

test('loop: an edit inside red-line is denied and flagged', async () => {
  // craft an llm that tries to edit a migration file
  const llm = () => ({
    tool_calls: [{ name: 'edit' as const, args: { path: 'packages/db/migrations/999_bad.sql', content: 'x' } }],
  });
  const res = await runLoop({ cwd: REPO, taskClass: 'doer', llm, maxSteps: 4 });
  assert.equal(res.denied, 1);
  assert.equal(res.ok, false); // routing ok but a denial means not clean
});

test('loop: an in-scope edit is applied (mutation counted)', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'bebop-'));
  const target = path.join(dir, 'note.txt');
  const llm = (msgs: any[]) => {
    const called = msgs.some((m) => m.role === 'tool' && m.name === 'edit');
    if (!called) return { tool_calls: [{ name: 'edit' as const, args: { path: target, content: 'hi' } }] };
    return { tool_calls: [{ name: 'done' as const, args: {} }] };
  };
  const res = await runLoop({ cwd: dir, taskClass: 'doer', llm, maxSteps: 6, scope: [`${dir}/**`] });
  assert.equal(res.mutations, 1);
  assert.equal(fs.readFileSync(target, 'utf8'), 'hi');
});
