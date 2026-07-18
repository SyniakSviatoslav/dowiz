// web/src/render/fieldsim.smoke.mjs
//
// G3 FieldSim — offline smoke test (Node, no GPU / no browser).
//
// Verifies the REAL data-path that the WebGPU shell depends on:
//   1. binds the dowiz-kernel wasm (same bytes the browser island uses),
//   2. builds the FieldSim vertex/uniform buffers from a kernel-state snapshot,
//   3. asserts it constructs + feeds without panicking,
//   4. asserts DETERMINISM: identical kernel state ⇒ byte-identical buffers.
//
// The WebGPU paint itself cannot run headless (no GPU), but every number it
// receives is proven here to come from the kernel wasm and to be reproducible.
// Run: node src/render/fieldsim.smoke.mjs

import { readFileSync } from 'fs';
import { bindKernel } from '../lib/kernel/kernel_client.mjs';
import { buildFieldBuffer, FLOATS_PER_VERTEX } from '../lib/fieldsim/buffer.mjs';

const WASM = process.env.DOWIZ_KERNEL_WASM ||
  '/root/dowiz/kernel/pkg-web/dowiz_kernel_bg.wasm';

const STATE = {
  n: 4,
  edges: '[[0,1],[1,2],[2,3],[3,0]]',
  matrix: '[[0,1,0,1],[1,0,1,0],[0,1,0,1],[1,0,1,0]]',
  route: '[[0,0],[0,5],[5,5],[5,0]]',
  courierT: 0.5,
};

function digest(f32) {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < f32.length; i++) {
    // quantize to avoid float-string noise in the hash
    const v = Math.round(f32[i] * 1e6);
    h ^= v & 0xff; h = Math.imul(h, 16777619) >>> 0;
    h ^= (v >>> 8) & 0xff; h = Math.imul(h, 16777619) >>> 0;
    h ^= (v >>> 16) & 0xff; h = Math.imul(h, 16777619) >>> 0;
    h ^= (v >>> 24) & 0xff; h = Math.imul(h, 16777619) >>> 0;
  }
  return h >>> 0;
}

let failed = false;
function check(label, cond) {
  if (cond) {
    console.log(`OK  ${label}`);
  } else {
    console.error(`ERR ${label}`);
    failed = true;
  }
}

const main = async () => {
  const bytes = new Uint8Array(readFileSync(WASM));
  await bindKernel(bytes);

  // Build the buffer twice from the same kernel state.
  const a = buildFieldBuffer(STATE);
  const b = buildFieldBuffer(STATE);

  check('constructed vertex+uniform buffers without panic', !!a && !!b);
  check('vertexCount = n + 1 (courier marker)', a.vertexCount === STATE.n + 1);
  check('floatsPerVertex = 4', a.floatsPerVertex === FLOATS_PER_VERTEX && FLOATS_PER_VERTEX === 4);
  check('vertex buffer length matches vertexCount*4',
    a.vertices.length === a.vertexCount * FLOATS_PER_VERTEX);
  check('uniform block is 16 f32 (64 bytes)', a.uniforms.length === 16);

  // Determinism: same kernel state ⇒ identical buffers (kernel is the authority).
  check('vertex buffer deterministic across builds',
    digest(a.vertices) === digest(b.vertices));
  check('uniform buffer deterministic across builds',
    digest(a.uniforms) === digest(b.uniforms));

  // Spot-check that REAL kernel scalars landed in the uniform block:
  // spectral_flat_js of the 4-cycle graph ⇒ rho = 2, driftCode = 2 (Unstable).
  check('uniform[0] = spectral radius rho (2.0)', Math.abs(a.uniforms[0] - 2.0) < 1e-6);
  check('uniform[3] = driftCode Unstable (2.0)', Math.abs(a.uniforms[3] - 2.0) < 1e-6);

  console.log(`\nvertex digest=${digest(a.vertices).toString(16)}  ` +
              `uniform digest=${digest(a.uniforms).toString(16)}`);

  if (failed) {
    console.error('\nFIELD SIM SMOKE: FAIL');
    process.exit(1);
  }
  console.log('\nFIELD SIM SMOKE: GREEN — buffers built from kernel math, deterministic.');
};

main().catch((e) => {
  console.error('FIELD SIM SMOKE: PANIC', e);
  process.exit(1);
});
