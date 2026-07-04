import type { MenuContext } from '@deliveryos/voice';
import type { VoiceMenuCategory } from './types.js';

/**
 * Build the matcher's MenuContext (id+name only — matcher.ts:12-15) from the tenant's loaded menu.
 * This feeds the matcher's fuzzy product/category NAME resolution (matcher.ts:117-141) — the ONLY
 * place tenant menu data crosses into the engine, and only as read-only {id,name} pairs. No price,
 * availability, or modifier data crosses this boundary (the engine never needs it to match a
 * transcript to a slot); those fields stay on the apps/web side and are consulted only by the
 * handlers (handlers.ts) when a STATEFUL proposal is actually applied.
 */
export function buildMenuContext(categories: readonly VoiceMenuCategory[]): MenuContext {
  const products: { id: string; name: string }[] = [];
  const cats: { id: string; name: string }[] = [];
  for (const category of categories) {
    cats.push({ id: category.id, name: category.name });
    for (const product of category.products) {
      products.push({ id: product.id, name: product.name });
    }
  }
  return { products, categories: cats };
}
