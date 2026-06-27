import { test } from 'node:test';
import { RuleTester } from 'eslint';
import plugin from '../src/index.js';

// RuleTester (ESLint 9 flat) — espree/JS by default; our code samples are valid JS.
// The rules gate on filename (`.test.ts`/`.spec.ts`), so each case sets `filename`.
const rt = new RuleTester();

test('no-tautological-assertion — red on expect(bool)/assert.ok(truthy), green on real assertions', () => {
  rt.run('no-tautological-assertion', plugin.rules['no-tautological-assertion'], {
    valid: [
      { code: 'expect(res.status()).toBe(200)', filename: 'a.test.ts' },
      { code: 'assert.ok(order.id)', filename: 'a.test.ts' },
      { code: 'assert.equal(x, 1)', filename: 'a.test.ts' },
      // non-test file → rule is inert even on a tautology
      { code: 'expect(true).toBe(true)', filename: 'a.ts' },
    ],
    invalid: [
      { code: 'expect(true).toBeTruthy()', filename: 'a.test.ts', errors: 1 },
      { code: 'expect(false).toBe(false)', filename: 'a.spec.ts', errors: 1 },
      { code: 'assert.ok(true)', filename: 'a.test.ts', errors: 1 },
      { code: 'assert(1)', filename: 'a.test.ts', errors: 1 },
      { code: "assert.ok('non-empty')", filename: 'a.test.ts', errors: 1 },
    ],
  });
});

test('no-swallowed-catch — red on .catch(()=>{}), green on a handling body', () => {
  rt.run('no-swallowed-catch', plugin.rules['no-swallowed-catch'], {
    valid: [
      { code: 'p.catch((e) => { console.error(e); })', filename: 'a.ts' },
      { code: 'p.catch((e) => log(e))', filename: 'a.ts' }, // expression body, not empty
      { code: 'p.then(ok)', filename: 'a.ts' },
    ],
    invalid: [
      { code: 'p.catch(() => {})', filename: 'a.ts', errors: 1 },
      { code: 'page.goto(u).catch(async () => {})', filename: 'a.test.ts', errors: 1 },
    ],
  });
});

test('no-truthy-on-identifier — red on expect(token/id/url).toBeTruthy(), green on a value', () => {
  rt.run('no-truthy-on-identifier', plugin.rules['no-truthy-on-identifier'], {
    valid: [
      { code: 'expect(total).toBeTruthy()', filename: 'a.test.ts' }, // not an id/token name
      { code: 'expect(res.access_token).toBe(expected)', filename: 'a.test.ts' }, // exact, not truthy
      { code: 'expect(order.id).toBeTruthy()', filename: 'a.ts' }, // non-test → inert
    ],
    invalid: [
      { code: 'expect(order.id).toBeTruthy()', filename: 'a.test.ts', errors: 1 },
      { code: 'expect(authToken).toBeDefined()', filename: 'a.test.ts', errors: 1 },
      { code: 'expect(j.access_token).toBeTruthy()', filename: 'a.spec.ts', errors: 1 },
      { code: 'expect(trackUrl).toBeTruthy()', filename: 'a.test.ts', errors: 1 },
    ],
  });
});

test('no-permissive-status-assertion — red on expect([..]).toContain(x), green on exact toBe (ACTIVATED)', () => {
  rt.run('no-permissive-status-assertion', plugin.rules['no-permissive-status-assertion'], {
    valid: [
      { code: 'expect(res.status()).toBe(200)', filename: 'a.test.ts' },
      { code: 'expect(["a","b"]).toContain(x)', filename: 'a.test.ts' }, // non-numeric array → fine
      { code: 'expect([200,400]).toContain(x)', filename: 'a.ts' }, // non-test → inert
    ],
    invalid: [
      { code: 'expect([200, 400, 500]).toContain(res.status())', filename: 'a.test.ts', errors: 1 },
      { code: 'expect([401, 403, 404]).toContain(s)', filename: 'a.spec.ts', errors: 1 },
    ],
  });
});
