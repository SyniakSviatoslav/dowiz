/**
 * verify:error-contract (ADR-0010, A2 guardrail) — STATIC, no DB/network.
 *
 * The A1 envelope made `code` the BE↔FE contract: the FE branches on SCREAMING_SNAKE machine
 * codes (CheckoutPage / apiClient), and the BE emits them verbatim. Nothing stops a future edit
 * from renaming a BE code string (breaking the FE branch silently) or "normalizing" a lowercase
 * business-outcome token (breaking the reasons[] consumer). This gate fails the build on either.
 *
 * It asserts:
 *   1. B1 — each FE-CONSUMED contract code still appears as a string literal in apps/api/src
 *      (renaming the emitter → red) AND in the FE branch site (renaming the consumer → red).
 *   2. B15 — the lowercase business-outcome token `item_unavailable` stays lowercase in BOTH
 *      preflight.ts (emitter) and CheckoutPage.tsx (consumer) — a sweep must never SCREAMING_SNAKE it.
 *   3. The envelope `code` field is SCREAMING_SNAKE-shaped where ApiError is constructed in routes.
 *
 * This is a coupling/stability gate, complementing the live-path e2e (error-contract.spec).
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../../..', import.meta.url));
const API_SRC = join(ROOT, 'apps/api/src');
const WEB_SRC = join(ROOT, 'apps/web/src');

function walk(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) walk(p, out);
    else if (/\.(ts|tsx)$/.test(p)) out.push(p);
  }
  return out;
}

const apiFiles = walk(API_SRC);
const apiBlob = apiFiles.map((f) => readFileSync(f, 'utf8')).join('\n');
const failures: string[] = [];

// 1. FE-consumed contract codes (Appendix A — break UX if renamed). Each must exist as a literal
//    in the API routes (the emitter) AND in the FE (the consumer).
const FE_CONSUMED: { code: string; feFile: string }[] = [
  { code: 'MIN_ORDER_NOT_MET', feFile: 'pages/client/CheckoutPage.tsx' },
  { code: 'CASH_AMOUNT_TOO_LOW', feFile: 'pages/client/CheckoutPage.tsx' },
  { code: 'NOT_DELIVERABLE', feFile: 'pages/client/CheckoutPage.tsx' },
  { code: 'SLUG_TAKEN', feFile: 'pages/MenuFirstOnboarding.tsx' },
  { code: 'UNSUPPORTED_TYPE', feFile: 'pages/MenuFirstOnboarding.tsx' },
];

for (const { code, feFile } of FE_CONSUMED) {
  if (!apiBlob.includes(`'${code}'`) && !apiBlob.includes(`"${code}"`)) {
    failures.push(`BE emitter missing contract code '${code}' — renamed without FE lockstep? (B1)`);
  }
  let feSrc = '';
  try {
    feSrc = readFileSync(join(WEB_SRC, feFile), 'utf8');
  } catch {
    failures.push(`FE consumer file not found: ${feFile} (moved? update this gate)`);
    continue;
  }
  if (!feSrc.includes(`'${code}'`) && !feSrc.includes(`"${code}"`)) {
    failures.push(`FE consumer ${feFile} no longer branches on '${code}' — contract drift (B1)`);
  }
}

// 2. B15 — `item_unavailable` is a lowercase business-outcome token (reasons[] namespace, OUTSIDE
//    the envelope). It must stay lowercase in emitter + consumer; a SCREAMING_SNAKE sweep breaks it.
const preflight = apiFiles.find((f) => f.endsWith('preflight.ts'));
if (!preflight || !readFileSync(preflight, 'utf8').includes(`'item_unavailable'`)) {
  failures.push(`preflight.ts must emit lowercase 'item_unavailable' (B15 — reasons[] namespace)`);
}
try {
  const checkout = readFileSync(join(WEB_SRC, 'pages/client/CheckoutPage.tsx'), 'utf8');
  if (!checkout.includes(`'item_unavailable'`)) {
    failures.push(`CheckoutPage.tsx must consume lowercase 'item_unavailable' (B15)`);
  }
  if (/MIN_ORDER_NOT_MET/.test(checkout) === false) {
    failures.push(`CheckoutPage.tsx lost the MIN_ORDER_NOT_MET branch (money gate — must not regress)`);
  }
} catch {
  failures.push(`CheckoutPage.tsx not found (moved? update this gate)`);
}

// 3. ApiError construction in routes uses a SCREAMING_SNAKE code (positional arg 2). Catch an
//    obviously non-contract code literal like a number or lowercase passed as the code.
const badApiError = apiBlob.match(/new ApiError\(\s*\d+\s*,\s*'([a-z][a-zA-Z]*)'/g);
if (badApiError) {
  failures.push(
    `ApiError constructed with a non-SCREAMING_SNAKE code: ${badApiError.join(', ')} (envelope.code must be SCREAMING_SNAKE — B1/B15)`,
  );
}

if (failures.length) {
  console.error('❌ verify:error-contract FAILED:\n' + failures.map((f) => `  - ${f}`).join('\n'));
  process.exit(1);
}
console.log(`✅ verify:error-contract: ${FE_CONSUMED.length} FE-consumed codes + item_unavailable namespace intact across BE↔FE.`);
