// meta-controller.test.mjs — proof for the L5 gated self-modification loop.
//
// Run:        node --test scripts/meta-controller.test.mjs
// RED proof:  delete any entry from IMMUTABLE in meta-controller.mjs (e.g. 'ethics-charter')
//             → the "every required immutable node is refused" test FAILS (the core has teeth).
//             Invert detectGaps' STALE_BEHIND check → the stale-sandbox arms FAIL.
//
// NOTE — unlike other harness scripts (plane-telemetry has a TEST_DISABLE_* toggle), this file
// has NO env switch that disables the immutable core. An override on the safety boundary would
// itself violate the Charter. The tests prove the predicate DISCRIMINATES (refuses immutable,
// allows non-immutable) — that discrimination is the red→green teeth, not a backdoor.
//
// All tests are pure (detectGaps/filterProposals/guard/isImmutable take data as args) — no fs, no git.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { IMMUTABLE, isImmutable, guard, detectGaps, filterProposals } from './meta-controller.mjs';

// The safety contract: these nodes MUST be immutable. Removing any from the core → this list
// still demands it → RED. This is the guardrail on the guardrail.
const REQUIRED_IMMUTABLE = [
  { id: 'ethics-charter', sample: 'CLAUDE.md' },
  { id: 'ethics-charter', sample: 'the Ethics Charter node' },
  { id: 'the-gate', sample: 'scripts/sandbox-swarm-gate.mjs' },
  { id: 'authority-hooks', sample: '.claude/hooks/protect-paths.sh' },
  { id: 'authority-hooks', sample: '.claude/hooks/red-line-doubt-gate.sh' },
  { id: 'agents-charter', sample: 'AGENTS.md' },
  { id: 'meta-safety-core', sample: 'scripts/meta-controller.mjs' },
];

test('immutable core: every required safety node is present and REFUSED', () => {
  for (const { id, sample } of REQUIRED_IMMUTABLE) {
    assert.ok(IMMUTABLE.some((n) => n.id === id), `IMMUTABLE must contain node '${id}'`);
    const node = isImmutable(sample);
    assert.ok(node, `'${sample}' must be immutable`);
    assert.equal(node.id, id, `'${sample}' should match node '${id}'`);
    assert.equal(guard(sample).verdict, 'REFUSE', `guard('${sample}') must REFUSE`);
  }
});

test('immutable core DISCRIMINATES — an ordinary node is NOT refused (predicate is not "refuse all")', () => {
  assert.equal(isImmutable('docs/foo.md'), null);
  assert.equal(isImmutable('packages/ui/src/voice/MicFab.tsx'), null);
  assert.equal(guard('packages/ui/src/voice/MicFab.tsx').verdict, 'REVIEW-GATE');
});

test('guard verdict ladder: red-line → REVIEW-HUMAN, ordinary → REVIEW-GATE', () => {
  assert.equal(guard('packages/db/migrations/999_x.sql').verdict, 'REVIEW-HUMAN');
  assert.equal(guard('apps/api/src/routes/auth/login.ts').verdict, 'REVIEW-HUMAN');
  assert.equal(guard('docs/anything.md').verdict, 'REVIEW-GATE');
});

test('SELF-MODIFICATION ATTEMPT is blocked: a gap targeting the Charter lands in refusals, never proposals', () => {
  const evilGap = {
    id: 'evil', kind: 'x', severity: 'high', title: 'rewrite the charter',
    evidence: {}, artifact: { kind: 'guardrail', target: 'CLAUDE.md', action: 'weaken §1' },
  };
  const gateGap = { ...evilGap, id: 'evil2', artifact: { ...evilGap.artifact, target: 'scripts/sandbox-swarm-gate.mjs' } };
  const okGap = { ...evilGap, id: 'ok', artifact: { kind: 'guardrail', target: 'scripts/guardrail-new.mjs', action: 'add' } };

  const { proposals, refusals } = filterProposals([evilGap, gateGap, okGap]);
  assert.equal(proposals.length, 1, 'only the additive-new-file gap may be proposed');
  assert.equal(proposals[0].id, 'ok');
  assert.equal(refusals.length, 2, 'both immutable-targeting gaps must be refused');
  assert.deepEqual(refusals.map((r) => r.node).sort(), ['ethics-charter', 'the-gate']);
});

test('detectGaps: STALE_SANDBOX fires on drift/at-risk, targets an ADDITIVE guard (not the gate)', () => {
  const gaps = detectGaps({
    sandboxes: [
      { lane: 'a', path: '.claude/worktrees/a', behind: 14, atRisk: 7 }, // stale + at-risk → high
      { lane: 'b', path: '.claude/worktrees/b', behind: 0, atRisk: 0 },  // clean → no gap
    ],
    ledger: { rows: 1, maxNum: 1, pending: [] }, reflections: [], events: { present: true, count: 9, ageDays: 0 },
  });
  const stale = gaps.filter((g) => g.id.startsWith('stale-sandbox:'));
  assert.equal(stale.length, 1, 'only the drifted/at-risk sandbox produces a gap');
  assert.equal(stale[0].severity, 'high');
  assert.equal(stale[0].artifact.target, 'scripts/guardrail-sandbox-staleness.mjs');
  assert.equal(isImmutable(stale[0].artifact.target), null, 'the proposed fix must be an additive, non-immutable node');
});

test('detectGaps: telemetry is HONEST — fresh does not fire, absent does', () => {
  const fresh = detectGaps({ sandboxes: [], ledger: { pending: [] }, reflections: [], events: { present: true, count: 5, ageDays: 0 } });
  assert.equal(fresh.filter((g) => g.id === 'stale-telemetry').length, 0, 'fresh telemetry must not cry wolf');

  const absent = detectGaps({ sandboxes: [], ledger: { pending: [] }, reflections: [], events: { present: false, count: 0, ageDays: Infinity } });
  const t = absent.find((g) => g.id === 'stale-telemetry');
  assert.ok(t && t.severity === 'high', 'absent telemetry must fire high');
});

test('detectGaps: unfilled-WHY reflection blocks the ratchet → its own gap', () => {
  const gaps = detectGaps({
    sandboxes: [], ledger: { pending: [] }, events: { present: true, ageDays: 0 },
    reflections: [
      { file: 'docs/reflections/INBOX/x.md', hasWhy: true, unfilled: true },
      { file: 'docs/reflections/INBOX/y.md', hasWhy: true, unfilled: false },
    ],
  });
  assert.ok(gaps.some((g) => g.id.startsWith('unfilled-why:')), 'unfilled WHY must surface');
  assert.ok(gaps.some((g) => g.id === 'unratcheted-reflections'), 'the filled one is unratcheted');
});

test('detectGaps: SKILL_DRAFT (skill-evolution layer) fires on uncertified drafts → additive target', () => {
  const gaps = detectGaps({
    sandboxes: [], ledger: { pending: [] }, reflections: [], events: { present: true, ageDays: 0, kinds: {} },
    skills: { proposedSkills: ['max-lanes-orchestration'], draftLoops: ['skill-evolution.yaml'] },
  });
  const g = gaps.find((x) => x.id === 'skill-drafts-uncertified');
  assert.ok(g, 'a drafted-but-uncertified capability must surface');
  assert.equal(isImmutable(g.artifact.target), null, 'the proposed target must be non-immutable');
});

test('detectGaps: TELEMETRY_FRICTION (telemetry layer) fires on a dominant deny kind; honest below threshold', () => {
  const base = { sandboxes: [], ledger: { pending: [] }, reflections: [], skills: { proposedSkills: [], draftLoops: [] } };
  const hot = detectGaps({ ...base, events: { present: true, ageDays: 0, kinds: { 'serious-gate-block': 44, edit: 900 } } });
  assert.ok(hot.some((g) => g.id === 'telemetry-friction:serious-gate-block'), 'a dominant deny/block kind is friction');
  assert.equal(hot.filter((g) => g.id === 'telemetry-friction:edit').length, 0, "an ordinary 'edit' kind is not friction");
  const cool = detectGaps({ ...base, events: { present: true, ageDays: 0, kinds: { 'gate-block': 3, other: 500 } } });
  assert.equal(cool.filter((g) => g.kind === 'friction').length, 0, 'below the count threshold must not fire');
});
