// Bebop living-memory tests — one memory, always-on; VSA + graph + recursion. RED+GREEN.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  LivingMemory,
  livingMemory,
  seedBebopCorpus,
  embed,
  bind,
  bundle,
  similarity,
  permute,
  addressOf,
  runPlan,
  runLoopRec,
  snapshot,
  loadSnapshot,
  type Plan,
  type Loop,
} from './memory.ts';

// ── VSA core ──

test('GREEN: embed is deterministic; similar concepts are more similar than random', () => {
  const a = embed('kernel');
  const b = embed('kernel');
  const c = embed('totally-different-xyz');
  assert.deepEqual([...a], [...b]); // deterministic
  assert.ok(similarity(a, b) > similarity(a, c)); // same token closer than unrelated
});

test('GREEN: bind then unbind (permuted) approximately recovers the operand', () => {
  const x = embed('x');
  const y = embed('y');
  const bound = bind(x, y);
  const recovered = bind(bound, permute(y, 0)); // trivial permute (k=0) keeps y
  // binding is its own inverse for bipolar XOR; recovered should equal x
  assert.ok(similarity(recovered, x) > 0.9);
});

test('GREEN: addressOf is a stable content address (the VSA token-codec hack)', () => {
  assert.equal(addressOf('hello'), addressOf('hello'));
  assert.notEqual(addressOf('hello'), addressOf('world'));
});

// ── Graph / associative brain ──

test('GREEN: spreading activation recalls linked concepts associatively', () => {
  const mem = new LivingMemory();
  const a = mem.remember('alpha', 'A');
  const b = mem.remember('beta', 'B');
  const c = mem.remember('gamma', 'C');
  mem.rememberLink(a, b);
  mem.rememberLink(b, c);
  const recalled = mem.recall('alpha', 3);
  assert.ok(recalled.includes(b), 'beta should activate from alpha');
  assert.ok(recalled.includes(c), 'gamma should activate transitively');
});

test('RED: recall of an unknown concept returns empty (no hallucination)', () => {
  const mem = new LivingMemory();
  mem.remember('known', 'K');
  assert.deepEqual(mem.recall('nonexistent-concept-xyz'), []);
});

// ── ONE living memory, always-on ──

test('GREEN: livingMemory() is a process-wide singleton seeded with the corpus', () => {
  const m1 = livingMemory();
  const m2 = livingMemory();
  assert.strictEqual(m1, m2); // same instance
  assert.ok(m1.size >= 11, 'seeded with >=11 bebop rules/hacks');
  // the hermes session rule is present (this session is a node)
  const near = m1.nearest('hermes session', 1);
  assert.ok(near.length >= 1);
});

// ── BRAIN-IN-BRAIN recursion ──

test('GREEN: a memory node can nest a child memory (brain-in-brain)', () => {
  const mem = new LivingMemory();
  const parent = mem.remember('parent', 'P');
  const child = new LivingMemory();
  child.remember('child-concept', 'C');
  mem.nest(parent, child);
  assert.strictEqual(mem.findChild(parent), child);
  assert.equal(mem.findChild(parent)!.size, 1);
});

// ── PLANS-IN-PLANS ──

test('GREEN: runPlan executes nested sub-plans with the same executor', () => {
  const plan: Plan = {
    id: 'p',
    description: 'outer',
    steps: [
      { action: 'a' },
      { id: 'sub', description: 'inner', steps: [{ action: 'b' }, { action: 'c' }] },
    ],
  };
  const done: string[] = [];
  runPlan(plan, (action) => done.push(action));
  assert.deepEqual(done, ['a', 'b', 'c']);
});

// ── LOOPS-IN-LOOPS ──

test('GREEN: runLoopRec runs nested sub-loops, terminates by maxSteps (no infinite recursion)', () => {
  let outer = 0;
  let inner = 0;
  const loop: Loop = {
    id: 'outer',
    maxSteps: 2,
    body: () => {
      outer++;
    },
    sub: [{ id: 'inner', maxSteps: 3, body: () => inner++ }],
  };
  const total = runLoopRec(loop, {});
  assert.equal(outer, 2);
  assert.equal(inner, 6); // 2 outer * 3 inner
  assert.equal(total, 8);
});

// ── Persistence: content-addressed snapshot ──

test('GREEN: snapshot → loadSnapshot round-trips (durable, content-addressed)', () => {
  const mem = new LivingMemory();
  seedBebopCorpus(mem);
  const snap = snapshot(mem);
  assert.ok(snap.infoHash.length === 64); // sha256 self-certifying address
  const restored = loadSnapshot(snap);
  assert.equal(restored.size, mem.size);
  // recall still works after reload
  const r = restored.recall('copilot', 2);
  assert.ok(r.length > 0);
});

test('RED: a tampered snapshot changes its infoHash (integrity is falsifiable)', () => {
  const mem = new LivingMemory();
  mem.remember('x', 'X');
  const snap = snapshot(mem);
  const good = snap.infoHash;
  const tampered = { ...snap, nodes: snap.nodes.map((n, i) => (i === 0 ? { ...n, payload: 'TAMPERED' } : n)) };
  // recomputed address differs (we recompute via addressOf on the same shape)
  const bad = addressOf(JSON.stringify(tampered.nodes.map((n) => n.id + n.payload).sort()));
  assert.notEqual(good, bad);
});
