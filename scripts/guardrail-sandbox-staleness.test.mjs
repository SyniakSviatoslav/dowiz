// guardrail-sandbox-staleness.test.mjs — proof for the meta-controller-proposed staleness guard.
//
// Run:       node --test scripts/guardrail-sandbox-staleness.test.mjs
// RED proof: change isAtRisk's `&&` to `||` in guardrail-sandbox-staleness.mjs
//            → the "fresh WIP does not fire" test FAILS (the predicate would cry wolf on normal WIP).
//
// scanWorktrees hits real git read-only (worktree list / status); CI/clean clones have no
// sandbox worktrees, so the live test only asserts the row CONTRACT, never a count.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { isAtRisk, scanWorktrees, STALE_BEHIND } from './guardrail-sandbox-staleness.mjs';

test('isAtRisk: STALE + UNTRACKED → true (the lose-forever condition)', () => {
  assert.equal(isAtRisk({ behind: 14, untracked: 7 }), true);
});

test('isAtRisk: fresh worktree with untracked WIP → false (no crying wolf)', () => {
  assert.equal(isAtRisk({ behind: 0, untracked: 9 }), false);
});

test('isAtRisk: stale but only MODIFIED tracked files (git-recoverable) → false', () => {
  assert.equal(isAtRisk({ behind: 20, untracked: 0 }), false);
});

test('isAtRisk: boundary at STALE_BEHIND', () => {
  assert.equal(isAtRisk({ behind: STALE_BEHIND, untracked: 1 }), true);
  assert.equal(isAtRisk({ behind: STALE_BEHIND - 1, untracked: 1 }), false);
});

test('scanWorktrees: every row honours the contract (atRisk === isAtRisk(row))', () => {
  for (const r of scanWorktrees()) {
    assert.equal(typeof r.behind, 'number');
    assert.equal(typeof r.untracked, 'number');
    assert.equal(r.atRisk, isAtRisk(r), `row ${r.lane}: atRisk must equal the predicate`);
  }
});
