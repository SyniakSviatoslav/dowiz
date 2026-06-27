import test from 'node:test';
import assert from 'node:assert/strict';
import { assertDevAuthDisabledInProd, type Env } from '@deliveryos/config';

// ADR-0003 boot-guard D: a production box must NEVER carry a dev-auth surface. This
// rehearses D's prod-firing behavior as a pure function (no real boot needed), so the
// dangerous direction (NODE_ENV=production + any dev knob) is proven in CI on every push
// — closing "D's prod path is untestable pre-prod". The INVERSE direction (prod
// NODE_ENV != production) is covered pre-traffic by the release_command guard, not here.

// Minimal Env-shaped object; only the fields D inspects matter.
const base = (over: Partial<Env>): Env => ({
  NODE_ENV: 'production',
  ALLOW_DEV_LOGIN: 'false',
  ...over,
} as Env);

test('assertDevAuthDisabledInProd (boot-guard D)', async (t) => {
  await t.test('throws on prod when ALLOW_DEV_LOGIN is true', () => {
    assert.throws(() => assertDevAuthDisabledInProd(base({ ALLOW_DEV_LOGIN: 'true' })), /dev-auth surface/);
  });

  await t.test('throws on prod when DEV_AUTH_SECRET is set (the live-incident shape)', () => {
    assert.throws(() => assertDevAuthDisabledInProd(base({ DEV_AUTH_SECRET: 'leaked' })), /DEV_AUTH_SECRET/);
  });

  await t.test('treats an EMPTY-STRING DEV_AUTH_SECRET as unset (inert, not an offender)', () => {
    // Empty string is falsy, so the guard does NOT flag it. This is correct, not a gap:
    // devLoginAllowed() activates only on `!!env.DEV_AUTH_SECRET` (apps/api/src/plugins/dev-guard.ts),
    // so an empty secret can never carry a dev-auth surface. This pins that equivalence so a future
    // change to `=== undefined` (which would diverge from the `!!` activation check) goes red here.
    assert.doesNotThrow(() => assertDevAuthDisabledInProd(base({ DEV_AUTH_SECRET: '' })));
    // …and the empty value must not appear in any offender list when combined with a real offender.
    assert.throws(
      () => assertDevAuthDisabledInProd(base({ DEV_AUTH_SECRET: '', ALLOW_DEV_LOGIN: 'true' })),
      (err: unknown) => err instanceof Error && /ALLOW_DEV_LOGIN/.test(err.message) && !/DEV_AUTH_SECRET/.test(err.message),
    );
  });

  await t.test('throws on prod when a dev keypair / kid is present', () => {
    assert.throws(() => assertDevAuthDisabledInProd(base({ JWT_DEV_KID: 'dev' })), /JWT_DEV_KID/);
    assert.throws(() => assertDevAuthDisabledInProd(base({ JWT_DEV_PRIVATE_KEY: 'x' })), /JWT_DEV_PRIVATE_KEY/);
    assert.throws(() => assertDevAuthDisabledInProd(base({ JWT_DEV_PUBLIC_KEY: 'x' })), /JWT_DEV_PUBLIC_KEY/);
  });

  await t.test('lists every offender in the error', () => {
    assert.throws(
      () => assertDevAuthDisabledInProd(base({ ALLOW_DEV_LOGIN: 'true', DEV_AUTH_SECRET: 'leaked', JWT_DEV_KID: 'dev' })),
      /ALLOW_DEV_LOGIN.*DEV_AUTH_SECRET.*JWT_DEV_KID/,
    );
  });

  await t.test('does NOT throw on a clean prod box', () => {
    assert.doesNotThrow(() => assertDevAuthDisabledInProd(base({})));
  });

  await t.test('is inert in non-prod (dev knobs allowed)', () => {
    assert.doesNotThrow(() =>
      assertDevAuthDisabledInProd(base({ NODE_ENV: 'development', ALLOW_DEV_LOGIN: 'true', DEV_AUTH_SECRET: 's', JWT_DEV_KID: 'dev' })),
    );
    assert.doesNotThrow(() =>
      assertDevAuthDisabledInProd(base({ NODE_ENV: 'test', ALLOW_DEV_LOGIN: 'true', DEV_AUTH_SECRET: 's' })),
    );
  });
});
