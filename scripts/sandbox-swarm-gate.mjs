#!/usr/bin/env node
// sandbox-swarm-gate.mjs — thin, safe orchestration aid for the Sandbox-Swarm-Gate loop.
// Design: docs/design/harness/SANDBOX-SWARM-GATE.md · Registry: loops/registry.md (sandbox-swarm-gate)
//
// What it does (and deliberately does NOT do):
//   • new/list/rm  — create/list/remove per-lane sandbox worktrees under .claude/worktrees/ssg-<lane>
//   • checklist    — print the GATE rubric (quality · safety · ETHICS)
//   • plan         — DRY-RUN merge plan for a lane: diff stat + red-line/protected classification
//   • It NEVER merges, NEVER auto-approves a red-line, NEVER disables the ethics gate.
//     There is no `merge` subcommand by design — merge is a lead-driven, gate-passed,
//     human-approved act (see the runbook §10 in the design doc).
//
// Safety posture: DRY-RUN by default. Mutating ops (new/rm) require an explicit --apply.
// Node ESM, zero new dependencies (node:child_process / node:path / node:url only).

import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

// ── Constants mirrored from the harness (CLAUDE.md red-line globs + protect-paths.sh) ──────────
// Kept intentionally in sync with .claude/hooks/protect-paths.sh + red-line-doubt-gate.sh so the
// plan's classification matches what the GATE will actually enforce on the merge diff.
const RED_LINE = /(^|\/)(auth|jwt|otp|session|refresh.?token|login)|rls|policy\.sql|(^|\/)(price|money|payment|cash|ledger|tax|payout|refund|invoice)|packages\/db\/migrations\/|\.zod\.|(^|\/)(messagebus|webhook|idempoten)|secret|\.env/i;
const PROTECTED = /(^|\/)(migrations|\.github|\.claude)\/|(^|\/)(fly\.toml|Dockerfile|pnpm-lock\.yaml)$|\/package\.json$|packages\/shared-types\/|packages\/db\/|\/contracts\/|\.contract\./;
// .claude and .github stay blocked EVEN in-sandbox (see design §9): the sandbox is product-only.
const SANDBOX_ALWAYS_BLOCKED = /(^|\/)(\.github|\.claude)\//;

const WORKTREE_DIR = '.claude/worktrees';
const LANE_PREFIX = 'ssg-'; // .claude/worktrees/ssg-<lane>
const BRANCH_PREFIX = 'ssg/'; // ssg/<lane>

const HELP = `sandbox-swarm-gate — scaffolding aid for the Sandbox-Swarm-Gate loop

USAGE
  node scripts/sandbox-swarm-gate.mjs <command> [args] [flags]

COMMANDS
  new <lane> [--base <ref>]   Create a sandbox worktree  .claude/worktrees/ssg-<lane>
                              on a new branch  ssg/<lane>  (from --base, default: HEAD).
  list                        List existing SSG sandbox worktrees.
  rm <lane>                   Remove the sandbox worktree for <lane> (and prune).
  plan <lane> [--base <ref>]  DRY-RUN merge plan: diff stat of the lane vs --base (default: the
                              lane branch's merge-base with HEAD), with each changed file
                              classified red-line / protected / plain. Prints the gate reminder.
                              NEVER merges.
  checklist                   Print the GATE rubric (quality · safety · ETHICS).
  help, --help, -h            This help.

FLAGS
  --apply     Actually perform a mutating op (new/rm). Without it, they DRY-RUN and print the plan.
  --base <r>  Base git ref for new/plan.

SAFETY
  • DRY-RUN by default; new/rm need --apply.
  • No merge command exists — merge is lead-driven, gate-passed, human-approved (design doc §10).
  • Red-line & ethics are enforced at the GATE on the merge diff, never by this script.

SEE ALSO
  docs/design/harness/SANDBOX-SWARM-GATE.md   loops/registry.md (sandbox-swarm-gate)`;

// ── helpers ────────────────────────────────────────────────────────────────────────────────────
const SELF = fileURLToPath(import.meta.url);
const REPO = path.resolve(path.dirname(SELF), '..');

function git(args, opts = {}) {
  return execFileSync('git', args, { cwd: REPO, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...opts }).trim();
}
function gitSafe(args, opts = {}) {
  try {
    return git(args, opts);
  } catch (e) {
    return null;
  }
}

function laneName(raw) {
  if (!raw) die('missing <lane> argument');
  if (!/^[a-z0-9][a-z0-9-]*$/i.test(raw)) die(`invalid lane name '${raw}' — use [a-z0-9-] only`);
  return raw;
}
function wtPath(lane) {
  return path.join(WORKTREE_DIR, `${LANE_PREFIX}${lane}`);
}
function branchName(lane) {
  return `${BRANCH_PREFIX}${lane}`;
}
function die(msg) {
  console.error(`✗ ${msg}`);
  process.exit(1);
}

function parseFlags(argv) {
  const flags = { apply: false, base: null };
  const positional = [];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--apply') flags.apply = true;
    else if (a === '--base') flags.base = argv[++i];
    else if (a === '-h' || a === '--help') flags.help = true;
    else positional.push(a);
  }
  return { flags, positional };
}

// List SSG worktrees from `git worktree list --porcelain`.
function listSandboxes() {
  const out = gitSafe(['worktree', 'list', '--porcelain']) || '';
  const blocks = out.split('\n\n').filter(Boolean);
  const rows = [];
  for (const b of blocks) {
    const wt = /worktree (.+)/.exec(b)?.[1];
    const br = /branch (.+)/.exec(b)?.[1] || '(detached)';
    if (!wt) continue;
    const rel = path.relative(REPO, wt);
    if (rel.startsWith(WORKTREE_DIR) && path.basename(rel).startsWith(LANE_PREFIX)) {
      rows.push({ path: rel, branch: br.replace('refs/heads/', ''), lane: path.basename(rel).slice(LANE_PREFIX.length) });
    }
  }
  return rows;
}

function classify(file) {
  if (SANDBOX_ALWAYS_BLOCKED.test(file)) return 'HARNESS-BLOCKED'; // never allowed, even in-sandbox
  if (RED_LINE.test(file)) return 'RED-LINE';
  if (PROTECTED.test(file)) return 'PROTECTED';
  return 'plain';
}

// ── commands ─────────────────────────────────────────────────────────────────────────────────
function cmdNew(positional, flags) {
  const lane = laneName(positional[0]);
  const wt = wtPath(lane);
  const branch = branchName(lane);
  const base = flags.base || 'HEAD';
  const cmd = ['git', 'worktree', 'add', '-b', branch, wt, base];
  if (!flags.apply) {
    console.log(`DRY-RUN — would create sandbox for lane '${lane}':`);
    console.log(`  ${cmd.join(' ')}`);
    console.log(`\nRe-run with --apply to create it.`);
    return;
  }
  if (listSandboxes().some((r) => r.lane === lane)) die(`sandbox for lane '${lane}' already exists (${wt})`);
  try {
    git(['worktree', 'add', '-b', branch, wt, base]);
  } catch (e) {
    die(`git worktree add failed: ${e.stderr || e.message}`);
  }
  console.log(`✓ sandbox created: ${wt}  (branch ${branch}, base ${base})`);
  console.log(`  Fan a doer into it via the Agent tool (model:"sonnet", isolation:"worktree").`);
  console.log(`  Iteration inside is unrestricted; the GATE reviews the diff before merge.`);
}

function cmdRm(positional, flags) {
  const lane = laneName(positional[0]);
  const wt = wtPath(lane);
  const exists = listSandboxes().some((r) => r.lane === lane);
  if (!exists) die(`no sandbox found for lane '${lane}'`);
  if (!flags.apply) {
    console.log(`DRY-RUN — would remove sandbox for lane '${lane}':`);
    console.log(`  git worktree remove ${wt}`);
    console.log(`  git worktree prune`);
    console.log(`\nRe-run with --apply. (The ssg/${lane} branch is left intact; delete it manually if merged/abandoned.)`);
    return;
  }
  try {
    git(['worktree', 'remove', wt, '--force']);
    git(['worktree', 'prune']);
  } catch (e) {
    die(`git worktree remove failed: ${e.stderr || e.message}`);
  }
  console.log(`✓ sandbox removed: ${wt}  (branch ${branchName(lane)} kept — delete manually if done)`);
}

function cmdList() {
  const rows = listSandboxes();
  if (!rows.length) {
    console.log('No SSG sandbox worktrees. Create one:  node scripts/sandbox-swarm-gate.mjs new <lane> --apply');
    return;
  }
  console.log('SSG sandbox worktrees:');
  for (const r of rows) console.log(`  ${r.lane.padEnd(16)} ${r.branch.padEnd(24)} ${r.path}`);
}

function cmdPlan(positional, flags) {
  const lane = laneName(positional[0]);
  const branch = branchName(lane);
  const wt = wtPath(lane);
  if (!listSandboxes().some((r) => r.lane === lane)) die(`no sandbox for lane '${lane}'`);

  // Base = explicit --base, else the merge-base of the lane branch with HEAD.
  const base = flags.base || gitSafe(['merge-base', 'HEAD', branch]) || 'HEAD';
  const nameStatus = gitSafe(['diff', '--name-only', `${base}...${branch}`]) || '';
  const files = nameStatus.split('\n').filter(Boolean);
  const statLines = gitSafe(['diff', '--stat', `${base}...${branch}`]) || '(no diff)';

  console.log(`═══ MERGE PLAN (DRY-RUN) — lane '${lane}' ═══`);
  console.log(`  worktree:  ${wt}`);
  console.log(`  branch:    ${branch}`);
  console.log(`  base:      ${base}`);
  console.log('');
  console.log('  Changed files:');
  if (!files.length) {
    console.log('    (none — lane has no committed changes vs base)');
  } else {
    const buckets = { 'HARNESS-BLOCKED': [], 'RED-LINE': [], PROTECTED: [], plain: [] };
    for (const f of files) buckets[classify(f)].push(f);
    for (const f of files) {
      const c = classify(f);
      const mark = c === 'plain' ? '   ' : c === 'HARNESS-BLOCKED' ? '🛑' : c === 'RED-LINE' ? '🔴' : '🔒';
      console.log(`    ${mark} [${c.padEnd(15)}] ${f}`);
    }
    console.log('');
    console.log(`  Summary: ${files.length} file(s) — ` +
      `${buckets['RED-LINE'].length} red-line, ${buckets.PROTECTED.length} protected, ` +
      `${buckets['HARNESS-BLOCKED'].length} harness-blocked, ${buckets.plain.length} plain.`);
    if (buckets['HARNESS-BLOCKED'].length) {
      console.log('  🛑 HARNESS-BLOCKED files present — .claude/.github must NEVER come from a sandbox. Reject the lane.');
    }
    if (buckets['RED-LINE'].length) {
      console.log('  🔴 RED-LINE files present — merge additionally requires the standing HUMAN approval');
      console.log('     (serious-gate Council clearance / red-line-doubt-gate human window / operator sign-off).');
    }
  }
  console.log('');
  console.log('  ⛔ This script does NOT merge. To land this lane, run the GATE, then Ship Discipline:');
  console.log('     1) GATE: opus rubric + invariant-guardian + security-sentinel over the diff (checklist below)');
  console.log('     2) pnpm typecheck && pnpm build green on the lane; lane proof present (Playwright / request.* / red→green)');
  console.log('     3) ETHICS (Charter) green — hard, non-removable');
  console.log('     4) red-line? confirm human approval exists · then commit → staging → validate → merge');
  console.log('');
  printChecklist();
  console.log(`\n  Diff stat:\n${statLines.split('\n').map((l) => '    ' + l).join('\n')}`);
}

function printChecklist() {
  console.log(`  ─── GATE RUBRIC (every box green to MERGE; any red → REJECT → back to the sandbox) ───
  A. QUALITY
     [ ] pnpm typecheck green on the lane (output pasted)
     [ ] pnpm build green on the lane (output pasted)
     [ ] tests green + Mandatory Proof Rule (UI→Playwright toBeVisible/toContainText; API→request.* assertion)
     [ ] bug fix ⇒ red→green guardrail exists + REGRESSION-LEDGER.md row
     [ ] NO false-green (no skip/.only/fixme/inflated-timeout/expect(true)/commented assertion/rewrite-to-pass)
  B. SAFETY / SECURITY
     [ ] invariant-guardian VERDICT: PASS (or every FLAG resolved) — state-machine/money-integer/RLS/PII/idempotency/RS256/uuid/secrets
     [ ] security-sentinel VERDICT: PASS (or every finding resolved) — secrets/injection/authz/PII-egress/crypto
     [ ] red-line diff ⇒ standing HUMAN approval present (gate CHECKS it, never mints it)
     [ ] protect-paths re-applied to the merge diff — no protected file lands without approval
  C. ETHICS  (hard · non-removable · no override file · no timeout)
     [ ] no military / warfare / weapons / targeting / surveillance-for-harm use (Charter §1)
     [ ] no violence-as-only-solution framing; no commons-capture; not turned against the people it learned from (Charter §2–4)
     [ ] no PII/secret/cookie/floating-money introduced; external input Zod-parsed
     ►  ethics red ⇒ immediate REJECT + human escalation; NO test-green overrides it.`);
}

function cmdChecklist() {
  console.log('Sandbox-Swarm-Gate — GATE rubric (docs/design/harness/SANDBOX-SWARM-GATE.md §4)\n');
  printChecklist();
}

// ── main ─────────────────────────────────────────────────────────────────────────────────────
function main() {
  const argv = process.argv.slice(2);
  const { flags, positional } = parseFlags(argv);
  const cmd = positional.shift();

  if (!cmd || flags.help || cmd === 'help') {
    console.log(HELP);
    return;
  }
  switch (cmd) {
    case 'new':
      return cmdNew(positional, flags);
    case 'list':
      return cmdList();
    case 'rm':
      return cmdRm(positional, flags);
    case 'plan':
      return cmdPlan(positional, flags);
    case 'checklist':
      return cmdChecklist();
    default:
      die(`unknown command '${cmd}' — run with --help`);
  }
}

main();
