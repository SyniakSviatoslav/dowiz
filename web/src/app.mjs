// app.mjs — kernel-driven field UI.
//
// This module ONLY renders data the Rust kernel computes in wasm. It never
// re-implements geo/spectral/FSM math in JS (that is the legacy approach the
// directive forbids). The kernel module is bound via the web glue's init().

import init, * as KernelModule from "../../kernel/pkg-web/dowiz_kernel.js";
import { routeProgress, spectralReport, fsmReport, bindKernel } from "./lib/kernel/kernel_client.mjs";

// ── demo inputs (real data fed to the real kernel) ──────────────────────────
// A small geo route (lat,lng pairs). The kernel's geo_progress_flat_js takes a
// QUERY POINT (lat,lng) and returns: the perpendicular distance from that point
// to the route, the snapped nearest point on the route, and the segment index.
const ROUTE = [
  [50.4501, 30.5234], // Kyiv center
  [50.4541, 30.5238],
  [50.4589, 30.5280],
  [50.4635, 30.5361],
  [50.4680, 30.5470],
];
// A 2-cycle adjacency [[0,1],[1,0]]: eigenvalues ±1 → spectral radius 1,
// gap 0 → drift "Resonant". Fed to the kernel's spectral math.
const MATRIX = [[0, 1], [1, 0]];

// ── geometry helpers (for placing a moving *query* point; NOT for the math) ──
function segLen(a, b) {
  const dLat = a[0] - b[0], dLng = a[1] - b[1];
  return Math.hypot(dLat, dLng);
}
function pointAtFrac(frac) {
  // walk the polyline by fractional arc length to get an anchor point on it
  const total = ROUTE.reduce((s, p, i) => (i ? s + segLen(ROUTE[i - 1], p) : 0), 0);
  let target = frac * total, acc = 0;
  for (let i = 1; i < ROUTE.length; i++) {
    const L = segLen(ROUTE[i - 1], ROUTE[i]);
    if (acc + L >= target) {
      const u = L ? (target - acc) / L : 0;
      return [ROUTE[i - 1][0] + u * (ROUTE[i][0] - ROUTE[i - 1][0]),
              ROUTE[i - 1][1] + u * (ROUTE[i][1] - ROUTE[i - 1][1])];
    }
    acc += L;
  }
  return ROUTE[ROUTE.length - 1];
}
function perpAt(frac, mag) {
  // unit perpendicular to the local route direction, scaled by mag
  const a = pointAtFrac(Math.max(0, frac - 0.01));
  const b = pointAtFrac(Math.min(1, frac + 0.01));
  const dLat = b[0] - a[0], dLng = b[1] - a[1];
  const len = Math.hypot(dLat, dLng) || 1;
  return [-dLng / len * mag, dLat / len * mag];
}

// ── animation state ─────────────────────────────────────────────────────────
let phase = 0; // 0..1 sweep of the query point across the route

// ── DOM ─────────────────────────────────────────────────────────────────────
const $ = (id) => document.getElementById(id);
const cv = $("cv");
const ctx = cv.getContext("2d");

function set(id, txt, cls) {
  const el = $(id);
  if (!el) return;
  el.textContent = txt;
  if (cls) el.className = `v ${cls}`;
}

// ── canvas: route polyline + the kernel-snapped point + the query point ─────
function drawRoute(query, snap) {
  const W = cv.width, H = cv.height, pad = 16;
  ctx.clearRect(0, 0, W, H);
  const lats = ROUTE.map((p) => p[0]).concat(query ? [query[0]] : []);
  const lngs = ROUTE.map((p) => p[1]).concat(query ? [query[1]] : []);
  const minLat = Math.min(...lats), maxLat = Math.max(...lats);
  const minLng = Math.min(...lngs), maxLng = Math.max(...lngs);
  const sx = (lng) => pad + ((lng - minLng) / (maxLng - minLng || 1)) * (W - 2 * pad);
  const sy = (lat) => H - pad - ((lat - minLat) / (maxLat - minLat || 1)) * (H - 2 * pad);
  // polyline
  ctx.strokeStyle = "#25405f"; ctx.lineWidth = 2; ctx.beginPath();
  ROUTE.forEach((p, i) => (i ? ctx.lineTo(sx(p[1]), sy(p[0])) : ctx.moveTo(sx(p[1]), sy(p[0]))));
  ctx.stroke();
  // vertices
  ctx.fillStyle = "#3a5a80";
  ROUTE.forEach((p) => { ctx.beginPath(); ctx.arc(sx(p[1]), sy(p[0]), 3, 0, 7); ctx.fill(); });
  // query point (where the courier is)
  if (query) { ctx.fillStyle = "#ffb454"; ctx.beginPath(); ctx.arc(sx(query[1]), sy(query[0]), 5, 0, 7); ctx.fill(); }
  // kernel-snapped point (nearest on route)
  if (snap && Number.isFinite(snap.snappedLat) && Number.isFinite(snap.snappedLng)) {
    ctx.fillStyle = "#7fd1ff"; ctx.beginPath(); ctx.arc(sx(snap.snappedLng), sy(snap.snappedLat), 4, 0, 7); ctx.fill();
  }
}

// ── render one frame from the kernel ─────────────────────────────────────────
function render() {
  // anchor walks along the route; query point sweeps perpendicular across it
  const anchorFrac = phase;
  const sweep = (phase * 2 - 1); // -1..1, crosses 0 at phase=0.5
  const [pLat, pLng] = perpAt(anchorFrac, 0.004 * sweep);
  const anchor = pointAtFrac(anchorFrac);
  const query = [anchor[0] + pLat, anchor[1] + pLng];

  // geo: feed the REAL query point to the REAL kernel
  const g = routeProgress(JSON.stringify(ROUTE), query[0], query[1]);
  if (g.ok) {
    // remainingM from the kernel == perpendicular distance query→route
    set("rem", `${g.remainingM.toExponential(2)} deg`, "warn");
    set("snap", `(${g.snappedLat.toFixed(5)}, ${g.snappedLng.toFixed(5)})`);
    set("seg", `${g.segmentIndex}`);
    drawRoute(query, g);
  } else {
    set("rem", "n/a", "bad");
  }

  // spectral
  const s = spectralReport(JSON.stringify(MATRIX));
  if (s) {
    set("rho", s.rho.toFixed(4));
    set("gap", s.gap.toFixed(4));
    set("fie", s.fiedler.toFixed(4));
    const cls = s.drift === "Damped" ? "good" : s.drift === "Resonant" ? "warn" : "bad";
    set("drift", s.drift, cls);
  } else {
    set("drift", "rejected", "bad");
  }

  // FSM signature (drift-gate)
  const f = fsmReport();
  if (f) {
    set("fsm", `${f.vertices ?? "?"} / ${f.edges ?? "?"}`);
    set("acyc", f.is_acyclic ? "yes" : "NO", f.is_acyclic ? "good" : "bad");
  } else {
    set("acyc", "n/a", "bad");
  }
}

async function main() {
  // Boot the kernel wasm (web glue auto-loads dowiz_kernel_bg.wasm relative to it).
  await init();
  bindKernel(KernelModule); // give the adapter the live web module
  render();

  const tick = $("tick");
  if (tick) tick.addEventListener("click", () => { phase = (phase + 0.1) % 1; render(); });

  // auto-advance so the demo is alive
  setInterval(() => { phase = (phase + 0.01) % 1; render(); }, 500);
}

main().catch((e) => {
  console.error("kernel UI boot failed:", e);
  const pre = document.createElement("pre");
  pre.style.color = "#ff6b6b";
  pre.textContent = "kernel boot error: " + e.message;
  document.body.appendChild(pre);
});
