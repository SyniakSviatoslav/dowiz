// agent-governance/resonator.test.ts
//
// RED+GREEN node:test parity with bebop2/core/src/resonator.rs. Runs via `npx tsx --test`.
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  runResonator,
  rollbackToBest,
  L2Metric,
  defaultClone,
  type Actors,
  type LoopConfig,
  type Reference,
  type ResonatorResult,
} from './resonator.ts';

function makeActors(
  generate: Actors<number[]>['generate'],
  reflect: Actors<number[]>['reflect'],
  supervise: Actors<number[]>['supervise'],
): Actors<number[]> {
  return { generate, reflect, supervise };
}

test('converging loop resonates under epsilon', () => {
  const reference: Reference<number[]> = { value: [0.0, 0.0, 0.0] };
  const initial = [10.0, 20.0, 30.0];
  const actors = makeActors(
    (s) => s.map((x) => x * 0.9),
    (proposed, refv) => {
      const refined = proposed.map((p, i) => p + 0.99999 * (refv[i] - p));
      const err = Math.sqrt(refined.reduce((a, p, i) => a + (p - refv[i]) ** 2, 0));
      const quality = Math.min(1.0, 1.0 / (1.0 + err));
      return { refined, quality };
    },
    () => true,
  );
  const cfg: LoopConfig = {
    max_iterations: 1000,
    delta_threshold: 1e-6,
    stall_patience: 50,
    lyapunov_guard: true,
  };
  const res = runResonator(reference, initial, actors, L2Metric, cfg, defaultClone);
  assert.equal(res.termination, 'Converged');
  assert.ok(res.final_error < 1e-6, `final error tiny, got ${res.final_error}`);
  assert.ok(res.checkpoints.length < cfg.max_iterations, 'stopped early, not fused');
  const best = rollbackToBest(res);
  assert.ok(Math.abs(best.error - res.final_error) < 1e-12);
});

test('runaway loop frozen by lyapunov guard', () => {
  const reference: Reference<number[]> = { value: [0.0, 0.0] };
  const initial = [1.0, 1.0];
  const actors = makeActors(
    (s) => s.map((x) => x * 1.5),
    (proposed) => ({ refined: proposed.slice(), quality: 0.9 }),
    () => true,
  );
  const cfg: LoopConfig = {
    max_iterations: 32,
    delta_threshold: 1e-9,
    stall_patience: 100,
    lyapunov_guard: true,
  };
  const res = runResonator(reference, initial, actors, L2Metric, cfg, defaultClone);
  assert.notEqual(res.termination, 'Converged');
  assert.ok(Number.isFinite(res.final_error), 'no NaN/inf blowup');
  assert.ok(res.total_drift < 1e-9, `guard froze motion, drift ~0, got ${res.total_drift}`);
});

test('runaway loop blows fuse when guard off', () => {
  const reference: Reference<number[]> = { value: [0.0, 0.0] };
  const initial = [1.0, 1.0];
  const actors = makeActors(
    (s) => s.map((x) => x * 1.5),
    (proposed) => ({ refined: proposed.slice(), quality: 0.9 }),
    () => true,
  );
  const cfg: LoopConfig = {
    max_iterations: 32,
    delta_threshold: 1e-9,
    stall_patience: 100,
    lyapunov_guard: false,
  };
  const res = runResonator(reference, initial, actors, L2Metric, cfg, defaultClone);
  assert.notEqual(res.termination, 'Converged');
  assert.equal(res.termination, 'Fused', `must hit the fuse, got ${res.termination}`);
  assert.ok(res.total_drift > 0.0, `divergence produced drift, got ${res.total_drift}`);
});

test('reference re-injection prevents drift', () => {
  const reference: Reference<number[]> = { value: [2.0, -1.0] };
  const initial = [2.0, -1.0];
  // Reflector re-injects the reference every tick and ignores the (weak) generator perturbation,
  // so the error stays bounded below ε and drift stays tiny.
  const actors = makeActors(
    (s) => s.map((x) => x + 0.001), // weak generator perturbation
    (proposed, refv) => {
      const refined = refv.slice();
      const err = Math.sqrt(refined.reduce((a, p, i) => a + (p - refv[i]) ** 2, 0));
      return { refined, quality: 1.0 / (1.0 + err) };
    },
    () => true,
  );
  const cfg: LoopConfig = {
    max_iterations: 100,
    delta_threshold: 1e-6,
    stall_patience: 50,
    lyapunov_guard: true,
  };
  const res = runResonator(reference, initial, actors, L2Metric, cfg, defaultClone);
  assert.equal(res.termination, 'Converged');
  assert.ok(res.total_drift < 0.1, `re-injection keeps drift tiny, got ${res.total_drift}`);
});

test('rollback returns best checkpoint', () => {
  const reference: Reference<number[]> = { value: [0.0, 0.0] };
  const initial = [2.0, 2.0];
  const actors = makeActors(
    (s) => s.map((x) => x * 0.5), // converges toward 0
    (proposed, refv) => {
      const refined = proposed.map((p, i) => p + 0.999 * (refv[i] - p));
      const err = Math.sqrt(refined.reduce((a, p, i) => a + (p - refv[i]) ** 2, 0));
      return { refined, quality: 1.0 / (1.0 + err) };
    },
    () => true,
  );
  const cfg: LoopConfig = {
    max_iterations: 50,
    delta_threshold: 1e-9,
    stall_patience: 100,
    lyapunov_guard: true,
  };
  const res = runResonator(reference, initial, actors, L2Metric, cfg, defaultClone);
  const best = rollbackToBest(res);
  assert.ok(best.error <= 0.5 + 1e-9, `rollback beats start, got ${best.error}`);
  assert.equal(best.error, res.final_error);
});

test('guard off still diverges (no magic convergence)', () => {
  const reference: Reference<number[]> = { value: [0.0, 0.0] };
  const initial = [1.0, 1.0];
  const actors = makeActors(
    (s) => s.map((x) => x * 2.0), // diverges
    (proposed) => ({ refined: proposed.slice(), quality: 0.9 }),
    () => true,
  );
  const cfg: LoopConfig = {
    max_iterations: 16,
    delta_threshold: 1e-12,
    stall_patience: 100,
    lyapunov_guard: false,
  };
  const res = runResonator(reference, initial, actors, L2Metric, cfg, defaultClone);
  assert.equal(res.termination, 'Fused');
  assert.notEqual(res.termination, 'Converged');
  assert.ok(res.total_drift > 0.0);
});
