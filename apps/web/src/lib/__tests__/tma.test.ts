import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  isTmaFlagEnabled,
  detectTelegramWebApp,
  applyTmaThemeAttributes,
  initTelegramMiniApp,
  setupTmaBackButton,
  type TelegramWebApp,
} from '../tma.js';

function makeMockWebApp(overrides: Partial<TelegramWebApp> = {}): { win: any; calls: Record<string, number> } {
  const calls: Record<string, number> = {};
  const bump = (k: string) => () => { calls[k] = (calls[k] ?? 0) + 1; };
  const webApp: TelegramWebApp = {
    colorScheme: 'dark',
    themeParams: { bg_color: '#111111', text_color: '#ffffff' },
    ready: bump('ready'),
    expand: bump('expand'),
    ...overrides,
  };
  return { win: { Telegram: { WebApp: webApp } }, calls };
}

function makeMockRoot(): { root: { setAttribute: (k: string, v: string) => void }; attrs: Record<string, string> } {
  const attrs: Record<string, string> = {};
  return { root: { setAttribute: (k, v) => { attrs[k] = v; } }, attrs };
}

describe('tma — flag gating', () => {
  it('is off by default (no env / unset)', () => {
    assert.equal(isTmaFlagEnabled({}), false);
  });

  it('is off for any value other than the literal string "true"', () => {
    assert.equal(isTmaFlagEnabled({ VITE_TMA_ENABLED: 'false' }), false);
    assert.equal(isTmaFlagEnabled({ VITE_TMA_ENABLED: '1' }), false);
  });

  it('is on only for "true"', () => {
    assert.equal(isTmaFlagEnabled({ VITE_TMA_ENABLED: 'true' }), true);
  });
});

describe('tma — detection', () => {
  it('returns undefined outside Telegram (no window.Telegram)', () => {
    assert.equal(detectTelegramWebApp({}), undefined);
  });

  it('returns undefined when Telegram exists but WebApp does not (honest CSP gap — script never loaded)', () => {
    assert.equal(detectTelegramWebApp({ Telegram: {} }), undefined);
  });

  it('returns the WebApp object when present', () => {
    const { win } = makeMockWebApp();
    assert.equal(detectTelegramWebApp(win), win.Telegram.WebApp);
  });
});

describe('tma — theme mapping (data-attributes only, never restyles)', () => {
  it('returns an empty map when there is no WebApp', () => {
    assert.deepEqual(applyTmaThemeAttributes(undefined), {});
  });

  it('maps colorScheme + themeParams to data-tma-* keys, snake_case -> kebab-case', () => {
    const { win } = makeMockWebApp();
    const attrs = applyTmaThemeAttributes(win.Telegram.WebApp);
    assert.equal(attrs['data-tma'], 'true');
    assert.equal(attrs['data-tma-scheme'], 'dark');
    assert.equal(attrs['data-tma-bg-color'], '#111111');
    assert.equal(attrs['data-tma-text-color'], '#ffffff');
  });

  it('applies attributes onto an injected root (no global document dependency)', () => {
    const { win } = makeMockWebApp();
    const { root, attrs } = makeMockRoot();
    applyTmaThemeAttributes(win.Telegram.WebApp, root);
    assert.equal(attrs['data-tma'], 'true');
    assert.equal(attrs['data-tma-scheme'], 'dark');
  });

  it('skips non-string/empty theme values', () => {
    const { win } = makeMockWebApp({ themeParams: { bg_color: '', hint_color: undefined } });
    const attrs = applyTmaThemeAttributes(win.Telegram.WebApp);
    assert.equal('data-tma-bg-color' in attrs, false);
    assert.equal('data-tma-hint-color' in attrs, false);
  });

  // GUARDRAIL (LOW security, integration-review fix): themeParams is an UNTRUSTED bridge
  // object — non-allowlisted keys and non-hex values must never become attributes, or a
  // hostile page context could mint arbitrary data-* names/values on <html>.
  it('drops non-allowlisted theme keys and non-hex-color values (untrusted bridge)', () => {
    const { win } = makeMockWebApp({
      themeParams: {
        bg_color: '#222222',
        'onload{}': '#333333',
        evil_key: 'javascript:alert(1)',
        text_color: 'url(https://evil.example/x)',
      } as any,
      colorScheme: 'dark" onmouseover="x' as any,
    });
    const attrs = applyTmaThemeAttributes(win.Telegram.WebApp);
    assert.equal(attrs['data-tma-bg-color'], '#222222');
    assert.equal(Object.keys(attrs).some(k => k.includes('onload') || k.includes('evil')), false);
    assert.equal('data-tma-text-color' in attrs, false);
    assert.equal('data-tma-scheme' in attrs, false);
  });
});

describe('tma — init (ready + expand, flag + presence gated)', () => {
  it('no-ops when the flag is off, even inside Telegram', () => {
    const { win, calls } = makeMockWebApp();
    const applied = initTelegramMiniApp({ win, enabled: false });
    assert.equal(applied, false);
    assert.equal(calls.ready, undefined);
    assert.equal(calls.expand, undefined);
  });

  it('no-ops when the flag is on but window.Telegram.WebApp is absent (honest gap)', () => {
    const applied = initTelegramMiniApp({ win: {}, enabled: true });
    assert.equal(applied, false);
  });

  it('calls ready()+expand() and applies theme attrs when flag is on and WebApp is present', () => {
    const { win, calls } = makeMockWebApp();
    const { root, attrs } = makeMockRoot();
    const applied = initTelegramMiniApp({ win, root, enabled: true });
    assert.equal(applied, true);
    assert.equal(calls.ready, 1);
    assert.equal(calls.expand, 1);
    assert.equal(attrs['data-tma'], 'true');
  });

  it('swallows a throwing ready()/expand() (defensive — some WebViews lag the API)', () => {
    const { win } = makeMockWebApp({
      ready: () => { throw new Error('boom'); },
      expand: () => { throw new Error('boom'); },
    });
    assert.doesNotThrow(() => initTelegramMiniApp({ win, enabled: true }));
  });
});

describe('tma — back button (trivial in-app nav)', () => {
  it('returns a no-op cleanup outside Telegram', () => {
    const cleanup = setupTmaBackButton(() => {}, {});
    assert.doesNotThrow(cleanup);
  });

  it('shows the button and registers the callback when present', () => {
    let registered: (() => void) | undefined;
    const showCalls: number[] = [];
    const { win } = makeMockWebApp({
      BackButton: {
        show: () => showCalls.push(1),
        hide: () => {},
        onClick: (cb) => { registered = cb; },
        offClick: () => {},
      },
    });
    const onBack = () => {};
    setupTmaBackButton(onBack, win);
    assert.equal(showCalls.length, 1);
    assert.equal(registered, onBack);
  });

  it('cleanup un-registers the callback and hides the button', () => {
    let offClickCalled = false;
    let hideCalled = false;
    const { win } = makeMockWebApp({
      BackButton: {
        show: () => {},
        hide: () => { hideCalled = true; },
        onClick: () => {},
        offClick: () => { offClickCalled = true; },
      },
    });
    const cleanup = setupTmaBackButton(() => {}, win);
    cleanup();
    assert.equal(offClickCalled, true);
    assert.equal(hideCalled, true);
  });
});
