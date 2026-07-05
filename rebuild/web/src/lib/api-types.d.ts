// GENERATED-TYPES STAND-IN — src/lib/api-types.d.ts
//
// Intended provenance: `openapi-typescript` run against
// docs/design/rebuild-plan/openapi-contracts/openapi-s1-storefront-read.yaml (the S1 contract,
// untracked in git, read-only from the main tree per lane R2 instructions).
//
// BLOCKED: this worktree's `protect-paths.sh` + `guard-bash.sh` hooks hard-block creating
// rebuild/web/package.json and running `npm install <pkg>` / `npx <pkg>` for ANY new
// dependency (dependency mutations require council/human approval — see rebuild/web/README.md
// "Blocker" section). `openapi-typescript` itself could not be installed or executed, so these
// types are HAND-TRANSCRIBED 1:1 from the S1 YAML's `components.schemas` (verified against the
// live file at /root/dowiz/docs/design/rebuild-plan/openapi-contracts/openapi-s1-storefront-read.yaml,
// 2026-07-04). Once the dependency gate clears, replace this file by running:
//
//   npx openapi-typescript ../../docs/design/rebuild-plan/openapi-contracts/openapi-s1-storefront-read.yaml \
//     -o src/lib/api-types.d.ts
//
// and diff against this hand-authored version — any drift is a real contract change to review,
// not an artifact of the stand-in.

export interface ErrorEnvelope {
  code: string;
  message: string;
  fields?: Array<{ path: string; code: string }>;
  correlationId: string;
  retryAfterMs?: number;
  status: number;
  /** Legacy alias, always equal to message. */
  error: string;
}

export interface PublicModifier {
  id: string;
  name: string;
  /** Signed integer minor units. */
  price_delta: number;
  /** Always true on the wire (WHERE m.available=true). */
  available: boolean;
  sort_order: number;
}

export interface PublicModifierGroup {
  id: string;
  name: string;
  min_select: number;
  max_select: number;
  required: boolean;
  /** null -> client infers radio (max_select===1) vs checkbox. */
  display_type: 'radio' | 'checkbox' | 'select' | 'quantity' | null;
  sort_order: number;
  modifiers: PublicModifier[];
}

export interface PublicMenuProduct {
  id: string;
  name: string;
  description: string | null;
  /** Integer minor units (ADR-0005). Interpret with menu.currency.minor_unit. */
  price: number;
  /** x-quirk: ALWAYS true on live payloads — unavailable products are filtered server-side. */
  available: boolean;
  image_key: string | null;
  /** uuid of the primary product_media row, or null. */
  primary_media_id: string | null;
  imageUrl?: string | null;
  /**
   * Free-form JSONB characteristics layer (ADR-0014). Known keys: kcal, protein, fat, carbs
   * (numbers), allergens (EU-14 ids), tags, taste, bom, stock_count, chef_pick, ingredients,
   * image_url, description_sq. Untyped beyond that — additionalProperties: true on the wire.
   */
  attributes: Record<string, unknown> | null;
  prep_time_minutes: number | null;
  modifier_groups: PublicModifierGroup[];
}

export interface PublicMenuCategory {
  id: string;
  name: string;
  sort_order: number;
  products: PublicMenuProduct[];
}

export interface PublicMenu {
  menu_version: number;
  /** uuid; null ONLY on shadow previews. */
  location_id: string | null;
  /** x-quirk: camelCase alias of location_id — always mirrors it. */
  locationId: string | null;
  location_name: string;
  default_locale: string;
  supported_locales: string[];
  currency: {
    /** ISO 4217 (ALL default). */
    code: string;
    /** Runtime authority for minor-unit interpretation (0 for ALL). */
    minor_unit: number;
  };
  /** Present+true ONLY for shadow-tenant previews — the sole non-orderable-preview signal. */
  is_preview?: boolean;
  categories: PublicMenuCategory[];
}

export interface WeeklyHoursDay {
  day: 'monday' | 'tuesday' | 'wednesday' | 'thursday' | 'friday' | 'saturday' | 'sunday';
  isOpen: boolean;
  open: string | null;
  close: string | null;
}

export interface PublicLocationInfo {
  id: string;
  name: string;
  slug: string;
  currency_code: string;
  currency_minor_unit: number;
  default_locale: string;
  deliveryFeeFlat: number | null;
  freeDeliveryThreshold: number | null;
  minOrderValue: number | null;
  /** Config fraction (e.g. 0.2) — NOT money. */
  taxRate: number;
  priceIncludesTax: boolean;
  hasDistanceTiers: boolean;
  lat: number | null;
  lng: number | null;
  address: string | null;
  phone: string | null;
  /** Legacy boolean (hours_json day-window AND !delivery_paused). */
  isOpen: boolean;
  /** busy = open AND kitchen_busy_until in the future. */
  status: 'open' | 'closed' | 'busy';
  closesAt: string | null;
  weeklyHours: WeeklyHoursDay[] | null;
  googleRating: number | null;
  googleReviewCount: number | null;
  googleMapsUrl: string | null;
  googlePlaceId: string | null;
  socialInstagram: string | null;
  socialFacebook: string | null;
}

export interface PublicTheme {
  primaryColor: string | null;
  bgColor: string | null;
  textColor: string | null;
  logoUrl: string | null;
  locationName: string;
  headingFont: string | null;
  bodyFont: string | null;
  /** x-quirk: null (not []) when the DB column is not an array. */
  supportedLocales: string[] | null;
}

/** Minimal path map for the 3 S1 operations the /s/[slug] page skeleton consumes. */
export interface S1Paths {
  '/public/locations/{locationIdOrSlug}/menu': {
    get: { response: PublicMenu; error: ErrorEnvelope };
  };
  '/public/locations/{slug}/info': {
    get: { response: PublicLocationInfo; error: ErrorEnvelope };
  };
  '/api/public/theme/{slug}': {
    get: { response: PublicTheme; error: ErrorEnvelope };
  };
}
