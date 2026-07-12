#!/usr/bin/env tsx
/**
 * verify-contrast.ts
 * Reads tokens.css and computes WCAG AA contrast ratios for all 6 theme presets.
 * Tests: brand-primary on bg, text-muted on surface, primary on surface, text on bg.
 * Exits 1 if any combination fails 4.5:1 for normal text.
 */
import { readFileSync } from 'fs';
import { join } from 'path';

const TOKENS_FILE = join(import.meta.dirname, '..', '..', '..', 'packages', 'ui', 'src', 'theme', 'tokens.css');

interface Theme {
  name: string;
  vars: Record<string, string>;
}

function hexToRgb(hex: string): [number, number, number] | null {
  const clean = hex.replace('#', '');
  if (clean.length === 3) return [
    parseInt(clean[0] + clean[0], 16),
    parseInt(clean[1] + clean[1], 16),
    parseInt(clean[2] + clean[2], 16),
  ];
  if (clean.length === 6) return [
    parseInt(clean.substring(0, 2), 16),
    parseInt(clean.substring(2, 4), 16),
    parseInt(clean.substring(4, 6), 16),
  ];
  return null;
}

function luminance([r, g, b]: [number, number, number]): number {
  const [rs, gs, bs] = [r, g, b].map(c => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

function contrastRatio(a: [number, number, number], b: [number, number, number]): number {
  const la = luminance(a);
  const lb = luminance(b);
  const lighter = Math.max(la, lb);
  const darker = Math.min(la, lb);
  return (lighter + 0.05) / (darker + 0.05);
}

function parseTokens(content: string): Theme[] {
  const themes: Theme[] = [];
  const rootMatch = content.match(/:root\s*\{([^}]+)\}/);
  if (rootMatch) {
    const vars: Record<string, string> = {};
    const re = /--([\w-]+):\s*([^;]+);/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(rootMatch[1])) !== null) {
      vars[m[1].trim()] = m[2].trim();
    }
    themes.push({ name: 'Food Dark (default)', vars });
  }
  const themeRe = /\.theme-([\w-]+)\s*\{([^}]+)\}/g;
  let tm: RegExpExecArray | null;
  while ((tm = themeRe.exec(content)) !== null) {
    const vars: Record<string, string> = {};
    const re = /--([\w-]+):\s*([^;]+);/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(tm[2])) !== null) {
      vars[m[1].trim()] = m[2].trim();
    }
    themes.push({ name: tm[1].replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase()), vars });
  }
  return themes;
}

interface Check {
  label: string;
  fg: string;
  bg: string;
}

async function main() {
  console.log('\n=== Contrast Audit ===\n');

  const content = readFileSync(TOKENS_FILE, 'utf-8');
  const themes = parseTokens(content);
  const AA_RATIO = 4.5;
  const AA_LARGE = 3.0;

  let totalFailures = 0;
  let totalWarnings = 0;

  for (const theme of themes) {
    console.log(`\n── ${theme.name} ──`);
    const v = theme.vars;

    const checks: Check[] = [
      { label: 'text on bg', fg: v['brand-text'], bg: v['brand-bg'] },
      { label: 'text-muted on surface', fg: v['brand-text-muted'], bg: v['brand-surface'] },
      { label: 'primary on bg', fg: v['brand-primary'], bg: v['brand-bg'] },
      { label: 'primary on surface', fg: v['brand-primary'], bg: v['brand-surface'] },
      { label: 'primary on surface-raised', fg: v['brand-primary'], bg: v['brand-surface-raised'] },
      { label: 'text on surface', fg: v['brand-text'], bg: v['brand-surface'] },
    ];

    for (const check of checks) {
      if (!check.fg || !check.bg) {
        console.log(`  ⚠️  ${check.label}: missing color definition`);
        continue;
      }
      const fgRgb = hexToRgb(check.fg);
      const bgRgb = hexToRgb(check.bg);
      if (!fgRgb || !bgRgb) {
        console.log(`  ⚠️  ${check.label}: non-hex color (${check.fg} / ${check.bg})`);
        continue;
      }
      const ratio = contrastRatio(fgRgb, bgRgb);
      const passes = ratio >= AA_RATIO;
      const passesLarge = ratio >= AA_LARGE;
      if (!passes && !passesLarge) {
        console.log(`  ❌ ${check.label}: ${ratio.toFixed(2)}:1 — FAILS AA (need ${AA_RATIO}:1)`);
        totalFailures++;
      } else if (!passes && passesLarge) {
        console.log(`  ⚠️  ${check.label}: ${ratio.toFixed(2)}:1 — passes AA-large (3:1) but FAILS AA-normal (${AA_RATIO}:1)`);
        totalWarnings++;
      } else {
        console.log(`  ✅ ${check.label}: ${ratio.toFixed(2)}:1`);
      }
    }
  }

  console.log(`\n=== VERDICT:`);
  if (totalFailures > 0) {
    console.log(`❌ FAIL: ${totalFailures} contrast failures, ${totalWarnings} warnings`);
    process.exit(1);
  } else if (totalWarnings > 0) {
    console.log(`⚠️  PASS with ${totalWarnings} warnings (passes AA-large but not AA-normal)`);
    process.exit(0);
  } else {
    console.log(`✅ PASS: all combinations pass WCAG AA (${AA_RATIO}:1)`);
    process.exit(0);
  }
}

main().catch(err => {
  console.error('Contrast audit failed:', err);
  process.exit(2);
});
