#!/usr/bin/env node
// Guardrail — NO ORPHAN GUARDRAILS ("connect the islands", brain-in-brain 2026-07-07).
//
// A guardrail that no runner invokes is DEAD machinery: it "passes" by never running — a
// false-positive green, exactly what Verified-by-Math forbids. The living-knowledge cross-layer
// analysis surfaced infra guardrails that were islands; 3 were genuinely unrun (definer-search-path,
// no-set-cookie, sandbox-staleness — all real security/hygiene gates). This gate makes "orphan
// guardrail" a permanent red line: every scripts/guardrail-*.mjs MUST be referenced by at least one
// RUNNER (run-armaments.sh · .husky/* · package.json · .github/**). Connecting the islands, forever.
//
// Falsifiable: `--self-test` proves it FLAGS an unreferenced guardrail and PASSES a referenced one.
//
// Run: node scripts/guardrail-no-orphan-guardrails.mjs   |   --self-test
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { execSync } from 'node:child_process';

const ROOT = execSync('git rev-parse --show-toplevel').toString().trim();

// pure core (self-testable): which guardrail names are NOT present in the concatenated runner text.
function findOrphans(names, runnerText) {
  return names.filter((n) => !runnerText.includes(n));
}

function readIf(rel) { const p = join(ROOT, rel); return existsSync(p) ? readFileSync(p, 'utf8') : ''; }
function readDirFiles(rel) {
  const p = join(ROOT, rel); if (!existsSync(p)) return '';
  let out = '';
  for (const f of readdirSync(p)) { try { out += readFileSync(join(p, f), 'utf8') + '\n'; } catch { /* dir */ } }
  return out;
}

function selfTest() {
  const failures = [];
  const ck = (name, ok) => { if (ok) console.log(`  ✓ ${name}`); else { console.error(`  ✗ ${name}`); failures.push(name); } };
  const runner = 'node scripts/guardrail-alpha.mjs || exit 1\nrun ... node scripts/guardrail-beta.mjs';
  const orphans = findOrphans(['guardrail-alpha.mjs', 'guardrail-beta.mjs', 'guardrail-gamma.mjs'], runner);
  ck('flags an unreferenced guardrail (gamma)', orphans.includes('guardrail-gamma.mjs'));
  ck('passes referenced guardrails (alpha, beta)', !orphans.includes('guardrail-alpha.mjs') && !orphans.includes('guardrail-beta.mjs'));
  ck('exactly one orphan', orphans.length === 1);
  ck('all-referenced → zero orphans', findOrphans(['guardrail-alpha.mjs'], runner).length === 0);
  if (failures.length) { console.error(`\n✗ guardrail-no-orphan-guardrails --self-test: ${failures.length} failed.`); process.exit(1); }
  console.log('\n✓ guardrail-no-orphan-guardrails --self-test: flags unreferenced guardrails, passes referenced ones.');
  process.exit(0);
}

if (process.argv.includes('--self-test')) selfTest();

// runners = everywhere a guardrail can be invoked.
const runnerText = [
  readIf('scripts/run-armaments.sh'),
  readDirFiles('.husky'),
  readIf('package.json'),
  readDirFiles('.github/workflows'),
].join('\n');

const names = readdirSync(join(ROOT, 'scripts'))
  .filter((f) => /^guardrail-.*\.mjs$/.test(f) && !/\.test\.mjs$/.test(f)); // .test.mjs = a test OF a guardrail, not an enforced guardrail

const orphans = findOrphans(names, runnerText);
if (orphans.length) {
  console.error(`✗ guardrail-no-orphan-guardrails: ${orphans.length} guardrail(s) referenced by NO runner (dead machinery / false-positive green):`);
  for (const o of orphans) console.error(`  - scripts/${o}: wire it into run-armaments.sh / .husky/pre-commit / package.json / .github, or delete it (§7·B).`);
  console.error('\nAn unrun guardrail cannot fail → it validates nothing (Verified-by-Math). Connect the island.');
  process.exit(1);
}
console.log(`✓ guardrail-no-orphan-guardrails: all ${names.length} guardrails are wired to a runner (no islands).`);
