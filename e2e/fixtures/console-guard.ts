/**
 * Sense 2 · Console / Runtime stream guard (Non-Pixel Verification Net).
 *
 * Reads what pixels and vision models physically cannot: runtime errors,
 * swallowed fetches, React warnings, and — the bullseye — SSR↔hydration
 * mismatches from the `spa-proxy` hot path. Hydration flashes vanish before a
 * screenshot is taken; this fixture catches them deterministically.
 *
 * 🔴 IRON RULE: console.error / console.warning / pageerror = HARD FAIL.
 *    Hydration patterns are NEVER allowlisted — fix the cause, not the symptom.
 *    The allowlist is for genuinely benign third-party noise only; keep it empty
 *    as long as humanly possible (routing > allowlisting).
 *
 * Usage: `import { test, expect } from '../fixtures/console-guard';` — the guard
 * is an `auto` fixture, so every test in the file is protected with zero calls.
 * For red-proof / opt-out specs, import { attachConsoleGuard } and assert manually.
 */
import { test as base, expect, type Page } from '@playwright/test';

/** Genuinely-benign, non-app noise only. Each entry needs a written reason. */
const ALLOW: RegExp[] = [
  // (intentionally empty — discover during the sweep, route the cause, don't mute)
];

/** Hydration desync signatures — ALWAYS fail, regardless of ALLOW. */
const HYDRATION = [
  'Hydration failed',
  'did not match',
  'Text content does not match',
  'server rendered HTML',
  'Minified React error #418', // text content mismatch
  'Minified React error #423', // hydration error
  'Minified React error #425', // text content does not match server-rendered HTML
];

export interface ConsoleGuard {
  errors: string[];
  /** Throws (via expect) if anything illegal was captured. */
  assertClean: () => void;
}

/**
 * Attach console/runtime listeners to a page and return the live error buffer.
 * Standalone so red-proof specs can assert detection without self-failing.
 */
export function attachConsoleGuard(page: Page): ConsoleGuard {
  const errors: string[] = [];
  page.on('console', (m) => {
    const t = m.type();
    if (t !== 'error' && t !== 'warning') return;
    const txt = m.text();
    const isHydration = HYDRATION.some((h) => txt.includes(h));
    if (!isHydration && ALLOW.some((re) => re.test(txt))) return; // hydration never allow-listed
    errors.push(`[${t}] ${txt}`);
  });
  page.on('pageerror', (e) => errors.push(`[pageerror] ${e.message}`));
  return {
    errors,
    assertClean() {
      expect(errors, `Console/runtime not clean:\n${errors.join('\n')}`).toEqual([]);
    },
  };
}

export const test = base.extend<{ consoleGuard: ConsoleGuard }>({
  consoleGuard: [
    async ({ page }, use) => {
      const guard = attachConsoleGuard(page);
      await use(guard);
      guard.assertClean();
    },
    { auto: true },
  ],
});

export { expect };
