export function getAllergenStyle(_allergen: string) {
  // Unified, theme-cohesive allergen chip. The previous per-allergen pastel
  // rainbow read as light "stickers" stuck on the dark storefront — the single
  // biggest "templated" tell. This one chip derives from the active theme: an
  // opaque, slightly-deepened surface with brand text. Opaque → still readable
  // over a product photo on ProductCard; theme-driven → cohesive on any palette
  // (light or dark). The allergen NAME still carries the meaning; colour-coding
  // is redundant with the labelled allergen section in the detail modal.
  return {
    bg: 'color-mix(in srgb, var(--brand-surface) 84%, #000)',
    text: 'var(--brand-text)',
  };
}
