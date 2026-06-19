// Allergen chip palette. Each chip is self-contained (solid opaque background +
// dark ink) so contrast does NOT depend on the surface it sits on — including
// dark themes and image overlays on ProductCard. All pairs meet WCAG 2.2 AA
// (>= 4.5:1 for the small chip text). Avoid the previous 12%-alpha backgrounds:
// on a dark/photo surface they let the backdrop bleed through and the medium
// ink washed out below AA.
export const ALLERGEN_COLORS: Record<string, { bg: string; text: string }> = {
  gluten: { bg: '#fef3c7', text: '#854d0e' },    // amber  — 6.4:1
  dairy: { bg: '#dbeafe', text: '#1e40af' },      // blue   — 7.0:1
  eggs: { bg: '#fef9c3', text: '#854d0e' },        // yellow — 6.9:1
  soy: { bg: '#dcfce7', text: '#166534' },         // green  — 6.3:1
  nuts: { bg: '#ffedd5', text: '#9a3412' },        // orange — 6.0:1
  peanuts: { bg: '#ffedd5', text: '#9a3412' },     // orange — 6.0:1
  shellfish: { bg: '#fee2e2', text: '#991b1b' },   // red    — 6.5:1
  fish: { bg: '#cffafe', text: '#155e75' },        // cyan   — 6.1:1
  sesame: { bg: '#f3e8ff', text: '#6b21a8' },      // purple — 6.8:1
};

export function getAllergenStyle(allergen: string) {
  const key = allergen.toLowerCase();
  // Fallback: slate chip, dark ink — 8.6:1 on its own background.
  return ALLERGEN_COLORS[key] || { bg: '#e2e8f0', text: '#334155' };
}
