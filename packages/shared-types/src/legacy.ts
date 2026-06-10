import { z } from 'zod';

// ─── Order Item Input ────────────────────────────────────────────────
export const OrderItemInput = z.object({
  product_id: z.string().uuid(),
  quantity: z.number().int().positive().max(99),
  modifier_ids: z.array(z.string().uuid()).optional().default([]),
}).strict();
export type OrderItemInput = z.infer<typeof OrderItemInput>;

// ─── Create Order Input ──────────────────────────────────────────────
export const CreateOrderInput = z.object({
  locationId: z.string().uuid(),
  type: z.literal('delivery'),
  items: z.array(OrderItemInput).min(1),
  customer: z.object({
    phone: z.string().min(6).max(20).optional(),
    name: z.string().min(1).max(120).optional(),
  }).strict().optional(),
  delivery: z.object({
    pin: z.object({
      lat: z.number().min(-90).max(90),
      lng: z.number().min(-180).max(180),
    }).strict(),
    address_text: z.string().min(1).max(500).optional(),
  }).strict(),
  payment: z.object({
    method: z.literal('cash'),
  }).strict(),
  cash_pay_with: z.number().int().positive().optional(),
  idempotency_key: z.string().uuid(),
  // Preflight (E27)
  acknowledged_codes: z.array(z.string()).max(10).optional().default([]),
  otp_code: z.string().length(6).regex(/^\d{6}$/).optional(),
}).strict();
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
  type: z.literal('delivery'),
  deliveryAddress: z.string().nullable(),
  subtotal: z.number().int(),
  total: z.number().int(),
  paymentMethod: z.literal('cash'),
  paymentOutcome: z.string(),
  createdAt: z.string(),
  timeoutAt: z.string().nullable(),
  items: z.array(OrderItemResponse),
}).strict();
export type OrderResponse = z.infer<typeof OrderResponse>;

// ─── AuthToken ───────────────────────────────────────────────────────
const AuthBase = { sub: z.string().uuid(), iat: z.number().int(), exp: z.number().int(), kid: z.string() };
export const AuthToken = z.discriminatedUnion('role', [
  z.object({ role: z.literal('owner'), userId: z.string().uuid(), ...AuthBase }).strict(),
  z.object({ role: z.literal('courier'), activeLocationId: z.string().uuid(), jti: z.string().uuid().optional(), ...AuthBase }).strict(),
  z.object({
    role: z.literal('customer'),
    orderId: z.string().uuid(),
    locationId: z.string().uuid(),
    phone: z.string(),
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

export interface ParseResult {
  draft: CanonicalMenuDraft;
  issues: ParseIssue[];
  summary: { valid: number; errors: number; warnings: number; mode: ParseMode; low_confidence_count?: number };
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
