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
  primary_media_id: z.string().nullable().optional(),
}).strict();
export type PublicProduct = z.infer<typeof PublicProduct>;

/**
 * Cinematic product media (Phase 2, dark behind MEDIA_RICH_ENABLED).
 * Resolved view served by the lazy media endpoint — `url`/`posterUrl` are
 * already absolute (server resolves storage_key → /media/ or http(s) passthrough).
 * See docs/design/cinematic-product-media/phase2-contract.md.
 */
export type ProductMediaKind = 'image' | 'video' | 'spin' | 'model';
export interface ProductMedia {
  id: string;
  kind: ProductMediaKind;
  url: string;                 // resolved absolute URL to the asset (server resolves storage_key → /media/ or passthrough)
  posterUrl?: string | null;   // video/spin poster (raster only)
  mimeType: string;
  width?: number | null;
  height?: number | null;
  durationMs?: number | null;
  alt?: string | null;
  sortOrder: number;
  meta?: { frameCount?: number; frameUrls?: string[] } | null; // spin: ordered frame URLs
}

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
