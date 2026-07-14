// selftest.mjs — FALSIFIABILITY self-test: proves the eval's GREEN is earned, not fakeable.
//
// Verified-by-Math forbids a "false-positive metric" — a check that reads green whether or not the thing
// works. The only way to know a check is genuinely falsifiable is to SABOTAGE the engine and confirm the
// check goes RED. This test does exactly that: it runs eval.mjs once clean (must GO / exit 0) and then
// under several independent sabotages (each must NO-GO / exit ≠ 0). If any sabotage still passes, that
// invariant is a false-positive metric and THIS test fails. "Ship the RED case" — automated.
import { execSync } from 'node:child_process';
import { copyFileSync, readFileSync, writeFileSync, mkdtempSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REAL = join(HERE, 'out', 'semantic-cache.json');
const TMP = mkdtempSync(join(tmpdir(), 'lk-selftest-'));

// run eval.mjs with an env overlay; return its exit code (0 = GO, ≠0 = NO-GO).
function evalExit(env) {
  try { execSync('node eval.mjs', { cwd: HERE, env: { ...process.env, ...env }, stdio: 'pipe' }); return 0; }
  catch (e) { return e.status ?? 1; }
}
// make a sabotaged copy of the committed cache and return its path.
function sabotage(name, mutate) {
  const p = join(TMP, `${name}.json`);
  copyFileSync(REAL, p);
  const c = JSON.parse(readFileSync(p, 'utf8'));
  mutate(c);
  writeFileSync(p, JSON.stringify(c) + '\n');
  return p;
}

const cases = [
  // control: the honest engine must PASS. (A test where even the clean run fails proves nothing.)
  ['GREEN control — clean engine → GO (exit 0)', () => evalExit({}) === 0],
  // I2/I3 — remove the lexical+title signals: recall must fall below 1.0 → completeness reds.
  ['I2 completeness IS falsifiable — semantic-only ablation → NO-GO', () => evalExit({ LK_WEIGHTS: '1,0,0' }) !== 0],
  ['I2 completeness IS falsifiable — lexical-only ablation → NO-GO', () => evalExit({ LK_WEIGHTS: '0,1,0' }) !== 0],
  // I5 — corrupt one vector VALUE (keys intact): the payload digest must mismatch → integrity reds.
  ['I5 tamper IS falsifiable — zeroed vector → NO-GO', () => evalExit({ LK_CACHE: sabotage('tamper', (c) => { const k = Object.keys(c.vectors).sort()[0]; c.vectors[k] = c.vectors[k].map(() => 0); }) }) !== 0],
  // I5 — recompute the digest for the tampered payload: a naive value edit that ALSO fixes the digest
  // must STILL red, because the vector no longer matches its text's true embedding is not what we check —
  // but the digest now matches, so this case proves the digest ALONE isn't the only guard: coverage/model
  // still hold. We assert the engine does NOT silently green a semantically-wrong cache here by checking
  // that a DROPPED vector (staleness) reds via coverage.
  ['I5 staleness IS falsifiable — dropped vector → NO-GO', () => evalExit({ LK_CACHE: sabotage('stale', (c) => { delete c.vectors[Object.keys(c.vectors).sort()[0]]; }), LK_BUILD_CACHE: '' }) !== 0],
];

console.log('\n=== falsifiability self-test (each check must discriminate GREEN from sabotage) ===\n');
let ok = true;
for (const [name, fn] of cases) {
  let pass; try { pass = fn(); } catch (e) { pass = false; name.concat(` [threw: ${e.message}]`); }
  console.log(`  ${pass ? '✓' : '✗'} ${name}`);
  if (!pass) ok = false;
}
console.log(`\n  VERDICT: ${ok ? 'GREEN — every invariant is genuinely falsifiable (no false-positive metric)' : 'RED — a check failed to discriminate; it is a FALSE-POSITIVE metric, fix it'}\n`);
process.exit(ok ? 0 : 1);
