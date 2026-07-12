import { z } from 'zod';

export const DropoffPreference = z.enum([
  'hand_to_me',
  'leave_at_door',
  'meet_outside',
  'meet_in_lobby',
]);
export type DropoffPreference = z.infer<typeof DropoffPreference>;

export const SubstitutionDefault = z.enum([
  'replace_similar',
  'remove_refund',
  'cancel_order',
  'contact_me',
]);
export type SubstitutionDefault = z.infer<typeof SubstitutionDefault>;

export const OrderPreferences = z.object({
  dropoff: z.object({
    preference: DropoffPreference.optional(),
    note: z.string().max(200).optional(),
    entrance: z.string().max(100).optional(),
    floor: z.number().int().min(1).max(50).optional(),
    apartment: z.string().max(100).optional(),
    code: z.string().max(20).optional(),
  }).optional(),
  substitution: SubstitutionDefault.optional(),
}).strict();
export type OrderPreferences = z.infer<typeof OrderPreferences>;

export const OrderItemInput = z.object({
  product_id: z.string().uuid(),
  quantity: z.number().int().positive().max(99),
  modifier_ids: z.array(z.string().uuid()).optional().default([]),
}).strict();
export type OrderItemInput = z.infer<typeof OrderItemInput>;

// Canonical v1 messenger kinds — MUST stay in sync with packages/db migration
// 1790000000074 (customers/couriers/orders CHECK constraints) and
// apps/web/src/lib/messenger.ts (MESSENGER_KINDS). The legacy enum here only had
// 3 kinds, so any 'phone'/'signal'/'simplex' contact (incl. every "deliver to
// someone else" receiver) failed VALIDATION_FAILED with HTTP 400 (G03).
export const MessengerKind = z.enum(['phone', 'whatsapp', 'viber', 'telegram', 'signal', 'simplex']);
export type MessengerKind = z.infer<typeof MessengerKind>;

// "Deliver to someone else" — the receiver's own contact channel (ADR-0016 /
// ADR-checkout-communication). Optional (null when shipping to the customer);
// when present, all three fields are required so the persistence path never
// receives a partial receiver. Previously ABSENT → .strict() rejected it → 400.
export const ReceiverInput = z.object({
  name: z.string().min(1).max(120),
  messenger_kind: MessengerKind,
  handle: z.string().min(1).max(120),
}).strict();
export type ReceiverInput = z.infer<typeof ReceiverInput>;

  // ─── Create Order Input ──────────────────────────────────────────────
  export const CreateOrderInput = z.object({
    locationId: z.string().uuid(),
    type: z.enum(['delivery', 'pickup']),
    items: z.array(OrderItemInput).min(1),
    customer: z.object({
      phone: z.string().min(6).max(20).optional(),
      name: z.string().min(1).max(120).optional(),
      // UX-2 messenger deep-link (optional secondary contact channel).
      messenger_kind: MessengerKind.optional(),
      messenger_handle: z.string().min(1).max(120).optional(),
    }).strict().optional(),
    // "Deliver to someone else" — receiver's own contact (nullable; same-receiver = null).
    receiver: ReceiverInput.optional(),
    delivery: z.object({
      pin: z.object({
        lat: z.number().min(-90).max(90),
        lng: z.number().min(-180).max(180),
      }).strict(),
      address_text: z.string().min(1).max(500).optional(),
    }).strict().optional(),
    payment: z.object({
      method: z.literal('cash'),
    }).strict(),
    cash_pay_with: z.number().int().positive().optional(),
    delivery_instructions: z.string().max(500).optional(),
    // UX-3 entry-anchor photo: the R2 key returned by the anonymous upload endpoint.
    delivery_photo_key: z.string().max(200).regex(/^entry-photos\/[A-Za-z0-9._-]+\.webp$/).optional(),
    // UX-4: optional courier tip (integer minor units, >= 0).
    tip_amount: z.number().int().min(0).max(10_000_000).optional(),
    prefs: OrderPreferences.optional(),
    idempotency_key: z.string().uuid(),
    // Preflight (E27)
    acknowledged_codes: z.array(z.string()).max(10).optional().default([]),
    otp_code: z.string().length(6).regex(/^\d{6}$/).optional(),
  }).strict().superRefine((val, ctx) => {
    // Delivery orders must carry a delivery pin; pickup orders must not.
    if (val.type === 'delivery' && !val.delivery) {
      ctx.addIssue({ code: z.ZodIssueCode.custom, path: ['delivery'], message: 'delivery is required for delivery orders' });
    }
  });

export type CreateOrderInput = z.infer<typeof CreateOrderInput>;

// ─── Order Status ────────────────────────────────────────────────────
export const OrderStatusEnum = z.enum([
  'PENDING',
  'CONFIRMED',
  'PREPARING',
  'READY',
  'IN_DELIVERY',
  'DELIVERED',
  'REJECTED',
  'CANCELLED',
  'SCHEDULED',
  'PICKED_UP',
]);
export type OrderStatus = z.infer<typeof OrderStatusEnum>;

// ─── Status Update Input ─────────────────────────────────────────────
export const StatusUpdateInput = z.object({
  status: OrderStatusEnum,
}).strict();
export type StatusUpdateInput = z.infer<typeof StatusUpdateInput>;

// ─── Order Item Response ─────────────────────────────────────────────
export const OrderItemResponse = z.object({
  id: z.string().uuid(),
  productId: z.string().uuid(),
  nameSnapshot: z.string(),
  priceSnapshot: z.number().int(),
  quantity: z.number().int().positive(),
}).strict();
export type OrderItemResponse = z.infer<typeof OrderItemResponse>;

// ─── Order Response ──────────────────────────────────────────────────
export const OrderResponse = z.object({
  id: z.string().uuid(),
  locationId: z.string().uuid(),
  customerId: z.string().uuid().nullable(),
  status: OrderStatusEnum,
  type: z.enum(['delivery', 'pickup']),
  deliveryAddress: z.string().nullable(),
  deliveryInstructions: z.string().nullable(),
  subtotal: z.number().int(),
  total: z.number().int(),
  paymentMethod: z.literal('cash'),
  paymentOutcome: z.string(),
  createdAt: z.string(),
  timeoutAt: z.string().nullable(),
  items: z.array(OrderItemResponse),
}).strict();
export type OrderResponse = z.infer<typeof OrderResponse>;

// ─── Customer Order Status Response (CR-5) ───────────────────────────
export const CustomerOrderStatusResponse = z.object({
  id: z.string().uuid(),
  status: OrderStatusEnum,
  type: z.enum(['delivery', 'pickup']).optional(),
  deliveryAddress: z.string().nullable(),
  deliveryInstructions: z.string().nullable(),
  total: z.number().int(),
  items: z.array(OrderItemResponse),
  createdAt: z.string(),
  etaMinutes: z.number().int().nullable(),
  courierName: z.string().nullable(),
  courierPhoneMasked: z.string().nullable(),
  courierPosition: z.object({
    lat: z.number(),
    lng: z.number(),
  }).nullable(),
  deliveryLat: z.number().nullable(),
  deliveryLng: z.number().nullable(),
  // ORDER-TRACKING: per-transition timestamps (ISO strings), nullable until the
  // step is reached. Optional keeps older payloads valid under .strict().
  confirmedAt: z.string().nullable().optional(),
  preparingAt: z.string().nullable().optional(),
  readyAt: z.string().nullable().optional(),
  inDeliveryAt: z.string().nullable().optional(),
  deliveredAt: z.string().nullable().optional(),
  pickedUpAt: z.string().nullable().optional(),
}).strict();
export type CustomerOrderStatusResponse = z.infer<typeof CustomerOrderStatusResponse>;

// ─── AuthToken ───────────────────────────────────────────────────────
const AuthBase = { sub: z.string().uuid(), iat: z.number().int(), exp: z.number().int(), kid: z.string() };
export const AuthToken = z.discriminatedUnion('role', [
  z.object({ role: z.literal('owner'), userId: z.string().uuid(), activeLocationId: z.string().uuid().optional(), ...AuthBase }).strict(),
  z.object({ role: z.literal('courier'), activeLocationId: z.string().uuid(), jti: z.string().uuid().optional(), ...AuthBase }).strict(),
  z.object({
    role: z.literal('customer'),
    orderId: z.string().uuid(),
    locationId: z.string().uuid(),
    // P0-PII: phone intentionally NOT in the customer claim (PII in a long-lived
    // bearer token). Consumers resolve the phone server-side via orderId / sub.
    ...AuthBase,
  }).strict(),
]);
export type AuthToken = z.infer<typeof AuthToken>;

// ─── Menu Import Types ───────────────────────────────────────────────
export const ParseModeEnum = z.enum(['replace', 'add_only', 'merge']);
export type ParseMode = z.infer<typeof ParseModeEnum>;

export interface CanonicalCategory {
  externalKey: string;
  name: string;
}

export interface CanonicalProduct {
  externalKey: string;
  categoryKey: string;
  name: string;
  description?: string;
  price: number;
  currency: string;
  available: boolean;
  attributesJson?: Record<string, unknown>;
  imageKey?: string;
}

export interface CanonicalModifierGroup {
  externalKey: string;
  name: string;
  minSelect: number;
  maxSelect: number;
  required: boolean;
}

export interface CanonicalModifier {
  externalKey: string;
  groupKey: string;
  name: string;
  priceDelta: number;
  available: boolean;
  sortOrder?: number;
}

export interface CanonicalMenuDraft {
  categories: CanonicalCategory[];
  products: CanonicalProduct[];
  modifierGroups: CanonicalModifierGroup[];
  modifiers: CanonicalModifier[];
  links: { productKey: string; groupKey: string; sortOrder: number }[];
  translations: { entity: 'category' | 'product'; entityKey: string; locale: string; name?: string; description?: string }[];
}

export interface ParseIssue {
  rowNumber: number;
  column?: string;
  code: 'INVALID_PRICE' | 'INVALID_PRICE_FOR_MINOR_UNIT' | 'MISSING_REQUIRED' | 'POTENTIALLY_UNSAFE_VALUE' | 'UNKNOWN_COLUMN' | 'EMPTY_ROW' | 'INVALID_QUANTITY' | 'INVALID_MODIFIER_PRICE_DELTA' | 'DUPLICATE_KEY' | 'CURRENCY_MISMATCH' | 'PARSE_ERROR';
  message: string;
  severity: 'error' | 'warning';
  raw?: string;
}

// Restaurant-level metadata extracted from a menu document (PDF/photo). Business
// contact info only — NOT customer PII. Surfaced for review and written to the
// location on commit so onboarding can pre-fill address/phone from the menu.
export interface RestaurantMeta {
  name?: string;
  address?: string;
  phone?: string;
  hoursText?: string;
}

export interface ParseResult {
  draft: CanonicalMenuDraft;
  issues: ParseIssue[];
  summary: { valid: number; errors: number; warnings: number; mode: ParseMode; low_confidence_count?: number };
  restaurant?: RestaurantMeta;
}

export interface CsvParseConfig {
  delimiter?: ',' | ';' | '\t';
  hasHeader?: boolean;
  columnMap?: Record<string, string>;
  locale?: string;
  expectedCurrency?: string;
  currencyMinorUnit?: number;
}

export interface AiOcrConfig {
  source_locale?: string;
  target_locale?: string;
  model?: string;
  llm_instructions?: string;
}

export interface TranslateRequest {
  texts: string[];
  from: string;
  to: string;
  context?: 'menu_item' | 'category' | 'modifier' | 'description';
}

export interface TranslateResponse {
  translations: string[];
  provider_id: string;
  model_id: string;
  pii_redacted_count: number;
  duration_ms: number;
}
