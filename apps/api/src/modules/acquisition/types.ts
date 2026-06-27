import { z } from 'zod';

// P6-1 — acquisition lifecycle types. Module-local for now (no cross-package consumer yet);
// promote to packages/shared-types only when a second package needs them.

export const ACQUISITION_STATES = [
  'SOURCED',
  'PLACE_INGESTED',
  'MENU_EXTRACTED',
  'ENRICHED',
  'PROVISIONED',
  'VERIFIED',
  'CLAIM_OFFERED',
  'CLAIMED',
  // exit / terminal states
  'MENU_NOT_FOUND',
  'LOW_QUALITY',
  'MANUAL_REVIEW',
  'DISQUALIFIED',
  'ABANDONED',
] as const;

export type AcquisitionState = (typeof ACQUISITION_STATES)[number];

export const acquisitionStateSchema = z.enum(ACQUISITION_STATES);

export const PRODUCT_SOURCES = ['owner', 'imported', 'ai_inferred', 'place'] as const;
export type ProductSource = (typeof PRODUCT_SOURCES)[number];

export const acquisitionSourceSchema = z
  .object({
    id: z.string().uuid(),
    place_id: z.string().min(1),
    state: acquisitionStateSchema,
    place_raw: z.unknown().nullable(),
    website_url: z.string().nullable(),
    menu_kind: z.string().nullable(),
    menu_draft: z.unknown().nullable(),
    confidence: z.number().int().min(0).max(100).nullable(),
    org_id: z.string().uuid().nullable(),
    location_id: z.string().uuid().nullable(),
    failure_reason: z.string().nullable(),
    claimed_at: z.coerce.date().nullable(),
    created_at: z.coerce.date(),
    updated_at: z.coerce.date(),
  })
  .strict();

export type AcquisitionSource = z.infer<typeof acquisitionSourceSchema>;
