// §8 LOOP-END PROPAGATION — on loop end, the harness turns a run's patterns / issues /
// carry-forward into ADVISORY propagation directives: a memory note, a reflection
// (docs/reflections/INBOX), and a checklist of OTHER loops / agents / docs / guardrails
// to update so a lesson learned in one loop doesn't have to be re-learned elsewhere.
//
// Doctrine: the harness EMITS; the worker / librarian ENACTS. It never auto-edits other
// loops/agents/docs (that would be a loop mutating sibling surfaces unreviewed) — it
// produces the directives + a reflection stub, exactly like the self-improvement loop.

import type { RunRecord } from './types.js';

export type PropagationKind = 'memory' | 'guardrail' | 'loop' | 'agent' | 'doc';
export interface PropagationTarget { kind: PropagationKind; what: string; why: string }
export interface Propagation {
  memory_note: string;
  reflection: string;
  targets: PropagationTarget[];
}

/** Derive the propagation directives deterministically from the run record. */
export function buildPropagation(r: RunRecord): Propagation {
  const targets: PropagationTarget[] = [];
  const recurring = r.patterns.filter((p) => /recurring/i.test(p));

  // A recurring pattern is a systemic root → promote to a deterministic guardrail.
  for (const p of recurring) {
    targets.push({ kind: 'guardrail', what: `promote → guardrail (red→green): ${p.replace(/^recurring:?\s*/i, '')}`, why: 'recurring across this run — a lesson must become a gate so it stops recurring' });
  }
  // Surfaced issues + any learnings → a durable memory note (cross-session recall).
  if (r.issues.length || r.patterns.length) {
    targets.push({ kind: 'memory', what: `write/update a memory note for "${r.loop}" with this run's issues + learnings`, why: 'so the next session does not re-discover them' });
  }
  // Carry-forward → the surfaces that should change before the next run.
  for (const g of r.carry_forward.guards) targets.push({ kind: 'guardrail', what: g, why: 'carry-forward guard from this run' });
  for (const w of r.carry_forward.watch) targets.push({ kind: 'doc', what: `document/track watch-item: ${w}`, why: 'carry-forward watch from this run' });
  // A red-line issue → propagate the lesson to sibling loops + agents, not just this one.
  if (r.issues.some((i) => /auth|rls|tenant|secret|money|pii|payment/i.test(i))) {
    targets.push({ kind: 'agent', what: 'update the security/invariant reviewer agents with this red-line finding', why: 'a red-line lesson must reach every loop/agent that touches that surface' });
    targets.push({ kind: 'loop', what: 'add this red-line check to sibling loops with the same scope', why: 'prevent the same class slipping through a different loop' });
  }

  const memory_note =
    `${r.loop} run #${r.run_index} (${r.outcome}): ${r.what_done}` +
    (r.issues.length ? ` · ISSUES: ${r.issues.join('; ')}` : '') +
    (r.patterns.length ? ` · PATTERNS: ${r.patterns.join('; ')}` : '');

  const reflection = [
    `# Reflection — ${r.loop} #${r.run_index} (${r.t_end})`,
    '',
    `**WHAT:** ${r.what_done}`,
    `**OUTCOME:** ${r.outcome}${r.breaker_reason ? ` (${r.breaker_reason})` : ''}`,
    '',
    '**WHY (causal root — fill in):** <why did the issues/patterns arise, not just where>',
    '',
    '**ISSUES:**',
    ...(r.issues.length ? r.issues.map((i) => `- ${i}`) : ['- (none)']),
    '',
    '**PROPAGATE TO:**',
    ...targets.map((t) => `- [${t.kind}] ${t.what} — ${t.why}`),
    '',
    `_Advisory: the worker/librarian enacts; do not auto-edit sibling surfaces._`,
  ].join('\n');

  return { memory_note, reflection, targets };
}

/** §8 block appended to the printed report. */
export function renderPropagation(p: Propagation): string {
  const L: string[] = [];
  L.push('8. LOOP-END PROPAGATION (advisory — worker/librarian enacts)');
  L.push(`   MEMORY  ${p.memory_note}`);
  if (p.targets.length) for (const t of p.targets) L.push(`   → [${t.kind}] ${t.what}`);
  else L.push('   → (nothing to propagate)');
  return L.join('\n');
}
