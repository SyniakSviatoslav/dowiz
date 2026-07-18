<script>
  // web/src/components/FieldSim.svelte
  //
  // G3 FieldSim — DOM-free WebGPU render shell (Astro/Svelte island).
  //
  // This component performs NO domain math. It:
  //   1. fetches + binds the dowiz-kernel wasm (via kernel_client.mjs, whose
  //      24/24 exports are already wired),
  //   2. asks buffer.mjs to build the vertex/uniform typed buffers from a
  //      real kernel-state snapshot (all values come from wasm),
  //   3. paints them on a single <canvas> WebGPU surface.
  //
  // The islet is `client:only` so it never runs server-side; the only painted
  // surface is the WebGPU canvas — no per-frame DOM, no charts, no TS logic.
  // Svelte component (allowed; .svelte, not .ts). All computation is delegated
  // to the bound kernel exports.

  import { onMount } from 'svelte';
  import { bindKernel } from '../lib/kernel/kernel_client.mjs';
  import { buildFieldBuffer, FLOATS_PER_VERTEX } from '../lib/fieldsim/buffer.mjs';
  import { WGSL } from '../lib/fieldsim/shader.mjs';

  // Kit of REAL kernel inputs (graph adjacency + courier route). These are the
  // only values the field is derived from — every painted number is wasm output.
  export let state = {
    n: 4,
    edges: '[[0,1],[1,2],[2,3],[3,0]]',
    matrix: '[[0,1,0,1],[1,0,1,0],[0,1,0,1],[1,0,1,0]]',
    route: '[[0,0],[0,5],[5,5],[5,0]]',
    courierT: 0.5,
  };
  export let wasmPath = '/kernel/dowiz_kernel_bg.wasm';

  let canvas;
  let raf = 0;
  let device;

  async function start() {
    // 1) bind the kernel (real math authority).
    const res = await fetch(wasmPath);
    if (!res.ok) {
      console.error(`[FieldSim] kernel wasm fetch failed: ${res.status} ${wasmPath}`);
      return;
    }
    const bytes = new Uint8Array(await res.arrayBuffer());
    try {
      await bindKernel(bytes);
    } catch (e) {
      console.error('[FieldSim] kernel bind failed:', e);
      return;
    }

    // 2) build the field buffers from REAL kernel math.
    let buf;
    try {
      buf = buildFieldBuffer(state);
    } catch (e) {
      console.error('[FieldSim] buffer build failed:', e);
      return;
    }

    // 3) WebGPU surface.
    if (!('gpu' in navigator)) {
      console.warn('[FieldSim] WebGPU unavailable in this browser — nothing painted.');
      return;
    }
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) {
      console.warn('[FieldSim] no WebGPU adapter — nothing painted.');
      return;
    }
    device = await adapter.requestDevice();
    const context = canvas.getContext('webgpu');
    const format = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format, alphaMode: 'opaque' });

    const vbuf = device.createBuffer({
      size: buf.vertices.byteLength,
      usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    });
    device.queue.writeBuffer(vbuf, 0, buf.vertices);

    const ubuf = device.createBuffer({
      size: buf.uniforms.byteLength,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });

    const module = device.createShaderModule({ code: WGSL });
    const pipeline = device.createRenderPipeline({
      layout: 'auto',
      vertex: {
        module,
        entryPoint: 'vs',
        buffers: [{
          arrayStride: FLOATS_PER_VERTEX * 4,
          attributes: [
            { shaderLocation: 0, offset: 0, format: 'float32x2' },
            { shaderLocation: 1, offset: 8, format: 'float32' },
            { shaderLocation: 2, offset: 12, format: 'float32' },
          ],
        }],
      },
      fragment: { module, entryPoint: 'fs', targets: [{ format }] },
      primitive: { topology: 'point-list' },
    });

    const bindGroup = device.createBindGroup({
      layout: pipeline.getBindGroupLayout(0),
      entries: [{ binding: 0, resource: { buffer: ubuf } }],
    });

    let frame = 0;
    function render() {
      buf.uniforms[8] = frame; // frame counter (render scaffolding, not domain math)
      device.queue.writeBuffer(ubuf, 0, buf.uniforms);
      const enc = device.createCommandEncoder();
      const pass = enc.beginRenderPass({
        colorAttachments: [{
          view: context.getCurrentTexture().createView(),
          clearValue: { r: 0.043, g: 0.059, b: 0.09, a: 1 },
          loadOp: 'clear',
          storeOp: 'store',
        }],
      });
      pass.setPipeline(pipeline);
      pass.setBindGroup(0, bindGroup);
      pass.setVertexBuffer(0, vbuf);
      pass.draw(buf.vertexCount, 1, 0, 0);
      pass.end();
      device.queue.submit([enc.finish()]);
      frame++;
      raf = requestAnimationFrame(render);
    }
    render();
  }

  onMount(() => {
    start();
    return () => {
      if (raf) cancelAnimationFrame(raf);
      if (device) device.destroy();
    };
  });
</script>

<!-- The ONLY painted surface is this WebGPU canvas. No DOM-based rendering. -->
<canvas bind:this={canvas} width="800" height="600" role="img"
        aria-label="dowiz FieldSim — kernel-driven spectral/geo field rendered via WebGPU"></canvas>
