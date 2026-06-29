// ── Coherent palette derivation ───────────────────────────────────────────
// A tenant supplies at most a primary, a background and a text colour (often
// just a primary extracted from a logo). The public storefront, however, needs
// a FULL set of tokens — surface, surface-raised, border, muted text, primary
// hover/light, accent. Historically ClientLayout passed `var(--brand-surface)`
// for those, so they silently fell back to the default *dark* preset and a
// light tenant theme rendered dark-text-on-dark cards (contrast ~1.08:1).
//
// derivePalette() closes that gap: from whatever minimal colours exist it
// computes every remaining token from the BACKGROUND's luminance so the result
// is always internally coherent and contrast-safe, light or dark. Pure, no deps
// — safe to run on the server (theme storage) and the client (ThemeProvider).

import type { ThemeConfig } from './ThemeProvider.js';

interface RGB { r: number; g: number; b: number }

const clamp = (n: number, lo = 0, hi = 255) => Math.min(hi, Math.max(lo, n));

export function parseColor(input: string | null | undefined): RGB | null {
  if (!input) return null;
  const s = input.trim();
  // #rgb / #rrggbb
  const hex = s.replace(/^#/, '');
  if (/^[0-9a-fA-F]{3}$/.test(hex)) {
    const c = hex.split('');
    return { r: parseInt(c[0]! + c[0]!, 16), g: parseInt(c[1]! + c[1]!, 16), b: parseInt(c[2]! + c[2]!, 16) };
  }
  if (/^[0-9a-fA-F]{6}$/.test(hex)) {
    return { r: parseInt(hex.slice(0, 2), 16), g: parseInt(hex.slice(2, 4), 16), b: parseInt(hex.slice(4, 6), 16) };
  }
  // rgb()/rgba()
  const m = s.match(/^rgba?\(\s*(\d+)[,\s]+(\d+)[,\s]+(\d+)/i);
  if (m) return { r: +m[1]!, g: +m[2]!, b: +m[3]! };
  return null;
}

export function toHex({ r, g, b }: RGB): string {
  const h = (n: number) => clamp(Math.round(n)).toString(16).padStart(2, '0');
  return `#${h(r)}${h(g)}${h(b)}`;
}

// Relative luminance (WCAG)
function luminance({ r, g, b }: RGB): number {
  const ch = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b);
}

export function contrastRatio(a: RGB, b: RGB): number {
  const la = luminance(a), lb = luminance(b);
  const hi = Math.max(la, lb), lo = Math.min(la, lb);
  return (hi + 0.05) / (lo + 0.05);
}

export function isLight(c: RGB): boolean {
  return luminance(c) > 0.5;
}

// Linear mix of two colours, t=0 → a, t=1 → b
function mix(a: RGB, b: RGB, t: number): RGB {
  return { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t };
}

const WHITE: RGB = { r: 255, g: 255, b: 255 };
const BLACK: RGB = { r: 17, g: 17, b: 17 };

// Pick whichever of white/near-black has the higher contrast against bg.
function readableOn(bg: RGB): RGB {
  return contrastRatio(WHITE, bg) >= contrastRatio(BLACK, bg) ? WHITE : BLACK;
}

// Nudge `text` until it clears `min` contrast on `bg`, walking toward the
// higher-contrast pole (white or black). Returns the original when it passes.
function ensureContrast(text: RGB, bg: RGB, min: number): RGB {
  if (contrastRatio(text, bg) >= min) return text;
  const pole = readableOn(bg);
  let t = 0.15;
  let candidate = mix(text, pole, t);
  while (contrastRatio(candidate, bg) < min && t < 1) {
    t += 0.15;
    candidate = mix(text, pole, t);
  }
  return candidate;
}

export interface PaletteInput {
  primary?: string | null;
  bg?: string | null;
  text?: string | null;
  surface?: string | null;
  accent?: string | null;
}

/**
 * Derive a complete, coherent, contrast-safe ThemeConfig from minimal inputs.
 * Everything not supplied is computed from the background's luminance.
 */
export function derivePalette(input: PaletteInput): ThemeConfig {
  const primary = parseColor(input.primary) || { r: 234, g: 79, b: 22 }; // food-dark default
  const bg = parseColor(input.bg) || { r: 18, g: 18, b: 18 };
  const light = isLight(bg);
  // Foreground pole = the high-contrast extreme for this bg (black on light,
  // white on dark). Surfaces/borders all step toward it so they stay in the bg's
  // hue family yet read as distinct: lighter cards on dark, inset cards on light.
  const fg = readableOn(bg);

  // Text: honour the tenant's choice but never below AA on the background.
  const text = ensureContrast(parseColor(input.text) || fg, bg, 4.5);

  // Surfaces step toward the foreground pole so a card is always visibly
  // separated from the page (dark → lighter, light → a hair darker than bg).
  const surface = parseColor(input.surface) || mix(bg, fg, light ? 0.035 : 0.06);
  const surfaceRaised = mix(bg, fg, light ? 0.07 : 0.11);

  // Border: a low-contrast divider that works on bg AND surface.
  const border = mix(bg, fg, light ? 0.12 : 0.18);

  // Muted text: faded toward bg but kept ≥ 4.5:1 on the surface (WCAG AA for normal text;
  // raised from 3:1 — a deliberate accessibility-over-subtlety call across all tenant brands).
  const textMuted = ensureContrast(mix(text, bg, 0.42), surface, 4.5);

  // Primary hover: lighten on dark, darken on light for a perceptible delta.
  const primaryHover = mix(primary, light ? BLACK : WHITE, 0.18);
  const primaryLight = `rgba(${Math.round(primary.r)}, ${Math.round(primary.g)}, ${Math.round(primary.b)}, 0.12)`;

  // Primary-as-TEXT: the brand primary is chosen as a fill colour and is often
  // too low-contrast to read as text on a surface (e.g. rose #e11d48 on a light
  // pink surface = 4.0). This is the same hue nudged toward the readable pole
  // until it clears AA on the surface — use it for primary-coloured text/prices,
  // keep raw --brand-primary for fills.
  const primaryReadable = ensureContrast(primary, surface, 4.5);

  // Primary as a CTA FILL (button background with text on it). The naive tokens.css default
  // (`color-mix(primary 85%, black)` + hard-coded white text) ships an ILLEGIBLE button when a
  // tenant picks a pale primary: white text on a pale fill is sub-AA. Derive the pair together —
  // `onPrimary` is the best text pole for the primary, and `primaryStrong` is the primary nudged
  // until that text clears AA on it — so a CTA is always readable for ANY brand colour.
  const onPrimary = readableOn(primary);
  const primaryStrong = ensureContrast(primary, onPrimary, 4.5);

  // Accent: neutral chip/section tint, slightly stronger than the raised surface.
  const accent = parseColor(input.accent)
    ? toHex(parseColor(input.accent)!)
    : toHex(mix(bg, fg, light ? 0.05 : 0.14));

  return {
    primary: toHex(primary),
    primaryHover: toHex(primaryHover),
    primaryReadable: toHex(primaryReadable),
    primaryStrong: toHex(primaryStrong),
    onPrimary: toHex(onPrimary),
    primaryLight,
    accent,
    bg: toHex(bg),
    surface: toHex(surface),
    surfaceRaised: toHex(surfaceRaised),
    text: toHex(text),
    textMuted: toHex(textMuted),
    border: toHex(border),
  };
}
