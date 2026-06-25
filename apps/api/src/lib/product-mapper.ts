import { getImageUrl } from './image-url.js';

const APP_BASE = process.env.APP_BASE_URL || 'https://dowiz.fly.dev';

export function mapProductRow(r: Record<string, any>): Record<string, any> {
  const bom: any[] = r.attributes?.bom ?? [];
  const allergens = new Set<string>();
  for (const line of bom) {
    if (Array.isArray(line.allergens)) line.allergens.forEach((a: string) => allergens.add(a));
  }
  return {
    id: r.id,
    name: r.name,
    price: r.price,
    prepTimeMinutes: r.prep_time_minutes ?? null,
    description: r.description,
    available: r.is_available,
    categoryId: r.category_id,
    imageUrl: getImageUrl(r.image_key, APP_BASE),
    imageKey: r.image_key,
    sortOrder: r.sort_order ?? 0,
    stockCount: r.attributes?.stock_count ?? null,
    taste: r.attributes?.taste ?? null,
    recipeLines: bom.length ? bom : null,
    allergens: allergens.size ? Array.from(allergens).sort() : null,
    attributes: r.attributes || null,
    createdAt: r.created_at ?? null,
  };
}
