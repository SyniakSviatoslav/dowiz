import { test } from 'node:test';
import assert from 'node:assert/strict';
import { reactAction, hadRewrite, type ReactStep } from './reactAction.js';

// A fake ApiError shaped like apps/web/src/lib/apiClient.ts's ApiError (has .code / .status).
function apiErr(code: string, status = 403, message = code): Error {
  const e: any = new Error(message);
  e.code = code;
  e.status = status;
  return e as Error;
}

test('reactAction: a clean first attempt records exactly 1 visible step, ok=true', async () => {
  const res = await reactAction<number>({
    action: async ({ attempt }) => attempt, // succeeds on first try
  });
  assert.equal(res.error, undefined, 'no error on clean success');
  assert.ok(res.result !== undefined);
  assert.equal(res.trace.length, 1, 'exactly one visible step');
  assert.equal(res.trace[0].ok, true);
  assert.equal(res.trace[0].evalScore, 100);
  assert.equal(res.trace[0].rewrote, false);
});

test('reactAction: a denied owner cancel → system rewrite is VISIBLE (2 steps, hadRewrite=true)', async () => {
  // The action denies an OWNER-driven cancel, but a System rewrite clears it — proving the retry
  // promo demos hide is now auditable.
  let actor: 'owner' | 'system' = 'owner';
  const res = await reactAction<{ actor: string }>({
    action: async ({ attempt }) => {
      if (actor === 'owner') throw apiErr('CORRIDOR_BREACH_ACTOR_GATE');
      return { actor };
    },
    reflect: (_err, attempt) => {
      // Only the first attempt is owner; rewrite to system so the next attempt passes. Use the
      // attempt index to make sure this really is the first denial (not a blina retry).
      if (attempt === 1) {
        actor = 'system';
        return { actor: 'system' };
      }
      return null;
    },
  });
  assert.ok(res.result !== undefined, 'rewrite landed and the action succeeded');
  assert.equal(res.trace.length, 2, 'both iterations recorded — the retry is visible, not hidden');
  assert.equal(res.trace[0].ok, false, 'first attempt (owner) is the visible denial');
  assert.equal(res.trace[1].ok, true, 'second attempt (system) succeeds');
  assert.ok(hadRewrite(res.trace), 'the rewrite must be visible in the trace');
  assert.equal(res.trace[0].evalScore, 0);
  assert.equal(res.trace[1].evalScore, 100);
});

test('reactAction: iteration count is configurable and honored (1 stops before the rewrite lands)', async () => {
  let actor: 'owner' | 'system' = 'owner';
  const one = await reactAction<{ actor: string }>({
    maxAttempts: 1,
    action: async () => {
      if (actor === 'owner') throw apiErr('CORRIDOR_BREACH_ACTOR_GATE');
      return { actor };
    },
    reflect: (_err, attempt) => {
      if (attempt === 1) {
        actor = 'system';
        return { actor: 'system' };
      }
      return null;
    },
  });
  assert.ok(one.error !== undefined, 'maxAttempts=1 must NOT allow the rewrite to land');
  assert.equal(one.trace.length, 1, 'only one attempt happened');

  // And maxAttempts=2 DOES let it land (proves the count is real, not cosmetic).
  actor = 'owner';
  const two = await reactAction<{ actor: string }>({
    maxAttempts: 2,
    action: async () => {
      if (actor === 'owner') throw apiErr('CORRIDOR_BREACH_ACTOR_GATE');
      return { actor };
    },
    reflect: (_err, attempt) => {
      if (attempt === 1) {
        actor = 'system';
        return { actor: 'system' };
      }
      return null;
    },
  });
  assert.ok(two.error === undefined, 'maxAttempts=2 allows the rewrite to succeed');
  assert.equal(two.trace.length, 2);
});

test('reactAction: a genuinely illegal call with no valid rewrite STOPS at iteration 1 (no spin)', async () => {
  // A terminal order absorbing CANCEL is a hard error — there is NO valid rewrite. Must stop at 1.
  const res = await reactAction<number>({
    action: async () => {
      throw apiErr('ILLEGAL_TRANSITION', 409);
    },
    reflect: () => null, // honest: declare no valid rewrite, stop
  });
  assert.ok(res.error !== undefined);
  assert.equal(res.trace.length, 1, 'must not loop 3x on an unrewritable denial');
  assert.equal(res.trace[0].ok, false);
  assert.equal(res.trace[0].evalScore, 0);
  assert.equal(hadRewrite(res.trace), false, 'no rewrite happened — the denial is honest');
});

test('reactAction: the real-time eval gate rejects a result that did not throw but is wrong', async () => {
  // "No error" ≠ "correct". The eval gate catches a semantically-bad but non-throwing result.
  const res = await reactAction<{ total: number }>({
    action: async () => ({ total: 0 }), // no throw, but the computed total is wrong
    evaluate: (result) => {
      if (result.total <= 0) return { passed: false, score: 20, notes: 'total must be > 0' };
      return { passed: true, score: 100, notes: 'ok' };
    },
  });
  assert.ok(res.error !== undefined, 'eval gate must reject the bad result');
  assert.match(res.error!.message, /eval gate failed/);
  assert.equal(res.trace[0].evalScore, 20);
  assert.equal(res.trace[0].ok, false);
});

test('reactAction: onIteration fires exactly once per visible step', async () => {
  const seen: ReactStep[] = [];
  let actor: 'owner' | 'system' = 'owner';
  await reactAction<{ actor: string }>({
    maxAttempts: 3,
    onIteration: (s) => seen.push(s),
    action: async () => {
      if (actor === 'owner') throw apiErr('CORRIDOR_BREACH_ACTOR_GATE');
      return { actor };
    },
    reflect: (_err, attempt) => {
      if (attempt === 1) {
        actor = 'system';
        return { actor: 'system' };
      }
      return null;
    },
  });
  assert.equal(seen.length, 2, 'callback saw both visible iterations');
  assert.equal(seen[0].iter, 1);
  assert.equal(seen[1].iter, 2);
});
