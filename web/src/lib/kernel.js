// Kernel WASM loader (browser / --target web glue).
// Lazy singleton: init() fetches + instantiates the wasm module once, then the
// exported kernel functions become callable. SSR-safe: callers must invoke
// these only in the browser (guard `window`), since the web-target glue relies
// on fetch/URL for wasm instantiation.
import init, * as K from './kernel/dowiz_kernel.js';

let _ready = null;

async function ready() {
  if (!_ready) _ready = init();
  await _ready;
  return K;
}

// Map the Storefront payload to the kernel items_json shape.
//   Storefront: { locationId, items:[{product_id, modifier_ids, quantity}], cash_pay_with }
//   Kernel:     [{ product_id, modifier_ids, quantity, unit_price }]
// unit_price is 0 until the server prices the line; the kernel still folds
// status (Pending) and stamps a real order id.
function toKernelItems(payload) {
  return (payload.items || []).map((it) => ({
    product_id: it.product_id,
    modifier_ids: it.modifier_ids || [],
    quantity: it.quantity,
    unit_price: 0,
  }));
}

// Create a Pending order via the kernel. Returns the parsed Order JSON:
//   { id, status, items, subtotal, total, created_at_ms, channel }
export async function placeOrder(payload) {
  const k = await ready();
  const itemsJson = JSON.stringify(toKernelItems(payload));
  // place_order_js(customer_id, items_json, channel) — customer_id optional.
  const orderJson = k.place_order_js(undefined, itemsJson, 'storefront');
  return JSON.parse(orderJson);
}

// Apply a status transition. `next` is an OrderStatus name string
// (Pending|Confirmed|Preparing|Ready|InDelivery|Delivered|Rejected|Cancelled|Scheduled|PickedUp).
// Throws on an illegal transition.
export async function applyEvent(order, next) {
  const k = await ready();
  const updated = k.apply_event_js(JSON.stringify(order), next);
  return JSON.parse(updated);
}

// Aggregate a batch of channel events into attribution + anomaly counts.
export async function channelLedger(events) {
  const k = await ready();
  return JSON.parse(k.channel_ledger_js(JSON.stringify(events)));
}
