import { test } from 'node:test';
import assert from 'node:assert/strict';
import { assessAdmissibility, designLoop, validateDesign, findExistingLoop } from '../src/loop-builder.js';

const REPO = process.cwd(); // /root/dowiz when tests run from the repo root

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
