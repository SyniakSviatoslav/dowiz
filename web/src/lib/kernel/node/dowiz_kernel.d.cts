/* tslint:disable */
 
/**
 * Ingest a batch of channel events and return aggregated attribution + anomaly
 * counts as JSON: `{orders_by_channel: [[channel,count]...], funnel: {channel:
 * [[status,count]...]}, anomalies: u64}`.
 *
 * `events_json` is an array of `{order_id, channel, status, at_ms}`.
 * `status` is the status name string.
 * @param {string} events_json
 * @returns {string}
 */
export function channel_ledger_js(events_json: string): string;
/**
 * Create a new `Pending` order from a JSON item list.
 *
 * `items_json` is a JSON array of
 * `{product_id, modifier_ids: [], quantity: i64, unit_price: i64}`.
 * Returns the created `Order` serialized to JSON.
 * @param {string | undefined} customer_id
 * @param {string} items_json
 * @param {string | undefined} [channel]
 * @returns {string}
 */
export function place_order_js(customer_id: string | undefined, items_json: string, channel?: string): string;
/**
 * Reduce a raw `(order_id, status, at_ms)` event stream to an anomaly count
 * (`u64`). `events_json` is an array of `{order_id, channel, status, at_ms}`
 * (the `channel` field is accepted but ignored by the reducer).
 * @param {string} events_json
 * @returns {bigint}
 */
export function reduce_anomalies_js(events_json: string): bigint;
/**
 * Advance an order one step. `next_status` is the status name (e.g. "CONFIRMED").
 * Returns the updated order JSON, or a `JsValue` error string on an illegal
 * transition (same status / illegal edge / scaffold disabled).
 * @param {string} order_json
 * @param {string} next_status
 * @returns {string}
 */
export function apply_event_js(order_json: string, next_status: string): string;
