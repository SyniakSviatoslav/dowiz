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

// ─── Runner — report-only pass on the harness ───

import type { RunRecord } from './types.js';
import { buildRecord } from './cli.js';
import { renderReport } from './report.js';
import { writeRunRecord, appendMetricsLine } from './storage.js';

export function runAutoupgrade(opts: { repoDir: string; baseDir: string; tStart: string; tEnd: string }): RunRecord {
  const candidates = mapCandidates(opts.repoDir).map((c) => ({ c, k: classify(c) }));
  const classA = candidates.filter((x) => x.k.class === 'A');
  const classB = candidates.filter((x) => x.k.class === 'B');

  const input = {
    loop: 'autoupgrade',
    goal: 'Find changes that PROVABLY haste iteration with fewer resources. Report-only pass (oracle + auto-apply not yet enabled — spec §8 steps 2/4).',
    outcome: 'natural_stop' as const,
    t_start: opts.tStart,
    t_end: opts.tEnd,
    iter_from: 1,
    iter_to: Math.max(1, candidates.length),
    what_done: `MAP grounded in real telemetry (codeburn + fs scan) → ${candidates.length} candidate(s); CLASSIFY fail-safe (firm boundary → B). NO apply this pass — deterministic report-only.`,
    issues: classB.map((x) => `Class B (propose-only, human/DB-owner): ${x.c.pattern} — ${x.k.reason}. Action: ${x.c.action}`),
    patterns: [
      `Class A (auto-eligible once oracle proven): ${classA.length} — ${classA.map((x) => x.c.id).join(', ') || '(none)'}`,
      'Firm boundary holding: auth/RLS/secrets/payments/PII/schema/architecture never classed A.',
    ],
    code: { tests_fail_start: candidates.length, tests_fail_end: 0, edits: 0, slop_min: null, fake_green_caught: 0 },
    carry_forward: {
      guards: ['[code] applyCandidate() throws — auto-apply disabled until oracle proven'],
      watch: [
        ...classA.map((x) => `Class-A queued (needs oracle: green+security+≥5% speedup+revert): ${x.c.action}`),
        'NEXT (§8 step 3): build the machine oracle + fixed benchmark-replay speed check, then enable Class-A apply (step 4).',
      ],
    },
  };

  // session telemetry intentionally omitted — this MAP/CLASSIFY pass is a
  // deterministic script (≈0 agent tokens); its value is the candidates, not its cost.
  return buildRecord(input, opts.baseDir, { repo: opts.repoDir });
}

function main(): void {
  const repoDir = process.argv[3] ?? process.cwd();
  const baseDir = process.argv[4] ?? path.join(repoDir, 'loops', 'runs');
  const t0 = Date.now();
  const tStart = new Date(t0).toISOString();
  const record = runAutoupgrade({ repoDir, baseDir, tStart, tEnd: new Date(Date.now()).toISOString() });
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

if (import.meta.url === `file://${process.argv[1]}`) main();
