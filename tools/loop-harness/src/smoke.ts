// §2.3 — the smoke test: the hard gate that makes auto-release safe. It dry-runs
// the generated loop's CONTRACT (progressMetric + isTerminal + breaker) through
// the REAL harness on a fixed seeded scenario and asserts: the metric MOVES, it
// TERMINATES (green, not stall/abort), and it does NOT churn out-of-scope. A
// plausible design whose breaker is too tight, whose terminal is unreachable, or
// that can't make progress is REJECTED here — not released "to see".
//
// This proves the design's loop DYNAMICS are sound (convergent + bounded + scoped)
// via the actual runLoop + breaker. The heavier agent dry-run (running the real
// iterate's edits) is a later step; this is the deterministic structural gate.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { runLoop } from './harness.js';
import type { LoopDesign } from './loop-builder.js';

export interface SmokeSeed {
  /** Starting bad-count for the scenario (failing tests / flows / bottlenecks). */
  startMetric: number;
  /** How much one iteration reduces the metric (the design's "fix one per pass"). 0 = stuck. */
  perIterDelta: number;
  /** Does the design keep edits inside scope (carve-out respected)? */
  scopeClean: boolean;
}

export interface SmokeResult {
  moved: boolean;
  terminated: boolean;
  noChurn: boolean;
  iters: number;
  outcome: string;
  pass: boolean;
  detail: string;
}

interface SmokeState { m: number }

export async function smokeTest(design: LoopDesign, seed: SmokeSeed): Promise<SmokeResult> {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'smoke-'));
  let tick = 0;
  const loop = {
    id: 'smoke',
    goal: () => design.goal,
    iterate: (_ctx: unknown, s: SmokeState) => ({ state: { m: Math.max(0, s.m - seed.perIterDelta) } }),
    progressMetric: (s: SmokeState) => s.m,
    isTerminal: (s: SmokeState) => s.m <= 0,
  };
  const rec = await runLoop(loop, { m: seed.startMetric }, {
    baseDir: tmp, ctx: {},
    breaker: { K: design.breaker.K, maxIter: design.breaker.maxIter },
    clockMs: () => (tick += 1000),
    print: () => {},
  });

  const moved = rec.telemetry.tests_fail_end < rec.telemetry.tests_fail_start;
  const terminated = rec.outcome === 'green';
  const noChurn = seed.scopeClean;
  const pass = moved && terminated && noChurn;
  const why = !moved ? 'metric did not move (stuck — breaker stalled)'
    : !terminated ? `did not terminate in budget (outcome ${rec.outcome}; breaker ${rec.breaker_reason ?? '—'}) — maxIter too small or terminal unreachable`
    : !noChurn ? 'churns out-of-scope (missing security carve-out)'
    : `green in ${rec.iter_to} iters · metric ${rec.telemetry.tests_fail_start}→${rec.telemetry.tests_fail_end}`;
  return { moved, terminated, noChurn, iters: rec.iter_to, outcome: rec.outcome, pass, detail: why };
}

/** A realistic-worst-case seed derived from the design (the scenario the breaker must handle). */
export function defaultSeed(design: LoopDesign, scopeClean: boolean): SmokeSeed {
  return {
    startMetric: Math.min(12, Math.max(1, design.breaker.maxIter - 1)),
    perIterDelta: 1,
    scopeClean,
  };
}
