/**
 * verify:all — composite pre-deploy verification gate
 * Runs in order: env → db → rls → secrets → lint → typecheck → migrations → launch
 * Exits 1 at first failure.
 */
import { execSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const MIGRATIONS_DIR = join(ROOT, 'packages/db/migrations');

const STEPS = [
  { name: 'verify:env', cmd: 'pnpm verify:env' },
  { name: 'verify:db', cmd: 'pnpm verify:db' },
  { name: 'verify:rls', cmd: 'pnpm verify:rls' },
  { name: 'verify:secrets', cmd: 'pnpm verify:secrets' },
  { name: 'migration ordering', cmd: `tsx scripts/verify-migrations.ts` },
  { name: 'lint', cmd: 'pnpm lint' },
  { name: 'typecheck', cmd: 'pnpm typecheck' },
  { name: 'i18n coverage', cmd: `tsx apps/api/scripts/verify-i18n-coverage.ts` },
  { name: 'contrast audit', cmd: `tsx apps/api/scripts/verify-contrast.ts` },
  { name: 'event wiring (NX)', cmd: 'pnpm verify:event-wiring' },
  { name: 'connection lifecycle (NX)', cmd: 'pnpm verify:connection-lifecycle' },
  { name: 'owner active-membership (ADR-0004 P-d)', cmd: 'pnpm guardrail:owner-active' },
  { name: 'spike boundary (operating-model)', cmd: 'pnpm guardrail:spike-boundary' },
];

function run(cmd: string): { ok: boolean; output: string } {
  try {
    const output = execSync(cmd, { cwd: ROOT, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'], timeout: 120000 });
    console.log(output.trim());
    return { ok: true, output };
  } catch (e: any) {
    return { ok: false, output: e.stdout || e.message };
  }
}

async function main() {
  console.log('══════════════════════════════════════════');
  console.log('  verify:all — full pre-deployment gate');
  console.log('══════════════════════════════════════════\n');

  let failures = 0;

  for (const step of STEPS) {
    process.stdout.write(`[${step.name}] ... `);
    const result = run(step.cmd);
    if (result.ok) {
      console.log('✅');
    } else {
      console.log('❌');
      console.error(result.output.slice(0, 1000));
      failures++;
    }
  }

  console.log(`\n${failures === 0 ? '✅ ALL PASSED' : `❌ ${failures} FAILURES`}`);
  process.exit(failures > 0 ? 1 : 0);
}

main();
