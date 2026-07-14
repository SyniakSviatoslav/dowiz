// kernel_client.mjs — thin, dependency-free adapter over the kernel wasm surface.
//
// Per the directive (engine relies on kernel/Rust math; JS/TS is legacy): this file
// NEVER re-implements geo/spectral math. It only decodes the kernel's flat bridge
// protocol (the exact layout the engine bridge::geo / bridge::spectral mirror-pin),
// so any mismatch with the Rust side is a compile/test failure, not silent drift.
//
// Works in node (loads the wasm-bindgen nodejs glue) and in the browser (the web
// glue is injected via bindKernel() after `await init()`).
//
// Fail-closed boundary: the kernel's wasm entry points return Result<_, JsValue>,
// so malformed/empty input surfaces as a *thrown* error, not a sentinel. This
// adapter converts that into the contract's fail-closed sentinel (null / ok:false)
// so the UI and tests never observe a phantom result from bad input.

// ── kernel module resolution (node vs browser) ──────────────────────────────
// Node (tests): load the wasm-bindgen nodejs glue. Browser: the web glue is
// injected via bindKernel() after `await init()` — we must NOT statically import
// the nodejs glue in a browser, because it uses `require` and throws on load.
let kernel = null;

if (typeof process !== "undefined" && process.versions && process.versions.node) {
  // Top-level await: resolves the same nodejs glue the tests rely on.
  kernel = await import("../../../../kernel/pkg/dowiz_kernel.js");
}

// Browser: call with the module returned by `await init()` from the web glue.
export function bindKernel(mod) {
  kernel = mod;
}

// Browser readiness guard — surfaces a clear error instead of a silent null.
function k() {
  if (!kernel) throw new Error("kernel not initialised: call bindKernel(await init()) first");
  return kernel;
}

// Run a kernel call; on rejection (malformed/empty matrix → Rust Result error)
// return the fail-closed `fallback` instead of letting the exception crash the UI.
function safe(label, fn, fallback) {
  try {
    return fn();
  } catch (e) {
    console.warn(`kernel ${label} rejected (fail-closed):`, e.message || e);
    return fallback;
  }
}

// ── geo bridge (mirror of engine/src/bridge.rs::geo) ──────────────────────────
export function routeProgress(polylineJson, posLat, posLng) {
  const raw = safe("geo_progress_flat_js", () => k().geo_progress_flat_js(polylineJson, posLat, posLng), null);
  if (typeof raw !== "string") {
    return { ok: false, remainingM: Number.POSITIVE_INFINITY, snappedLat: 0, snappedLng: 0, segmentIndex: -1 };
  }
  const p = raw.replace(/[[\]]/g, "").split(",").map(Number);
  if (p.length < 4 || p.some((x) => Number.isNaN(x))) {
    return { ok: false, remainingM: Number.POSITIVE_INFINITY, snappedLat: 0, snappedLng: 0, segmentIndex: -1 };
  }
  return {
    ok: true,
    remainingM: p[0],
    snappedLat: p[1],
    snappedLng: p[2],
    segmentIndex: p[3] | 0,
  };
}

// ── spectral bridge (mirror of engine/src/bridge.rs::spectral) ────────────────
export function spectralReport(matrixJson) {
  const raw = safe("spectral_flat_js", () => k().spectral_flat_js(matrixJson), null);
  if (typeof raw !== "string") return null;
  const p = raw.replace(/[[\]]/g, "").split(",").map(Number);
  // layout: [rho, gap, fiedler, drift_code, n, e1re, e1im, ...]
  if (p.length < 5 || p.some((x) => Number.isNaN(x))) return null;
  const rho = p[0], gap = p[1], fiedler = p[2], driftCode = p[3] | 0, n = p[4] | 0;
  if (p.length < 5 + 2 * n) return null; // truncated payload → fail-closed
  const drift = driftCode === 0 ? "Damped" : driftCode === 1 ? "Resonant" : "Unstable";
  const eig = [];
  for (let i = 0; i < n; i++) eig.push({ re: p[5 + 2 * i], im: p[6 + 2 * i] });
  return { rho, gap, fiedler, drift, driftCode, n, eigenvalues: eig };
}

// ── FSM signature (drift-gate) — used by the dashboard red/green status ───────
export function fsmReport() {
  const raw = safe("fsm_graph_report_js", () => k().fsm_graph_report_js(), null);
  if (typeof raw !== "string") return null;
  try { return JSON.parse(raw); } catch { return null; }
}
