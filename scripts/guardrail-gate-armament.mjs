#!/usr/bin/env node
// Guardrail — the governance gates must actually be ARMED, not just registered.
//
// Regression (P0 gate-rearm, 2026-07-02): serious-gate.sh checked only `[ -s serious-cleared ]`,
// so a file of accumulated council slugs held the gate open for EVERY serious surface from
// 2026-06-21 to 2026-07-02 (last DENY 06-21, 400+ blind ALLOWs — money code, auth, a migration).
// red-line-doubt-gate.sh likewise honored a single 2026-06-23 confirmation for 9 days, and the
// Bash lane had no gate at all. Registration alone (guardrail-hook-matchers) can't catch this
// class: the hook runs but never denies. This simulates the hooks hermetically (fixture
// CLAUDE_PROJECT_DIR outside the repo) and asserts DENY/ALLOW semantics.
//
// Run: node scripts/guardrail-gate-armament.mjs
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, utimesSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const ROOT = process.cwd();
const HOOKS = join(ROOT, '.claude/hooks');
const FIX = mkdtempSync(join(tmpdir(), 'gate-armament-'));
const STATE = join(FIX, '.claude/state');
mkdirSync(STATE, { recursive: true });

const env = { ...process.env, CLAUDE_PROJECT_DIR: FIX };
delete env.GIT_DIR;
delete env.GIT_WORK_TREE;

function runHook(hook, toolInput) {
  return spawnSync('bash', [join(HOOKS, hook)], {
    input: JSON.stringify({ tool_input: toolInput }),
    cwd: FIX, // outside any git repo → hook resolves ROOT via CLAUDE_PROJECT_DIR
    env,
    encoding: 'utf8',
    timeout: 15000,
  });
}
// jq emits compact JSON, the python3 fallback emits `"key": "value"` with a space — accept both.
const denies = (r) => /"permissionDecision":\s*"deny"/.test(r.stdout || '');

const failures = [];
function check(name, ok, detail) {
  if (ok) console.log(`  ✓ ${name}`);
  else { console.error(`  ✗ ${name}${detail ? ` — ${detail}` : ''}`); failures.push(name); }
}

const now = Math.floor(Date.now() / 1000);
const seriousFile = { file_path: join(FIX, 'apps/api/src/routes/payments.ts') }; // matches SERIOUS (payment)
const clearedPath = join(STATE, 'serious-cleared');

// ── serious-gate.sh ─────────────────────────────────────────────────────────
console.log('serious-gate.sh:');
writeFileSync(clearedPath, 'old-slug-one\nold-slug-two\n'); // legacy bare slugs (the 06-21→07-02 hole)
check('legacy bare-slug clearance does NOT open the gate (DENY)', denies(runHook('serious-gate.sh', seriousFile)));

writeFileSync(clearedPath, `expired-slug|${now - 10}\n`);
check('expired clearance does NOT open the gate (DENY)', denies(runHook('serious-gate.sh', seriousFile)));

writeFileSync(clearedPath, `fresh-slug|${now + 3600}\n`);
const fresh = runHook('serious-gate.sh', seriousFile);
check('fresh slug|expiry clearance opens the gate (ALLOW)', fresh.status === 0 && !denies(fresh));
const log = join(FIX, '.claude/logs/classification.log');
check('ALLOW is logged with the clearing slug', existsSync(log) && readFileSync(log, 'utf8').includes('cleared(fresh-slug)'));

writeFileSync(clearedPath, '');
const nonSerious = runHook('serious-gate.sh', { file_path: join(FIX, 'apps/web/src/components/Footer.tsx') });
check('non-serious surface passes without clearance (ALLOW)', nonSerious.status === 0 && !denies(nonSerious));

// ── red-line-doubt-gate.sh ──────────────────────────────────────────────────
console.log('red-line-doubt-gate.sh:');
const migration = { file_path: join(FIX, 'packages/db/migrations/999_test.sql') };
const confirmPath = join(STATE, 'redline-confirmed');
rmSync(confirmPath, { force: true });
check('irreversible migration without confirmation (DENY)', denies(runHook('red-line-doubt-gate.sh', migration)));

writeFileSync(confirmPath, 'test confirmation');
const confirmed = runHook('red-line-doubt-gate.sh', migration);
check('fresh (<60min) confirmation releases the gate (ALLOW)', confirmed.status === 0 && !denies(confirmed));

const twoHoursAgo = new Date(Date.now() - 2 * 3600 * 1000);
utimesSync(confirmPath, twoHoursAgo, twoHoursAgo);
check('stale (>60min) confirmation does NOT release the gate (DENY)', denies(runHook('red-line-doubt-gate.sh', migration)));

// ── guard-bash.sh (blocking = exit 2) ───────────────────────────────────────
console.log('guard-bash.sh:');
const bash = (command) => runHook('guard-bash.sh', { command });
check('sed -i into .claude/hooks blocked (exit 2)', bash('sed -i s/a/b/ .claude/hooks/serious-gate.sh').status === 2);
check('redirect into .github blocked (exit 2)', bash('echo x > .github/workflows/ci.yml').status === 2);
check('agent writing its own gate override blocked (exit 2)', bash('echo bypass > .claude/state/serious-override').status === 2);
check('git push origin main blocked (exit 2)', bash('git push origin main').status === 2);
check('prod fly deploy blocked (exit 2)', bash('flyctl deploy --remote-only').status === 2);
check('pnpm add blocked (exit 2)', bash('pnpm add leftpad').status === 2);
check('staging fly deploy allowed (Ship Discipline)', bash('flyctl deploy -a dowiz-staging --remote-only').status === 0);
check('plain command allowed', bash('ls -la apps/').status === 0);
check('read-only access to protected path allowed', bash('cat .claude/hooks/serious-gate.sh').status === 0);
check('stderr-scrub: hook run with 2>/dev/null allowed', bash('bash .claude/hooks/serious-gate.sh 2>/dev/null').status === 0);
check('feature-branch push allowed', bash('git push origin feat/some-branch').status === 0);

rmSync(FIX, { recursive: true, force: true });

if (failures.length) {
  console.error(`\n✗ guardrail-gate-armament: ${failures.length} case(s) failed — a governance gate is disarmed or over-blocking.`);
  process.exit(1);
}
console.log('\n✓ guardrail-gate-armament: all gates armed — stale clearances deny, fresh ones allow, Bash lane governed.');
