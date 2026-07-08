// Bebop doctrine tests — "as above, so below" (universal Checker gate) + native copilot mode.
// RED+GREEN: every GREEN proves a behavior; every RED proves the guardrail can actually fire.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  applyCommand,
  applyCommandChecked,
  defaultChecker,
  decide,
  genesis,
  type Command,
  type Checker,
} from './kernel.ts';
import { runCopilot, defaultChecker as copilotDefault, type CheckerFn } from './copilot.ts';
import { BEBOP_PRESET } from './profile.ts';
import { sha256hex } from './crypto.ts';

function cmd(partial: Partial<Command> = {}): Command {
  return {
    actor: { kind: 'node', id: 'n1' },
    action: 'INGEST',
    payload: sha256hex('x'),
    nonce: '1',
    ...partial,
  };
}

// ───────── "AS ABOVE, SO BELOW" — universal Checker gate ─────────

test('GREEN: applyCommandChecked admits a valid transition (doer below, checker above)', () => {
  const c = cmd();
  const { quarantined, envelopes } = applyCommandChecked(c, genesis(), defaultChecker);
  assert.equal(quarantined, false);
  assert.equal(envelopes.length, 1);
});

test('RED: applyCommandChecked QUARANTINES a transition that violates the invariant (fail-closed)', () => {
  // a checker that rejects INGEST with a specific payload
  const rejector: Checker = (_c, _b, _a, events) => {
    for (const e of events) if (e.type === 'INGESTED' && e.contentHash === sha256hex('x')) return { ok: false, reason: 'forbidden payload' };
    return { ok: true };
  };
  const c = cmd();
  const res = applyCommandChecked(c, genesis(), rejector);
  assert.equal(res.quarantined, true);
  assert.equal(res.reason, 'forbidden payload');
  // the state must be UNCHANGED (not admitted)
  assert.equal(res.state.ingested.has(c.payload), false);
  assert.equal(res.envelopes[0].event.type, 'DENIED');
});

test('GREEN: the SAME checker shape is reused at the mesh scale (signature+invariant symmetry)', () => {
  // at the kernel scale, defaultChecker is a pure invariant; at the mesh scale, the receiver runs the
  // same invariant over the envelope. Here we assert the invariant is pure + deterministic so it CAN
  // be reused identically at both scales.
  const c = cmd({ nonce: 'A' });
  const r1 = defaultChecker(c, genesis(), genesis(), decide(c, genesis()));
  const r2 = defaultChecker(c, genesis(), genesis(), decide(c, genesis()));
  assert.deepEqual(r1, r2);
  assert.equal(r1.ok, true);
});

// ───────── NATIVE COPILOT MODE (DEFAULT) ─────────

test('GREEN: copilot is DEFAULT-on and runs doer then a DISTINCT checker when one is available', () => {
  let checked = false;
  const checker: CheckerFn = (task, out) => {
    checked = true;
    assert.ok(task.length > 0);
    return out ? 'approve' : 'reject';
  };
  // default-on (no enabled flag), and force a checker distinct from the doer to prove independence
  const res = runCopilot({
    task: 'refactor the kernel',
    profile: BEBOP_PRESET,
    forcedChecker: 'claude', // a DIFFERENT backend than the native doer
    checker,
    runNative: (t) => ({ ok: true, backend: 'native', summary: `did: ${t}`, exitCode: 0 }),
  });
  assert.equal(res.verdict, 'approve');
  assert.equal(checked, true); // the checker (above) actually ran
  assert.equal(res.doer, 'native');
  assert.notEqual(res.checker, res.doer); // checker MUST differ from doer
});

test('GREEN: copilot DEFAULT-on still runs the checker even when only the native backend is present', () => {
  let checked = false;
  const res = runCopilot({
    task: 'x',
    // no forcedChecker, only native available -> checker falls back to native stub but STILL runs
    checker: (_t, out) => {
      checked = true;
      return out ? 'approve' : 'reject';
    },
    runNative: (t) => ({ ok: true, backend: 'native', summary: `did: ${t}`, exitCode: 0 }),
  });
  assert.equal(res.verdict, 'approve');
  assert.equal(checked, true); // above still watched below
});

test('RED: copilot QUARANTINES (ok=false) when the checker rejects the doer output', () => {
  const res = runCopilot({
    task: 'do something',
    profile: BEBOP_PRESET,
    checker: (_t, out) => (out && /fail/i.test(out) ? 'reject' : 'approve'),
    runNative: () => ({ ok: false, backend: 'native', summary: 'failed to run', exitCode: 1 }),
  });
  assert.equal(res.verdict, 'reject');
  assert.equal(res.ok, false); // not permitted to proceed
});

test('GREEN: copilot can be explicitly disabled (doer only, no checker over the result)', () => {
  let checked = false;
  const res = runCopilot({
    task: 'x',
    enabled: false,
    checker: () => {
      checked = true;
      return 'approve';
    },
    runNative: () => ({ ok: true, backend: 'native', summary: 'ok', exitCode: 0 }),
  });
  assert.equal(checked, false); // checker not invoked when disabled
  assert.equal(res.note, 'copilot disabled');
});
