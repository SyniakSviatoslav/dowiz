import { test } from 'node:test';
import { RuleTester } from 'eslint';
import tsParser from '@typescript-eslint/parser';
import plugin from '../src/index.js';

// A TS-parser RuleTester for rules that inspect TypeScript-only AST (type annotations).
const tsrt = new RuleTester({ languageOptions: { parser: tsParser } });

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

test('no-prod-base-in-test — red on a prod-host literal, green on staging/VITE_BASE_URL', () => {
  rt.run('no-prod-base-in-test', plugin.rules['no-prod-base-in-test'], {
    valid: [
      { code: "const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev'", filename: 'a.test.ts' },
      { code: "const B = 'https://dowiz.fly.dev'", filename: 'a.ts' }, // non-test → inert
    ],
    invalid: [
      { code: "const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev'", filename: 'a.test.ts', errors: 1 },
      { code: "await page.goto('https://dowiz.app/s/demo')", filename: 'a.spec.ts', errors: 1 },
    ],
  });
});

test('no-permissive-status-assertion — red on expect([..]).toContain(x), green on exact toBe (ACTIVATED)', () => {
  rt.run('no-permissive-status-assertion', plugin.rules['no-permissive-status-assertion'], {
    valid: [
      { code: 'expect(res.status()).toBe(200)', filename: 'a.test.ts' },
      { code: 'expect(["a","b"]).toContain(x)', filename: 'a.test.ts' }, // non-numeric array → fine
      { code: 'expect([200, 201]).toContain(res.status())', filename: 'a.test.ts' }, // pure-success either/or → fine
      { code: 'expect([200, 204]).toContain(s)', filename: 'a.test.ts' }, // pure-success → fine
      { code: 'expect([200,400]).toContain(x)', filename: 'a.ts' }, // non-test → inert
    ],
    invalid: [
      { code: 'expect([200, 400, 500]).toContain(res.status())', filename: 'a.test.ts', errors: 1 },
      { code: 'expect([401, 403, 404]).toContain(s)', filename: 'a.spec.ts', errors: 1 },
    ],
  });
});

test('no-recipe-only-allergen-read — red on bomToNutrition(p).allergens in the storefront, green via computeAllergenSurface', () => {
  rt.run('no-recipe-only-allergen-read', plugin.rules['no-recipe-only-allergen-read'], {
    valid: [
      // converged single source — the only allowed allergen basis
      { code: 'const s = computeAllergenSurface(p.attributes, recipeAllergens(p)); s.known.includes(x)', filename: '/r/__fixtures__/menu.tsx' },
      { code: 'allergenSurfaceOf(p).known.includes(filterAllergen)', filename: '/r/__fixtures__/menu.tsx' },
      // macro reads off bomToNutrition are fine (not a safety read)
      { code: 'const n = bomToNutrition(p); n.kcal > 0', filename: '/r/__fixtures__/menu.tsx' },
      // non-storefront file → rule is inert even on the recipe-only read
      { code: 'bomToNutrition(p).allergens.includes(x)', filename: '/r/apps/web/src/other.tsx' },
    ],
    invalid: [
      { code: 'bomToNutrition(p).allergens.includes(filterAllergen)', filename: '/r/__fixtures__/menu.tsx', errors: 1 },
      { code: 'const a = bomToNutrition(detailProduct).allergens', filename: '/r/__fixtures__/menu.tsx', errors: 1 },
    ],
  });
});

test('no-voice-engine-callback — red on a callback/handler param in packages/voice exported signatures, green on data/object-port params', () => {
  tsrt.run('no-voice-engine-callback', plugin.rules['no-voice-engine-callback'], {
    valid: [
      // object port (interface with a method) — not a function-typed param
      { code: 'export interface Transcriber { transcribe(a: Float32Array): Promise<string>; }', filename: '/r/packages/voice/src/transcriber.ts' },
      // handlers OBJECT param — TSTypeReference, not a function type (the sink legitimately holds handlers)
      { code: 'export class Gate { constructor(private h: VoiceHandlers) {} }', filename: '/r/packages/voice/src/confirmation-gate.ts' },
      // plain data params
      { code: 'export function matchIntent(t: string, l: string): void {}', filename: '/r/packages/voice/src/matcher.ts' },
      // AsyncIterable of data is a data stream, not a callback
      { code: 'export class E { async *intents(u: AsyncIterable<Float32Array>): AsyncIterableIterator<string> { yield ""; } }', filename: '/r/packages/voice/src/whisper-provider.ts' },
      // a NON-exported helper is inert (only the exported public boundary is the engine surface)
      { code: 'function helper(cb: (x: number) => void): void { cb(1); }', filename: '/r/packages/voice/src/matcher.ts' },
      // #private method is not public surface
      { code: 'export class E { #onResult(cb: (x: number) => void) { cb(1); } }', filename: '/r/packages/voice/src/whisper-provider.ts' },
      // a function-typed param OUTSIDE packages/voice is inert (rule is scoped)
      { code: 'export function f(cb: (x: number) => void): void { cb(1); }', filename: '/r/apps/web/src/thing.ts' },
    ],
    invalid: [
      { code: 'export function wireEngine(handler: (p: unknown) => void): void { handler(null); }', filename: '/r/packages/voice/src/engine.ts', errors: 1 },
      { code: 'export class E { constructor(private onIntent: (p: unknown) => void) {} }', filename: '/r/packages/voice/src/engine.ts', errors: 1 },
      { code: 'export class E { on(event: string, cb: (p: unknown) => void): void {} }', filename: '/r/packages/voice/src/engine.ts', errors: 1 },
      { code: 'export interface Src { subscribe(onResult: (p: unknown) => void): void; }', filename: '/r/__fixtures__/bad-voice-engine-callback.ts', errors: 1 },
      // a union that includes a function type is still a callback surface
      { code: 'export function f(cb: null | ((p: unknown) => void)): void {}', filename: '/r/packages/voice/src/engine.ts', errors: 1 },
    ],
  });
});

test('no-voice-app-import — red on packages/voice importing apps/web / a fetch-client / a Cart* mutator, green on relative + data-only imports', () => {
  // TS parser: the valid cases use `import type` (TS-only syntax espree can't parse).
  tsrt.run('no-voice-app-import', plugin.rules['no-voice-app-import'], {
    valid: [
      // in-package relative imports — the normal shape of the engine's own modules
      { code: "import { classify } from './capability-table.js';", filename: '/r/packages/voice/src/confirmation-gate.ts' },
      { code: "import type { IntentKind } from './types.js';", filename: '/r/packages/voice/src/capability-table.ts' },
      { code: "import { normalize } from '../normalize.js';", filename: '/r/packages/voice/src/__tests__/matcher.test.ts' },
      // node builtins / test runner — inert
      { code: "import { describe, it } from 'node:test';", filename: '/r/packages/voice/src/__tests__/capability-table.test.ts' },
      // a package NAME that merely CONTAINS "cart" as a substring, not a Cart* module path — not flagged
      { code: "import { scaffold } from 'cartography-utils';", filename: '/r/packages/voice/src/matcher.ts' },
      // outside packages/voice entirely — rule is scoped, inert even on a real forbidden import
      { code: "import { addItem } from '../../apps/web/src/lib/CartProvider';", filename: '/r/apps/web/src/other.ts' },
    ],
    invalid: [
      // apps/web — the whole consuming app, at any relative depth
      { code: "import { addItem } from '../../../apps/web/src/lib/CartProvider';", filename: '/r/packages/voice/src/matcher.ts', errors: 1 },
      { code: "import { setSortBy } from 'apps/web/src/pages/client/MenuPage';", filename: '/r/packages/voice/src/matcher.ts', errors: 1 },
      // a Cart* mutator module, even from inside packages/voice's own tree
      { code: "import { useSharedCart } from '../../CartProvider.js';", filename: '/r/packages/voice/src/mock-provider.ts', errors: 1 },
      // known fetch/API-client packages
      { code: "import axios from 'axios';", filename: '/r/packages/voice/src/transcriber.ts', errors: 1 },
      { code: "import { request } from './lib/api-client.js';", filename: '/r/packages/voice/src/whisper-provider.ts', errors: 1 },
      // dynamic import + re-export forms must be caught too, not just static ImportDeclaration
      { code: "export * from '../../apps/web/src/lib/CartProvider.js';", filename: '/r/packages/voice/src/index.ts', errors: 1 },
      // a forbidden import inside a __tests__ file — this rule covers tests too (unlike no-voice-engine-callback)
      { code: "import { addItem } from '../../../apps/web/src/lib/CartProvider';", filename: '/r/packages/voice/src/__tests__/matcher.test.ts', errors: 1 },
      // the rule's own fixture-scope proof
      { code: "import { addItem } from '../../apps/web/src/lib/CartProvider';", filename: '/r/__fixtures__/bad-voice-app-import.ts', errors: 1 },
    ],
  });
});
