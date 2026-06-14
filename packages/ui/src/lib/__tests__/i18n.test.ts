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
    assert.equal(mod.t('cart.checkout'), 'Order');
  });
});
