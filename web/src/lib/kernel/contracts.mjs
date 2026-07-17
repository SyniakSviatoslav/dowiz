// contracts.mjs — predefined JS types + fail-closed validators for the kernel
// `_js` export surface (BLUEPRINT-P-G §3). The authoritative shapes are the
// Rust structs in `kernel/src/wasm.rs`; each typedef cites the struct it
// mirrors. Zero imports — this file is the honest "predefined types" form for a
// no-TypeScript surface. Validators never throw; they return a KernelResult,
// exactly like a kernel rejection, so the UI can never render unvalidated
// kernel-shaped data (T15, blueprint §3).
//
// @typedef {Object} KernelResult
// @property {boolean} ok
// @property {*} [value]  present when ok===true
// @property {string} [err]  present when ok===false
//
// @typedef {Object} OrderItem
// @property {string} product_id
// @property {string[]} modifier_ids
// @property {number} quantity
// @property {number} unit_price   integer minor units
//
// @typedef {Object} OrderOut  mirrors wasm.rs:163-175 order_to_out
// @property {string} id
// @property {string|null} customer_id
// @property {string} status
// @property {OrderItem[]} items
// @property {number} subtotal   integer minor units
// @property {number} total      integer minor units
// @property {number} created_at_ms
// @property {string|null} channel
// @property {number|null} cash_pay_with  integer minor units
//
// @typedef {Object} EstimateOut  mirrors wasm.rs:364-373
// @property {boolean} fee_known
// @property {(number|null)} delivery_fee   integer minor units
// @property {(number|null)} tax_total      integer minor units (null = overflow degrade)
// @property {(number|null)} total           integer minor units (null = degrade)
// @property {boolean} min_not_met
//
// @typedef {Object} ProgressOut  mirrors wasm.rs:439-444
// @property {number} remaining_m
// @property {{lat:number,lng:number}} snapped
// @property {number} segment_index
//
// @typedef {Object} LedgerOut  mirrors wasm.rs:109-119
// @property {[string,number][]} orders_by_channel
// @property {Object<string,[string,number][]>} funnel
// @property {number} anomalies
//
// @typedef {Object} FsmReport  mirrors wasm.rs:412-415 doc
// @property {number} vertices
// @property {number} edges
// @property {boolean} is_acyclic
// @property {number} cyclomatic
// @property {number} spectral_radius
// @property {number} reachable_from_pending
// @property {number} reachable_states
// @property {number} topological_len
//
// @typedef {Object} SpectralFlat  mirrors wasm.rs:736-744 layout doc
// @property {number} rho
// @property {number} gap
// @property {number} fiedler
// @property {number} drift_code
// @property {number} n
// @property {number[]} eigen   flat [re1,im1,re2,im2,...]

/**
 * @param {unknown} x
 * @returns {boolean}
 */
function isSafeInt(x) {
  return typeof x === 'number' && Number.isInteger(x) && Number.isSafeInteger(x);
}

/**
 * Fail-closed JSON parse of a KernelResult<string> from a kernel bridge.
 * @param {{ok:boolean,value?:string,err?:string}} res
 * @returns {{ok:boolean,value?:any,err?:string}}
 */
export function parseJsonResult(res) {
  if (!res.ok) return { ok: false, err: res.err || 'kernel rejected' };
  try {
    return { ok: true, value: JSON.parse(res.value) };
  } catch {
    return { ok: false, err: 'kernel returned non-JSON' };
  }
}

/**
 * Validate an OrderOut (mirrors wasm.rs order_to_out). Money fields must be
 * safe integers. Returns KernelResult.
 * @param {unknown} raw
 * @returns {{ok:boolean,value?:any,err?:string}}
 */
export function validateOrderOut(raw) {
  if (!raw || typeof raw !== 'object') return { ok: false, err: 'order: not an object' };
  const o = /** @type {any} */ (raw);
  if (typeof o.id !== 'string') return { ok: false, err: 'order: id missing' };
  if (typeof o.status !== 'string') return { ok: false, err: 'order: status missing' };
  if (!Array.isArray(o.items)) return { ok: false, err: 'order: items not array' };
  for (const it of o.items) {
    if (typeof it?.product_id !== 'string') return { ok: false, err: 'order: item.product_id missing' };
    if (!Array.isArray(it?.modifier_ids)) return { ok: false, err: 'order: item.modifier_ids not array' };
    if (!isSafeInt(it?.quantity)) return { ok: false, err: 'order: item.quantity not safe int' };
    if (!isSafeInt(it?.unit_price)) return { ok: false, err: 'order: item.unit_price not safe int (money)' };
  }
  if (!isSafeInt(o.subtotal)) return { ok: false, err: 'order: subtotal not safe int (money)' };
  if (!isSafeInt(o.total)) return { ok: false, err: 'order: total not safe int (money)' };
  if (o.customer_id !== null && typeof o.customer_id !== 'string')
    return { ok: false, err: 'order: customer_id type' };
  if (o.channel !== null && typeof o.channel !== 'string')
    return { ok: false, err: 'order: channel type' };
  if (o.cash_pay_with !== null && !isSafeInt(o.cash_pay_with))
    return { ok: false, err: 'order: cash_pay_with not safe int (money)' };
  if (!isSafeInt(o.created_at_ms)) return { ok: false, err: 'order: created_at_ms not safe int' };
  return { ok: true, value: o };
}

/**
 * Validate an EstimateOut (mirrors wasm.rs:364-373). Money fields may be null
 * (overflow degrade) but if present must be safe integers.
 * @param {unknown} raw
 * @returns {{ok:boolean,value?:any,err?:string}}
 */
export function validateEstimateOut(raw) {
  if (!raw || typeof raw !== 'object') return { ok: false, err: 'estimate: not an object' };
  const o = /** @type {any} */ (raw);
  if (typeof o.fee_known !== 'boolean') return { ok: false, err: 'estimate: fee_known type' };
  if (o.delivery_fee !== null && !isSafeInt(o.delivery_fee))
    return { ok: false, err: 'estimate: delivery_fee not safe int (money)' };
  if (o.tax_total !== null && !isSafeInt(o.tax_total))
    return { ok: false, err: 'estimate: tax_total not safe int (money)' };
  if (o.total !== null && !isSafeInt(o.total))
    return { ok: false, err: 'estimate: total not safe int (money)' };
  if (typeof o.min_not_met !== 'boolean') return { ok: false, err: 'estimate: min_not_met type' };
  return { ok: true, value: o };
}

/**
 * Validate a ProgressOut (mirrors wasm.rs:439-444).
 * @param {unknown} raw
 * @returns {{ok:boolean,value?:any,err?:string}}
 */
export function validateProgressOut(raw) {
  if (!raw || typeof raw !== 'object') return { ok: false, err: 'progress: not an object' };
  const o = /** @type {any} */ (raw);
  if (typeof o.remaining_m !== 'number') return { ok: false, err: 'progress: remaining_m type' };
  if (typeof o.snapped?.lat !== 'number' || typeof o.snapped?.lng !== 'number')
    return { ok: false, err: 'progress: snapped type' };
  if (!isSafeInt(o.segment_index)) return { ok: false, err: 'progress: segment_index not safe int' };
  return { ok: true, value: o };
}

/**
 * Validate a LedgerOut (mirrors wasm.rs:109-119). BTreeMap ⇒ deterministic keys.
 * @param {unknown} raw
 * @returns {{ok:boolean,value?:any,err?:string}}
 */
export function validateLedgerOut(raw) {
  if (!raw || typeof raw !== 'object') return { ok: false, err: 'ledger: not an object' };
  const o = /** @type {any} */ (raw);
  if (!Array.isArray(o.orders_by_channel)) return { ok: false, err: 'ledger: orders_by_channel not array' };
  if (typeof o.funnel !== 'object' || o.funnel === null) return { ok: false, err: 'ledger: funnel not object' };
  if (!isSafeInt(o.anomalies)) return { ok: false, err: 'ledger: anomalies not safe int' };
  return { ok: true, value: o };
}

/**
 * Validate an FsmReport (mirrors wasm.rs:412-415 doc).
 * @param {unknown} raw
 * @returns {{ok:boolean,value?:any,err?:string}}
 */
export function validateFsmReport(raw) {
  if (!raw || typeof raw !== 'object') return { ok: false, err: 'fsm: not an object' };
  const o = /** @type {any} */ (raw);
  const needNum = ['vertices', 'edges', 'cyclomatic', 'spectral_radius', 'reachable_from_pending', 'reachable_states', 'topological_len'];
  for (const k of needNum) if (typeof o[k] !== 'number') return { ok: false, err: `fsm: ${k} type` };
  if (typeof o.is_acyclic !== 'boolean') return { ok: false, err: 'fsm: is_acyclic type' };
  return { ok: true, value: o };
}

/**
 * Validate a SpectralFlat (mirrors wasm.rs:736-744 layout). Input is a numeric
 * array [rho, gap, fiedler, drift_code, n, re1, im1, ...].
 * @param {unknown} raw
 * @returns {{ok:boolean,value?:any,err?:string}}
 */
export function validateSpectralFlat(raw) {
  if (!Array.isArray(raw) || raw.length < 5) return { ok: false, err: 'spectral_flat: array<5' };
  for (const v of raw) if (typeof v !== 'number') return { ok: false, err: 'spectral_flat: non-number' };
  const n = raw[4];
  if (!isSafeInt(n) || raw.length !== 5 + 2 * n) return { ok: false, err: 'spectral_flat: eigen length mismatch' };
  return {
    ok: true,
    value: { rho: raw[0], gap: raw[1], fiedler: raw[2], drift_code: raw[3], n, eigen: raw.slice(5) },
  };
}

/**
 * Fail-closed guard for a safe-integer passed to an i64 kernel param.
 * @param {unknown} x
 * @param {string} name
 * @returns {{ok:boolean,value?:bigint,err?:string}}
 */
export function toI64(x, name) {
  if (!isSafeInt(x)) return { ok: false, err: `${name}: not a safe integer (i64)` };
  return { ok: true, value: BigInt(/** @type {number} */ (x)) };
}
