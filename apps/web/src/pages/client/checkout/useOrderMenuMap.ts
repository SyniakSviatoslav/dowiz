import { useState, useEffect } from 'react';

export interface OrderMenuEntry { image?: string; kcal: number; protein: number; fat: number; carbs: number }

// Per-product nutrition summed from its BOM food lines (mirrors the storefront derivation) — for the
// collapsible order summary. Packaging/utensil lines excluded; missing data → zeros.
function productNutritionFromBom(attributes: any): { kcal: number; protein: number; fat: number; carbs: number } {
  const bom = attributes && typeof attributes === 'object' ? (attributes as any).bom : null;
  const acc = { kcal: 0, protein: 0, fat: 0, carbs: 0 };
  if (!Array.isArray(bom)) return acc;
  for (const l of bom) {
    if (typeof l?.kcal === 'number') acc.kcal += l.kcal;
    if (typeof l?.proteinG === 'number') acc.protein += l.proteinG;
    if (typeof l?.fatG === 'number') acc.fat += l.fatG;
    if (typeof l?.carbsG === 'number') acc.carbs += l.carbsG;
  }
  return { kcal: Math.round(acc.kcal), protein: Math.round(acc.protein), fat: Math.round(acc.fat), carbs: Math.round(acc.carbs) };
}

// productId → { image, per-unit nutrition } from the public menu (for thumbnails + combined nutrition).
// Fetch the menu once to enrich the order summary with product thumbnails + per-unit nutrition.
// Best-effort: if it fails the summary still shows names/qty/price (no photos/nutrition).
export function useOrderMenuMap(slug: string | undefined) {
  const [orderMenuMap, setOrderMenuMap] = useState<Record<string, OrderMenuEntry>>({});

  useEffect(() => {
    if (!slug) return;
    let cancelled = false;
    fetch(`/public/locations/${slug}/menu`)
      .then(r => (r.ok ? r.json() : null))
      .then((d: any) => {
        if (cancelled || !d) return;
        const map: Record<string, OrderMenuEntry> = {};
        const walk = (x: any) => {
          if (Array.isArray(x)) { x.forEach(walk); return; }
          if (x && typeof x === 'object') {
            if (Array.isArray(x.products)) for (const p of x.products) {
              if (p?.id) map[p.id] = { image: p.imageUrl || undefined, ...productNutritionFromBom(p.attributes) };
            }
            for (const k of Object.keys(x)) if (k !== 'products') walk(x[k]);
          }
        };
        walk(d);
        setOrderMenuMap(map);
      })
      .catch(() => { /* best-effort */ });
    return () => { cancelled = true; };
  }, [slug]);

  return orderMenuMap;
}
