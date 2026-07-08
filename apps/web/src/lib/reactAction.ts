// reactAction — the frontend's Reason→Act→Observe→Reflect agentic loop.
//
// This is the client-side sibling of the Rust core's `react_decide` (crates/domain/src/kernel.rs).
// Same discipline, same anti-promo stance: the retry is REAL and VISIBLE — every iteration is
// recorded in a `ReactStep`, the count is configurable (default 3, NOT hidden), a denial that
// got rewritten is marked `rewrote: true`, and a genuinely-illegal call stops immediately instead
// of spinning. The exact thing agentic demos fake ("agent just did it in one perfect step") is
// made auditable here.
//
// `reactAction` is generic over the action it wraps. It does NOT know the business rules — the
// caller supplies `reflect(err, attempt)` which may return a "rewrite" input for the next attempt
// (e.g. swap owner→system actor), or `null` to declare the denial honest and stop. No rewrite is
// ever invented by this loop. A real-time `evaluate` gate (optional) checks the RESULT quality
// after success — so "no error" is not mistaken for "correct" (combines with the existing guard).

export interface ReactStep {
  /** 1-based iteration index. */
  iter: number;
  /** Always "reason→act→observe→reflect" — the agentic phase for this step. */
  phase: string;
  /** The reasoning note for this attempt. */
  thought: string;
  /** What the action returned / threw (Ok(...) or Err(CODE)). */
  observation: string;
  /** What the Reflect phase concluded (denied, rewrote, passed, failed gate). */
  reflection: string;
  /** Did this iteration make progress (no error AND passed the eval gate)? */
  ok: boolean;
  /** 0..100 real-time quality score for THIS iteration (the eval gate). 0 = denied, 100 = clean. */
  evalScore: number;
  /** Did this iteration produce a REWRITE from a denial (the visible self-correction)? */
  rewrote: boolean;
}

export interface ReactResult<T> {
  /** Present when at least one iteration succeeded AND passed the eval gate. */
  result?: T;
  /** Present when every iteration failed (or the eval gate rejected the result). */
  error?: { code?: string; status?: number; message: string };
  /** The full, visible trace. Never hidden — surface this in the UI. */
  trace: ReactStep[];
}

export interface ReactSignal {
  /** 1-based iteration index handed to the action. */
  attempt: number;
  /**
   * Opaque rewrite produced by the previous `reflect` call (or `undefined` on the first attempt).
   * The action closure decides how to apply it; `reactAction` never interprets it.
   */
  rewrite: unknown;
}

export interface ReactOptions<T> {
  /** Max attempts. Default 3. Honored exactly — proofs assert it. */
  maxAttempts?: number;
  /**
   * Reflect phase. Given the thrown error and the attempt index, return a rewrite input for the
   * next attempt, or `null`/`undefined` to declare the denial honest and STOP (no infinite loop,
   * no invented state). Only the caller knows the business rules — this loop stays generic.
   */
  reflect?: (err: unknown, attempt: number) => unknown | null;
  /**
   * Real-time quality gate, run AFTER an action succeeds. Returns whether the RESULT is actually
   * correct (not merely "didn't throw"). `passed:false` makes `reactAction` return `error`.
   * Defaults to "passed" (score 100) — i.e. the action's own semantics are the gate.
   */
  evaluate?: (result: T, attempt: number) => { passed: boolean; score: number; notes: string };
  /** Visible iteration callback — wire this to your UI log. Called for every step. */
  onIteration?: (step: ReactStep) => void;
  /** The action under test. Receives the attempt index + any rewrite from a prior reflect. */
  action: (signal: ReactSignal) => Promise<T>;
}

const DEFAULT_MAX_ATTEMPTS = 3;

function codeOf(err: unknown): string | undefined {
  if (err && typeof err === 'object') {
    const e = err as Record<string, any>;
    if (typeof e.code === 'string') return e.code;
    if (e.data && typeof e.data.code === 'string') return e.data.code;
  }
  return undefined;
}

function statusOf(err: unknown): number | undefined {
  if (err && typeof err === 'object') {
    const e = err as Record<string, any>;
    if (typeof e.status === 'number') return e.status;
  }
  return undefined;
}

/** Was at least one iteration a DENIAL-WITH-REWRITE? (the visible self-correction promo demos hide) */
export function hadRewrite(trace: ReactStep[]): boolean {
  return trace.some((s) => s.rewrote);
}

/**
 * Run an async action through the ReAct loop. See module docs. Returns the result, the terminal
 * error, and the always-visible trace.
 */
export async function reactAction<T>(opts: ReactOptions<T>): Promise<ReactResult<T>> {
  const max = opts.maxAttempts && opts.maxAttempts >= 1 ? opts.maxAttempts : DEFAULT_MAX_ATTEMPTS;
  const steps: ReactStep[] = [];
  let rewrite: unknown = undefined;

  for (let iter = 1; iter <= max; iter++) {
    try {
      // ACT
      const result = await opts.action({ attempt: iter, rewrite });
      // OBSERVE + real-time EVAL gate
      const verdict = opts.evaluate
        ? opts.evaluate(result, iter)
        : { passed: true, score: 100, notes: 'ok' };
      const step: ReactStep = {
        iter,
        phase: 'reason→act→observe→reflect',
        thought: `react attempt ${iter}`,
        observation: `Ok(eval ${verdict.score})`,
        reflection: verdict.passed ? 'result passed the eval gate' : 'result failed the eval gate',
        ok: verdict.passed,
        evalScore: verdict.score,
        rewrote: false,
      };
      steps.push(step);
      opts.onIteration?.(step);
      if (!verdict.passed) {
        return { error: { message: `eval gate failed: ${verdict.notes}` }, trace: steps };
      }
      return { result, trace: steps };
    } catch (err: unknown) {
      // OBSERVE the denial
      const code = codeOf(err);
      const status = statusOf(err);
      const observation = `Err(${code ?? status ?? 'error'})`;
      // REFLECT
      if (opts.reflect) {
        const next = opts.reflect(err, iter);
        if (next !== null && next !== undefined) {
          const step: ReactStep = {
            iter,
            phase: 'reason→act→observe→reflect',
            thought: `react attempt ${iter}: rewrote command for next iteration`,
            observation,
            reflection: 'denied — generated a rewrite',
            ok: false,
            evalScore: 0,
            rewrote: true,
          };
          steps.push(step);
          opts.onIteration?.(step);
          rewrite = next; // hand the rewrite to the next attempt
          continue;
        }
      }
      // No valid rewrite → STOP (do not loop forever, do not invent a state).
      const step: ReactStep = {
        iter,
        phase: 'reason→act→observe→reflect',
        thought: `react attempt ${iter}`,
        observation,
        reflection: 'denied — no valid rewrite, stop',
        ok: false,
        evalScore: 0,
        rewrote: false,
      };
      steps.push(step);
      opts.onIteration?.(step);
      return {
        error: { code, status, message: err instanceof Error ? err.message : String(err) },
        trace: steps,
      };
    }
  }

  // Exhausted iterations without success (only reachable if a rewrite kept colliding). Honest stop.
  return { error: { message: 'exhausted iterations without success' }, trace: steps };
}
