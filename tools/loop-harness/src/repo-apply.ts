// §2/§4 — the reversible + benchmarkable Class-A apply adapter. Turns a repo-perf
// candidate into oracle hooks: apply a patch, measure a benchmark before/after,
// run green + security, and revert ATOMICALLY via `git checkout -- <paths>` (the
// recorded, tested revert §2.4 demands). Main is restored exactly on any rollback.
//
// ponytail: git-checkout isolation (not a separate worktree). Ceiling: it mutates
// the MAIN working tree during verify, so it is NOT safe under concurrent loops.
// Upgrade path: a git worktree + node_modules symlink when >1 loop can run at once
// (§6 says teamConcurrency 1 today, so this is adequate). The candidate's `paths`
// MUST be clean before apply (caller asserts) so the revert restores exactly.

import { execFileSync } from 'node:child_process';
import type { OracleHooks } from './oracle.js';
import { runBenchmark, type BenchmarkSpec } from './benchmark.js';

export interface RepoPerfSpec {
  repoDir: string;
  /** Files the patch touches — used for the atomic git revert. Must be tracked + clean. */
  paths: string[];
  /** Mutate the files (the candidate change). */
  applyPatch: () => void;
  /** Benchmark whose number must drop ≥ threshold (lower = better). */
  benchmark: BenchmarkSpec;
  /** Full test suite passes (§2.1). */
  green: () => Promise<boolean> | boolean;
  /** Standing RLS/auth/secret assertions pass (§2.2). */
  security: () => Promise<boolean> | boolean;
}

function git(repoDir: string, args: string[]): string {
  return execFileSync('git', ['-C', repoDir, ...args], { encoding: 'utf8' }).trim();
}

/** True iff every path is tracked and has no uncommitted changes (so revert restores exactly). */
export function pathsAreClean(repoDir: string, paths: string[]): boolean {
  const status = git(repoDir, ['status', '--porcelain', '--', ...paths]);
  return status === '';
}

export function makeRepoHooks(spec: RepoPerfSpec): OracleHooks {
  return {
    // Reversible iff the touched paths are clean+tracked before we start, so
    // `git checkout` restores the exact prior bytes. Fail-safe: dirty → not reversible.
    reversible: pathsAreClean(spec.repoDir, spec.paths),
    measure: () => runBenchmark(spec.benchmark),
    apply: () => spec.applyPatch(),
    revert: () => { git(spec.repoDir, ['checkout', '--', ...spec.paths]); },
    green: spec.green,
    security: spec.security,
  };
}
