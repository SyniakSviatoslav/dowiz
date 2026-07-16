// app.mjs — boots the kernel wasm and renders ρ / drift / FSM from KERNEL MATH ONLY.
// No JS/TS re-implementation of geo/spectral/FSM (AGENTS invariant: kernel is math authority).
import { bindKernel, spectral_radius_js, geo_progress_flat_js, fsm_graph_report_js } from './lib/kernel/kernel_client.mjs';

async function loadBytes() {
  // Browser: served by serve.mjs at /pkg-web/. Node: read from kernel/pkg-web.
  if (typeof fetch === 'function' && typeof window !== 'undefined') {
    const r = await fetch('/pkg-web/dowiz_kernel_bg.wasm');
    return new Uint8Array(await r.arrayBuffer());
  }
  const { readFileSync } = await import('fs');
  return new Uint8Array(readFileSync(new URL('../../kernel/pkg-web/dowiz_kernel_bg.wasm', import.meta.url)));
}

const bytes = await loadBytes();
await bindKernel(bytes);

// Spectral radius ρ (drift detector input).
const rho = spectral_radius_js('[[1,0],[0,1]]');
console.log('ρ (spectral_radius) =', rho.ok ? rho.value : 'ERR');

// FSM graph report (acyclicity / cyclomatic / signature).
const fsm = fsm_graph_report_js();
console.log('FSM =', fsm.ok ? fsm.value.slice(0, 200) : 'ERR');

// Geo progress along a polyline (route snap).
const geo = geo_progress_flat_js('[[0,0],[10,0]]', 5, 1);
console.log('geo (remaining_m,idx) =', geo.ok ? geo.value : 'ERR');

if (!rho.ok || !fsm.ok || !geo.ok) {
  console.error('KERNEL BRIDGE FAIL');
  process.exit(1);
}
console.log('KERNEL-DRIVEN UI GREEN — all bridges wired, math from wasm only.');
