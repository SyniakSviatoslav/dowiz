// Per-tenant storefront font system — SINGLE SOURCE OF TRUTH (shared by the SPA ThemeProvider,
// the demo-builder brand-ingest, and any SSR path). See docs/design/storefront-fonts/DESIGN.md.
//
// A tenant NEVER supplies a raw font-family or URL over the wire — only an allowlist `id` below.
// The id is resolved to (a) a CSS font stack applied via --brand-font-* and (b), for non-base ids,
// a Google-Fonts <link> href built from a FIXED template. An unknown id falls back to the default
// pairing — it is never rendered or fetched. This keeps dynamic font loading egress-safe.

export type FontRole = 'heading' | 'body' | 'both';

export interface FontSpec {
  /** Display label for the admin picker. */
  label: string;
  /** The Google Fonts family name (also the primary CSS family). */
  family: string;
  /** Full CSS font stack (family + generic fallbacks) applied to --brand-font-*. */
  stack: string;
  /** `family=…` segment for the Google Fonts css2 URL (weights/axes included). */
  googleSpec: string;
  /** Roles this face is appropriate for (gates the admin picker). */
  role: FontRole;
  /** True when the family is already in apps/web/index.html's static <link> (no dynamic load needed). */
  base: boolean;
}

const SERIF = 'Georgia, "Times New Roman", serif';
const SANS = 'system-ui, -apple-system, "Segoe UI", sans-serif';
// Inter is multi-script (Latin + Latin-Ext + full Cyrillic) and always statically loaded
// (apps/web/index.html), so it's a zero-cost, always-available fallback. Several display faces
// below (Fraunces, DM Serif Display, Yeseva One, Bebas Neue) ship Latin-only glyph sets — without
// this, a Ukrainian (Cyrillic) heading rendered in one of them silently falls back to the
// browser's generic serif/sans-serif default, which reads as a jarring mid-page font swap. Putting
// 'Inter' in the stack itself means the browser's per-GLYPH fallback (not per-element) kicks in:
// Latin characters still render in the display face, and any Cyrillic characters fall back to the
// same Inter used for chrome/body — so the page always looks intentional, never broken.
const CYRILLIC_SAFE_FALLBACK = "'Inter'";

// The ONLY selectable/loadable families. Keys are the stored/transmitted ids.
export const FONT_ALLOWLIST = {
  playfair:     { label: 'Playfair Display',   family: 'Playfair Display',   stack: `'Playfair Display', ${SERIF}`,   googleSpec: 'Playfair+Display:wght@400;500;600;700',  role: 'heading', base: true },
  cormorant:    { label: 'Cormorant Garamond', family: 'Cormorant Garamond', stack: `'Cormorant Garamond', ${SERIF}`, googleSpec: 'Cormorant+Garamond:wght@400;500;600;700', role: 'heading', base: true },
  // Latin-only faces (no Cyrillic) — CYRILLIC_SAFE_FALLBACK inserted before the generic fallback.
  dmserif:      { label: 'DM Serif Display',   family: 'DM Serif Display',   stack: `'DM Serif Display', ${CYRILLIC_SAFE_FALLBACK}, ${SERIF}`, googleSpec: 'DM+Serif+Display',                       role: 'heading', base: true },
  fraunces:     { label: 'Fraunces',           family: 'Fraunces',           stack: `'Fraunces', ${CYRILLIC_SAFE_FALLBACK}, ${SERIF}`,         googleSpec: 'Fraunces:opsz,wght@9..144,400;9..144,500;9..144,600;9..144,700', role: 'heading', base: true },
  yeseva:       { label: 'Yeseva One',         family: 'Yeseva One',         stack: `'Yeseva One', ${CYRILLIC_SAFE_FALLBACK}, ${SERIF}`,       googleSpec: 'Yeseva+One',                             role: 'heading', base: true },
  spacegrotesk: { label: 'Space Grotesk',      family: 'Space Grotesk',      stack: `'Space Grotesk', ${SANS}`,       googleSpec: 'Space+Grotesk:wght@400;500;600;700',     role: 'heading', base: false },
  bebas:        { label: 'Bebas Neue',         family: 'Bebas Neue',         stack: `'Bebas Neue', ${CYRILLIC_SAFE_FALLBACK}, ${SANS}`,        googleSpec: 'Bebas+Neue',                             role: 'heading', base: false },
  poppins:      { label: 'Poppins',            family: 'Poppins',            stack: `'Poppins', ${SANS}`,             googleSpec: 'Poppins:wght@400;500;600;700',           role: 'both',    base: false },
  montserrat:   { label: 'Montserrat',         family: 'Montserrat',         stack: `'Montserrat', ${SANS}`,          googleSpec: 'Montserrat:wght@400;500;600;700',        role: 'both',    base: false },
  quicksand:    { label: 'Quicksand',          family: 'Quicksand',          stack: `'Quicksand', ${SANS}`,           googleSpec: 'Quicksand:wght@400;500;600;700',         role: 'both',    base: false },
  nunito:       { label: 'Nunito',             family: 'Nunito',             stack: `'Nunito', ${SANS}`,              googleSpec: 'Nunito:wght@400;500;600;700',            role: 'both',    base: false },
  inter:        { label: 'Inter',              family: 'Inter',              stack: `'Inter', ${SANS}`,               googleSpec: 'Inter:wght@400;500;600;700',             role: 'body',    base: true },
  dmsans:       { label: 'DM Sans',            family: 'DM Sans',            stack: `'DM Sans', ${SANS}`,             googleSpec: 'DM+Sans:wght@400;500;600;700',           role: 'both',    base: true },
} satisfies Record<string, FontSpec>;

export type FontId = keyof typeof FONT_ALLOWLIST;

export interface FontPairing { heading: FontId; body: FontId }

// Default pairing when nothing else is known — near the historical hardcoded Playfair look.
export const DEFAULT_FONT_PAIRING: FontPairing = { heading: 'playfair', body: 'inter' };

// Cuisine/character → a tasteful default pairing. The seed for Tier-0 (no extracted/owner font).
const CUISINE_FONT_PAIRINGS: Record<string, FontPairing> = {
  italian:     { heading: 'fraunces',  body: 'dmsans' },
  pizzeria:    { heading: 'fraunces',  body: 'dmsans' },
  trattoria:   { heading: 'fraunces',  body: 'dmsans' },
  sushi:       { heading: 'cormorant', body: 'dmsans' },
  japanese:    { heading: 'cormorant', body: 'dmsans' },
  burger:      { heading: 'bebas',     body: 'inter' },
  american:    { heading: 'bebas',     body: 'inter' },
  fastfood:    { heading: 'bebas',     body: 'inter' },
  cafe:        { heading: 'fraunces',  body: 'inter' },
  bakery:      { heading: 'fraunces',  body: 'inter' },
  coffee:      { heading: 'fraunces',  body: 'inter' },
  kebab:       { heading: 'dmsans',    body: 'dmsans' },
  street:      { heading: 'spacegrotesk', body: 'inter' },
  finedining:  { heading: 'cormorant', body: 'inter' },
  fine_dining: { heading: 'cormorant', body: 'inter' },
};

/** True when `id` is a real allowlisted font. Narrowing guard. */
export function isFontId(id: unknown): id is FontId {
  return typeof id === 'string' && Object.prototype.hasOwnProperty.call(FONT_ALLOWLIST, id);
}

/** Ids selectable for a given role in the admin picker (heading picker excludes body-only, etc.). */
export function fontIdsForRole(role: 'heading' | 'body'): FontId[] {
  return (Object.keys(FONT_ALLOWLIST) as FontId[]).filter((id) => {
    const r = FONT_ALLOWLIST[id].role;
    return r === role || r === 'both';
  });
}

/** Resolve an id (or unknown) to its CSS font stack; falls back to the default pairing's face. */
export function fontStack(id: string | null | undefined, role: 'heading' | 'body'): string {
  const safe = isFontId(id) ? id : DEFAULT_FONT_PAIRING[role];
  return FONT_ALLOWLIST[safe].stack;
}

/** Cuisine → default pairing (case/space/hyphen-insensitive). Unknown cuisine → DEFAULT_FONT_PAIRING. */
export function fontPairingForCuisine(cuisine: string | null | undefined): FontPairing {
  if (!cuisine) return DEFAULT_FONT_PAIRING;
  const key = cuisine.toLowerCase().replace(/[\s-]+/g, '');
  return CUISINE_FONT_PAIRINGS[key] ?? CUISINE_FONT_PAIRINGS[key.replace(/_/g, '')] ?? DEFAULT_FONT_PAIRING;
}

/**
 * Build a Google-Fonts css2 href for the given ids, EXCLUDING base families already in index.html.
 * Returns null when nothing extra needs loading. URL is constructed only from allowlist googleSpecs —
 * never from tenant free-text (egress-safe, see DESIGN.md threat model).
 */
export function googleFontsHref(ids: Array<string | null | undefined>): string | null {
  const specs = Array.from(
    new Set(
      ids
        .filter(isFontId)
        .filter((id) => !FONT_ALLOWLIST[id].base)
        .map((id) => FONT_ALLOWLIST[id].googleSpec)
    )
  );
  if (specs.length === 0) return null;
  return `https://fonts.googleapis.com/css2?${specs.map((s) => `family=${s}`).join('&')}&display=swap`;
}
