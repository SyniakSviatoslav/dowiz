#!/usr/bin/env node
// guardrail-sandbox-staleness.mjs
//
// PROVENANCE: this guardrail was PROPOSED by the meta-controller (gap `stale-sandbox`,
// see docs/design/harness/META-CONTROLLER.md) and landed by a human through the gate. It is
// an ADDITIVE new node — the meta-controller may never edit the immutable sandbox-swarm-gate.mjs,
// so it grows the system with new guardrails instead. This is the L5 loop's output made concrete.
//
// WHAT: fails when a sandbox worktree is STALE (≥N commits behind HEAD) AND carries UNTRACKED work
// that `git worktree remove --force` (which sandbox-swarm-gate.mjs `rm --apply` runs) would destroy
// forever. That is exactly the condition that stranded ~1,800 lines of voice FE on 2026-07-03.
//
// Untracked (??) is the loss-risk signal: modified TRACKED files are recoverable from git; untracked
// files in a force-removed worktree are gone. Plain WIP on a fresh worktree does NOT fail (no crying wolf).
//
// Worktrees live under .claude/worktrees/ (local only) → this is a no-op in CI / clean clones.
//   report (default, exit 0) · --ci/--strict (exit 1 if any stale+untracked sandbox).
//
// Run: node scripts/guardrail-sandbox-staleness.mjs [--ci]   ·   Test: node --test scripts/guardrail-sandbox-staleness.test.mjs
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const SELF = fileURLToPath(import.meta.url);
const REPO = path.resolve(path.dirname(SELF), '..');
export const STALE_BEHIND = 5;

/** PURE predicate — the guardrail's contract. At-risk = stale AND has untracked work. */
export function isAtRisk({ behind, untracked }, staleBehind = STALE_BEHIND) {
  return behind >= staleBehind && untracked > 0;
}

function git(args, opts = {}) {
  try { return execFileSync('git', args, { cwd: REPO, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...opts }).trim(); }
  catch { return ''; }
}

/** Scan every .claude/worktrees/* sandbox for drift + untracked/modified counts. */
export function scanWorktrees() {
  const out = git(['worktree', 'list', '--porcelain']);
  const head = git(['rev-parse', 'HEAD']) || 'HEAD';
  const rows = [];
  for (const block of out.split('\n\n').filter(Boolean)) {
    const wt = /worktree (.+)/.exec(block)?.[1];
    if (!wt) continue;
    const rel = path.relative(REPO, wt);
    if (!rel.startsWith('.claude/worktrees')) continue;
    const wtHead = /HEAD ([0-9a-f]+)/.exec(block)?.[1] || '';
    const mb = git(['merge-base', wtHead || 'HEAD', head]) || wtHead;
    const behind = Number(git(['rev-list', '--count', `${mb}..${head}`]) || 0);
    const lines = git(['status', '--short'], { cwd: wt }).split('\n').filter(Boolean);
    const untracked = lines.filter((l) => l.startsWith('??')).length;
    const row = { lane: path.basename(rel), behind, untracked, modified: lines.length - untracked };
    row.atRisk = isAtRisk(row);
    rows.push(row);
  }
  return rows;
}

function main() {
  const ci = process.argv.includes('--ci') || process.argv.includes('--strict');
  const rows = scanWorktrees();
  if (!rows.length) { console.log('✓ guardrail-sandbox-staleness: no sandbox worktrees.'); process.exit(0); }
  for (const r of rows) {
    const mark = r.atRisk ? '🔴' : r.behind >= STALE_BEHIND ? '🟡' : '✓ ';
    console.log(`  ${mark} ${r.lane.padEnd(30)} ${String(r.behind).padStart(3)} behind · ${r.untracked} untracked · ${r.modified} modified`);
  }
  const risky = rows.filter((r) => r.atRisk);
  if (risky.length) {
    console.log(`\n  🔴 ${risky.length} sandbox(es) STALE (≥${STALE_BEHIND} behind) carrying UNTRACKED work a --force prune would destroy.`);
    console.log('     PRESERVE first (tar → tracked, cf. docs/design/voice-control/sandbox-snapshots/README.md),');
    console.log('     then rebase-onto-HEAD to integrate, or remove the worktree once snapshotted.');
    if (ci) { console.error('✗ guardrail-sandbox-staleness: FAIL (--ci) — at-risk sandbox work present.'); process.exit(1); }
    console.log('\n  (report mode — run with --ci to fail)');
  } else {
    console.log('\n✓ guardrail-sandbox-staleness: no at-risk sandbox work.');
  }
  process.exit(0);
}

if (process.argv[1] && path.resolve(process.argv[1]) === SELF) main();
