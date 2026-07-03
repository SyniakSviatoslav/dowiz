// Pure, framework-free helper for the storefront's product-detail "What's inside" section
// (Huel-style macro tiles + ingredient chips — docs/design/storefront-polish/AWWWARDS-RESEARCH.md
// §5). Deliberately kept out of DishStats.tsx (which pulls in React/JSX) so it stays trivially
// unit-testable with the plain `node:test` pattern used across apps/web/src/**/*.test.ts — no DOM,
// no React import required to exercise the decision logic.

/** The subset of DishStats' `DishMacros` this predicate actually needs. */
export interface NutritionKcal {
  kcal?: number | null;
}

/**
 * True when there is anything meaningful to show: a positive kcal value, or at least one named
 * ingredient. This is the single "has data" predicate — reused by DishStats.tsx's own render guard
 * AND the product-detail "What's inside" section (MenuPage.tsx) — so every storefront surface
 * agrees on when to render a nutrition/ingredients block. Never an empty panel, never a row of
 * zeros (storefront-polish research principle #6: "show data only when it exists").
 */
export function hasDishData(
  macros: NutritionKcal | null | undefined,
  ingredientNames: ReadonlyArray<string | null | undefined> | null | undefined,
): boolean {
  const kcal = typeof macros?.kcal === 'number' && Number.isFinite(macros.kcal) ? macros.kcal : 0;
  const hasNamedIngredient = (ingredientNames ?? []).some((n) => typeof n === 'string' && n.length > 0);
  return kcal > 0 || hasNamedIngredient;
}
