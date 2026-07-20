/**
 * metric-core checks.config.mjs
 *
 * Declare every deterministic check that gates "done".
 * Edit the `cmd` strings to match your actual tool paths.
 *
 * Fields:
 *   name      – short identifier for the check
 *   gate      – "hard" (blocks exit 0) or "soft" (advisory only)
 *   cmd       – shell command to run (string, passed to exec)
 *   timeoutMs – max wall-clock time before the check is killed
 *   cwd       – working directory (defaults to repo root)
 *   preCheck  – optional: function(report) to validate the check's own JSON output
 */

import { fileURLToPath } from 'url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));

/**
 * Resolve a path relative to the metric-core/ directory.
 */
function p(relativePath) {
  return fileURLToPath(new URL(relativePath, import.meta.url));
}

export default [
  // ── TypeScript compiler ──────────────────────────────────────
  {
    name: 'tsc',
    gate: 'hard',
    cmd: 'pnpm typecheck',
    timeoutMs: 120_000,
  },

  // ── ESLint ───────────────────────────────────────────────────
  {
    name: 'lint',
    gate: 'soft',            // advisory — many pre-existing warnings
    cmd: 'pnpm lint',
    timeoutMs: 120_000,
  },

  // ── Money-contract guardrail ─────────────────────────────────
  {
    name: 'check-money',
    gate: 'hard',
    cmd: `node ${p('../.agents/skills/deliveryos-money-contract/scripts/check-money.mjs')}`,
    timeoutMs: 60_000,
  },

  // ── RLS / tenant-isolation guardrail ─────────────────────────
  {
    name: 'check-rls',
    gate: 'hard',
    cmd: `node ${p('../.agents/skills/deliveryos-rls-tenant-isolation/scripts/check-rls.mjs')}`,
    timeoutMs: 60_000,
  },

  // ── Contract-map conformance ─────────────────────────────────
  // Wire your actual contract-map check here, or remove this entry.
  {
    name: 'check-contracts',
    gate: 'soft',
    cmd: `echo "OK: contract-map check placeholder — wire npm run check:contracts"`,
    timeoutMs: 10_000,
  },

  // ── Playwright convergence smoke ─────────────────────────────
  {
    name: 'playwright-smoke',
    gate: 'hard',
    cmd: `npx playwright test --grep @smoke --reporter=list`,
    timeoutMs: 180_000,
    cwd: p('../e2e'),
  },

  // ── Environment variable coverage ────────────────────────────
  {
    name: 'verify-env',
    gate: 'hard',
    cmd: 'pnpm verify:env',
    timeoutMs: 30_000,
  },
];
