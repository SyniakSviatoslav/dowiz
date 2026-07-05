// The MAP source for `repo-perf:` candidates — the piece that lets the loop
// auto-KEEP a real, proven, reversible speedup. §0 firm rule: patches are
// DETERMINISTIC + MECHANICAL, never autonomously LLM-written. So the only
// auto-tunable class is OPERATOR-DECLARED config tuning (§3 "tune harness config /
// cache TTLs"): the human declares a knob + a BOUNDED set of safe candidate values
// + a benchmark; the loop mechanically tries each, benchmarks it, and the oracle
// keeps the value that is ≥5% faster (else atomic rollback). The operator bounds
// the search space (safety); the loop does the search + measure + keep (autonomy).

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import type { Candidate } from './autoupgrade.js';
import type { RepoPerfSpec } from './repo-apply.js';
import type { BenchmarkSpec } from './benchmark.js';

export interface CmdSpec { cmd: string; args: string[] }
export interface Tunable {
  id: string;
  /** Repo-relative file holding the knob. */
  file: string;
  /** Regex source; CAPTURE GROUP 1 is the current value to replace. */
  find: string;
  /** Bounded set of safe alternative values to try (the operator's safety boundary). */
  candidates: string[];
  /** Benchmark: 'wall_ms' median, or parse a number from stdout via regex source. */
  benchmark: { cmd: string; args: string[]; metric: 'wall_ms' | { parse: string } };
  /** exit 0 = green / security-ok. Default: assume unaffected (true) if omitted. */
  green?: CmdSpec;
  security?: CmdSpec;
}

function runExitZero(repoDir: string, c?: CmdSpec): boolean {
  if (!c) return true;
  try { execFileSync(c.cmd, c.args, { cwd: repoDir, stdio: 'ignore', timeout: 600_000 }); return true; }
  catch { return false; }
}

/** Replace capture-group-1 of `find` in the file with `value` (mechanical, in place). */
function setValue(repoDir: string, file: string, find: string, value: string): void {
  const p = path.join(repoDir, file);
  const src = fs.readFileSync(p, 'utf8');
  const re = new RegExp(find);
  const out = src.replace(re, (match, g1) => match.replace(g1, value));
  fs.writeFileSync(p, out);
}

function currentValue(repoDir: string, file: string, find: string): string | null {
  const p = path.join(repoDir, file);
  if (!fs.existsSync(p)) return null;
  const m = fs.readFileSync(p, 'utf8').match(new RegExp(find));
  return m ? m[1]! : null;
}

function benchOf(repoDir: string, t: Tunable): BenchmarkSpec {
  return {
    cmd: t.benchmark.cmd, args: t.benchmark.args, cwd: repoDir,
    metric: t.benchmark.metric === 'wall_ms' ? 'wall_ms' : { parse: new RegExp(t.benchmark.metric.parse) },
  };
}

/** A repo-perf Candidate for trying `value` on tunable `t`, with its reversible+benchmarkable spec. */
function tuneCandidate(repoDir: string, t: Tunable, from: string, value: string): Candidate {
  const perf: RepoPerfSpec = {
    repoDir, paths: [t.file],
    applyPatch: () => setValue(repoDir, t.file, t.find, value),
    benchmark: benchOf(repoDir, t),
    green: () => runExitZero(repoDir, t.green),
    security: () => runExitZero(repoDir, t.security),
  };
  return {
    id: `repo-perf:tune:${t.id}:${value}`,
    pattern: `tune ${t.id} (${t.file}): ${from} → ${value}`,
    source: 'config-tune detector (operator-declared tunable)',
    area: 'perf config tune dev-loop',
    evidence: `declared safe candidate value; benchmark decides (oracle keeps iff ≥5% faster)`,
    expected_speedup: 'measured by benchmark-replay; kept only if ≥5%',
    blast_radius: 'low',
    reversible: true,
    action: `set ${t.file} ${t.id} = ${value} (mechanical; git-revert)`,
    perf,
  };
}

/** Emit repo-perf candidates from operator-declared tunables (each non-current candidate value). */
export function configTuneDetector(repoDir: string, tunables: Tunable[]): Candidate[] {
  const out: Candidate[] = [];
  for (const t of tunables) {
    const cur = currentValue(repoDir, t.file, t.find);
    if (cur == null) continue; // knob not found — skip silently (declaration stale)
    for (const v of t.candidates) {
      if (v !== cur) out.push(tuneCandidate(repoDir, t, cur, v));
    }
  }
  return out;
}

/** Load the operator's declared tunables (loops/autoupgrade.tunables.json), or []. */
export function loadTunables(repoDir: string): Tunable[] {
  const p = path.join(repoDir, 'loops', 'autoupgrade.tunables.json');
  if (!fs.existsSync(p)) return [];
  try {
    const parsed = JSON.parse(fs.readFileSync(p, 'utf8'));
    return Array.isArray(parsed?.tunables) ? parsed.tunables : [];
  } catch { return []; }
}
