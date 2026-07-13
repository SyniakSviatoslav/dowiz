// agent-governance/resonator.ts
//
// TypeScript port of bebop2/core/src/resonator.rs — the deterministic closed-loop
// feedback controller (the Gortai electrical analogy applied: ground / fuse / drift /
// rollback). See /root/bebop-repo/docs/GORTAI-ANALOGY.md for the full mapping.
//
// This module is the HIGH-LEVEL orchestrator that wires the governance math core
// (drift detector in index.ts, etc.) into one controlled feedback loop. It does NOT
// re-implement Lyapunov/SMC — those live in bebop2. It provides:
//   • immutable Reference (the electrical "ground")
//   • three pluggable actors: Generator (source), Reflector (transformer + quality),
//     Supervisor (circuit breaker)
//   • Δ-threshold convergence, max-iteration fuse, stall patience, drift accumulator,
//     Lyapunov chaos watchdog (freeze on divergence), rollback-to-best.
//
// Zero deps. Offline. Deterministic. Pure functions + one stateful run().

export type Termination = 'Converged' | 'Fused' | 'Stalled';

export interface LoopConfig {
  max_iterations: number;   // the fuse
  delta_threshold: number;  // ε — convergence band
  stall_patience: number;   // ticks of no-improvement (with weak reflector) before Stalled
  lyapunov_guard: boolean;  // chaos watchdog: freeze on divergent step
}

export interface Reference<S> {
  value: S;
}

export interface Checkpoint<S> {
  state: S;
  error: number;
  quality: number;
}

export interface ResonatorResult<S> {
  final_state: S;
  final_error: number;
  termination: Termination;
  total_drift: number;       // Σ |step| — the "current that flowed the wrong way"
  checkpoints: Checkpoint<S>[];
}

/**
 * Metric: distance between a candidate state and the immutable reference.
 * Must be ≥ 0 and finite. Smaller = closer to ground.
 */
export interface Metric<S> {
  distance(candidate: S, reference: S): number;
}

export interface Actors<S> {
  generate: (current: S) => S;
  reflect: (proposed: S, reference: S) => { refined: S; quality: number };
  supervise: (refined: S, reference: S, quality: number) => boolean;
}

// ── built-in metric ────────────────────────────────────────────────────────
/** L2 (Euclidean) metric over number[] — the default. */
export const L2Metric: Metric<number[]> = {
  distance(a: number[], b: number[]): number {
    let s = 0;
    const n = Math.min(a.length, b.length);
    for (let i = 0; i < n; i++) {
      const d = a[i] - b[i];
      s += d * d;
    }
    return Math.sqrt(s);
  },
};

function cloneNumberArray(s: number[]): number[] {
  return s.slice();
}

/**
 * Run the resonator loop.
 *
 * Tick: generate → reflect → supervise → commit/hold → measure → check fuse.
 * If `lyapunov_guard` is on and a committed step would INCREASE error (diverge),
 * the chaos watchdog FREEZES adaptation: keep the previous (ground-closer) state,
 * record zero drift for that tick, and continue. This is the overload protection.
 */
export function runResonator<S>(
  reference: Reference<S>,
  initial: S,
  actors: Actors<S>,
  metric: Metric<S>,
  config: LoopConfig,
  clone: (s: S) => S,
): ResonatorResult<S> {
  let current = clone(initial);
  let error = metric.distance(current, reference.value);
  let bestErr = error;
  let stallCount = 0;
  let totalDrift = 0;
  let termination: Termination = 'Fused';
  const checkpoints: Checkpoint<S>[] = [];

  for (let i = 0; i < config.max_iterations; i++) {
    const proposed = actors.generate(current);
    const { refined, quality } = actors.reflect(proposed, reference.value);

    // chaos watchdog: if lyapunov_guard on and the step is unstable (moves away from ground),
    // freeze (deny commit). Mirrors bebop2 stabilizer::stabilize_step fail-closed spirit.
    let allowed = actors.supervise(refined, reference.value, quality);
    if (config.lyapunov_guard && allowed) {
      const nextErr = metric.distance(refined, reference.value);
      if (nextErr > error + 1e-9) {
        allowed = false; // moving away from ground ⇒ unstable step ⇒ freeze
      }
    }

    const committedState = allowed ? clone(refined) : clone(current);
    const committed = allowed;
    const newError = metric.distance(committedState, reference.value);
    // drift: accumulated change in error (the "current that flowed the wrong way").
    totalDrift += Math.abs(newError - error);

    if (newError < bestErr) {
      bestErr = newError;
      stallCount = 0;
    } else {
      stallCount += 1;
    }

    checkpoints.push({
      state: clone(committedState),
      error: newError,
      quality: committed ? quality : 0.0,
    });

    current = committedState;
    error = newError;

    if (error < config.delta_threshold) {
      termination = 'Converged';
      break;
    }
    // Stall only when NOT improving AND the reflector is weak (low quality). A
    // high-quality plateau near convergence is NOT a stall — it is (almost) resonated.
    if (stallCount >= config.stall_patience && quality < 0.5) {
      termination = 'Stalled';
      break;
    }
  }

  return {
    final_state: clone(current),
    final_error: error,
    termination,
    total_drift: totalDrift,
    checkpoints,
  };
}

/** Rollback to the lowest-error checkpoint (idempotent with final_state when Converged). */
export function rollbackToBest<S>(result: ResonatorResult<S>): Checkpoint<S> {
  let best = result.checkpoints[0];
  for (const cp of result.checkpoints) {
    if (cp.error < best.error) best = cp;
  }
  return best;
}

// Re-export the number[] clone for callers that use the default L2 metric.
export const defaultClone = cloneNumberArray;
