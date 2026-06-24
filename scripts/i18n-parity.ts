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
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { catalog } from '../packages/ui/src/lib/i18n-catalog.ts';

const STRICT = process.argv.includes('--strict');
const LOCALES = ['sq', 'en', 'uk'] as const;
const issues: string[] = [];

// Check 1 (hard): every catalog entry carries a non-empty, non-TODO value in every locale.
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

// Check 2 (coverage): keys used in code via t('literal', …) but ABSENT from the catalog render
// their English fallback in EVERY locale — the silent-English gap (e.g. state.* was invisible
// until 2026-06-24). Reported as a warning (non-fatal unless --strict) since a backlog pre-exists.
const catalogKeys = new Set(Object.keys(catalog));
const usedKeys = new Set<string>();
function walk(dir: string) {
  for (const name of readdirSync(dir)) {
    if (name === 'node_modules' || name === 'dist') continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p);
    else if (/\.tsx?$/.test(name) && !p.endsWith('i18n-catalog.ts')) {
      const src = readFileSync(p, 'utf8');
      for (const m of src.matchAll(/\bt\(\s*'([^']+)'/g)) usedKeys.add(m[1]!);
    }
  }
}
for (const root of ['apps/web/src', 'packages/ui/src']) walk(root);
const usedButMissing = [...usedKeys].filter(k => !catalogKeys.has(k)).sort();
const total = Object.keys(catalog).length;

if (usedButMissing.length) {
  const tag = STRICT ? 'FAIL' : 'WARN';
  console.error(`\n[coverage ${tag}] ${usedButMissing.length} key(s) used in code via t('…') but absent from the catalog`);
  console.error("  → these render English in EVERY locale (silent gap). Backfill into i18n-catalog.ts:");
  console.error(usedButMissing.map(k => `    ${k}`).join('\n'));
}

if (issues.length) {
  console.error(issues.join('\n'));
  console.error(`\ni18n parity FAIL: ${issues.length} issue(s) across ${total} keys × ${LOCALES.length} locales.`);
  console.error('Fix: add the missing/translated string in packages/ui/src/lib/i18n-catalog.ts.');
  process.exit(1);
}
if (STRICT && usedButMissing.length) {
  console.error(`\ni18n parity FAIL (--strict): ${usedButMissing.length} used-but-uncatalogued key(s).`);
  process.exit(1);
}
console.log(`i18n parity OK: ${total} keys × ${LOCALES.length} locales — all present, no TODO drafts.` +
  (usedButMissing.length ? `  (${usedButMissing.length} uncatalogued code keys warned — see above)` : ''));
