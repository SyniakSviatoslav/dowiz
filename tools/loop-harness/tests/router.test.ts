import { test } from 'node:test';
import assert from 'node:assert/strict';
import { route, scoreMatches } from '../src/router.js';
import type { RegistryLoop } from '../src/registry.js';

const REG: RegistryLoop[] = [
  { id: 'i18n', goal: 'untranslated strings down + al/en green', trigger_tags: ['i18n', 'translation', 'locale', 'parity'], scope_class: 'A', security_carveout: [], status: 'active' },
  { id: 'convergence', goal: 'UI server flows to green', trigger_tags: ['e2e', 'flow', 'green', 'qa', 'wiring'], scope_class: 'A', security_carveout: [], status: 'active' },
  { id: 'retired-x', goal: 'old', trigger_tags: ['perf'], scope_class: 'A', security_carveout: [], status: 'retired' },
];

test('router — DIRECT is the default for one-shot / non-iterative commands', () => {
  for (const c of ['git status', 'explain this function', 'show the order schema', 'list the routes', 'what does mapOrderRow do']) {
    assert.equal(route(c, REG).outcome, 'DIRECT', `"${c}" should be DIRECT`);
  }
});

test('router — RUN a registered loop on a tag match', () => {
  const d = route('run the i18n coverage loop for untranslated strings', REG);
  assert.equal(d.outcome, 'RUN');
  assert.equal(d.loopId, 'i18n');
  assert.match(d.announce, /RUN i18n/);
});

test('router — retired loops are not matched', () => {
  // "perf" only belongs to a retired loop → no active match → admissible → BUILD
  const d = route('performance loop for slow queries', REG);
  assert.notEqual(d.outcome, 'RUN');
});

test('router — BUILD when loop-worthy + no match + oracle-admissible', () => {
  const d = route('BE polishing loop to get backend green', REG);
  assert.equal(d.outcome, 'BUILD');
  assert.match(d.goal!, /BE polishing/);
});

test('router — BOUNCE when loop-worthy but no admissible metric', () => {
  const d = route('polish the UI to make it feel nicer in a loop', REG);
  assert.equal(d.outcome, 'BOUNCE');
  assert.match(d.reason, /metric|measurable/i);
});

test('router — never returns RUN/BUILD without a loop/goal; always announces', () => {
  for (const c of ['git status', 'qa loop', 'BE polishing loop', 'make it pretty in a loop']) {
    const d = route(c, REG);
    assert.match(d.announce, /^\[router\] →/);
    if (d.outcome === 'RUN') assert.ok(d.loopId);
    if (d.outcome === 'BUILD') assert.ok(d.goal);
  }
});

test('scoreMatches — ranks by tag overlap; ignores retired', () => {
  const m = scoreMatches('i18n parity loop', REG);
  assert.equal(m[0]!.loop.id, 'i18n');
  assert.ok(!m.some((x) => x.loop.id === 'retired-x'));
});
