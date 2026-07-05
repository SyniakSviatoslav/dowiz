/**
 * verify:menu-parse (ADR-0011 B1) — the cascade-swap regression gate. Deterministic, no network.
 *
 * Each fixture `fixtures/<name>.json` = { name, expected, actual }. `expected` is the hand-authored
 * golden parse of a real menu; `actual` is the parser's CURRENT output for that menu (regenerate it
 * after a model/cascade swap: run the importer on the source, paste its draft into `actual`). The
 * gate scores actual vs expected and exit(1)s if any fixture drops below thresholds — so a model swap
 * that degrades parsing cannot land silently. Does NOT replace the human draft-review (B9).
 *
 * Fixture set grows on each real-world miss: when a live menu mis-parses, add it as a fixture.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { scoreParse, THRESHOLDS, type EvalReport } from './scorer.js';

const FIX_DIR = fileURLToPath(new URL('./fixtures', import.meta.url));

export interface FixtureResult { name: string; report: EvalReport }

export function runEval(): FixtureResult[] {
  const files = readdirSync(FIX_DIR).filter((f) => f.endsWith('.json'));
  return files.map((f) => {
    const fx = JSON.parse(readFileSync(join(FIX_DIR, f), 'utf8'));
    return { name: fx.name ?? f, report: scoreParse(fx.expected, fx.actual) };
  });
}

// CLI entry (verify:menu-parse). Importing the module (the test) does not run this.
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const results = runEval();
  let failed = 0;
  for (const { name, report } of results) {
    const r = report;
    const line = `price ${(r.priceExact.rate * 100).toFixed(0)}% · recall ${(r.itemRecall.rate * 100).toFixed(0)}% · mods ${(r.modifierStructure.rate * 100).toFixed(0)}%`;
    if (report.pass) console.log(`✅ ${name.padEnd(24)} ${line}`);
    else { failed++; console.error(`❌ ${name.padEnd(24)} ${line}  →  ${report.failures.join('; ')}`); }
  }
  console.log(`\n${results.length} fixtures · thresholds price=${THRESHOLDS.priceExact * 100}% recall=${THRESHOLDS.itemRecall * 100}% mods=${THRESHOLDS.modifierStructure * 100}%`);
  if (failed > 0) { console.error(`\n❌ verify:menu-parse FAILED — ${failed} fixture(s) below threshold (cascade-swap regression?).`); process.exit(1); }
  console.log('✅ verify:menu-parse: all fixtures within thresholds.');
}
