#!/usr/bin/env node
/**
 * deliveryos-money-contract checker
 * Scans for violations of the money & rounding contract.
 * Exit code != 0 when violations found.
 */
import { readFileSync, existsSync, statSync } from 'fs';
import { join, resolve } from 'path';
import { glob } from 'node:fs/promises';

const REPO_ROOT = resolve(new URL('.', import.meta.url).pathname, '../../..', '..', '..');
const TARGET_PATH = process.argv[2] ? resolve(process.argv[2]) : null;

// Files to scan
const SRC_PATTERNS = [
  'apps/api/src/**/*.ts',
  'apps/web/src/**/*.{ts,tsx}',
  'packages/domain/src/**/*.ts',
  'packages/shared-types/src/**/*.ts',
  'packages/ui/src/**/*.{ts,tsx}',
  'packages/core/src/**/*.ts',
];

// Files to exclude
const EXCLUDE = [
  'node_modules',
  'dist',
  '.git',
  'graphify-out',
  '.agents',
  '.claude',
  'eval-viewer',
  '__tests__',
  '*.test.ts',
  '*.spec.ts',
  'scripts/check-money.mjs',
  'utils.ts', // formatMoney lives here — exempt
];

function shouldExclude(filePath) {
  return EXCLUDE.some(e => filePath.includes(e) || filePath.endsWith(e));
}

// Patterns that indicate violations
const VIOLATIONS = [
  {
    id: 'FLOAT_MONEY_FIELD',
    rule: 'Money fields must be integer (number) not float — never use decimal values for ALL amounts',
    test: (line, file) => {
      // Catches: `.toFixed(2)` on money fields, `amount.toFixed` outside formatMoney
      if (line.includes('.toFixed(') && !file.includes('utils.ts') && !file.includes('formatMoney')) {
        const moneyKeywords = ['price', 'total', 'subtotal', 'fee', 'tax', 'amount', 'cost', 'cash', 'payout', 'delivery_fee', 'discount'];
        if (moneyKeywords.some(k => line.toLowerCase().includes(k))) {
          return true;
        }
      }
      return false;
    }
  },
  {
    id: 'EUR_IN_ORDER_MATH',
    rule: 'EUR conversion result must never flow into subtotal/total/delivery_fee/tax or POST /orders payload',
    test: (line, file) => {
      // Catches `eurAmount` or `* rate` being assigned to a money field
      if (line.includes('* rate') || line.includes('*rate') || line.includes('eurAmount') || line.includes('eur_amount')) {
        const assignTargets = ['subtotal', 'total', 'delivery_fee', 'tax', 'price', 'amount'];
        const isViolation = assignTargets.some(t => {
          const idx = line.indexOf(t);
          if (idx === -1) return false;
          const suffix = line.substring(idx + t.length, idx + t.length + 5);
          return suffix.includes('=') || suffix.includes('return') || suffix.trim() === '';
        });
        if (isViolation) return true;
      }
      return false;
    }
  },
  {
    id: 'ADHOC_ROUNDING',
    rule: 'Rounding must go through formatMoney() in utils.ts — no ad-hoc Math.round on monetary values',
    test: (line, file) => {
      if (file.includes('utils.ts')) return false;
      if (line.includes('Math.round(') || line.includes('Math.floor(') || line.includes('Math.ceil(')) {
        const moneyKeywords = ['price', 'total', 'subtotal', 'fee', 'tax', 'amount', 'cost', 'money', 'cash', 'payout'];
        return moneyKeywords.some(k => line.toLowerCase().includes(k));
      }
      return false;
    }
  },
  {
    id: 'ADHOC_FORMATTING',
    rule: 'Price formatting must use formatMoney/formatALL/PriceDisplay/fmtPrice — never ad-hoc template literals',
    test: (line, file) => {
      if (file.includes('utils.ts') || file.includes('PriceDisplay')) return false;
      // Catches `${amount} ALL` or `${x} Lek` or similar
      if (line.match(/`.*\$\{.*\}\s*(ALL|Lek|lek|EUR|€)\s*`/)) return true;
      if (line.match(/`.*(ALL|Lek|lek|EUR|€)\s*\$\{.*\}\s*`/)) return true;
      return false;
    }
  },
];

async function scan() {
  const violations = [];
  const files = new Set();

  if (TARGET_PATH) {
    const stat = statSync(TARGET_PATH);
    if (stat.isDirectory()) {
      for (const pattern of ['**/*.ts', '**/*.tsx', '**/*.js', '**/*.mjs']) {
        for await (const file of glob(join(TARGET_PATH, pattern), { nodir: true })) {
          if (shouldExclude(file)) continue;
          files.add(file);
        }
      }
    } else {
      files.add(TARGET_PATH);
    }
  } else {
    for (const pattern of SRC_PATTERNS) {
      for await (const file of glob(join(REPO_ROOT, pattern), { nodir: true })) {
        const relPath = file.replace(REPO_ROOT, '').replace(/\\/g, '/');
        if (shouldExclude(relPath)) continue;
        files.add(file);
      }
    }
  }

  for (const filePath of files) {
    const relPath = filePath.replace(REPO_ROOT, '').replace(/\\/g, '/');
    try {
      const content = readFileSync(filePath, 'utf-8');
      const lines = content.split('\n');
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        for (const v of VIOLATIONS) {
          if (v.test(line, relPath)) {
            violations.push({
              file: relPath,
              line: i + 1,
              rule: v.rule,
              id: v.id,
              snippet: line.trim(),
            });
          }
        }
      }
    } catch (err) {
      // skip unreadable
    }
  }

  // Deduplicate
  const seen = new Set();
  const unique = violations.filter(v => {
    const key = `${v.file}:${v.line}:${v.id}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });

  const result = {
    passed: unique.length === 0,
    violations: unique,
    summary: {
      total: unique.length,
      byRule: {},
    },
  };

  for (const v of unique) {
    result.summary.byRule[v.id] = (result.summary.byRule[v.id] || 0) + 1;
  }

  console.log(JSON.stringify(result, null, 2));

  if (unique.length > 0) {
    process.exit(1);
  }
}

scan().catch(err => {
  console.error(JSON.stringify({ passed: false, error: err.message }));
  process.exit(1);
});
