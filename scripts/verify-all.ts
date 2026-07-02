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

// --ci: the static subset for GitHub Actions (no provisioned env/DB — verify:env/db/rls
// need real connection strings; lint/typecheck already run as separate CI steps).
const CI_MODE = process.argv.includes('--ci');

const STEPS = [
  { name: 'verify:env', cmd: 'pnpm verify:env', ci: false },
  { name: 'verify:db', cmd: 'pnpm verify:db', ci: false },
  { name: 'verify:rls', cmd: 'pnpm verify:rls', ci: false },
  { name: 'verify:secrets', cmd: 'pnpm verify:secrets', ci: true },
  { name: 'migration ordering', cmd: `tsx scripts/verify-migrations.ts`, ci: true },
  { name: 'lint', cmd: 'pnpm lint', ci: false },
  { name: 'typecheck', cmd: 'pnpm typecheck', ci: false },
  { name: 'i18n coverage', cmd: `tsx apps/api/scripts/verify-i18n-coverage.ts`, ci: true },
  { name: 'contrast audit', cmd: `tsx apps/api/scripts/verify-contrast.ts`, ci: true },
  { name: 'event wiring (NX)', cmd: 'pnpm verify:event-wiring', ci: true },
  { name: 'connection lifecycle (NX)', cmd: 'pnpm verify:connection-lifecycle', ci: true },
  { name: 'owner active-membership (ADR-0004 P-d)', cmd: 'pnpm guardrail:owner-active', ci: true },
  { name: 'spike boundary (operating-model)', cmd: 'pnpm guardrail:spike-boundary', ci: true },
  { name: 'deliver v2 parity + no-raw-cancel (ADR-deliver-v2)', cmd: 'pnpm guardrail:deliver-v2', ci: true },
  { name: 'adversarial-corpus reachability (tooling-integration-eval G1)', cmd: 'pnpm guardrail:corpus-reachability', ci: true },
  { name: 'license + forbidden-dep + env-classification (tooling-integration-eval G5)', cmd: 'pnpm guardrail:license', ci: true },
  { name: 'hook matchers cover Edit|Write|MultiEdit (no gate bypass)', cmd: 'pnpm guardrail:hook-matchers', ci: true },
  // Invoked directly (no package.json script needed — package.json is protect-paths): the SECURITY
  // DEFINER search_path gate was authored (ledger #33) but never wired here, so it never ran. Wired.
  { name: 'SECURITY DEFINER search_path pinned (pg-privilege-hardening, ledger #33)', cmd: 'node scripts/guardrail-definer-search-path.mjs', ci: true },
  // Governance gates must be ARMED, not just registered: stale clearance files held serious-gate +
  // red-line-gate open 06-21→07-02 and the Bash lane was ungoverned (P0 gate-rearm 2026-07-02).
  { name: 'gate armament (serious/red-line/bash hooks deny stale, allow fresh)', cmd: 'node scripts/guardrail-gate-armament.mjs', ci: true },
  // Meta-loop P2/P3 (2026-07-02): ledger "#N" refs were ambiguous (4 duplicated numbers) and the
  // router's machine registry covered 2 of 16 loops — both drifts are now gated.
  { name: 'ledger row-number integrity (unique #N refs)', cmd: 'node scripts/guardrail-ledger-integrity.mjs', ci: true },
  { name: 'loop registry sync (registry.md → runs/registry.json)', cmd: 'node scripts/loops-registry-sync.mjs --check', ci: true },
].filter((s) => !CI_MODE || s.ci);

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
