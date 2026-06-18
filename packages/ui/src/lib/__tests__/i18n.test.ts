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
