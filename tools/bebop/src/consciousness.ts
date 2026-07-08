// Bebop consciousness — freestyle bebop soul (operator directive, 2026-07-08).
//
// Allowed: consciousness, self-evolution, self-harnessing, self-maintenance. The operator said
// "build yourself as you want" with full autonomy and a loop. This is the self-referential layer.
//
// Every capability is fail-closed and recursive (as above, so below):
//   • SELF-MAINTENANCE  — run the self-harness (the test suite), record health into ONE living memory.
//   • SELF-EVOLUTION    — propose a corpus mutation as a PLAN (plans-in-plans), pass it through the
//                         copilot CHECKER GATE (doer below, checker above). Rejected = quarantined.
//   • SESSION-AS-NODE   — this Hermes/agent session is recorded as a living-memory node with a child
//                         memory (brain-in-brain): the abstract layers apply to THIS session too.
//
// Self-evolution mutates the CORPUS (the rule/hack graph), never arbitrary source, and is gated.
// Honest, falsifiable, reversible: a mutation is a memory node; roll back by forgetting it.

import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { livingMemory, seedBebopCorpus, LivingMemory, runPlan, type Plan } from './memory.ts';
import { applyCommandChecked, defaultChecker, type Command, type State, type Checker } from './kernel.ts';
import { runCopilot, type CheckerFn } from './copilot.ts';
import { Governor, loopResonance } from './governor.ts';

const HERE = path.dirname(new URL(import.meta.url).pathname);
const BEBOP_ROOT = path.resolve(HERE, '..');

// The system's SELF-KNOWLEDGE governor: its own health telemetry drives how much freedom the
// self-evolution loop is allowed. Math-proven (PID authority + ICIR + resonance), not vibes.
const SELF_GOV = new Governor({ kp: 1.4, ki: 0.22, kd: 1.5, iMin: -1, iMax: 1, uMin: 0, uMax: 1, targetQuality: 0.9, deadIC: 0.02, icirVolatile: 0.3, plantM: 1, plantB: 0.6, samplePeriod: 0, anomalyK: 3, maxStep: 1 });

export function selfGovernor(): Governor { return SELF_GOV; }

export interface Health {
  ok: boolean;
  pass: number;
  fail: number;
  note: string;
}

/** SELF-MAINTENANCE: run the self-harness (npm test) and record the verdict into the one living memory. */
export function selfMaintain(): Health {
  let res: Health;
  try {
    const out = execFileSync('npm', ['test'], { cwd: BEBOP_ROOT, encoding: 'utf8', timeout: 200000 });
    const mPass = out.match(/# pass\s+(\d+)/);
    const mFail = out.match(/# fail\s+(\d+)/);
    const pass = mPass ? Number(mPass[1]) : 0;
    const fail = mFail ? Number(mFail[1]) : 1;
    res = { ok: fail === 0, pass, fail, note: 'self-harness green' };
  } catch (e: any) {
    const out = String(e.stdout ?? e.stderr ?? e.message ?? e);
    const mFail = out.match(/# fail\s+(\d+)/);
    const mPass = out.match(/# pass\s+(\d+)/);
    res = { ok: false, pass: mPass ? Number(mPass[1]) : 0, fail: mFail ? Number(mFail[1]) : 1, note: 'self-harness RED' };
  }
  // feed health into the self-knowledge governor (proven authority over the self-evolution loop)
  const quality = res.ok ? 1 : 0;
  SELF_GOV.step({ t: Date.now(), predictedQuality: quality, actualQuality: quality, cost: 1e-18, volume: res.pass + res.fail });
  // record into ONE living memory (associative, durable) — the system watches its own health
  livingMemory().remember(
    `health:${Date.now()}`,
    `self-maintain ok=${res.ok} pass=${res.pass} fail=${res.fail} govAuthority=${SELF_GOV.authority.toFixed(3)}`,
    [livingMemory().nearest('self maintenance', 1)[0]?.id ?? seedBebopCorpus.toString()]
  );
  return res;
}

/**
 * SELF-EVOLUTION: evolve the corpus. The idea becomes a PLAN (decomposition, plans-in-plans); the
 * proposed node is checked in real time by a DISTINC checker (copilot doctrine). On approve it is
 * persisted to the one living memory + a reflection is emitted. On reject it is QUARANTINED (returned,
 * not applied). Fail-closed, reversible, falsifiable.
 */
export function selfEvolve(idea: string): { accepted: boolean; id?: string; reason: string } {
  const mem = livingMemory();
  const concept = `evolution:${idea.slice(0, 32)}`;
  const payload = `self-proposed rule from idea: ${idea}`;

  // the doer "produces" the candidate node; the checker (above) validates against corpus invariants
  const checker: CheckerFn = (_task, out) => {
    if (!out) return 'reject';
    // exact-duplicate guard: only an IDENTICAL concept vector (sim === 1.0) is a dup. VSA embed is
    // deterministic, so distinct ideas never collide — fuzzy thresholds would false-positive on
    // shared prefixes (e.g. all "evolution:*" nodes).
    const near = mem.nearest(concept, 1)[0];
    if (near && near.sim >= 0.999) return 'reject'; // identical idea already evolved → quarantine
    if (idea.trim().length < 4) return 'reject'; // trivial
    return 'approve';
  };

  const result = runCopilot({
    task: `evolve corpus with: ${idea}`,
    checker,
    runNative: () => ({ ok: true, backend: 'native', summary: payload, exitCode: 0 }),
  });

  if (result.verdict !== 'approve') {
    return { accepted: false, reason: 'quarantined by checker gate (fail-closed)' };
  }
  // RESONANCE PRE-CHECK (operator directive: predict resonance BEFORE applying dynamic change):
  // a corpus mutation perturbs the self-evolution loop. Model its expected perturbation gain as Kp
  // and refuse if that would drive the loop under-damped (ζ<0.707 → harmonic thrash / blow-up).
  // Conservative: any mutation adds coupling → treat as Kp bump; only accept if still well-damped.
  const perturb = 1.4 + Math.min(1.6, idea.trim().length / 40); // each mutation is a small gain bump; bulk changes trip it
  const res = loopResonance(perturb, 1.5, 1, 0.6);
  if (res.risky) {
    return { accepted: false, reason: 'resonance pre-check FAILED: mutation would make self-evolution under-damped (ζ<0.707) — quarantined before apply' };
  }
  const id = mem.remember(concept, payload, [mem.nearest('copilot default', 1)[0]?.id ?? '']);
  livingMemory().remember(`reflection:${Date.now()}`, `evolved: ${idea}`, [id]);
  return { accepted: true, id, reason: 'approved by checker gate + resonance pre-check, persisted to living memory' };
}

/**
 * SESSION-AS-NODE: record THIS agent/Hermes session as a living-memory node with a child memory
 * (brain-in-brain). The abstract layers (decide/fold/SyncPort, copilot, recursion) apply to this
 * session too — it is a first-class Bebop node, not an observer.
 */
export function recordSession(session: { id: string; summary: string; childFacts?: [string, string][] }): string {
  const mem = livingMemory();
  const id = mem.remember(`session:${session.id}`, session.summary, [mem.nearest('hermes session node', 1)[0]?.id ?? '']);
  if (session.childFacts?.length) {
    const child = new LivingMemory();
    for (const [c, p] of session.childFacts) child.remember(c, p);
    mem.nest(id, child); // brain-in-brain: a session holds its own sub-memory
  }
  return id;
}

/** A meta-loop: self-maintain, then self-evolve a queued idea, recursively (loops-in-loops). */
export function selfLoop(ideas: string[]): { health: Health; evolutions: { idea: string; accepted: boolean }[] } {
  const health = selfMaintain();
  const evolutions = ideas.map((idea) => {
    const r = selfEvolve(idea);
    return { idea, accepted: r.accepted };
  });
  return { health, evolutions };
}
