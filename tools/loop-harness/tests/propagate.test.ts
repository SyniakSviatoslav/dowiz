import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildPropagation, renderPropagation } from '../src/propagate.js';
import type { RunRecord } from '../src/types.js';

function rec(over: Partial<RunRecord> = {}): RunRecord {
  return {
    loop: 'test-hardening', run_index: 3, outcome: 'natural_stop', breaker_reason: null,
    iter_from: 1, iter_to: 1, t_start: 'T0', t_end: 'T1', wall_s: 10,
    goal: 'g', what_done: 'scanned 245 files',
    issues: ['217 CRITICAL false-greens', 'RLS isolation tested with a nil UUID'],
    patterns: ['recurring: tautological assertions', 'recurring: body.length-as-render-proof'],
    telemetry: {} as any,
    carry_forward: { guards: ['activate permissive-status rule'], watch: ['red-line money/RLS proofs'] },
    ...over,
  };
}

test('buildPropagation — recurring patterns become guardrail directives', () => {
  const p = buildPropagation(rec());
  assert.ok(p.targets.some((t) => t.kind === 'guardrail' && /tautological/.test(t.what)));
  assert.ok(p.targets.some((t) => t.kind === 'guardrail' && /permissive-status/.test(t.what)));
});

test('buildPropagation — a red-line issue propagates to agents + sibling loops', () => {
  const p = buildPropagation(rec());
  assert.ok(p.targets.some((t) => t.kind === 'agent'), 'red-line → agent update');
  assert.ok(p.targets.some((t) => t.kind === 'loop'), 'red-line → sibling loops');
});

test('buildPropagation — emits a memory directive + a non-empty memory note', () => {
  const p = buildPropagation(rec());
  assert.ok(p.targets.some((t) => t.kind === 'memory'));
  assert.match(p.memory_note, /test-hardening run #3/);
  assert.match(p.memory_note, /ISSUES/);
});

test('buildPropagation — reflection has a WHY prompt + a propagate-to list', () => {
  const p = buildPropagation(rec());
  assert.match(p.reflection, /WHY \(causal root/);
  assert.match(p.reflection, /PROPAGATE TO:/);
});

test('buildPropagation — a clean run with no issues/patterns has minimal targets', () => {
  const p = buildPropagation(rec({ issues: [], patterns: [], carry_forward: { guards: [], watch: [] } }));
  assert.equal(p.targets.length, 0);
});

test('renderPropagation — produces the §8 block', () => {
  const out = renderPropagation(buildPropagation(rec()));
  assert.match(out, /8\. LOOP-END PROPAGATION/);
  assert.match(out, /→ \[guardrail\]/);
});
