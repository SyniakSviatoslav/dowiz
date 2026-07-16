// kernel.test.mjs — fail-closed assertions for the kernel wasm bridges (Node).
// GREEN gate (W17): kernel loads, spectral_radius_js('[[1,0],[0,1]]') => "1",
// malformed => {ok:false}, fsm + geo => {ok:true}. Plus grep proof (run by parent):
// NO haversine/eigen/FSM re-impl in web/src.
import { readFileSync } from 'fs';
import { bindKernel, spectral_radius_js, fsm_graph_report_js, geo_progress_flat_js } from './kernel_client.mjs';

const bytes = new Uint8Array(readFileSync(new URL('../../../../kernel/pkg-web/dowiz_kernel_bg.wasm', import.meta.url)));
await bindKernel(bytes);

function assert(cond, msg) {
  if (!cond) { console.error('FAIL:', msg); process.exit(1); }
  console.log('ok -', msg);
}

// 1. spectral radius of identity-like matrix [[1,0],[0,1]] => "1"
const s = spectral_radius_js('[[1,0],[0,1]]');
assert(s.ok && s.value === '1', `spectral_radius_js([[1,0],[0,1]]) === "1" (got ${JSON.stringify(s)})`);

// 2. malformed input => fail-closed {ok:false}
const bad = spectral_radius_js('not a matrix');
assert(!bad.ok, `malformed matrix => {ok:false} (got ${JSON.stringify(bad)})`);

// 3. fsm report => ok:true (acyclic 10-vertex sample)
const f = fsm_graph_report_js();
assert(f.ok, `fsm_graph_report_js => ok:true (got ${JSON.stringify(f).slice(0,80)})`);

// 4. geo progress => ok:true
const g = geo_progress_flat_js('[[0,0],[10,0]]', 5, 1);
assert(g.ok, `geo_progress_flat_js => ok:true (got ${JSON.stringify(g)})`);

console.log('W17 GREEN: all kernel bridges wired + fail-closed, zero JS math re-impl.');
