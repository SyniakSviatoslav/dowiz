#!/usr/bin/env node
// Guardrail — VERIFIED-BY-MATH (VbM): every proof the enforced armament suite relies on must be
// FALSIFIABLE. (Operator standing rule, 2026-07-07: "only verified with math is truly validated, no
// reliance on false/positive metrics." Key principles for ANY task verification —
//   1. does it work?   2. can it be proven with math?   3. can the math/proving/telemetry be falsified?)
//
// A proof that cannot fail is not a proof — it is a false-positive metric. This meta-armament enforces
// principle 3 on the harness's OWN proofs: it parses scripts/run-armaments.sh for every
// `node scripts/<x>.mjs` it runs (the proofs the pre-commit gate actually trusts) and asserts each:
//   • has a REACHABLE failure path  (process.exit(1|2)) — it CAN fail, and
//   • if it is an armament (uses check()/--self-test), it asserts at least one FAILURE outcome (a red
//     case: deny/block/exit-nonzero) — i.e. it is not an all-green tautology.
// Live-invariant guardrails (no check(), just exit-nonzero on a real violation) satisfy this via the
// failure path alone. See docs/operating-model/verified-by-math.md.
//
// This gate is itself falsifiable: `--self-test` proves it FLAGS a synthetic all-green proof and
// PASSES a red+green one.
//
// Run: node scripts/guardrail-falsifiable-proof.mjs   |   --self-test
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();

// A reachable non-zero exit → the proof can fail.
const RE_FAILPATH = /process\.exit\(\s*[12]\s*\)/;
// Armament pattern → it makes discrete pass/fail assertions.
const RE_ARMAMENT = /\bcheck\s*\(|--self-test|selfTest/;
// Broad "a FAILURE outcome is asserted" vocabulary. Broad on purpose: a false NEGATIVE (missing a
// real red case) is worse here than tolerating a proof that merely references failure semantics.
const RE_RED = /denies\(|blocks\(|nudges\(|=== *[12]\b|status === *[12]|exit ?[12]\b|→ *exit|\bDENY\b|\bdeny\b|\bblock\b|\bBLOCK\b|reject|REJECT|over-block|EXPIRED|MALFORMED|should fail|must red|toThrow|flagged|violation/;

// Verdict for one proof's source text.
function judge(src) {
  const hasFailPath = RE_FAILPATH.test(src);
  const isArmament = RE_ARMAMENT.test(src);
  const hasRed = RE_RED.test(src);
  const reasons = [];
  if (!hasFailPath) reasons.push('no reachable failure path (process.exit(1|2)) — it cannot fail, so it proves nothing');
  if (isArmament && !hasRed) reasons.push('armament asserts no FAILURE outcome (all-green) — a proof that cannot go red is a false-positive metric');
  return { falsifiable: reasons.length === 0, reasons, hasFailPath, isArmament, hasRed };
}

// The proofs the enforced suite runs = the ground-truth list (self-maintaining).
function enforcedProofs() {
  const arm = join(ROOT, 'scripts/run-armaments.sh');
  if (!existsSync(arm)) return [];
  const txt = readFileSync(arm, 'utf8');
  const set = new Set();
  for (const m of txt.matchAll(/node\s+(scripts\/[\w.-]+\.mjs)/g)) set.add(m[1]);
  return [...set];
}

function selfTest() {
  const failures = [];
  const ck = (name, ok) => { if (ok) console.log(`  ✓ ${name}`); else { console.error(`  ✗ ${name}`); failures.push(name); } };

  const allGreen = `const check=(n,ok)=>{}; check('works', foo.status === 0); check('also', bar === true); process.exit(0);`;
  const noFail = `console.log('everything is fine'); // never exits nonzero`;
  const falsifiable = `const check=(n,ok)=>{}; check('denies bad', denies(r)); check('allows good', !denies(r) && r.status === 0); if (bad) process.exit(1);`;
  const liveInvariant = `const errors=[]; if(x) errors.push('bad'); if(errors.length){ console.error('fail'); process.exit(1);} `;

  ck('all-green armament (only status===0, no red) → FLAGGED', judge(allGreen).falsifiable === false);
  ck('no failure path at all → FLAGGED', judge(noFail).falsifiable === false);
  ck('falsifiable armament (red + green + exit1) → PASSES', judge(falsifiable).falsifiable === true);
  ck('live-invariant guardrail (exit1 on violation, no check()) → PASSES', judge(liveInvariant).falsifiable === true);
  ck('enforcedProofs() finds the run-armaments list', enforcedProofs().length >= 5);

  if (failures.length) { console.error(`\n✗ guardrail-falsifiable-proof --self-test: ${failures.length} case(s) failed.`); process.exit(1); }
  console.log('\n✓ guardrail-falsifiable-proof --self-test: flags all-green/no-fail proofs, passes falsifiable + live-invariant ones.');
  process.exit(0);
}

if (process.argv.includes('--self-test')) selfTest();

const proofs = enforcedProofs();
const violations = [];
for (const rel of proofs) {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) { violations.push({ rel, reasons: ['referenced by run-armaments.sh but the file does not exist'] }); continue; }
  const v = judge(readFileSync(abs, 'utf8'));
  if (!v.falsifiable) violations.push({ rel, reasons: v.reasons });
}

if (violations.length) {
  console.error(`✗ guardrail-falsifiable-proof: ${violations.length} enforced proof(s) are NOT falsifiable (Verified-by-Math principle 3):`);
  for (const { rel, reasons } of violations) for (const r of reasons) console.error(`  - ${rel}: ${r}`);
  console.error('\nEvery enforced proof must be able to go RED. A proof that cannot fail is a false-positive metric. See docs/operating-model/verified-by-math.md.');
  process.exit(1);
}
console.log(`✓ guardrail-falsifiable-proof: all ${proofs.length} enforced proof(s) are falsifiable (have a red path; armaments assert a failure outcome).`);
