#!/usr/bin/env tsx
/**
 * verify-i18n-coverage.ts
 * Extracts all t('key') calls from frontend source and cross-references against
 * all 3 locales (sq, en, uk) in the key-major SSoT packages/ui/src/lib/i18n-catalog.ts
 * (i18n.ts only DERIVES locale views from it — never parse i18n.ts for keys).
 * Exits 1 if any key is missing from any locale.
 */
import { readFileSync, readdirSync, statSync } from 'fs';
import { join, extname, basename } from 'path';

const UI_SRC = join(import.meta.dirname, '..', '..', '..', 'packages', 'ui', 'src');
const WEB_SRC = join(import.meta.dirname, '..', '..', '..', 'apps', 'web', 'src');
const I18N_FILE = join(UI_SRC, 'lib', 'i18n-catalog.ts');

interface Locales {
  sq: Record<string, string>;
  en: Record<string, string>;
  uk: Record<string, string>;
}

// Parse the key-major catalog: 'key.name': { en: '…', sq: '…', uk: '…' }.
// Import the module instead of regexing? The catalog is plain data, but this script
// must stay runnable without a TS build of packages/ui — so a line parser it is.
function parseI18nFile(filePath: string): Locales {
  const content = readFileSync(filePath, 'utf-8');
  const locales: Locales = { sq: {}, en: {}, uk: {} };

  let currentKey: string | null = null;
  for (const line of content.split('\n')) {
    // "'client.foo_bar': {"  — a catalog entry opener (possibly single-line entry)
    const keyMatch = line.match(/^\s*'([^']+)':\s*\{/);
    if (keyMatch) currentKey = keyMatch[1];
    if (!currentKey) continue;
    // locale props on the opener line ("{ en: '…', sq: '…', uk: '…' }") or on their own lines
    for (const loc of ['sq', 'en', 'uk'] as const) {
      const re = new RegExp(`(?:[{,]\\s*|^\\s*)${loc}:\\s*['"\`]`);
      if (re.test(line)) locales[loc][currentKey] = true as any;
    }
    // entry closes ("}," on its own line, or a single-line "'key': { ... },")
    if (/\},?\s*$/.test(line)) currentKey = null;
  }
  return locales;
}

function collectKeys(filePath: string, depth = 0): Set<string> {
  const keys = new Set<string>();
  if (depth > 10) return keys;
  try {
    const stat = statSync(filePath);
    if (stat.isDirectory()) {
      const entries = readdirSync(filePath);
      for (const entry of entries) {
        if (entry.startsWith('node_modules') || entry.startsWith('dist') || entry.startsWith('.')) continue;
        const child = join(filePath, entry);
        const subKeys = collectKeys(child, depth + 1);
        subKeys.forEach(k => keys.add(k));
      }
    } else if (stat.isFile()) {
      const base = basename(filePath);
      if (base.includes('.test.') || base.includes('.spec.') || base.endsWith('.d.ts')) return keys;
      const ext = extname(filePath);
      if (!['.ts', '.tsx', '.js', '.jsx'].includes(ext)) return keys;
      const content = readFileSync(filePath, 'utf-8');
      const re = /(?<![a-zA-Z0-9_])t\(\s*'([a-z][\w.]*)'/g;
      let m: RegExpExecArray | null;
      while ((m = re.exec(content)) !== null) {
        if (m[1].startsWith('http') || m[1].startsWith('/') || m[1].startsWith('.')) continue;
        if (/^[a-z]+$/.test(m[1]) && m[1].length < 4) continue;
        keys.add(m[1]);
      }
    }
  } catch { /* skip unreadable */ }
  return keys;
}

async function main() {
  console.log('\n=== i18n Coverage Verify ===\n');

  const locales = parseI18nFile(I18N_FILE);
  const sqKeys = Object.keys(locales.sq);
  const enKeys = new Set(Object.keys(locales.en));
  const ukKeys = new Set(Object.keys(locales.uk));

  console.log(`Total keys: sq=${sqKeys.length}, en=${enKeys.size}, uk=${ukKeys.size}\n`);

  // Check: keys present in sq must also be in en and uk
  let missingEn = 0;
  let missingUk = 0;
  for (const key of sqKeys) {
    if (!enKeys.has(key)) {
      if (missingEn === 0) console.log('Keys present in sq but MISSING from en:');
      console.log(`  ${key}`);
      missingEn++;
    }
    if (!ukKeys.has(key)) {
      if (missingUk === 0) console.log('Keys present in sq but MISSING from uk:');
      console.log(`  ${key}`);
      missingUk++;
    }
  }

  if (missingEn > 0 || missingUk > 0) {
    console.log(`\n❌ FAIL: ${missingEn} keys missing from en, ${missingUk} keys missing from uk`);
  } else {
    console.log('✅ All sq keys present in en and uk');
  }

  // Check: keys used in source code must exist in at least sq (canonical locale)
  console.log('\nScanning source code for t() calls...');
  const sourceDirs = [WEB_SRC, UI_SRC];
  const usedKeys = new Set<string>();
  for (const dir of sourceDirs) {
    const dirKeys = collectKeys(dir);
    dirKeys.forEach(k => usedKeys.add(k));
  }

  console.log(`\nFound ${usedKeys.size} unique t() keys in source code`);
  let missingFromSq = 0;
  for (const key of usedKeys) {
    if (!locales.sq[key]) {
      if (missingFromSq === 0) console.log('Keys used in code but MISSING from all locales:');
      console.log(`  ${key}`);
      missingFromSq++;
    }
  }

  if (missingFromSq > 0) {
    console.log(`\n❌ FAIL: ${missingFromSq} keys used in source but missing from i18n.ts`);
  } else {
    console.log('✅ All source-code keys have translations in sq');
  }

  const totalErrors = missingEn + missingUk + missingFromSq;
  console.log(`\n=== VERDICT: ${totalErrors === 0 ? 'PASS' : 'FAIL'} (${totalErrors} issues) ===\n`);
  process.exit(totalErrors > 0 ? 1 : 0);
}

main().catch(err => {
  console.error('i18n coverage check failed:', err);
  process.exit(2);
});
