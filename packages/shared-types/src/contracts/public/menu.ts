import { z } from 'zod';

export const PublicProduct = z.object({
  id: z.string().uuid(),
  categoryId: z.string().uuid(),
  name: z.string(),
  description: z.string().nullable(),
  price: z.number().int(),
  available: z.boolean(),
  imageUrl: z.string().nullable(),
  allergens: z.array(z.string()),
  calories: z.number().int().nullable(),
  sortOrder: z.number().int(),
}).strict();
export type PublicProduct = z.infer<typeof PublicProduct>;

export const PublicCategory = z.object({
  id: z.string().uuid(),
  name: z.string(),
  sortOrder: z.number().int(),
  products: z.array(PublicProduct),
}).strict();
export type PublicCategory = z.infer<typeof PublicCategory>;

export const PublicLocation = z.object({
  id: z.string().uuid(),
  name: z.string(),
  slug: z.string(),
  phone: z.string(),
  address: z.string().nullable(),
  status: z.enum(['open','closed','busy']),
  closesAt: z.string().nullable(),
  rating: z.number().nullable(),
  reviewCount: z.number().int(),
  deliveryEta: z.string(),
  deliveryFee: z.number().int(),
  minOrder: z.number().int(),
  currencyCode: z.string(),
  menuVersion: z.number().int(),
  heroImageUrl: z.string().nullable(),
  logoUrl: z.string().nullable(),
  supportedLocales: z.array(z.string()),
  defaultLocale: z.string(),
  lat: z.number().nullable(),
  lng: z.number().nullable(),
}).strict();
export type PublicLocation = z.infer<typeof PublicLocation>;

export const PublicMenuResponse = z.object({
  location: PublicLocation,
  categories: z.array(PublicCategory),
}).strict();
export type PublicMenuResponse = z.infer<typeof PublicMenuResponse>;
