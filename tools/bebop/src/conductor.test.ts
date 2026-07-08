// Bebop conductor + 5-axis tests — RED+GREEN (Verified-by-Math, every gate must be falsifiable).
//
// GREEN cases prove the happy path works. RED cases prove a guard actually DENIES/fails on bad input
// (a test that cannot go red does not validate). No live backend binaries are required to run these.

import assert from 'node:assert/strict';
import os from 'node:os';
import path from 'node:path';
import fs from 'node:fs';
import test from 'node:test';

import { ADAPTERS, isAvailable, runBackend, type Backend } from './backend.ts';
import { selectBackend, rotate, probeAll } from './routing.ts';
import { BEBOP_PRESET, makeProfile, validateProfile, type Profile } from './profile.ts';
import { loadProfile, writeProfile, statusLine } from './init.ts';
import { emptyLedger, record, totalTokens, byBackend } from './token.ts';
import { runLoop } from './loop.ts';

// ---------- profile (axis 1-5 are data, not branching) ----------

test('GREEN: bebop preset is a valid profile with native last', () => {
  const p = BEBOP_PRESET;
  assert.equal(p.origin, 'hybrid');
  assert.equal(p.classKind, 'multi');
  assert.equal(p.narration, 'bebop');
  assert.equal(p.patrons, 'hybrid');
  assert.equal(p.looks, 'bebop');
  assert.ok(p.backendOrder.includes('native'));
  assert.equal(p.backendOrder[p.backendOrder.length - 1], 'native');
});

test('GREEN: makeProfile derives backend rotation from origin', () => {
  const claude = makeProfile({ origin: 'claude' });
  const open = makeProfile({ origin: 'opencode' });
  assert.equal(claude.backendOrder[0], 'claude');
  assert.equal(open.backendOrder[0], 'opencode');
  assert.ok(claude.backendOrder.includes('native'));
});

test('RED: validateProfile rejects a missing field', () => {
  assert.throws(() => validateProfile({ version: 1, origin: 'hybrid' }), /missing field/);
});

test('RED: validateProfile rejects unsupported version', () => {
  assert.throws(() => validateProfile({ version: 2, origin: 'hybrid', classKind: 'multi', narration: 'bebop', patrons: 'hybrid', looks: 'bebop', backendOrder: ['native'] }), /unsupported profile version/);
});

test('RED: validateProfile forces native into a rotation missing it', () => {
  const p = validateProfile({ version: 1, origin: 'hybrid', classKind: 'multi', narration: 'bebop', patrons: 'hybrid', looks: 'bebop', backendOrder: ['opencode'] });
  assert.ok(p.backendOrder.includes('native'));
});

// ---------- backend availability gate (uniform across all backends) ----------

test('GREEN: native backend is always available', () => {
  assert.equal(isAvailable('native'), true);
});

test('RED: an unavailable backend is reported unavailable (not crashed)', () => {
  // codex is "missing" in CI (no binary/key); the gate must return false, not throw.
  const ok = isAvailable('codex');
  assert.equal(typeof ok, 'boolean');
  if (!ADAPTERS.codex.detect()) assert.equal(ok, false);
});

test('GREEN: runBackend never shells out to an unavailable backend', () => {
  // If codex isn't installed, runBackend returns a clean "unavailable" result, no exec/hang.
  if (!ADAPTERS.codex.detect()) {
    const r = runBackend('codex', 'do something', {});
    assert.equal(r.ok, false);
    assert.match(r.summary, /unavailable/);
  } else {
    // codex IS available: runBackend must actually succeed (not just no-op)
    const r = runBackend('codex', 'do something', {});
    assert.equal(r.ok, true);
  }
});

// ---------- routing / rotation (the mesh fallback, as code) ----------

test('GREEN: selectBackend returns a real backend or native fallback', () => {
  const p = makeProfile({ origin: 'opencode' });
  const s = selectBackend(p, 'doer');
  assert.ok(s && ['opencode', 'claude', 'codex', 'hermes', 'goose', 'aider', 'native'].includes(s.backend));
});

test('GREEN: selectBackend always falls back to native when nothing installed', () => {
  // Contract: selectBackend must return a VALID backend that is available, or native. Build a profile
  // whose only non-native entry is genuinely unavailable (codex when its binary/keys are absent).
  const p: Profile = { ...BEBOP_PRESET, backendOrder: ['codex', 'native'] };
  const s = selectBackend(p, 'doer');
  assert.ok(s && (s.backend === 'native' || isAvailable(s.backend)));
  assert.ok(s.backend === 'native' || s.backend === 'codex');
});

test('GREEN: rotate skips the failed backend and returns another or native', () => {
  const p = makeProfile({ origin: 'opencode' });
  const r = rotate(p, 'opencode');
  assert.ok(r && r.backend !== 'opencode');
});

test('GREEN: probeAll returns one row per rotation entry', () => {
  const p = makeProfile({ origin: 'hybrid' });
  const rows = probeAll(p);
  assert.equal(rows.length, p.backendOrder.length);
  for (const r of rows) assert.equal(typeof r.available, 'boolean');
});

// ---------- kernel law: the envelope log is deterministic & recorded ----------

test('GREEN: dispatch records an envelope in the replayable log (kernel law)', async () => {
  let captured: any = null;
  const runNative = (t: string) => ({ ok: true, backend: 'native' as Backend, summary: `handled ${t}`, exitCode: 0 });
  const res = await runLoop({
    cwd: os.tmpdir(),
    taskClass: 'doer',
    profile: BEBOP_PRESET,
    maxSteps: 2,
    runNative,
    llm: async () => ({ tool_calls: [{ name: 'dispatch', args: { task: 'ship the fix' } }] }),
  });
  captured = res.log;
  assert.ok(captured.length >= 1);
  const d = captured.find((e: any) => e.event === 'dispatch');
  assert.ok(d, 'a dispatch envelope must be recorded');
  assert.equal(d.backend, 'native');
  assert.match(d.cause, /^[0-9a-f]{16}$/); // deterministic FNV-1a hash, not RNG
});

test('RED: same cause hashes identically (log determinism is what matters)', () => {
  const a = runLoop; // sanity import
  assert.ok(typeof a === 'function');
  // Re-run twice with the same injected llm → identical envelope cause for the same task.
  const runNative = (t: string) => ({ ok: true, backend: 'native' as Backend, summary: `h ${t}`, exitCode: 0 });
  const llm: any = async () => ({ tool_calls: [{ name: 'dispatch', args: { task: 'same-task' } }] });
  return (async () => {
    const r1 = await runLoop({ cwd: os.tmpdir(), taskClass: 'doer', profile: BEBOP_PRESET, maxSteps: 1, runNative, llm });
    const r2 = await runLoop({ cwd: os.tmpdir(), taskClass: 'doer', profile: BEBOP_PRESET, maxSteps: 1, runNative, llm });
    const e1 = r1.log.find((e: any) => e.event === 'dispatch');
    const e2 = r2.log.find((e: any) => e.event === 'dispatch');
    assert.equal(e1!.cause, e2!.cause);
  })();
});

// ---------- token ledger: central, cross-backend (§1.6) ----------

test('GREEN: ledger aggregates across multiple backends uniformly', () => {
  let l = emptyLedger();
  l = record(l, { backend: 'opencode', task: 'a', promptTokens: 10, completionTokens: 5, at: 1 });
  l = record(l, { backend: 'claude', task: 'b', promptTokens: 20, completionTokens: 10, at: 2 });
  assert.equal(totalTokens(l), 45);
  const by = byBackend(l);
  assert.equal(by.opencode, 15);
  assert.equal(by.claude, 30);
});

// ---------- init / persistence ----------

test('GREEN: writeProfile then loadProfile round-trips a valid profile', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'bebop-'));
  const orig = path.join(os.homedir(), '.bebop', 'settings.json');
  const backup = fs.existsSync(orig) ? fs.readFileSync(orig, 'utf8') : null;
  try {
    const p = makeProfile({ origin: 'claude', narration: 'sarcastic' });
    writeProfile(p);
    const loaded = loadProfile();
    assert.ok(loaded);
    assert.equal(loaded!.origin, 'claude');
    assert.equal(loaded!.narration, 'sarcastic');
    assert.equal(statusLine(loaded!).includes('claude'), true);
  } finally {
    if (backup) fs.writeFileSync(orig, backup);
    else fs.rmSync(path.join(os.homedir(), '.bebop', 'settings.json'), { force: true });
  }
});
