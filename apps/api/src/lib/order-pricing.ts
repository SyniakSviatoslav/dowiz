import { computeLineTotal } from './money.js';

// Pure pricing + modifier-group validation core extracted from the POST /orders
// hotspot (orders.ts). NO database, NO reply — it consumes the already-fetched
// product/modifier/group snapshot Maps and returns either the priced order-item
// rows or the first validation failure. The caller maps `ok: false` to a
// transaction ROLLBACK + reply.sendError(422, code, message), preserving the
// exact codes/messages the inline loop emitted. Keeping this side-effect-free
// makes the money path (RED LINE) unit-testable without a Postgres stub.

export interface PricingItemInput {
  product_id: string;
  quantity: number;
  modifier_ids?: string[];
}

export interface PricedModifierRow {
  modifierId: string;
  nameSnapshot: string;
  priceDeltaSnapshot: number;
}

export interface PricedOrderItemRow {
  productId: string;
  nameSnapshot: string;
  priceSnapshot: number; // price without modifiers
  quantity: number;
  modifiers: PricedModifierRow[];
}

export interface PricingError {
  code: string;
  message: string;
}

export type PricingResult =
  | { ok: true; subtotal: number; orderItemRows: PricedOrderItemRow[] }
  | { ok: false; error: PricingError };

interface ProductInfo {
  name: string;
  price: number;
}
interface ModifierInfo {
  name: string;
  price_delta: number;
  group_id: string;
}
interface GroupInfo {
  id: string;
  min_select: number;
  max_select: number;
  required: boolean;
}

export interface ComputeOrderPricingInput {
  items: PricingItemInput[];
  /** product_id → product snapshot (price authority). Caller guarantees every item's product is present. */
  productMap: Map<string, ProductInfo>;
  /** `${productId}_${modifierId}` → modifier snapshot (available modifiers only). */
  modMap: Map<string, ModifierInfo>;
  /** product_id → its modifier groups (for min/max-select validation). */
  groupsByProduct: Map<string, GroupInfo[]>;
}

/**
 * Replays the inline section-7 pricing loop of POST /orders as a pure function.
 * Returns the first validation failure (matching the original 422 code/message)
 * or the priced rows + subtotal in minor units.
 */
export function computeOrderPricing(input: ComputeOrderPricingInput): PricingResult {
  const { items, productMap, modMap, groupsByProduct } = input;

  let subtotal = 0;
  const orderItemRows: PricedOrderItemRow[] = [];

  for (const item of items) {
    const product = productMap.get(item.product_id)!;

    const groupRows = groupsByProduct.get(item.product_id) || [];

    const groupCounts = new Map<string, number>();
    const modifierPrices: number[] = [];
    const itemModifiersRows: PricedModifierRow[] = [];

    // Reject duplicate modifier ids on a single line-item before counting.
    const modifierIds = item.modifier_ids || [];
    const uniqueModIdsInItem = new Set(modifierIds);
    if (uniqueModIdsInItem.size !== modifierIds.length) {
      return { ok: false, error: { code: 'DUPLICATE_MODIFIER', message: 'Duplicate modifier' } };
    }

    for (const mid of modifierIds) {
      const modInfo = modMap.get(`${item.product_id}_${mid}`);
      if (!modInfo) {
        return {
          ok: false,
          error: { code: 'MODIFIER_UNAVAILABLE', message: `Modifier ${mid} unavailable or invalid for product` },
        };
      }
      const groupId = modInfo.group_id;
      groupCounts.set(groupId, (groupCounts.get(groupId) || 0) + 1);
      modifierPrices.push(modInfo.price_delta);
      itemModifiersRows.push({
        modifierId: mid,
        nameSnapshot: modInfo.name,
        priceDeltaSnapshot: modInfo.price_delta,
      });
    }

    // Validate min/max select per group.
    for (const gRow of groupRows) {
      const count = groupCounts.get(gRow.id) || 0;
      if (gRow.required && count < gRow.min_select) {
        return {
          ok: false,
          error: { code: 'MODIFIER_MIN_NOT_MET', message: `Modifier group ${gRow.id} min select not met` },
        };
      }
      if (count > gRow.max_select) {
        return {
          ok: false,
          error: { code: 'MODIFIER_MAX_EXCEEDED', message: `Modifier group ${gRow.id} max select exceeded` },
        };
      }
    }

    const lineTotal = computeLineTotal(product.price, modifierPrices, item.quantity);
    subtotal += lineTotal;

    orderItemRows.push({
      productId: item.product_id,
      nameSnapshot: product.name,
      priceSnapshot: product.price,
      quantity: item.quantity,
      modifiers: itemModifiersRows,
    });
  }

  return { ok: true, subtotal, orderItemRows };
}
