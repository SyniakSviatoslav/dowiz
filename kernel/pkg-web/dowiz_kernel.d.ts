/* tslint:disable */
/* eslint-disable */
/**
 * Boot-time FSM drift gate (fail-closed) — mirrors [`crate::kernel_boot_verify_fsm`].
 *
 * Call once at web-kernel init, before any order is placed or folded. Returns `"OK"`
 * when the live lifecycle graph matches the golden `FSM_GOLDEN_SIGNATURE`; on any
 * divergence returns an error string naming the moved fields so the host can refuse
 * to start the event bus. (Blueprint `spectral-graph-fsm` §4.)
 * @returns {string}
 */
export function boot_verify_fsm_js(): string;
/**
 * @param {number} remaining_m
 * @param {number} total_m
 * @param {number} baseline_s
 * @returns {string}
 */
export function geo_eta_js(remaining_m: number, total_m: number, baseline_s: number): string;
/**
 * @param {string} poly_json
 * @param {number} pos_lat
 * @param {number} pos_lng
 * @returns {string}
 */
export function geo_progress_js(poly_json: string, pos_lat: number, pos_lng: number): string;
/**
 * @param {bigint} subtotal
 * @param {string} cfg_json
 * @returns {string}
 */
export function estimate_order_total_js(subtotal: bigint, cfg_json: string): string;
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
 * @param {number} a_lat
 * @param {number} a_lng
 * @param {number} b_lat
 * @param {number} b_lng
 * @returns {string}
 */
export function geo_bearing_js(a_lat: number, a_lng: number, b_lat: number, b_lng: number): string;
/**
 * @param {number} remaining_m
 * @param {number} threshold_m
 * @returns {string}
 */
export function geo_is_arriving_js(remaining_m: number, threshold_m: number): string;
/**
 * @param {string} matrix_json
 * @returns {string}
 */
export function spectral_eigenvalues_js(matrix_json: string): string;
/**
 * @param {bigint} last_ts
 * @param {bigint} ts
 * @returns {string}
 */
export function geo_is_out_of_order_js(last_ts: bigint, ts: bigint): string;
/**
 * Advance an order one step. `next_status` is the status name (e.g. "CONFIRMED").
 * Returns the updated order JSON, or a `JsValue` error string on an illegal
 * transition (same status / illegal edge / scaffold disabled).
 * @param {string} order_json
 * @param {string} next_status
 * @returns {string}
 */
export function apply_event_js(order_json: string, next_status: string): string;
/**
 * @param {number} a_lat
 * @param {number} a_lng
 * @param {number} b_lat
 * @param {number} b_lng
 * @param {number} t
 * @returns {string}
 */
export function geo_lerp_js(a_lat: number, a_lng: number, b_lat: number, b_lng: number, t: number): string;
/**
 * Algebraic connectivity (Fiedler λ₂) of a graph from its adjacency matrix.
 * @param {string} adjacency_json
 * @returns {string}
 */
export function spectral_algebraic_connectivity_js(adjacency_json: string): string;
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
 * @param {string} matrix_json
 * @returns {string}
 */
export function spectral_gap_js(matrix_json: string): string;
/**
 * @param {string} poly_json
 * @param {number} pos_lat
 * @param {number} pos_lng
 * @returns {string}
 */
export function geo_progress_flat_js(poly_json: string, pos_lat: number, pos_lng: number): string;
/**
 * @param {string} matrix_json
 * @returns {string}
 */
export function spectral_classify_drift_js(matrix_json: string): string;
/**
 * Harmonic centrality H(v)=Σ 1/d(u,v) for every node `0..n` of an undirected
 * graph. `edges_json` is a JSON array of `[u, v]` pairs; `n` is the node count.
 * Returns a JSON array of length `n`. This is the SAME primitive the agent-kernel
 * (`centrality::harmonic_centrality`) uses for HK-05/HK-06 model routing + memory
 * ranking — both kernels share one compute source, parity-gated.
 * @param {number} n
 * @param {string} edges_json
 * @returns {string}
 */
export function harmonic_centrality_js(n: number, edges_json: string): string;
/**
 * @param {number} pt_lat
 * @param {number} pt_lng
 * @param {string} polygon_json
 * @returns {string}
 */
export function geo_point_in_polygon_js(pt_lat: number, pt_lng: number, polygon_json: string): string;
/**
 * @returns {string}
 */
export function fsm_graph_report_js(): string;
/**
 * @param {string} matrix_json
 * @returns {string}
 */
export function spectral_flat_js(matrix_json: string): string;
/**
 * Reduce a raw `(order_id, status, at_ms)` event stream to an anomaly count
 * (`u64`). `events_json` is an array of `{order_id, channel, status, at_ms}`
 * (the `channel` field is accepted but ignored by the reducer).
 * @param {string} events_json
 * @returns {bigint}
 */
export function reduce_anomalies_js(events_json: string): bigint;
/**
 * @param {string} prev_json
 * @param {string} next_json
 * @param {number} threshold_m
 * @returns {string}
 */
export function geo_should_snap_js(prev_json: string, next_json: string, threshold_m: number): string;
/**
 * @param {string} matrix_json
 * @returns {string}
 */
export function spectral_radius_js(matrix_json: string): string;
/**
 * @param {number} a_lat
 * @param {number} a_lng
 * @param {number} b_lat
 * @param {number} b_lng
 * @returns {string}
 */
export function geo_haversine_js(a_lat: number, a_lng: number, b_lat: number, b_lng: number): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly apply_event_js: (a: number, b: number, c: number, d: number) => Array;
  readonly boot_verify_fsm_js: () => Array;
  readonly channel_ledger_js: (a: number, b: number) => Array;
  readonly estimate_order_total_js: (a: number, b: number, c: number) => Array;
  readonly fsm_graph_report_js: () => Array;
  readonly geo_bearing_js: (a: number, b: number, c: number, d: number) => Array;
  readonly geo_eta_js: (a: number, b: number, c: number) => Array;
  readonly geo_haversine_js: (a: number, b: number, c: number, d: number) => Array;
  readonly geo_is_arriving_js: (a: number, b: number) => Array;
  readonly geo_is_out_of_order_js: (a: number, b: number) => Array;
  readonly geo_lerp_js: (a: number, b: number, c: number, d: number, e: number) => Array;
  readonly geo_point_in_polygon_js: (a: number, b: number, c: number, d: number) => Array;
  readonly geo_progress_flat_js: (a: number, b: number, c: number, d: number) => Array;
  readonly geo_progress_js: (a: number, b: number, c: number, d: number) => Array;
  readonly geo_should_snap_js: (a: number, b: number, c: number, d: number, e: number) => Array;
  readonly harmonic_centrality_js: (a: number, b: number, c: number) => Array;
  readonly place_order_js: (a: number, b: number, c: number, d: number, e: number, f: number) => Array;
  readonly reduce_anomalies_js: (a: number, b: number) => Array;
  readonly spectral_algebraic_connectivity_js: (a: number, b: number) => Array;
  readonly spectral_classify_drift_js: (a: number, b: number) => Array;
  readonly spectral_eigenvalues_js: (a: number, b: number) => Array;
  readonly spectral_flat_js: (a: number, b: number) => Array;
  readonly spectral_gap_js: (a: number, b: number) => Array;
  readonly spectral_radius_js: (a: number, b: number) => Array;
  readonly __wbindgen_export_0: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
