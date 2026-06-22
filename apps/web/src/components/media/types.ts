// Cinematic product-media client types.
//
// TODO import from @deliveryos/shared-types once the Data agent ships them
// (Phase-2 contract §"TS types"). Defined locally here so the renderer lane
// builds independently; keep this in 1:1 sync with the contract until the
// shared package exports them, then re-export from there and delete the locals.

export type ProductMediaKind = 'image' | 'video' | 'spin' | 'model';

export interface ProductMedia {
  id: string;
  kind: ProductMediaKind;
  /** Resolved absolute URL to the asset (server resolves storage_key → /media/ or passthrough). */
  url: string;
  /** Video/spin poster (raster only). */
  posterUrl?: string | null;
  mimeType: string;
  width?: number | null;
  height?: number | null;
  durationMs?: number | null;
  alt?: string | null;
  sortOrder: number;
  /** spin: ordered frame URLs (meta.frameUrls); frameCount is advisory. */
  meta?: { frameCount?: number; frameUrls?: string[] } | null;
}
