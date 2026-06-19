import { describe, it, before } from 'node:test';
import assert from 'node:assert/strict';

before(() => {
  (globalThis as any).document = { documentElement: { lang: '' } };
});

// Dynamic import after setting up document mock
let mod: typeof import('../i18n.js');
before(async () => {
  mod = await import('../i18n.js');
});

describe('i18n', () => {
  it('returns translation for known sq key', () => {
    assert.equal(mod.t('menu.title'), 'Menu');
  });

  it('returns translation for known en key', () => {
    mod.setLocale('en');
    assert.equal(mod.t('menu.title'), 'Menu');
  });

  it('returns fallback for missing key', () => {
    assert.equal(mod.t('nonexistent.key', 'Fallback'), 'Fallback');
  });

  it('returns key itself if no fallback', () => {
    assert.equal(mod.t('nonexistent.key'), 'nonexistent.key');
  });

  it('switches locale with setLocale', () => {
    mod.setLocale('sq');
    assert.equal(mod.t('cart.title'), 'Shporta');
    mod.setLocale('en');
    assert.equal(mod.t('cart.title'), 'Cart');
  });

  it('tracks current locale', () => {
    mod.setLocale('sq');
    assert.equal(mod.getLocale(), 'sq');
    mod.setLocale('en');
    assert.equal(mod.getLocale(), 'en');
  });

  it('has Albanian as default', () => {
    mod.setLocale('sq');
    assert.equal(mod.getLocale(), 'sq');
  });

  it('returns common translations', () => {
    mod.setLocale('sq');
    assert.equal(mod.t('common.loading'), 'Duke u ngarkuar...');
    assert.equal(mod.t('common.save'), 'Ruaj');
    assert.equal(mod.t('common.cancel'), 'Anulo');
  });

  it('returns auth translations', () => {
    assert.equal(mod.t('auth.login'), 'Hyr');
    assert.equal(mod.t('auth.phone'), 'Numri i telefonit');
  });

  it('returns cart translations in English', () => {
    mod.setLocale('en');
    assert.equal(mod.t('cart.empty'), 'Cart is empty');
    assert.equal(mod.t('cart.checkout'), 'Checkout');
  });

  // Onboarding QA O2: the stepper label must use a dedicated `admin.courier_step`
  // key WITHOUT the trailing colon that `admin.courier` carries (the latter is an
  // OrderCard label "Courier: John"). Reusing admin.courier dragged the colon into
  // the stepper, rendering "Courier:".
  it('O2: admin.courier_step exists in every locale and has no trailing colon', () => {
    for (const locale of ['en', 'sq', 'uk'] as const) {
      const step = mod.translate(locale, 'admin.courier_step');
      const card = mod.translate(locale, 'admin.courier');
      assert.notEqual(step, 'admin.courier_step', `admin.courier_step missing in ${locale}`);
      assert.ok(!step.endsWith(':'), `admin.courier_step must not end with ':' in ${locale} (got "${step}")`);
      assert.ok(card.endsWith(':'), `admin.courier (OrderCard label) should still keep its colon in ${locale}`);
    }
  });

  // Onboarding QA O4: the onboarding step uses a customer-facing label, not the
  // developer-flavoured "Order Flow Test".
  it('O4: admin.flow_test_step resolves in every locale', () => {
    for (const locale of ['en', 'sq', 'uk'] as const) {
      const v = mod.translate(locale, 'admin.flow_test_step');
      assert.notEqual(v, 'admin.flow_test_step', `admin.flow_test_step missing in ${locale}`);
    }
  });

  // Onboarding QA O3: the menu-step subhead must not promise a "PDF" import while
  // the only card offers CSV — keep the copy format-neutral.
  it('O3: admin.import_menu_desc no longer mentions PDF', () => {
    for (const locale of ['en', 'sq', 'uk'] as const) {
      const v = mod.translate(locale, 'admin.import_menu_desc');
      assert.ok(!/pdf/i.test(v), `admin.import_menu_desc still mentions PDF in ${locale} (got "${v}")`);
    }
  });

  // U3 regression: the admin login page now uses these keys instead of
  // hardcoded English, so they must resolve AND be localized per locale.
  it('localizes admin login-error keys across all locales', () => {
    for (const key of ['admin.error_login_failed', 'admin.error_invalid_credentials']) {
      const en = mod.translate('en', key);
      const sq = mod.translate('sq', key);
      const uk = mod.translate('uk', key);
      assert.notEqual(en, key, `${key} missing in en (returned the key)`);
      assert.notEqual(sq, key, `${key} missing in sq`);
      assert.notEqual(uk, key, `${key} missing in uk`);
      // non-English locales must differ from the English text (actually translated)
      assert.notEqual(sq, en, `${key} not localized for sq`);
      assert.notEqual(uk, en, `${key} not localized for uk`);
    }
  });
});
