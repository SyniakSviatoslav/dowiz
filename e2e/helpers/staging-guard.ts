// requireStaging — a hard guard for tests that MUTATE state (place orders, seed couriers,
// drive the lifecycle). The sweep found ~157 findings where a spec defaulted BASE to the
// PROD host and/or used the dev/mock-auth backdoor — i.e. a test run could write to prod.
// Call this in beforeAll of any mutating e2e so it FAILS FAST against prod/unknown targets.

const PROD = /dowiz\.(fly\.dev|app|org)/;

/** Throw unless `base` is an explicit non-prod (staging/local) target. */
export function requireStaging(base: string | undefined): void {
  if (!base) {
    throw new Error('requireStaging: VITE_BASE_URL is unset — refusing to run a mutating test against an unknown target');
  }
  if (PROD.test(base) && !base.includes('staging')) {
    throw new Error(`requireStaging: refusing to run a mutating test against PROD (${base}) — set VITE_BASE_URL to staging`);
  }
}

/**
 * True when `base` is the live PROD host (dowiz.fly.dev / .app / .org, but NOT the
 * `dowiz-staging.*` host). Use this to CONDITIONALLY SKIP mutating/auth-dependent tests
 * against prod — `test.skip(isProdTarget(BASE), '…')` — so a post-deploy smoke run reports
 * them as *skipped* (green) rather than throwing (red). The read-only subset still runs.
 * The mirror of `requireStaging`'s prod check, exposed as a predicate for per-test gating.
 */
export function isProdTarget(base: string | undefined): boolean {
  return !!base && PROD.test(base) && !base.includes('staging');
}
