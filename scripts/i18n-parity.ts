#!/usr/bin/env -S node --import tsx
/**
 * i18n-parity.ts — the "never miss a translation" gate (governance Tier 0).
 *
 * The single source of truth is packages/ui/src/lib/i18n-catalog.ts (key-major: each key carries
 * { en, sq, uk }). At runtime a missing locale silently falls back to English/the raw key — so a
 * forgotten translation ships invisibly. This gate makes that a HARD failure:
 *   every catalog entry must carry a non-empty en + sq + uk, with no leftover TODO draft.
 *
 * Run:  pnpm exec tsx scripts/i18n-parity.ts
 * Exit: 0 = all present; 1 = gaps (lists every offending key:locale).
 */
import { catalog } from '../packages/ui/src/lib/i18n-catalog.ts';

const LOCALES = ['sq', 'en', 'uk'] as const;
const issues: string[] = [];

for (const [key, entry] of Object.entries(catalog)) {
  for (const loc of LOCALES) {
    const v = (entry as Record<string, string | undefined>)[loc];
    if (v === undefined || v === null || String(v).trim() === '') {
      issues.push(`MISSING  ${loc}  ${key}`);
    } else if (/\bTODO\b/i.test(String(v))) {
      issues.push(`TODO     ${loc}  ${key}  → ${v}`);
    }
  }
}

const total = Object.keys(catalog).length;
if (issues.length) {
  console.error(issues.join('\n'));
  console.error(`\ni18n parity FAIL: ${issues.length} issue(s) across ${total} keys × ${LOCALES.length} locales.`);
  console.error('Fix: add the missing/translated string in packages/ui/src/lib/i18n-catalog.ts.');
  process.exit(1);
}
console.log(`i18n parity OK: ${total} keys × ${LOCALES.length} locales — all present, no TODO drafts.`);
