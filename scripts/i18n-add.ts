#!/usr/bin/env -S node --import tsx
/**
 * i18n-add.ts — add a translation key in ONE step (governance Tier 1).
 *
 * Replaces the old 3-edits-in-a-3000-line-file churn: writes one key into the single source of
 * truth (packages/ui/src/lib/i18n-catalog.ts) with all locales together. Any locale you don't
 * supply is inserted as a `TODO:` draft, which the parity gate (scripts/i18n-parity.ts) REFUSES to
 * ship — so a key can't be merged half-translated.
 *
 * Usage:
 *   pnpm exec tsx scripts/i18n-add.ts <key> "<english>" ["<albanian>"] ["<ukrainian>"]
 * Examples:
 *   pnpm exec tsx scripts/i18n-add.ts order.refunded "Order refunded"
 *   pnpm exec tsx scripts/i18n-add.ts order.refunded "Order refunded" "Porosia u rimbursua" "Кошти повернено"
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';

const CATALOG = 'packages/ui/src/lib/i18n-catalog.ts';
const [, , key, en, sq, uk] = process.argv;

if (!key || !en) {
  console.error('Usage: tsx scripts/i18n-add.ts <key> "<english>" ["<albanian>"] ["<ukrainian>"]');
  process.exit(2);
}

const text = readFileSync(CATALOG, 'utf8');
const needle = `${JSON.stringify(key)}:`;
if (text.includes(needle)) {
  console.error(`Key already exists: ${key} — edit it in ${CATALOG} instead.`);
  process.exit(1);
}

const draft = (v: string | undefined) => (v && v.trim() ? v : `TODO: ${en}`);
const entry =
  `  ${JSON.stringify(key)}: { ` +
  `en: ${JSON.stringify(en)}, ` +
  `sq: ${JSON.stringify(draft(sq))}, ` +
  `uk: ${JSON.stringify(draft(uk))} },\n`;

// Insert before the final closing `};` of the catalog object.
const close = text.lastIndexOf('};');
if (close === -1) {
  console.error(`Could not find catalog close in ${CATALOG}`);
  process.exit(1);
}
writeFileSync(CATALOG, text.slice(0, close) + entry + text.slice(close));

try {
  execSync(`pnpm exec prettier --write ${CATALOG}`, { stdio: 'ignore' });
} catch {
  /* prettier optional — gate + typecheck still run */
}

const pending = [!sq?.trim() && 'sq', !uk?.trim() && 'uk'].filter(Boolean);
console.log(`Added "${key}".`);
if (pending.length) {
  console.log(`⚠  ${pending.join(', ')} left as TODO draft — translate before commit, or the parity gate fails.`);
}
