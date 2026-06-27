import test from 'node:test';
import assert from 'node:assert/strict';
import { renderTheme } from '../src/lib/theme-renderer.js';

test('Theme Renderer', async (t) => {
  await t.test('pure function and determinism', () => {
    const input1 = {
      primary_color: '#0a7d2c',
      secondary_color: '#d62828',
      font_family: 'Inter' as const,
      logo_url: 'https://example.com/logo.png'
    };

    const out1 = renderTheme(input1);
    const out2 = renderTheme(input1);

    assert.equal(out1.cssHash, out2.cssHash, 'Hash should be deterministic');
    assert.equal(out1.css, out2.css, 'CSS should be deterministic');
  });

  await t.test('WCAG warnings', () => {
    // Light text on light background (low contrast)
    const outLight = renderTheme({
      primary_color: '#eeeeee',
      secondary_color: '#ffffff',
      bg_color: '#ffffff',
      font_family: 'system-ui'
    });
    assert.ok(outLight.warnings.includes('LOW_CONTRAST_PRIMARY'));

    // Dark text on light bg (good contrast)
    const outDark = renderTheme({
      primary_color: '#000000',
      secondary_color: '#111111',
      bg_color: '#ffffff',
      font_family: 'system-ui'
    });
    // deepEqual (not !includes) so the assertion fails if warnings is
    // undefined/null/throws or carries an unexpected token — a real value check.
    assert.deepEqual(outDark.warnings, []);
  });

  await t.test('adjustColor packs RGB channels in correct order (hover variant)', () => {
    // Known primary with distinct G and B channels so a channel-swap bug is
    // observable. #0a7d2c - 20 => correct #006918 (a buggy g|b<<8|r<<16 packing
    // would yield #001869). Asserts the exact derived hover value.
    const out = renderTheme({
      primary_color: '#0a7d2c',
      secondary_color: '#d62828',
      font_family: 'system-ui'
    });
    assert.ok(
      out.css.includes('--brand-primary-hover: #006918'),
      `Expected --brand-primary-hover: #006918, got css: ${out.css}`
    );
  });

  await t.test('logo_url is not a CSS-injection vector', () => {
    // Verbatim interpolation into url('...') must not let a payload break out
    // and inject a sibling rule. If it does, the injected declaration appears
    // in the output CSS.
    const out = renderTheme({
      primary_color: '#0a7d2c',
      secondary_color: '#d62828',
      font_family: 'system-ui',
      logo_url: "x'); } body { background: red; } a { content: url('y"
    });
    assert.ok(
      !out.css.includes('background: red'),
      `logo_url broke out of url() and injected CSS: ${out.css}`
    );
  });

  await t.test('Google Fonts subsets', () => {
    const out = renderTheme({
      primary_color: '#000',
      secondary_color: '#111',
      font_family: 'Roboto'
    });
    assert.ok(out.css.includes('latin-ext'), 'Should include latin-ext subset for Albanian');
  });
});
