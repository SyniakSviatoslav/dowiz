/* tslint:disable */
/* eslint-disable */

/**
 * Advance an order one step. `next_status` is the status name (e.g. "CONFIRMED").
 * Returns the updated order JSON, or a `JsValue` error string on an illegal
 * transition (same status / illegal edge / scaffold disabled).
 */
export function apply_event_js(order_json: string, next_status: string): string;

/**
 * Boot-time FSM drift gate (fail-closed) — mirrors [`crate::kernel_boot_verify_fsm`].
 *
 * Call once at web-kernel init, before any order is placed or folded. Returns `"OK"`
 * when the live lifecycle graph matches the golden `FSM_GOLDEN_SIGNATURE`; on any
 * divergence returns an error string naming the moved fields so the host can refuse
 * to start the event bus. (Blueprint `spectral-graph-fsm` §4.)
 */
export function boot_verify_fsm_js(): string;

/**
 * Ingest a batch of channel events and return aggregated attribution + anomaly
 * counts as JSON: `{orders_by_channel: [[channel,count]...], funnel: {channel:
 * [[status,count]...]}, anomalies: u64}`.
 *
 * `events_json` is an array of `{order_id, channel, status, at_ms}`.
 * `status` is the status name string.
 */
export function channel_ledger_js(events_json: string): string;

export function estimate_order_total_js(subtotal: bigint, cfg_json: string): string;

export function fsm_graph_report_js(): string;

export function geo_bearing_js(a_lat: number, a_lng: number, b_lat: number, b_lng: number): string;

export function geo_eta_js(remaining_m: number, total_m: number, baseline_s: number): string;

export function geo_haversine_js(a_lat: number, a_lng: number, b_lat: number, b_lng: number): string;

export function geo_is_arriving_js(remaining_m: number, threshold_m: number): string;

export function geo_is_out_of_order_js(last_ts: bigint, ts: bigint): string;

export function geo_lerp_js(a_lat: number, a_lng: number, b_lat: number, b_lng: number, t: number): string;

export function geo_point_in_polygon_js(pt_lat: number, pt_lng: number, polygon_json: string): string;

export function geo_progress_flat_js(poly_json: string, pos_lat: number, pos_lng: number): string;

export function geo_progress_js(poly_json: string, pos_lat: number, pos_lng: number): string;

export function geo_should_snap_js(prev_json: string, next_json: string, threshold_m: number): string;

/**
 * Harmonic centrality H(v)=Σ 1/d(u,v) for every node `0..n` of an undirected
 * graph. `edges_json` is a JSON array of `[u, v]` pairs; `n` is the node count.
 * Returns a JSON array of length `n`. This is the SAME primitive the agent-kernel
 * (`centrality::harmonic_centrality`) uses for HK-05/HK-06 model routing + memory
 * ranking — both kernels share one compute source, parity-gated.
 */
export function harmonic_centrality_js(n: number, edges_json: string): string;

/**
 * Create a new `Pending` order from a JSON item list.
 *
 * `items_json` is a JSON array of
 * `{product_id, modifier_ids: [], quantity: i64, unit_price: i64}`.
 * Returns the created `Order` serialized to JSON.
 */
export function place_order_js(customer_id: string | null | undefined, items_json: string, channel?: string | null): string;

/**
 * Reduce a raw `(order_id, status, at_ms)` event stream to an anomaly count
 * (`u64`). `events_json` is an array of `{order_id, channel, status, at_ms}`
 * (the `channel` field is accepted but ignored by the reducer).
 */
export function reduce_anomalies_js(events_json: string): bigint;

/**
 * Algebraic connectivity (Fiedler λ₂) of a graph from its adjacency matrix.
 */
export function spectral_algebraic_connectivity_js(adjacency_json: string): string;

export function spectral_classify_drift_js(matrix_json: string): string;

export function spectral_eigenvalues_js(matrix_json: string): string;

export function spectral_flat_js(matrix_json: string): string;

export function spectral_gap_js(matrix_json: string): string;

export function spectral_radius_js(matrix_json: string): string;
