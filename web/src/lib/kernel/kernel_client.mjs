// Self-contained kernel wasm runtime — NO npm deps, NO `import 'wbg'`.
// Runtime block re-synced verbatim against the CURRENT generated
// kernel/pkg-web/dowiz_kernel.js (wasm-bindgen 0.2.126 — the version the
// kernel's Cargo.lock actually pins; a stale 0.2.95 copy here broke every
// call with "Import #0 module=./dowiz_kernel_bg.js: module is not an object
// or function", since 0.2.126 renamed the import module key `wbg` →
// `./dowiz_kernel_bg.js`, the externref table export `__wbindgen_export_0` →
// `__wbindgen_externrefs`, and — for `Result<String, _>` returns — moved the
// isErr flag from tuple index 2 to index 3). Loads the kernel wasm in Node
// AND browser. The kernel owns ALL geo/spectral/FSM math; this shell only
// calls the wasm exports and decodes their multivalue returns. Zero re-impl.
//
// CAUTION: this is a hand-maintained copy of wasm-bindgen-generated glue, not
// a stable public ABI — re-sync it against the freshly generated
// kernel/pkg-web/dowiz_kernel.js after any `wasm-bindgen` CLI/crate version
// bump or wasm.rs surface change (new/changed `#[wasm_bindgen]` fn can shift
// the auto-numbered cast intrinsics wasm-bindgen emits).
let wasm;
const cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
  if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0)
    cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
  return cachedUint8ArrayMemory0;
}
function getStringFromWasm0(ptr, len) {
  ptr = ptr >>> 0;
  return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}
const cachedTextEncoder = new TextEncoder('utf-8');
function passStringToWasm0(arg) {
  const buf = cachedTextEncoder.encode(arg);
  const ptr = wasm.__wbindgen_malloc(buf.length, 1) >>> 0;
  getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
  return [ptr, buf.length];
}
function getImports() {
  // wasm-bindgen 0.2.126 imports by module-path key (matching the generated
  // glue's own filename), not the old fixed `"wbg"` key.
  const import0 = {
    // "Cast intrinsic for `Ref(String) -> Externref`" (wasm-bindgen's name for
    // this one, per the current generated glue) — same job the old
    // `__wbindgen_string_new` did.
    __wbindgen_cast_0000000000000001: function (arg0, arg1) { return getStringFromWasm0(arg0, arg1); },
    __wbindgen_init_externref_table: function () {
      const table = wasm.__wbindgen_externrefs;
      const offset = table.grow(4);
      table.set(0, undefined);
      table.set(offset + 0, undefined);
      table.set(offset + 1, null);
      table.set(offset + 2, true);
      table.set(offset + 3, false);
    },
  };
  return { './dowiz_kernel_bg.js': import0 };
}
// Bind the kernel wasm. `bytes` is a Uint8Array (Node: readFileSync; browser: fetch+arrayBuffer).
export async function bindKernel(bytes) {
  const { instance } = await WebAssembly.instantiate(bytes, getImports());
  wasm = instance.exports;
  wasm.__wbindgen_start();
  return wasm;
}
// Decode a multivalue shim return [ptr, len, externrefIdx, isErr] → fail-closed
// {ok:false} or {ok:true, value}. Used by every export that returns
// Result<String, JsValue>. (isErr lives at index 3, not 2, under wasm-bindgen
// 0.2.126 — see the file-header note.)
function decodeRet(ret) {
  const isErr = ret[3];
  if (isErr) {
    try { wasm.__externref_table_dealloc(ret[2]); } catch {}
    return { ok: false, err: 'kernel rejected' };
  }
  const s = getStringFromWasm0(ret[0], ret[1]);
  try { wasm.__wbindgen_free(ret[0], ret[1], 1); } catch {}
  return { ok: true, value: s };
}

// ── Already-bound Wave-1 (frozen signatures, never break userspace — §11) ──
export function spectral_radius_js(matrixJson) {
  const [p, l] = passStringToWasm0(matrixJson);
  return decodeRet(wasm.spectral_radius_js(p, l));
}
export function geo_progress_flat_js(polyJson, lat, lng) {
  const [p, l] = passStringToWasm0(polyJson);
  return decodeRet(wasm.geo_progress_flat_js(p, l, lat, lng));
}
export function fsm_graph_report_js() {
  return decodeRet(wasm.fsm_graph_report_js());
}

// ── G2 Wave A — order / money / boot gate (BLUEPRINT-P-G §5) ──
//
// place_order_js takes Option<String> customer_id / channel. Pass [0,0] (a null
// ptr/len pair) for None; otherwise a passStringToWasm0 tuple. ABI confirmed by
// the wasm.rs:789-798 place_order_round_trip oracle (PENDING + Σ qty·unit_price).
export function place_order_js(customerId, itemsJson, channel) {
  const c = customerId == null ? [0, 0] : passStringToWasm0(customerId);
  const [ip, il] = passStringToWasm0(itemsJson);
  const ch = channel == null ? [0, 0] : passStringToWasm0(channel);
  return decodeRet(wasm.place_order_js(c[0], c[1], ip, il, ch[0], ch[1]));
}
export function apply_event_js(orderJson, nextStatus) {
  const [op, ol] = passStringToWasm0(orderJson);
  const [np, nl] = passStringToWasm0(nextStatus);
  return decodeRet(wasm.apply_event_js(op, ol, np, nl));
}
// estimate_order_total_js(subtotal: i64, cfg_json). i64 requires a BigInt at the
// raw wasm call site (Number 1300 throws "Cannot convert to BigInt"). The wrapper
// accepts a JS safe integer, validates, and converts — never fabricates. Returns
// the SAME 4-tuple string protocol as the other Result<String> exports.
export function estimate_order_total_js(subtotal, cfgJson) {
  if (typeof subtotal !== 'number' || !Number.isInteger(subtotal) || !Number.isSafeInteger(subtotal))
    return { ok: false, err: 'estimate: subtotal not a safe integer (i64)' };
  const [cp, cl] = passStringToWasm0(cfgJson);
  return decodeRet(wasm.estimate_order_total_js(BigInt(subtotal), cp, cl));
}
// boot_verify_fsm_js — the fail-closed FSM drift gate. Call once at web-kernel
// init before any order (wasm.rs:326). Unwired before G (audit finding h).
export function boot_verify_fsm_js() {
  return decodeRet(wasm.boot_verify_fsm_js());
}

// ── G2 Wave B — product-adjacent analytics / geo / spectral-flat ──
export function channel_ledger_js(eventsJson) {
  const [p, l] = passStringToWasm0(eventsJson);
  return decodeRet(wasm.channel_ledger_js(p, l));
}
export function spectral_flat_js(matrixJson) {
  const [p, l] = passStringToWasm0(matrixJson);
  return decodeRet(wasm.spectral_flat_js(p, l));
}
export function geo_haversine_js(aLat, aLng, bLat, bLng) {
  return decodeRet(wasm.geo_haversine_js(aLat, aLng, bLat, bLng));
}
export function geo_lerp_js(aLat, aLng, bLat, bLng, t) {
  return decodeRet(wasm.geo_lerp_js(aLat, aLng, bLat, bLng, t));
}
export function geo_eta_js(remainingM, totalM, baselineS) {
  return decodeRet(wasm.geo_eta_js(remainingM, totalM, baselineS));
}
export function geo_is_arriving_js(remainingM, thresholdM) {
  return decodeRet(wasm.geo_is_arriving_js(remainingM, thresholdM));
}
export function geo_should_snap_js(prevJson, nextJson, thresholdM) {
  const [pp, pl] = passStringToWasm0(prevJson);
  const [np, nl] = passStringToWasm0(nextJson);
  return decodeRet(wasm.geo_should_snap_js(pp, pl, np, nl, thresholdM));
}
export function geo_progress_js(polyJson, posLat, posLng) {
  const [p, l] = passStringToWasm0(polyJson);
  return decodeRet(wasm.geo_progress_js(p, l, posLat, posLng));
}

// ── G2 Wave C — completes the surface ──
export function geo_bearing_js(aLat, aLng, bLat, bLng) {
  return decodeRet(wasm.geo_bearing_js(aLat, aLng, bLat, bLng));
}
export function geo_point_in_polygon_js(ptLat, ptLng, polygonJson) {
  const [p, l] = passStringToWasm0(polygonJson);
  return decodeRet(wasm.geo_point_in_polygon_js(ptLat, ptLng, p, l));
}
// geo_is_out_of_order_js(last_ts: i64, ts: i64). i64 input → BigInt. JS sends -1
// to mean "no previous timestamp" (None) per wasm.rs:610.
export function geo_is_out_of_order_js(lastTs, ts) {
  if (typeof lastTs !== 'number' || !Number.isInteger(lastTs) || !Number.isSafeInteger(lastTs))
    return { ok: false, err: 'geo_is_out_of_order: last_ts not a safe integer (i64)' };
  if (typeof ts !== 'number' || !Number.isInteger(ts) || !Number.isSafeInteger(ts))
    return { ok: false, err: 'geo_is_out_of_order: ts not a safe integer (i64)' };
  return decodeRet(wasm.geo_is_out_of_order_js(BigInt(lastTs), BigInt(ts)));
}
export function spectral_eigenvalues_js(matrixJson) {
  const [p, l] = passStringToWasm0(matrixJson);
  return decodeRet(wasm.spectral_eigenvalues_js(p, l));
}
export function spectral_gap_js(matrixJson) {
  const [p, l] = passStringToWasm0(matrixJson);
  return decodeRet(wasm.spectral_gap_js(p, l));
}
export function spectral_algebraic_connectivity_js(adjacencyJson) {
  const [p, l] = passStringToWasm0(adjacencyJson);
  return decodeRet(wasm.spectral_algebraic_connectivity_js(p, l));
}
export function spectral_classify_drift_js(matrixJson) {
  const [p, l] = passStringToWasm0(matrixJson);
  return decodeRet(wasm.spectral_classify_drift_js(p, l));
}
export function harmonic_centrality_js(n, edgesJson) {
  if (typeof n !== 'number' || !Number.isInteger(n) || !Number.isSafeInteger(n))
    return { ok: false, err: 'harmonic_centrality: n not a safe integer' };
  const [p, l] = passStringToWasm0(edgesJson);
  return decodeRet(wasm.harmonic_centrality_js(n, p, l));
}
// reduce_anomalies_js returns Result<u64, JsValue>. The wasm multivalue ABI is
// [value, errHandle, isErr] (authoritative: kernel/pkg-web/dowiz_kernel.js:629-637 —
// reads ret[0] as the full u64 via BigInt.asUintN(64, ret[0]), ret[2] as the error
// flag). The kernel traps (throws) on error; we surface that as {ok:false} so the
// caller never has to catch a wasm trap. Value is a JS BigInt for the i64 return;
// downcast to a safe Number, refusing overflow (blueprint §5: ledger carries the
// same count, so this path is fail-closed not lossy).
export function reduce_anomalies_js(eventsJson) {
  const [p, l] = passStringToWasm0(eventsJson);
  try {
    const ret = wasm.reduce_anomalies_js(p, l); // [value, errHandle, isErr]
    if (ret[2] !== 0) return { ok: false, err: 'kernel rejected' };
    const v = ret[0];
    const num = typeof v === 'bigint' ? Number(v) : v;
    if (typeof v === 'bigint' && !Number.isSafeInteger(num))
      return { ok: false, err: 'anomaly count exceeds safe integer' };
    return { ok: true, value: num };
  } catch (e) {
    return { ok: false, err: typeof e === 'object' && e?.message ? e.message : 'kernel rejected' };
  }
}

// 24/24 kernel _js exports now bound (3 frozen + 21 added in G).
