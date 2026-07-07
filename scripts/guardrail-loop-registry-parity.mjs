#!/usr/bin/env node
// Guardrail — loop-registry PARITY (fable-audit finding #4): a CERTIFIED loop must have its report
// artifact, and every cited path in the registry must exist. Certification without a report is
// dishonesty (finding #4/#14: two CERTIFIED rows said "звіт ВТРАЧЕНО" and rows cited 10 report files
// that don't exist). This makes the registry falsifiable: a lying CERTIFIED row goes RED.
//
// RULES (per loops/registry.md table row):
//   • CERTIFIED ⇒ the report cell names ≥1 EXISTING report file (not "—", not a deferred "(при cert)").
//   • Any cited report path (bare, not deferred) MUST exist.               (cited paths exist)
//   • Any cited card path (loops/*.yaml, not "—") MUST exist.
//   • DRAFT with report "—" or a deferred "(при cert)" marker is fine.
//
// Falsifiable: `--self-test` proves it FLAGS a lying-CERTIFIED row and a bogus citation, and PASSES a
// real-CERTIFIED row, a deferred DRAFT, and a clean DRAFT.
//
// Run: node scripts/guardrail-loop-registry-parity.mjs   |   --self-test
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const REG = join(ROOT, 'loops/registry.md');

function parseRows(md) {
  const rows = [];
  for (const line of md.split('\n')) {
    if (!line.trim().startsWith('|')) continue;
    const cells = line.split('|').map((c) => c.trim());
    // cells[0] is '' (leading pipe). Need id..report → indices 1..6.
    if (cells.length < 9) continue;
    const id = cells[1];
    if (!id || id === 'id' || /^:?-{2,}:?$/.test(id)) continue; // header / separator
    rows.push({ id, status: cells[4], card: cells[5], report: cells[6] });
  }
  return rows;
}

const paths = (cell, ext) => (cell.match(new RegExp(`[\\w./-]+\\.${ext}`, 'g')) || []);
const isDeferred = (cell) => /при\s*cert|when\s*cert/i.test(cell);
const isDash = (cell) => /^[—\-\s]*$/.test(cell);

function judge(rows, exists) {
  const violations = [];
  for (const r of rows) {
    // Anchor at the start of the status cell so a "DRAFT (was CERTIFIED…)" demotion note does NOT
    // count as certified (the status word is the FIRST token; parenthetical history is not status).
    const certified = /^\s*CERTIFIED\b/i.test(r.status);
    const reportPaths = paths(r.report, 'md').concat(paths(r.report, 'txt'));
    const deferred = isDeferred(r.report);

    if (certified) {
      const ok = !deferred && reportPaths.length > 0 && reportPaths.some(exists);
      if (!ok) violations.push(`[${r.id}] status=CERTIFIED but has no existing report artifact (report cell: "${r.report}") — a cert without its report is unfalsifiable. Demote to DRAFT or produce the report.`);
    }
    // cited (non-deferred) report paths must exist
    if (!deferred) for (const p of reportPaths) if (!exists(p)) violations.push(`[${r.id}] cites report "${p}" which does not exist. Blank the cell to "—" or create the file.`);
    // cited card paths must exist
    if (!isDash(r.card)) for (const p of paths(r.card, 'yaml')) if (!exists(p)) violations.push(`[${r.id}] cites card "${p}" which does not exist.`);
  }
  return violations;
}

function selfTest() {
  const md = `
| id | intent | version | статус | картка | звіт | пам'ять | тригер |
|---|---|---|---|---|---|---|---|
| good-cert | x | 1.0 | CERTIFIED | loops/good.yaml | loops/reports/good.md | — | /x |
| lying-cert | x | 1.0 | CERTIFIED | loops/l.yaml | — | — | /x |
| bogus-cite | x | 0.1 | DRAFT | loops/b.yaml | loops/reports/nope.md | — | /x |
| deferred | x | 0.1 | DRAFT | loops/d.yaml | loops/reports/later.md (при cert) | — | /x |
| clean-draft | x | 0.1 | DRAFT | loops/c.yaml | — | — | /x |
| demoted | x | 1.0 | DRAFT (was CERTIFIED; report lost) | loops/c.yaml | — | — | /x |
`;
  const present = new Set(['loops/good.yaml', 'loops/reports/good.md', 'loops/l.yaml', 'loops/b.yaml', 'loops/d.yaml', 'loops/c.yaml']);
  const exists = (p) => present.has(p);
  const rows = parseRows(md);
  const v = judge(rows, exists);
  const failures = [];
  const ck = (name, ok) => { if (ok) console.log(`  ✓ ${name}`); else { console.error(`  ✗ ${name}`); failures.push(name); } };

  ck('parses 6 data rows (skips header + separator)', rows.length === 6);
  ck('flags lying CERTIFIED row (report "—")', v.some((x) => x.includes('[lying-cert]')));
  ck('flags DRAFT citing a nonexistent report', v.some((x) => x.includes('[bogus-cite]')));
  ck('does NOT flag real-CERTIFIED row', !v.some((x) => x.includes('[good-cert]')));
  ck('does NOT flag deferred "(при cert)" DRAFT', !v.some((x) => x.includes('[deferred]')));
  ck('does NOT flag clean DRAFT (report "—")', !v.some((x) => x.includes('[clean-draft]')));
  ck('does NOT flag "DRAFT (was CERTIFIED…)" demotion note (anchor guard)', !v.some((x) => x.includes('[demoted]')));
  ck('exactly 2 violations total', v.length === 2);

  // clean registry → zero violations (the green side).
  const clean = md.split('\n').filter((l) => !/lying-cert|bogus-cite/.test(l)).join('\n');
  ck('clean registry → 0 violations', judge(parseRows(clean), exists).length === 0);

  if (failures.length) { console.error(`\n✗ guardrail-loop-registry-parity --self-test: ${failures.length} case(s) failed.`); process.exit(1); }
  console.log('\n✓ guardrail-loop-registry-parity --self-test: flags lying certs + bogus citations, passes honest rows.');
  process.exit(0);
}

if (process.argv.includes('--self-test')) selfTest();

if (!existsSync(REG)) { console.error(`✗ guardrail-loop-registry-parity: ${REG} not found.`); process.exit(1); }
const violations = judge(parseRows(readFileSync(REG, 'utf8')), (p) => existsSync(join(ROOT, p)));
if (violations.length) {
  console.error(`✗ guardrail-loop-registry-parity: ${violations.length} registry parity violation(s):`);
  for (const v of violations) console.error('  - ' + v);
  console.error('\nA CERTIFIED loop must have its report; a cited path must exist. Fix loops/registry.md.');
  process.exit(1);
}
console.log('✓ guardrail-loop-registry-parity: every CERTIFIED row has an existing report; all cited paths exist.');
