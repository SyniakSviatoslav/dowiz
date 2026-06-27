import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { assessAdmissibility, designLoop, validateDesign, findExistingLoop, runLoopBuilder, type LoopDesign } from '../src/loop-builder.js';

const REPO = process.cwd(); // /root/dowiz when tests run from the repo root

// Narrowing helper — extract a design (no `as any`) and fail loudly if a goal escalates.
function design(goal: string): LoopDesign {
  const out = designLoop(goal, REPO);
  assert.ok('design' in out, `${goal} should yield a design, not escalate`);
  return out.design;
}

test('admissibility — goals with a deterministic metric are admissible', () => {
  for (const g of ['BE polishing', 'QA loop', 'performance loop', 'i18n coverage loop']) {
    assert.equal(assessAdmissibility(g).admissible, true, `${g} should be admissible`);
  }
});

test('admissibility — subjective goals are REFUSED (no falsifiable metric → escalate)', () => {
  for (const g of ['make the UI prettier', 'make it feel faster', 'improve the architecture', 'better UX']) {
    const r = assessAdmissibility(g);
    assert.equal(r.admissible, false, `${g} must be inadmissible`);
    assert.match(r.reason, /metric|measurable|human/i);
  }
});

test('admissibility — fail-safe: an unknown goal with no template/metric → escalate (never guess)', () => {
  const r = assessAdmissibility('do the thing with the stuff');
  assert.equal(r.admissible, false);
  assert.match(r.reason, /no known template|measurable/i);
});

test('designLoop — BE polishing yields a deterministic oracle + the security carve-out', () => {
  const out = designLoop('BE polishing', REPO);
  assert.ok('design' in out);
  const d = (out as any).design;
  assert.match(d.oracle, /failing BE tests/i);
  assert.equal(d.scopeClass, 'A');
  assert.ok(d.carveOut.some((g: string) => /auth/.test(g)), 'auth carve-out present');
  assert.ok(d.carveOut.some((g: string) => /money/.test(g)), 'money carve-out present');
  assert.ok(d.breaker.maxIter > 0);
});

test('designLoop — a subjective goal escalates (no design)', () => {
  const out = designLoop('make it prettier', REPO);
  assert.ok('escalate' in out);
});

test('designLoop — reuse: QA / performance map to EXISTING loops (extend, not duplicate)', () => {
  // these cards exist in loops/
  assert.equal(findExistingLoop(REPO, 'performance'), 'performance');
  assert.equal(findExistingLoop(REPO, 'error-fix-convergence'), 'error-fix-convergence');
  const qa = designLoop('QA loop', REPO);
  assert.equal((qa as any).design.reuseOf, 'error-fix-convergence');
});

test('validateDesign — a well-formed design passes all structural checks', () => {
  const out = designLoop('BE polishing', REPO);
  const v = validateDesign((out as any).design);
  assert.equal(v.ok, true, JSON.stringify(v.checks.filter((c) => !c.pass)));
  assert.ok(v.checks.find((c) => c.name === 'security-carve-out')!.pass);
  assert.ok(v.checks.find((c) => c.name === 'sound-metric')!.pass);
});

test('validateDesign — not-duplicate carries an EXTEND directive (not a bare pass) when the design reuses an existing loop', () => {
  const d = design('QA loop');
  assert.equal(d.reuseOf, 'error-fix-convergence'); // reuse path active (not a new loop)
  const nd = validateDesign(d).checks.find((c) => c.name === 'not-duplicate')!;
  assert.equal(nd.pass, true);
  assert.match(nd.detail, /EXTEND existing loop "error-fix-convergence"/);
});

test('validateDesign — FAILS (ok=false) when the oracle is empty → sound-metric red', () => {
  const v = validateDesign({ ...design('BE polishing'), oracle: '' });
  assert.equal(v.ok, false);
  assert.equal(v.checks.find((c) => c.name === 'sound-metric')!.pass, false);
});

test('validateDesign — FAILS (ok=false) on a Class-A non-i18n design with no security carve-out', () => {
  const v = validateDesign({ ...design('BE polishing'), scopeClass: 'A', carveOut: [], goal: 'backend cleanup' });
  assert.equal(v.ok, false);
  assert.equal(v.checks.find((c) => c.name === 'security-carve-out')!.pass, false);
});

test('runLoopBuilder — admissible goal (register:false) designs + smoke-gates, queues a proposal, registers nothing', async () => {
  const baseDir = fs.mkdtempSync(path.join(os.tmpdir(), 'lb-int-'));
  const t = new Date().toISOString();
  const rec = await runLoopBuilder({ goal: 'i18n coverage loop', repoDir: REPO, baseDir, tStart: t, tEnd: t, register: false });

  assert.equal(rec.loop, 'loop-builder');
  assert.equal(rec.outcome, 'natural_stop');
  assert.match(rec.goal, /i18n coverage loop/);
  assert.match(rec.what_done, /DESIGNED a loop/);
  assert.match(rec.what_done, /Validation PASS/);
  assert.match(rec.what_done, /Smoke PASS/);
  assert.match(rec.what_done, /releasable — run with --register/); // register:false → not auto-registered

  // Side-effect proof: a sound-but-unregistered design goes to the human proposal queue, registry stays empty.
  assert.equal(fs.existsSync(path.join(baseDir, 'proposals.json')), true);
  assert.equal(fs.existsSync(path.join(baseDir, 'registry.json')), false);
});
