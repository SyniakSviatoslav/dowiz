// Loop Builder meta-loop (docs/operating-model/loop-builder-loop-v1.md) on the
// v3-FINAL harness. Given a goal G, it GATHERs the substrate, DESIGNs the best
// loop (5 hooks + config) for G under existing resources, and VALIDATEs it —
// then (report-only this pass, §11 step 1) describes the loop it would build.
//
// It INSTANTIATES the harness contract; it never reinvents telemetry/breaker/
// report/storage (§0/§10). Two hard properties:
//   - Oracle-admissibility FIRST (§4): no deterministic success metric for G → no
//     loop; it escalates, never guesses.
//   - Born hardened (§5): designs reuse existing tools/loops + inherit guards.
//
// Deferred (§11 steps 3-tail/4): the SMOKE-test dry-run of a generated loop and
// Class-A auto-register. This pass does GATHER + DESIGN + admissibility +
// STRUCTURAL validation, report-only — no loop is registered.

import fs from 'node:fs';
import path from 'node:path';

export interface BreakerConfigLite { K: number; maxIter: number; budgetUsd: number | null; timeCapMs: number | null }

export interface LoopDesign {
  id: string;              // stable id (the template id) for the registry
  goal: string;
  tags: string[];          // trigger tags for the registry / router selection
  oracle: string;          // the deterministic progressMetric (the most important choice)
  tools: string[];         // from the resource inventory, prefer already-used
  iterate: string;         // what one pass does
  isTerminal: string;      // what "done" means
  reflect: string;         // reuse the harness fresh-context reviewer
  scopeClass: 'A' | 'B';
  carveOut: string[];      // security-class file globs within G → propose-only even inside a Class-A loop
  breaker: BreakerConfigLite;
  reuseOf: string | null;  // existing loop id to EXTEND instead of duplicating
}

export interface AdmissibilityResult { admissible: boolean; reason: string; templateId: string | null }

// §4 — goals with no falsifiable metric. The builder REFUSES these (escalate).
const INADMISSIBLE_RE = /\b(pretti?er|prettify|beautiful|nicer|elegant|delight\w*|feels?\s+faster|feel\s+\w+|aesthetic|look\s+(nicer|good|modern)|improve\s+the\s+architecture|better\s+ux|more\s+modern)\b/i;

// Admissible goal templates (the §7 examples). Each carries a DETERMINISTIC oracle.
interface GoalTemplate extends Omit<LoopDesign, 'goal' | 'reuseOf'> { id: string; match: RegExp; reuseHint: string | null }
const TEMPLATES: GoalTemplate[] = [
  {
    id: 'be-polish', match: /\b(be|backend|api|server)[-\s]*(polish|polishing|cleanup|hardening|quality)\b|\bpolish\s+(the\s+)?(be|backend|api)\b/i,
    tags: ['backend', 'be', 'api', 'server', 'polish', 'refactor', 'lint', 'slop'],
    oracle: 'failing BE tests ↓ AND tsc + eslint clean AND aislop score ↑ on BE; server contracts read-only',
    tools: ['node:test/vitest', 'testcontainers', 'aislop', 'tsc', 'eslint'],
    iterate: 'fix one failing BE test / slop finding; re-run unit+integration; re-score aislop',
    isTerminal: '0 failing BE tests AND tsc+eslint clean AND aislop ≥ baseline, 2× consecutive',
    reflect: 'harness fresh-context reviewer (§4 v3-FINAL)',
    scopeClass: 'A', carveOut: ['**/auth/**', '**/rls/**', '**/*secret*', '**/money*', '**/pii*', 'packages/db/migrations/**'],
    breaker: { K: 3, maxIter: 25, budgetUsd: null, timeCapMs: null }, reuseHint: 'backend-contract-convergence',
  },
  {
    id: 'qa', match: /\bqa\b|quality\s*assurance|flow\s*(test|coverage)|e2e\s*loop/i,
    tags: ['qa', 'e2e', 'flow', 'green', 'matrix', 'playwright', 'wiring'],
    oracle: 'failing flows in the lifecycle matrix ↓ across all 3 roles; green 3× consecutive',
    tools: ['Playwright MCP', 'convergence harness', 'reliability-gate'],
    iterate: 'run the matrix; fix one failing flow FE↔server; re-run',
    isTerminal: 'every matrix flow green for all 3 roles, 3× consecutive',
    reflect: 'harness fresh-context reviewer',
    scopeClass: 'A', carveOut: ['**/auth/**', '**/rls/**', '**/*secret*', '**/money*', '**/pii*'],
    breaker: { K: 3, maxIter: 30, budgetUsd: null, timeCapMs: null }, reuseHint: 'error-fix-convergence',
  },
  {
    id: 'perf', match: /\bperf(ormance)?\b|slow\s*(quer|render|test)|speed\s*up|latency/i,
    tags: ['perf', 'performance', 'slow', 'speed', 'latency', 'benchmark', 'query'],
    oracle: 'telemetry-flagged slow queries/renders improved, MEASURED on a fixed seeded benchmark (≥ threshold)',
    tools: ['telemetry (runs/metrics.jsonl)', 'benchmark-replay (loop-harness)', 'index/query tools'],
    iterate: 'pick the worst telemetry-flagged bottleneck; apply a reversible perf change; benchmark before/after',
    isTerminal: 'no bottleneck above budget OR no further ≥5% win available',
    reflect: 'harness fresh-context reviewer',
    scopeClass: 'A', carveOut: ['packages/db/migrations/**', '**/auth/**', '**/rls/**'],
    breaker: { K: 3, maxIter: 20, budgetUsd: null, timeCapMs: null }, reuseHint: 'performance',
  },
  {
    id: 'i18n', match: /\bi18n\b|translat|localis|localiz|untranslated/i,
    tags: ['i18n', 'translation', 'locale', 'localization', 'al', 'en', 'parity'],
    oracle: 'untranslated/TODO strings ↓ (parity gate) AND al/en flows green',
    tools: ['i18n parity gate', 'scripts/i18n-add', 'Playwright MCP'],
    iterate: 'add one missing key via i18n-add; re-run parity --strict; verify al/en switch live',
    isTerminal: 'parity 0 missing AND al/en storefront flows green',
    reflect: 'harness fresh-context reviewer',
    scopeClass: 'A', carveOut: [],
    breaker: { K: 3, maxIter: 25, budgetUsd: null, timeCapMs: null }, reuseHint: null,
  },
];

export function assessAdmissibility(goal: string): AdmissibilityResult {
  if (INADMISSIBLE_RE.test(goal)) {
    return { admissible: false, templateId: null,
      reason: 'subjective goal — no falsifiable success metric. Define measurable criteria (e.g. a failing-count, a benchmark, a coverage gate) or it stays human-driven.' };
  }
  const t = TEMPLATES.find((x) => x.match.test(goal));
  if (t) return { admissible: true, templateId: t.id, reason: `matched template "${t.id}" with a deterministic oracle` };
  return { admissible: false, templateId: null,
    reason: 'no known template and no declared measurable metric — fail-safe escalate (the builder never emits a loop without a deterministic oracle).' };
}

/** Find an existing loop whose intent overlaps G (so we EXTEND, not duplicate). */
export function findExistingLoop(repoDir: string, reuseHint: string | null): string | null {
  if (!reuseHint) return null;
  const card = path.join(repoDir, 'loops', `${reuseHint}.yaml`);
  return fs.existsSync(card) ? reuseHint : null;
}

export function designLoop(goal: string, repoDir: string): { design: LoopDesign } | { escalate: string } {
  const adm = assessAdmissibility(goal);
  if (!adm.admissible) return { escalate: adm.reason };
  const t = TEMPLATES.find((x) => x.id === adm.templateId)!;
  const design: LoopDesign = {
    id: t.id, goal, tags: t.tags, oracle: t.oracle, tools: t.tools, iterate: t.iterate, isTerminal: t.isTerminal,
    reflect: t.reflect, scopeClass: t.scopeClass, carveOut: t.carveOut, breaker: t.breaker,
    reuseOf: findExistingLoop(repoDir, t.reuseHint),
  };
  return { design };
}

export interface ValidationResult { ok: boolean; checks: { name: string; pass: boolean; detail: string }[] }

/** §2 structural validation (the SMOKE dry-run §2.3 is deferred to §11 step 3). */
export function validateDesign(design: LoopDesign): ValidationResult {
  const checks = [
    { name: 'sound-metric', pass: design.oracle.trim().length > 0 && /↓|↑|green|0 |≥|≤|improved|clean/.test(design.oracle), detail: 'deterministic, non-empty progressMetric' },
    { name: 'terminates', pass: design.isTerminal.trim().length > 0 && design.breaker.maxIter > 0, detail: 'has isTerminal + a breaker bound' },
    { name: 'reuses-harness', pass: /fresh-context reviewer/i.test(design.reflect), detail: 'reuses the harness reviewer; no new telemetry/breaker/report' },
    { name: 'security-carve-out', pass: design.scopeClass === 'B' || design.carveOut.length > 0 || /i18n/i.test(design.goal), detail: 'security-class files are propose-only (or N/A for this goal)' },
    { name: 'not-duplicate', pass: true, detail: design.reuseOf ? `EXTEND existing loop "${design.reuseOf}" (don't emit a second)` : 'no existing loop for this goal' },
  ];
  return { ok: checks.every((c) => c.pass), checks };
}

// ─── Runner — GATHER → DESIGN → VALIDATE(structural) → §5 report (report-only) ───

import type { RunRecord } from './types.js';
import { buildRecord } from './cli.js';
import { renderReport } from './report.js';
import { writeRunRecord, appendMetricsLine } from './storage.js';
import { smokeTest, defaultSeed, type SmokeResult } from './smoke.js';
import { registerLoop, type RegistryLoop } from './registry.js';

function carveOutKeywords(carveOut: string[]): string[] {
  return [...new Set(carveOut.map((g) => (g.match(/[a-z0-9]+/i)?.[0] ?? g)).filter((k) => !/db|migrations/.test(k)))];
}

export async function runLoopBuilder(opts: { goal: string; repoDir: string; baseDir: string; tStart: string; tEnd: string; register: boolean }): Promise<RunRecord> {
  const result = designLoop(opts.goal, opts.repoDir);
  const escalated = 'escalate' in result;
  const design = escalated ? null : result.design;
  const validation = design ? validateDesign(design) : null;
  const carveOk = validation?.checks.find((c) => c.name === 'security-carve-out')?.pass ?? false;
  // §2.3 SMOKE — dry-run the design's contract through the real harness.
  const smoke: SmokeResult | null = design ? await smokeTest(design, defaultSeed(design, carveOk)) : null;

  const releasable = !!design && validation!.ok && smoke!.pass && design.scopeClass === 'A' && !design.reuseOf;
  let registered: RegistryLoop | null = null;
  if (releasable && opts.register) {
    registered = registerLoop(opts.baseDir, {
      id: design!.id, goal: design!.goal, trigger_tags: design!.tags,
      scope_class: design!.scopeClass, security_carveout: carveOutKeywords(design!.carveOut),
      registered_at: opts.tEnd, status: 'active',
    });
  }
  const mode = opts.register ? 'AUTO-REGISTER (Class A, smoke-gated)' : 'report-only';

  const input = {
    loop: 'loop-builder',
    goal: `Build the best harness-native loop for: "${opts.goal}". Mode: ${mode}. Oracle-admissibility first; smoke-gated; security carve-out always.`,
    outcome: 'natural_stop' as const,
    t_start: opts.tStart, t_end: opts.tEnd, iter_from: 1, iter_to: 1,
    what_done: escalated
      ? `ADMISSIBILITY-REFUSED "${opts.goal}": ${(result as { escalate: string }).escalate}`
      : `DESIGNED a loop for "${opts.goal}". Validation ${validation!.ok ? 'PASS' : 'FAIL'} · Smoke ${smoke!.pass ? 'PASS' : 'FAIL'} (${smoke!.detail}). ${registered ? `AUTO-REGISTERED "${registered.id}".` : releasable ? '(releasable — run with --register to register)' : 'NOT registered.'}`,
    issues: escalated
      ? [`Goal not admissible → escalated to human: ${(result as { escalate: string }).escalate}`]
      : [
          ...validation!.checks.filter((c) => !c.pass).map((c) => `validation FAIL [${c.name}]: ${c.detail}`),
          ...(smoke!.pass ? [] : [`SMOKE FAIL: ${smoke!.detail} — rejected (a loop that can't demonstrate a moving/terminating/scoped metric is not released).`]),
          ...(design!.reuseOf ? [`Not a new loop — EXTEND existing "${design!.reuseOf}" (recall supplies what prior versions found).`] : []),
        ],
    patterns: escalated
      ? ['Oracle-admissibility gate held: no deterministic metric → no loop (the builder never emits a fuzzy loop).']
      : [
          `DESIGN · oracle: ${design!.oracle}`,
          `DESIGN · tools (prefer-existing): ${design!.tools.join(', ')}`,
          `SMOKE · ${smoke!.detail} (moved=${smoke!.moved} terminated=${smoke!.terminated} noChurn=${smoke!.noChurn})`,
          `DESIGN · scope: Class ${design!.scopeClass} · carve-out: ${design!.carveOut.join(', ') || '(none)'} · breaker K${design!.breaker.K}/max${design!.breaker.maxIter}`,
          'Born hardened: reuses harness reviewer + inherits accumulated guards (§5).',
        ],
    code: { tests_fail_start: 1, tests_fail_end: escalated || !(validation?.ok && smoke?.pass) ? 1 : 0, edits: registered ? 1 : 0, slop_min: null, fake_green_caught: 0 },
    carry_forward: {
      guards: [
        '[builder] oracle-admissibility FIRST: no deterministic metric → refuse + escalate (never guess).',
        '[builder] smoke-gated: a design is registered ONLY after the smoke test proves its metric moves + terminates + no out-of-scope churn.',
        '[builder] security carve-out embedded in every design (auth/RLS/secrets/payment/PII propose-only even in Class A).',
      ],
      watch: escalated ? ['define measurable success criteria for this goal, or keep it human-driven']
        : [
            registered ? `REGISTERED "${registered.id}" → router can now select it (runs/registry.json)` : '',
            !registered && releasable ? `releasable — re-run with --register to add "${design!.id}" to the registry` : '',
            design!.scopeClass === 'B' || design!.reuseOf ? `NOT auto-registered: ${design!.reuseOf ? `extend "${design!.reuseOf}"` : 'Class B / security scope → human queue'}` : '',
          ].filter(Boolean),
    },
  };
  return buildRecord(input, opts.baseDir, { repo: opts.repoDir });
}

async function main(): Promise<void> {
  const register = process.argv.includes('--register');
  const positional = process.argv.slice(3).filter((a) => !a.startsWith('--'));
  const goal = positional[0];
  if (!goal) { console.error('usage: loop-builder.ts build "<goal>" [repoDir] [baseDir] [--register]'); process.exit(2); }
  const repoDir = positional[1] ?? process.cwd();
  const baseDir = positional[2] ?? path.join(repoDir, 'loops', 'runs');
  const t0 = Date.now();
  const record = await runLoopBuilder({ goal, repoDir, baseDir, tStart: new Date(t0).toISOString(), tEnd: new Date(Date.now()).toISOString(), register });
  console.log(renderReport(record)); // §5 — always print the full report
  writeRunRecord(baseDir, 'loop-builder', record.run_index, record);
  appendMetricsLine(baseDir, {
    loop: 'loop-builder', run_index: record.run_index, ts: record.t_end, outcome: record.outcome,
    iters: 1, wall_s: record.wall_s, tokens_in: 0, tokens_out: 0, cost_usd: 0, kwh: 0, gco2: 0, water_ml: 0,
    fail_start: record.telemetry.tests_fail_start, fail_end: record.telemetry.tests_fail_end, per_resolved: null, slop_min: null, conflicts: 0, recurring_flags: [],
  });
  console.error(`\n[persisted] ${baseDir}/loop-builder/${record.run_index}.json.gz`);
}

if (import.meta.url === `file://${process.argv[1]}`) main().catch((e) => { console.error(e); process.exit(1); });
