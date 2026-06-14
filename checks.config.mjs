// metric-core/checks.config.mjs
// The deterministic signals that make up the metric core (executable truth).
// Wire each `cmd` to YOUR actual command. `gate: "hard"` fails the run; "soft" is advisory.
// Order is the run order (serial — deterministic, no resource contention with Playwright).

export const CHECKS = [
  // Types & lint
  { name: "tsc", cmd: "npx tsc --noEmit", gate: "hard", timeoutMs: 120_000 },
  { name: "lint", cmd: "npx eslint .", gate: "soft", timeoutMs: 120_000 },

  // Contract-map conformance — TODO: replace with your real contract-check command.
  { name: "contracts", cmd: "npm run check:contracts", gate: "hard", timeoutMs: 120_000 },

  // Your two guardrail skills' deterministic checkers (executable invariants).
  // TODO: confirm the installed skill paths in your repo.
  {
    name: "money-contract",
    cmd: "node .opencode/skills/deliveryos-money-contract/scripts/check-money.mjs",
    gate: "hard",
    timeoutMs: 60_000,
  },
  {
    name: "rls-isolation",
    cmd: "node .opencode/skills/deliveryos-rls-tenant-isolation/scripts/check-rls.mjs",
    gate: "hard",
    timeoutMs: 60_000,
  },

  // The launch-gating smoke — runs against STAGING (E2E_BASE_URL must be set in env).
  { name: "playwright-convergence", cmd: "npx playwright test", gate: "hard", timeoutMs: 600_000 },
];
