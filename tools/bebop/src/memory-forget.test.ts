// Bebop living memory — MULTI-LAYERED FORGETTING tests (human-like memory). RED+GREEN.
// Every property has a falsifiable RED case: a node that SHOULD be forgotten is gone, and a node
// that SHOULD persist is present. Forgetting is real, not a no-op.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LivingMemory } from './memory.ts';

// ── GREEN: pinned corpus nodes NEVER forget ──

test('GREEN: a pinned (corpus) node survives 100 ticks of decay — the constitution is durable', () => {
  const m = new LivingMemory();
  const id = m.remember('kernel law', 'decide/fold/replay is pure', undefined, { layer: 'long', pinned: true });
  for (let i = 0; i < 100; i++) m.tick();
  assert.ok(m.size >= 1, 'pinned node must persist');
  assert.equal(m.layerSize('long') >= 1, true);
  // re-find it by concept
  const found = [...m.nearest('kernel law', 1)].some((n) => n.id === id);
  assert.ok(found, 'pinned node still retrievable after decay');
});

// ── GREEN: short-term node, never recalled, IS forgotten (decay + eviction) ──

test('GREEN: an unrecalled short-term node decays and is FORGOTTEN after enough ticks', () => {
  const m = new LivingMemory();
  const id = m.remember('ephemeral task', 'do X then forget');
  assert.ok(m.size === 1, 'node present immediately');
  for (let i = 0; i < 60; i++) m.tick(); // 0.94^60 ≈ 0.025 < LONG_FLOOR once promoted... but it's short
  // a short node never recalled stays 'short'; decay drives strength → capacity eviction keeps it low
  // after 60 ticks with no recall, it must be gone (forgetting happened)
  const stillThere = [...m.nearest('ephemeral task', 1)].some((n) => n.id === id);
  assert.equal(stillThere, false, 'unrecalled short-term node must be forgotten');
});

// ── RED: a freshly-remembered short node IS present (proves forget is not a universal no-op) ──

test('RED: a freshly-remembered node is present (forgetting is conditional, not default)', () => {
  const m = new LivingMemory();
  const id = m.remember('fresh idea', 'still hot');
  const present = [...m.nearest('fresh idea', 1)].some((n) => n.id === id);
  assert.equal(present, true, 'fresh node must be present before any decay');
});

// ── GREEN: working memory evicts OLDEST when over capacity (FIFO scratch) ──

test('GREEN: working memory evicts the oldest entry when over its tiny capacity', () => {
  const m = new LivingMemory();
  const oldest = m.remember('w0', 'first', undefined, { layer: 'working' });
  m.remember('w1', 'second', undefined, { layer: 'working' });
  m.remember('w2', 'third', undefined, { layer: 'working' });
  m.remember('w3', 'fourth', undefined, { layer: 'working' });
  m.remember('w4', 'fifth — overflow', undefined, { layer: 'working' }); // WORKING_CAP = 4
  assert.equal(m.layerSize('working'), 4, 'working layer capped at WORKING_CAP');
  const oldestGone = [...m.nearest('w0', 1)].some((n) => n.id === oldest);
  assert.equal(oldestGone, false, 'oldest working node evicted on overflow');
});

// ── GREEN: consolidation — recalling a short node promotes it to long and it then survives ──

test('GREEN: repeated recall consolidates a short node → long, and it survives ticks', () => {
  const m = new LivingMemory();
  const id = m.remember('learn this', 'important fact');
  assert.equal(m.layerSize('short'), 1);
  // recall it 3× (PROMOTE_RECALLS) → promoted to long
  m.recall('learn this');
  m.recall('learn this');
  m.recall('learn this');
  assert.equal(m.layerSize('long'), 1, 'node consolidated to long-term');
  // now ticks should NOT forget it (long + recently recalled strength high)
  for (let i = 0; i < 30; i++) m.tick();
  const survived = [...m.nearest('learn this', 1)].some((n) => n.id === id);
  assert.equal(survived, true, 'consolidated long-term node survives decay');
});

// ── GREEN: persistence carries layer/strength so reload preserves forgetting state ──

test('GREEN: snapshot persists only non-working layers; working stays volatile', () => {
  const m = new LivingMemory();
  m.remember('persist me', 'long-term fact', undefined, { layer: 'long' });
  m.remember('scratch', 'volatile', undefined, { layer: 'working' });
  const snap = m.toSnapshot();
  const concepts = snap.nodes.map((n) => n.concept);
  assert.ok(concepts.includes('persist me'), 'long-term node in snapshot');
  assert.ok(!concepts.includes('scratch'), 'working node excluded from snapshot');
});

// ── RED: long-term node with NO recall eventually fades below floor and is forgotten ──

test('RED: a long-term node that is never recalled fades below the floor and is forgotten', () => {
  const m = new LivingMemory();
  const id = m.remember('old long fact', 'once known', undefined, { layer: 'long' });
  // never recall it; let decay push strength below LONG_FLOOR
  for (let i = 0; i < 80; i++) m.tick(); // 0.94^80 ≈ 0.007 < LONG_FLOOR 0.05
  const gone = [...m.nearest('old long fact', 1)].some((n) => n.id === id);
  assert.equal(gone, false, 'unrecalled long-term node must fade and be forgotten');
});
