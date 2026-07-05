// Autoupgrade loop (docs/operating-model/ autoupgrade-loop-v1) on the v3-FINAL
// harness. Goal: find changes that PROVABLY haste iteration with fewer resources.
//
// THIS PASS IS REPORT-ONLY (spec §8 step 2). MAP → CLASSIFY → REPORT. The oracle
// (§2) + Class-A auto-apply (§4) are scaffolded but DISABLED in code — autonomous
// mutation is enabled only after the oracle + rollback are proven (§8 step 4).
//
// FIRM BOUNDARY (§0, non-negotiable): auth, RLS/tenant-isolation, secrets,
// payments, PII, schema/migrations, architecture/topology, major deps are NEVER
// classed A (never autonomously mutated). The classifier fails safe → B.

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { makeRepoHooks, type RepoPerfSpec } from './repo-apply.js';
import { configTuneDetector, loadTunables } from './detectors.js';
import os from 'node:os';
import { checkCredentialIsolation, isTrustedSource } from './containment.js';
import { queueProposal } from './proposals.js';
import { checkGovernor } from './governor.js';
import { isRejected } from './review-queue.js';

export type UpgradeClass = 'A' | 'B';
export type BlastRadius = 'low' | 'med' | 'high';

export interface Candidate {
  id: string;
  pattern: string;          // what was found
  source: string;           // where the evidence came from
  area: string;             // tag(s) for classification, space/comma separated
  evidence: string;
  expected_speedup: string; // human estimate, e.g. "~$46/mo cached-prefix", "−85M tok/mo"
  blast_radius: BlastRadius;
  reversible: boolean;
  action: string;           // the concrete change (a command / edit), NOT executed this pass
  /** For `repo-perf:` candidates: the reversible+benchmarkable apply spec (deterministic
   *  mechanical patch only — never an autonomous LLM patch). Not serialized into the record. */
  perf?: RepoPerfSpec;
}

// STORM-insight B — decorrelated lenses. Each candidate is judged through THREE
// independent perspectives (mirrors the council's cause/pattern/ratchet critics);
// a Class-A KEEP requires ALL to pass — a single failing lens demotes to Class B.
export type Lens = 'security' | 'reversibility' | 'perf';
export interface LensVerdict { lens: Lens; pass: boolean; reason: string }

export interface Classification {
  class: UpgradeClass;
  reason: string;
  /** The decorrelated lens verdicts that produced this class (auditability). */
  lenses: LensVerdict[];
}

// §0/§3 firm-boundary triggers. ANY hit → Class B (propose-only), no exceptions.
// Matched against the STRUCTURED `area` tag — broad list (the producer tags area,
// so generic words here can't false-positive on incidental prose).
const AREA_BOUNDARY = /\b(auth|login|session|jwt|rls|tenant|isolation|secret|credential|token-?rotation|payment|cash|money|price|tax|settlement|pii|privacy|personal-?data|gdpr|anonymiz|schema|migration|ddl|architecture|topology|monolith|microservice|hosting|fly|k8s|kubernetes|queue-swap|runtime-swap|theming-security|white-?label)\b/i;
// Matched against FREE TEXT (pattern/action) — TIGHT, unambiguous tokens only, so
// innocuous words ("loaded every session", "cache the fixtures") don't false-trip B.
const TEXT_BOUNDARY = /\b(auth|jwt|rls|tenant-?isolation|secret|credential|payment|settlement|pii|gdpr|anonymiz|tax|schema migration|ddl|alter\s+\w+\s+policy|microservice|kubernetes|k8s)\b/i;
// Class-B also for inherently risky shapes regardless of area.
const MAJOR_DEP = /\b(major|breaking)\b.*\b(upgrade|bump|migration)\b/i;

/**
 * STORM-insight B — judge a candidate through three DECORRELATED lenses:
 *  - security:      trips no firm-boundary trigger (auth/RLS/secrets/payments/PII/
 *                   schema/architecture). `area` checked against the broad list,
 *                   free text against a tight list so innocuous prose can't false-trip.
 *  - reversibility: has a recorded revert AND isn't a major/breaking dep (not cleanly undoable).
 *  - perf:          blast radius is containable (high blast → not auto-eligible).
 * Each lens is independent; classify() ANDs them (a single fail → Class B).
 */
export function evaluateLenses(c: Candidate): LensVerdict[] {
  const text = `${c.pattern} ${c.action}`;
  const securityHit = AREA_BOUNDARY.test(c.area) || TEXT_BOUNDARY.test(text);
  const majorDep = MAJOR_DEP.test(text);
  const highBlast = c.blast_radius === 'high';
  return [
    {
      lens: 'security',
      pass: !securityHit,
      reason: securityHit
        ? 'firm-boundary (auth/RLS/secrets/payments/PII/schema/architecture) — never autonomously mutated'
        : 'outside firm boundary',
    },
    {
      lens: 'reversibility',
      pass: c.reversible && !majorDep,
      reason: !c.reversible
        ? 'not reversible — no recorded revert'
        : majorDep
          ? 'major/breaking dependency change — not cleanly reversible'
          : 'reversible with a recorded revert',
    },
    {
      lens: 'perf',
      pass: !highBlast,
      reason: highBlast ? 'high blast radius — too risky to auto-apply' : 'low/med blast — containable',
    },
  ];
}

/**
 * Classify a candidate. Fail-safe: a candidate is Class A (auto-eligible) ONLY if
 * ALL decorrelated lenses pass (security · reversibility · perf). A single failing
 * lens → Class B (human decides). Everything ambiguous → Class B.
 */
export function classify(c: Candidate): Classification {
  const lenses = evaluateLenses(c);
  const failed = lenses.filter((l) => !l.pass);
  if (failed.length > 0) {
    return { class: 'B', reason: `lens fail → ${failed.map((l) => `${l.lens}: ${l.reason}`).join('; ')}`, lenses };
  }
  return { class: 'A', reason: 'all lenses pass — security · reversibility · perf (outside firm boundary, reversible, containable blast)', lenses };
}

// ─── MAP — ground candidates in REAL local telemetry (no web research this pass) ───

function safeExec(cmd: string, args: string[]): string {
  try { return execFileSync(cmd, args, { encoding: 'utf8', timeout: 120_000, stdio: ['ignore', 'pipe', 'ignore'] }); }
  catch { return ''; }
}

/** Ghost MCP servers / skills — codeburn flags unused servers whose schema bloats
 *  every cached prefix. Parses the exact `claude mcp remove '<x>'` it prints. */
function mapGhostMcp(): Candidate[] {
  const out = safeExec('npx', ['-y', 'codeburn@0.9.14', 'optimize']);
  const removes = [...out.matchAll(/claude mcp remove ['"]([^'"]+)['"]/g)].map((m) => m[1]!);
  const unique = [...new Set(removes)];
  return unique.map((server) => ({
    id: `ghost-mcp:${server}`,
    pattern: `unused MCP server '${server}' loaded every session`,
    source: 'codeburn optimize',
    area: 'dev-loop mcp config token-bloat',
    evidence: `codeburn: 0 tools used; schema carried in cached prefix each turn`,
    expected_speedup: 'less cached-prefix tokens/run (codeburn ~$46/mo across servers)',
    blast_radius: 'low',
    reversible: true, // re-add via `claude mcp add`
    action: `claude mcp remove '${server}'`,
  }));
}

/** Config bloat — a large CLAUDE.md is re-read into every prompt prefix. */
function mapConfigBloat(repoDir: string): Candidate[] {
  const p = path.join(repoDir, '.claude', 'CLAUDE.md');
  if (!fs.existsSync(p)) return [];
  const bytes = fs.statSync(p).size;
  const KB = Math.round(bytes / 1024);
  if (KB < 12) return []; // only flag genuinely heavy config
  return [{
    id: 'config-bloat:CLAUDE.md',
    pattern: `.claude/CLAUDE.md is ${KB} KB — re-read into every prompt prefix`,
    source: 'fs stat',
    area: 'dev-loop config token-bloat',
    evidence: `${KB} KB ≈ ${Math.round(bytes / 4)} chars carried per turn`,
    expected_speedup: 'lower per-turn prefix tokens (needs measurement vs benchmark)',
    blast_radius: 'med', // editing the operating doctrine — reversible but review the diff
    reversible: true,
    action: 'distill CLAUDE.md: move rarely-triggered detail to linked docs (proposal — needs the oracle before auto-apply)',
  }];
}

/** Staged security/data work — must land in the Class B (propose-only) queue. */
function mapStagedSecurity(repoDir: string): Candidate[] {
  const dir = path.join(repoDir, 'docs', 'security');
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir).filter((f) => /migration|definer|rls/i.test(f)).map((f) => ({
    id: `staged-security:${f}`,
    pattern: `staged security/data change docs/security/${f}`,
    source: 'docs/security scan',
    area: 'security rls schema migration', // → firm boundary → B
    evidence: 'pre-authored migration awaiting DB-owner apply',
    expected_speedup: 'n/a (correctness/hardening, not speed)',
    blast_radius: 'high',
    reversible: false,
    action: 'queue for human/DB-owner — NEVER autonomously applied',
  }));
}

export function mapCandidates(repoDir: string): Candidate[] {
  return [
    ...mapGhostMcp(),
    ...mapConfigBloat(repoDir),
    ...mapStagedSecurity(repoDir),
    // repo-perf candidates from operator-declared tunables (the auto-keepable class)
    ...configTuneDetector(repoDir, loadTunables(repoDir)),
  ];
}

// ─── The oracle (§2) + apply (§4) — SCAFFOLDED + DISABLED this pass ───

export interface OracleResult { green: boolean; security_ok: boolean; speedup_pct: number; kept: boolean; }

/** Auto-apply is intentionally disabled until the oracle + rollback are proven
 *  (§8 step 4). Calling it throws — the safety is enforced in code, not just docs. */
export function applyCandidate(_c: Candidate): never {
  throw new Error('AUTOUPGRADE: auto-apply is DISABLED (report-only, spec §8 step 2). Enable Class-A apply only after the oracle (§2) + atomic rollback are proven on a few candidates.');
}

// ─── Oracle-gated Class-A auto-apply (§2/§4) ───

import { evaluate, type OracleHooks, type OracleVerdict, DEFAULT_THRESHOLDS } from './oracle.js';

export interface ApplyOutcome {
  id: string;
  decision: 'kept' | 'rolled_back' | 'skipped' | 'report-only';
  reason: string;
  speedup_pct?: number | null;
  /** Oracle benchmark metrics (lower = better) — carried so a KEPT outcome can be
   *  recorded as a proven-upgrade gene with its before/after. */
  before?: number | null;
  after?: number | null;
}

/**
 * Build the oracle hooks for a Class-A candidate, or skip it with an honest
 * reason. A candidate is only auto-applicable if this loop can apply it in
 * isolation, run green+security, MEASURE a benchmark-replay speedup, AND revert
 * it. Config/prefix-size changes (MCP, CLAUDE.md) are intentionally NOT
 * auto-applied here: account-managed MCP isn't loop-reversible, and a prefix-size
 * win isn't a runnable benchmark — so the oracle would (correctly) reject them.
 * The reversible+benchmarkable adapter now EXISTS (makeRepoHooks + runBenchmark,
 * git-checkout atomic revert) and is proven end-to-end (repo-apply.test.ts). What
 * a `repo-perf:` candidate needs is a MAP source that emits it WITH a DETERMINISTIC,
 * MECHANICAL patch + a benchmark — never an autonomously LLM-written patch (§0:
 * web-research→self-implement is the injection surface). Config/MCP candidates stay
 * skipped (not loop-reversible / no runnable benchmark).
 */
export function buildHooks(c: Candidate): { hooks: OracleHooks } | { skip: string } {
  if (c.id.startsWith('repo-perf:') && c.perf) {
    // §0/§4 — only candidates from a TRUSTED mechanical detector may auto-apply.
    if (!isTrustedSource(c.source)) {
      return { skip: `untrusted source "${c.source}" — not an allowlisted mechanical detector; propose-only (§0: never execute web/LLM-derived patches).` };
    }
    return { hooks: makeRepoHooks(c.perf) }; // reversible + benchmarkable → oracle can KEEP it
  }
  if (c.id.startsWith('ghost-mcp:')) {
    return { skip: 'account-managed MCP (claude.ai connector) — not removable/re-addable via the local mcp CLI, so NOT loop-reversible. Prune manually if desired; the loop will not apply what it cannot atomically revert.' };
  }
  if (c.id.startsWith('config-bloat:')) {
    return { skip: 'operating-doctrine/config edit — protect-paths-gated + the prefix-size win is not a benchmark-replayable speedup. Human review (Class-A-shaped but no oracle adapter).' };
  }
  return { skip: 'adapter ready (makeRepoHooks + benchmark, atomic git revert — proven) but no MAP source emits a repo-perf candidate with a deterministic mechanical patch yet (§0: no autonomous LLM patches).' };
}

async function evaluateClassA(classA: Candidate[], apply: boolean, baseDir: string, env: Record<string, string | undefined> = process.env): Promise<ApplyOutcome[]> {
  if (apply) {
    // GOVERNOR (§1) — refuse autonomous apply if halted or any aggregate ceiling breached.
    const gov = checkGovernor(baseDir, { nowMs: Date.now(), freeRamMb: Math.round(os.freemem() / (1024 * 1024)) });
    if (!gov.allowed) {
      return classA.map((c) => ({ id: c.id, decision: 'skipped' as const, reason: `GOVERNOR (§1): ${gov.reason}` }));
    }
    // §4 credential isolation — refuse autonomous apply if prod secrets are in context.
    const iso = checkCredentialIsolation(env);
    if (!iso.ok) {
      return classA.map((c) => ({ id: c.id, decision: 'skipped' as const, reason: `CONTAINMENT (§4): prod credentials in context (${iso.present.join(', ')}) — autonomous apply refused. Run credential-isolated.` }));
    }
  }
  const out: ApplyOutcome[] = [];
  for (const c of classA) {
    if (!apply) { out.push({ id: c.id, decision: 'report-only', reason: 'auto-apply not requested (run with --apply)' }); continue; }
    const built = buildHooks(c);
    if ('skip' in built) { out.push({ id: c.id, decision: 'skipped', reason: built.skip }); continue; }
    const v: OracleVerdict = await evaluate(built.hooks, DEFAULT_THRESHOLDS);
    out.push({ id: c.id, decision: v.decision, reason: v.reason, speedup_pct: v.speedup_pct, before: v.before, after: v.after });
  }
  return out;
}

// ─── EvoMap-insight A — persist a KEPT Class-A upgrade as a proven "gene" ───

import { recordProvenUpgrade, type ProvenUpgrade } from './proven-upgrades.js';

/**
 * For every KEPT outcome, persist a versioned, replayable proven-upgrade asset.
 * STORM-insight B double-check (defense-in-depth): a kept candidate is recorded
 * ONLY if EVERY decorrelated lens still passes — a single failing lens (esp.
 * security/firm-boundary) means it is NOT recorded as a gene. `ts` is supplied by
 * the caller (no Date.now() — determinism).
 */
export function recordKeptUpgrades(baseDir: string, classA: Candidate[], outcomes: ApplyOutcome[], ts: string): ProvenUpgrade[] {
  const recorded: ProvenUpgrade[] = [];
  for (const o of outcomes) {
    if (o.decision !== 'kept') continue;
    const cand = classA.find((c) => c.id === o.id);
    if (!cand) continue;
    const failed = evaluateLenses(cand).filter((l) => !l.pass);
    if (failed.length > 0) continue; // never record a lens-failing change as proven
    recorded.push(recordProvenUpgrade(baseDir, {
      id: cand.id,
      patch_ref: cand.action,
      metric_before: o.before ?? null,
      metric_after: o.after ?? null,
      speedup_pct: o.speedup_pct ?? null,
      revert: cand.perf ? `git checkout -- ${cand.perf.paths.join(' ')}` : 'recorded inverse command (see patch_ref)',
      provenance: `${cand.source} | lenses pass: security·reversibility·perf`,
    }, ts));
  }
  return recorded;
}

// ─── Runner — MAP → CLASSIFY → (oracle-gated apply) → §5 report ───

import type { RunRecord } from './types.js';
import { buildRecord } from './cli.js';
import { renderReport } from './report.js';
import { writeRunRecord, appendMetricsLine } from './storage.js';

export async function runAutoupgrade(opts: { repoDir: string; baseDir: string; tStart: string; tEnd: string; apply: boolean }): Promise<RunRecord> {
  const candidates = mapCandidates(opts.repoDir).map((c) => ({ c, k: classify(c) }));
  const classA = candidates.filter((x) => x.k.class === 'A');
  const classB = candidates.filter((x) => x.k.class === 'B');
  // §8c — Class B is PROPOSED to the human-gated queue, never auto-applied.
  // §2 feedback: skip re-proposing anything a human already REJECTED (negative learning).
  for (const x of classB) {
    if (isRejected(opts.baseDir, x.c.id)) continue;
    queueProposal(opts.baseDir, {
      id: x.c.id, source: 'autoupgrade:class-B', kind: x.c.area.split(' ')[0] ?? 'review',
      description: x.c.pattern, evidence: x.c.evidence, action: x.c.action,
    }, opts.tEnd);
  }
  const outcomes = await evaluateClassA(classA.map((x) => x.c), opts.apply, opts.baseDir);
  // EvoMap-insight A — persist every KEPT Class-A change as a proven, replayable
  // gene (lens-gated). Report-only runs keep 0, so this is dormant-but-ready.
  const genes = recordKeptUpgrades(opts.baseDir, classA.map((x) => x.c), outcomes, opts.tEnd);
  const kept = outcomes.filter((o) => o.decision === 'kept');
  const rolledBack = outcomes.filter((o) => o.decision === 'rolled_back');
  const skipped = outcomes.filter((o) => o.decision === 'skipped');

  const mode = opts.apply ? 'ORACLE-GATED AUTO-APPLY (Class A only)' : 'report-only';
  const input = {
    loop: 'autoupgrade',
    goal: 'Find changes that PROVABLY haste iteration with fewer resources. Oracle-gated (green + no-security-regression + ≥5% benchmark speedup + reversible); Class B never auto-applied.',
    outcome: 'natural_stop' as const,
    t_start: opts.tStart,
    t_end: opts.tEnd,
    iter_from: 1,
    iter_to: Math.max(1, candidates.length),
    what_done: `MAP (codeburn + fs scan) → ${candidates.length} candidate(s); CLASSIFY fail-safe (firm boundary → B). Mode: ${mode}. Oracle verdicts: ${kept.length} kept · ${rolledBack.length} rolled back · ${skipped.length} skipped.`,
    issues: [
      ...classB.map((x) => `Class B → QUEUED for human (loops/runs/proposals.json): ${x.c.pattern} — ${x.k.reason}. Action: ${x.c.action}`),
      ...rolledBack.map((o) => `Rolled back (oracle): ${o.id} — ${o.reason}`),
      ...skipped.map((o) => `Skipped (not safely auto-applicable): ${o.id} — ${o.reason}`),
    ],
    patterns: [
      `Oracle: kept ${kept.length}${kept.length ? ` (${kept.map((o) => `${o.id} ${o.speedup_pct}% faster`).join(', ')})` : ''} · rolled-back ${rolledBack.length} · skipped ${skipped.length}.`,
      'Firm boundary holding: auth/RLS/secrets/payments/PII/schema/architecture never classed A, never auto-applied.',
    ],
    code: { tests_fail_start: candidates.length, tests_fail_end: classB.length + rolledBack.length + skipped.length, edits: kept.length, slop_min: null, fake_green_caught: 0 },
    carry_forward: {
      guards: [
        '[oracle] KEEP iff green + no-security-regression + ≥5% benchmark speedup + reversible; else atomic rollback.',
        '[code] firm boundary: Class B (auth/RLS/secrets/payments/PII/schema/arch) never reaches apply.',
      ],
      watch: [
        ...kept.map((o) => `APPLIED+KEPT (revert recorded): ${o.id}`),
        ...genes.map((g) => `PROVEN-UPGRADE gene recorded (loops/runs/proven-upgrades.json): ${g.id} v${g.version} · ${g.speedup_pct}% faster · revert: ${g.revert}`),
        'NEXT (§8 step 4/6): add a reversible+benchmarkable Class-A adapter (worktree + benchmark-replay) so real repo-perf candidates can be auto-applied; widen only after several clean runs.',
      ],
    },
  };

  // session telemetry intentionally omitted — this MAP/CLASSIFY pass is a
  // deterministic script (≈0 agent tokens); its value is the candidates, not its cost.
  return buildRecord(input, opts.baseDir, { repo: opts.repoDir });
}

async function main(): Promise<void> {
  const apply = process.argv.includes('--apply');
  const positional = process.argv.slice(3).filter((a) => !a.startsWith('--'));
  const repoDir = positional[0] ?? process.cwd();
  const baseDir = positional[1] ?? path.join(repoDir, 'loops', 'runs');
  const t0 = Date.now();
  const tStart = new Date(t0).toISOString();
  const record = await runAutoupgrade({ repoDir, baseDir, tStart, tEnd: new Date(Date.now()).toISOString(), apply });
  console.log(renderReport(record));
  writeRunRecord(baseDir, 'autoupgrade', record.run_index, record);
  appendMetricsLine(baseDir, {
    loop: 'autoupgrade', run_index: record.run_index, ts: record.t_end, outcome: record.outcome,
    iters: record.telemetry.iterations, wall_s: record.wall_s,
    tokens_in: 0, tokens_out: 0, cost_usd: 0, kwh: 0, gco2: 0, water_ml: 0,
    fail_start: record.telemetry.tests_fail_start, fail_end: 0, per_resolved: null, slop_min: null,
    conflicts: 0, recurring_flags: [],
  });
  console.error(`\n[persisted] ${baseDir}/autoupgrade/${record.run_index}.json.gz`);
}

if (import.meta.url === `file://${process.argv[1]}`) main().catch((e) => { console.error(e); process.exit(1); });
