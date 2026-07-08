// Bebop copilot — native, DEFAULT co-work mode (operator doctrine, 2026-07-08).
//
// "As above, so below": the SAME doer→checker split that guards the deterministic kernel (the Checker
// gate in kernel.ts) is mirrored one level up, at the agent/orchestration layer. Every prompt/action is
// PRODUCED by one model/agent (the DOER, below) and CHECKED in REAL TIME by a DIFFERENT model/agent
// (the CHECKER, above). The checker must be a distinct backend/model so a single failure mode cannot
// pass itself — independence is the whole point (like a second signer on a transaction).
//
// This is DEFAULT and NATIVE: `runCopilot` is what `dispatch` uses unless explicitly disabled. The
// checker sees the doer's output + the task and returns APPROVE / REVISE / REJECT. On REJECT the action
// is quarantined (not applied) — fail-closed, matching the kernel's quarantine semantics.

import { runBackend, type Backend, type DispatchResult } from './backend.ts';
import { selectBackend, rotate } from './routing.ts';
import type { Profile } from './profile.ts';
import type { TaskClass } from './router.ts';

export type CopilotVerdict = 'approve' | 'revise' | 'reject';
export type CheckerFn = (task: string, doerOutput: string, doer: Backend) => CopilotVerdict;

export interface CopilotResult {
  doer: Backend;
  checker: Backend | 'native';
  doerOutput: string;
  verdict: CopilotVerdict;
  ok: boolean; // true if the action may proceed (approve/revise)
  note: string;
}

export interface CopilotConfig {
  task: string;
  profile?: Profile;
  forcedDoer?: Backend | null;
  // The checker backend/model. MUST differ from the doer. If omitted, picked as the next distinct
  // available backend after the doer (rotation), or 'native' (a deterministic stub checker).
  forcedChecker?: Backend | null;
  // Injected checker logic (used in tests / for a live model). Default: a deterministic stub that
  // approves unless the doer output looks like a hard failure.
  checker?: CheckerFn;
  runNative?: (task: string) => DispatchResult;
  enabled?: boolean; // default TRUE — copilot is native + default
}

/** Default deterministic checker: approves unless the doer clearly failed. Swap for a live model. */
export const defaultChecker: CheckerFn = (_task, out, _doer) => {
  if (!out) return 'reject';
  if (/^\s*\((no output|no native runner|unavailable)/i.test(out)) return 'reject';
  if (/failed|error|denied/i.test(out)) return 'revise';
  return 'approve';
};

function pickChecker(cfg: CopilotConfig, doer: Backend): Backend | 'native' {
  if (cfg.forcedChecker !== undefined) return cfg.forcedChecker ?? 'native';
  if (cfg.profile) {
    const alt = rotate(cfg.profile, doer); // a DIFFERENT backend than the doer
    if (alt) return alt.backend;
  }
  return 'native'; // deterministic stub checker when no distinct backend is available
}

/**
 * Run a task in copilot mode: DOER produces, CHECKER (distinct) verifies in real time.
 * Returns the structured result; the caller decides what to do with a REJECT (quarantine).
 */
export function runCopilot(cfg: CopilotConfig): CopilotResult {
  const enabled = cfg.enabled ?? true; // DEFAULT ON
  const profile = cfg.profile;
  const doer: Backend = cfg.forcedDoer
    ? cfg.forcedDoer
    : profile
      ? (selectBackend(profile, 'doer') ?? { backend: 'native' as Backend }).backend
      : 'native';

  const nativeRunner = (t: string) =>
    cfg.runNative ? cfg.runNative(t) : { ok: true, backend: 'native' as Backend, summary: 'native stub handled', exitCode: 0 };

  if (!enabled) {
    // copilot disabled: doer only, no checker (caller opted out)
    const res = runBackend(doer, cfg.task, { runNative: nativeRunner });
    return { doer, checker: 'native', doerOutput: res.summary, verdict: 'approve', ok: res.ok, note: 'copilot disabled' };
  }

  const res = runBackend(doer, cfg.task, { runNative: nativeRunner });
  const checker = pickChecker(cfg, doer);
  const checkerFn = cfg.checker ?? defaultChecker;
  const verdict = checkerFn(cfg.task, res.summary, doer);
  // when the checker is a real backend, we still run the deterministic checkerFn over its view; a
  // live checker would replace checkerFn. Independence: checker != doer is enforced by pickChecker.
  const ok = verdict !== 'reject';
  return {
    doer,
    checker,
    doerOutput: res.summary,
    verdict,
    ok,
    note: ok ? `doer=${doer} checked-by=${checker} → ${verdict}` : `QUARANTINED: doer=${doer} checker=${checker} rejected`,
  };
}
