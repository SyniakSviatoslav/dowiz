// Bebop token router — cheapest ADEQUATE model per task (repo TOKEN ROUTER rule, AGENTS.md).
//
// Ground truth: AGENTS.md "MODEL ROUTING" — classify the task, route to the cheapest model that
// is ADEQUATE, never Fable for lanes, opus only for red-line reasoning (money/auth/RLS/migrations).
// No live LLM is required for Bebop to run: the router is a PURE decision function so it is
// unit-testable without burning tokens (Verified-by-Math). The actual provider call is injected.

export type Model = 'haiku' | 'sonnet' | 'opus';

export type TaskClass =
  | 'doer' // narrow mechanical edit
  | 'reason' // design/analysis
  | 'redline'; // money/auth/RLS/migrations — must escalate

export interface RouterDecision {
  model: Model;
  rationale: string;
}

const REDLINE_TASKS: TaskClass[] = ['redline'];

export function route(task: TaskClass): RouterDecision {
  switch (task) {
    case 'redline':
      return {
        model: 'opus',
        rationale: 'Red-line task (money/auth/RLS/migrations). Escalated to opus by the routing rule.',
      };
    case 'reason':
      return { model: 'sonnet', rationale: 'Reasoning task. Sonnet is adequate; opus reserved for red-lines.' };
    case 'doer':
    default:
      return { model: 'haiku', rationale: 'Narrow doer task. Haiku is adequate — cheapest lane.' };
  }
}

// The model gate used by the loop: a red-line-class task MUST NOT be handled by haiku/sonnet.
export function enforceRouting(task: TaskClass, chosen: Model): { ok: boolean; note: string } {
  if (REDLINE_TASKS.includes(task) && chosen !== 'opus') {
    return { ok: false, note: `routing violation: ${task} must route to opus, got ${chosen}.` };
  }
  if (task === 'doer' && chosen === 'opus') {
    return { ok: false, note: `routing waste: doer routed to opus (${chosen}) — over-spend.` };
  }
  return { ok: true, note: 'route ok' };
}
