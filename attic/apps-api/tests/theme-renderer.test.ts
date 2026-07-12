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
    assert.ok(!outDark.warnings.includes('LOW_CONTRAST_PRIMARY'));
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
