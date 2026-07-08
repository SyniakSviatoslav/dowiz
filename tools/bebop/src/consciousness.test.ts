// Bebop consciousness tests — self-maintenance, self-evolution, session-as-node. RED+GREEN.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  selfMaintain,
  selfEvolve,
  recordSession,
} from './consciousness.ts';
import { livingMemory } from './memory.ts';

// ── SELF-MAINTENANCE ──

test('GREEN: selfMaintain runs the self-harness and records health into the one living memory', () => {
  const h = selfMaintain();
  assert.equal(typeof h.ok, 'boolean');
  assert.ok(h.pass >= 0 && h.fail >= 0);
  // the health event was recorded (memory grew)
  assert.ok(livingMemory().size > 0);
});

// ── SELF-EVOLUTION (fail-closed) ──

test('GREEN: selfEvolve ACCEPTS a valid, novel (short) idea and persists it to living memory', () => {
  const before = livingMemory().size;
  const r = selfEvolve('cache PQ keys'); // short, well-damped mutation → passes resonance pre-check
  assert.equal(r.accepted, true);
  assert.ok(r.id, 'a persisted node id is returned');
  assert.ok(livingMemory().size >= before); // did not shrink
});

test('RED: selfEvolve QUARANTINES a trivial idea (fail-closed, not applied)', () => {
  const r = selfEvolve('x'); // < 4 chars → checker rejects
  assert.equal(r.accepted, false);
  assert.match(r.reason, /quarantined/i);
});

test('RED: selfEvolve QUARANTINES a near-duplicate idea', () => {
  selfEvolve('use spreading activation for associative recall');
  const r = selfEvolve('use spreading activation for associative recall'); // same → duplicate
  assert.equal(r.accepted, false);
});

test('RED: selfEvolve QUARANTINES a bulk mutation that would make self-evolution under-damped (resonance pre-check)', () => {
  // a very long/structural idea represents a large coupling gain → loopResonance flags ζ<0.707
  const bulk = 'restructure the entire corpus graph by rewiring every node edge weight and adding recursive sub-loops across all layers simultaneously';
  const r = selfEvolve(bulk);
  assert.equal(r.accepted, false);
  assert.match(r.reason, /resonance/i);
});

// ── SESSION-AS-NODE (brain-in-brain) ──

test('GREEN: recordSession records THIS session as a living-memory node with a child memory', () => {
  const id = recordSession({
    id: 'hermes-test-session',
    summary: 'this hermes session is a bebop node',
    childFacts: [['sub-fact', 'a session holds its own sub-memory']],
  });
  assert.ok(id);
  const child = livingMemory().findChild(id);
  assert.ok(child, 'session node nests a child memory (brain-in-brain)');
  assert.equal(child!.size, 1);
});
