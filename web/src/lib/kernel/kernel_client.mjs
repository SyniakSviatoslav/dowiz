// Self-contained kernel wasm runtime — NO npm deps, NO `import 'wbg'`.
// Runtime block copied verbatim from in-repo wasm/demo/pkg/dowiz_wasm.js
// (wasm-bindgen 0.2.95, same CLI the kernel pins). Loads the kernel wasm in
// Node AND browser. The kernel owns ALL geo/spectral/FSM math; this shell only
// calls the wasm exports and decodes their multivalue returns. Zero re-impl.
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
  const imports = {};
  imports.wbg = {};
  imports.wbg.__wbindgen_string_new = function (arg0, arg1) { return getStringFromWasm0(arg0, arg1); };
  imports.wbg.__wbindgen_throw = function (arg0, arg1) { throw new Error(getStringFromWasm0(arg0, arg1)); };
  imports.wbg.__wbindgen_init_externref_table = function () {
    const table = wasm.__wbindgen_export_0;
    const offset = table.grow(4);
    table.set(0, undefined);
    table.set(offset + 0, undefined);
    table.set(offset + 1, null);
    table.set(offset + 2, true);
    table.set(offset + 3, false);
  };
  return imports;
}
// Bind the kernel wasm. `bytes` is a Uint8Array (Node: readFileSync; browser: fetch+arrayBuffer).
export async function bindKernel(bytes) {
  const { instance } = await WebAssembly.instantiate(bytes, getImports());
  wasm = instance.exports;
  wasm.__wbindgen_start();
  return wasm;
}
// Decode a multivalue shim return [ptr, len, isErr, ext] → fail-closed {ok:false} or {ok:true, value}.
function decodeRet(ret) {
  const isErr = ret[2];
  if (isErr) return { ok: false, err: 'kernel rejected' };
  const s = getStringFromWasm0(ret[0], ret[1]);
  try { wasm.__wbindgen_free(ret[0], ret[1], 1); } catch {}
  return { ok: true, value: s };
}
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
