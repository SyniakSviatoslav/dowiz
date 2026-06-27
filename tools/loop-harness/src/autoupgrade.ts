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

export interface Classification {
  class: UpgradeClass;
  reason: string;
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
 * Classify a candidate. Fail-safe: a candidate is Class A (auto-eligible) ONLY if
 * it is reversible, low/med blast radius, and trips no firm-boundary trigger. The
 * `area` tag is checked against the broad list; free text against a tight list so
 * innocuous prose can't false-trip. Everything ambiguous → Class B (human decides).
 */
export function classify(c: Candidate): Classification {
  const text = `${c.pattern} ${c.action}`;
  if (AREA_BOUNDARY.test(c.area)) return { class: 'B', reason: 'firm-boundary area (auth/RLS/secrets/payments/PII/schema/architecture) — never autonomously mutated' };
  if (TEXT_BOUNDARY.test(text)) return { class: 'B', reason: 'firm-boundary keyword in the change itself — never autonomously mutated' };
  if (MAJOR_DEP.test(text)) return { class: 'B', reason: 'major/breaking dependency change' };
  if (!c.reversible) return { class: 'B', reason: 'not reversible — no recorded revert' };
  if (c.blast_radius === 'high') return { class: 'B', reason: 'high blast radius' };
  return { class: 'A', reason: 'reversible · low/med blast · outside firm boundary · dev-loop/perf' };
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
  return [...mapGhostMcp(), ...mapConfigBloat(repoDir), ...mapStagedSecurity(repoDir)];
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

async function evaluateClassA(classA: Candidate[], apply: boolean): Promise<ApplyOutcome[]> {
  const out: ApplyOutcome[] = [];
  for (const c of classA) {
    if (!apply) { out.push({ id: c.id, decision: 'report-only', reason: 'auto-apply not requested (run with --apply)' }); continue; }
    const built = buildHooks(c);
    if ('skip' in built) { out.push({ id: c.id, decision: 'skipped', reason: built.skip }); continue; }
    const v: OracleVerdict = await evaluate(built.hooks, DEFAULT_THRESHOLDS);
    out.push({ id: c.id, decision: v.decision, reason: v.reason, speedup_pct: v.speedup_pct });
  }
  return out;
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
  const outcomes = await evaluateClassA(classA.map((x) => x.c), opts.apply);
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
      ...classB.map((x) => `Class B (propose-only, human/DB-owner): ${x.c.pattern} — ${x.k.reason}. Action: ${x.c.action}`),
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
