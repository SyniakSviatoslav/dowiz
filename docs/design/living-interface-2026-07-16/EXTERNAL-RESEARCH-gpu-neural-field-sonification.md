# External research (operator-supplied, 2026-07-16, verbatim) — Real-Time GPU Neural-Field Rendering + Signal Sonification in the Browser

> Pasted by the operator as input to the living-interface arc. Preserved verbatim (including all
> citations/links) per this project's own rule: the corpus is the source of truth, not chat
> history. Do not treat this as dowiz canon — it is third-party research to be filtered through
> dowiz's own house rules (Rust/wasm-first, zero-JS-math, DECART-gated new deps, `wgpu` as sole
> graphics dependency per `physics-ui-capture-blueprint.md`) in the synthesis pass, not adopted
> verbatim. In particular: this report's default recommendation (Three.js/TSL, a JS framework) is
> in tension with dowiz's zero-JS-math house rule and needs explicit reconciliation, not silent
> adoption — see `INTEGRATION-RISK-MAP.md` and the synthesis roadmap for the resolution.

---

## TL;DR
- **Build it on WebGPU compute shaders (Three.js r171+ `WebGPURenderer` + TSL, with automatic WebGL2 fallback), not AI image generation.** A compute kernel updates neuron/particle state in GPU storage buffers; a spiking-neuron model (Izhikevich) drives both the glow animation and the audio. This is a proven stack: WebGPU compute renders 1M+ particles at 60fps, and 10M particles at ~63fps on a GTX 1060.
- **Neural signals → sound is the distinctive direction**: run the spiking sim in a compute shader, reduce to a compact per-frame activity summary (firing rate, spike events, mean membrane potential) on the GPU, read that small buffer back, and feed a Web Audio `AudioWorklet` running WASM/Faust DSP (granular/FM/additive synthesis). The reverse audio-reactive mode (Blender-style FFT via `AnalyserNode`, fftSize 4096, Hann window) is a small add-on that writes FFT bins into a GPU texture/buffer.
- **Feasible scale at 60fps**: 100k–1M rendered points/particles; ~10⁴–10⁵ *dynamically simulated* Izhikevich neurons in-browser (native CUDA sims like NeMo reach ~40,000 neurons real-time, GeNN 100,000 — treat these as upper bounds for the math, not web guarantees). Use real morphology (SWC from NeuroMorpho.org; H01/MICrONS meshes) for hero neurons plus procedural space-colonization trees for the dense field.

---

## Key Findings

1. **WebGPU is Baseline as of January 2026** — Chrome/Edge 113+, Safari 26 (macOS Tahoe/iOS/iPadOS/visionOS 26), Firefox 141+ (Windows) / 145+ (macOS ARM64). WebGL2 remains the fallback (no compute stage; emulate via transform feedback / float-texture ping-pong). Ship WebGPU-primary + WebGL2 fallback; Three.js delivers both from one renderer.
2. **Compute shaders are the core enabler.** WebGL2 has no compute; large stateful simulations must fake it. WebGPU storage buffers + compute pipelines remove the CPU↔GPU round-trip and are 15–150× faster for particle updates.
3. **The glow aesthetic is a solved pipeline**: emissive HDR (>1.0) nodes + additive-blended synapse filaments + selective/UnrealBloom + ACES/AgX tone mapping + depth-of-field + dark fog. Three.js r183+ `RenderPipeline` (node-based, WebGPU-native) replaces `EffectComposer`.
4. **Morphology** = space-colonization algorithm (Runions 2007) for procedural dendritic/axonal trees, or load real SWC (NeuroMorpho.org, 270k+ reconstructions) and H01/MICrONS connectome meshes. Render as instanced tubes/fat-lines + point-sprite somata.
5. **Signal dynamics**: Izhikevich model is the sweet spot (2 ODEs, ~13 FLOPs/neuron, ~20 cortical firing patterns). LIF is cheaper; Hodgkin-Huxley + cable equation gives true action-potential *propagation* along axons but is far heavier. All map cleanly to WGSL.
6. **Sonification**: `AudioWorklet` (dedicated audio thread) + WASM DSP (Faust→WASM, or Rust/C++ via Emscripten) is the low-latency path. Spikes → note/grain triggers; firing rate → density/pitch; membrane potential → filter cutoff.
7. **Toolchain**: Three.js TSL (recommended, write-once → WGSL+GLSL); Babylon.js (WebGPU-first since v5); Rust+wgpu→WASM for maximal control; taichi.js / gpu.io for GPGPU convenience.
8. **AI stays optional**: learned dynamics/style, denoising, upscaling, procedural assistance — never the core renderer.

---

## Details

### 1. GPU-accelerated particle systems in the browser

**WebGPU compute (primary path).** All state in GPU storage buffers; dispatch one thread per particle/neuron with `@workgroup_size(64)` or `256`. Ping-pong two buffers (read A → write B, swap). Documented ceilings:
- **1,000,000 particles at 60fps** with physics + interactivity (Three.js galaxy demo; markaicode 1M physics demo).
- **10,000,000 particles at ~63fps on a GTX 1060** [TU Wien](https://www.cg.tuwien.ac.at/research/publications/2023/PETER-2023-PSW/PETER-2023-PSW-.pdf) (TU Wien bachelor thesis, 2023) — which also found **vertex pulling beats instancing** [TU Wien](https://www.cg.tuwien.ac.at/research/publications/2023/PETER-2023-PSW/PETER-2023-PSW-.pdf) for point rendering on modern high-end GPUs.
- O(N²) all-pairs forces are the bottleneck; use **spatial hashing/binning** (grid + prefix-sum + atomics) for O(N) neighbor queries. Building these on GPU needs atomics — WGSL has them, WebGL2 does not.
- WGSL computes in **f32** (no f16 storage type; pack into `rgba16float` textures or `u32` via `pack4x8unorm` to halve memory). Particle ≈ 16–32 bytes; pad `vec3` to 16-byte alignment.

**WebGL2 fallback (GPGPU).** (a) **Transform feedback** — process a 1D array, vertex shader writes output buffer, no readback. (b) **Float-texture ping-pong FBOs** — state in `RGBA32F` textures, full-screen fragment pass updates them, `texelFetch` reads neighbors (the classic Shadertoy "falling sand"/particle pattern). Instanced rendering via core WebGL2 instancing. Practical ceiling ~10⁴–low-10⁵ simulated particles at 60fps.

**Rendering the points**: point sprites (`gl_PointSize`, hardware-capped) or instanced quads/billboards (no cap, required for large sprites and trails). Three.js WebGPU: `SpriteNodeMaterial` + `instancedArray`/`storage`; `positionNode = positionBuffer.toAttribute()`.

**WebGPU vs WebGL2**:
| | WebGL2 | WebGPU |
|---|---|---|
| Compute shaders | ❌ (emulate via FBO/TF) | ✅ native |
| Atomics / workgroup shared mem | ❌ | ✅ |
| CPU overhead | high (single-thread submit) | low (multi-thread encode) |
| Particle sim ceiling @60fps | ~10⁴–10⁵ | 10⁶–10⁷ |
| Browser support 2026 | universal | Baseline (Linux/old-iOS gaps) |
| Precision | lowp on some devices | f32 guaranteed |

### 2. The glowing neural aesthetic

**Emissive + bloom pipeline** (the "cinematic neuron" look):
1. Render nodes with **emissive HDR color** (values >1.0, e.g. `color(0,2,4).mul(fresnel)`) into a float (`rgba16float`) target — do **not** tone-map yet.
2. **Additive blending** for synaptic filaments and trails so overlapping lines accumulate brightness → the dense luminous-web look.
3. **Selective bloom**: threshold-extract bright pixels → multi-mip gaussian blur → composite. Three.js: `UnrealBloomPass` (WebGL) or `bloom()` node in `RenderPipeline` (WebGPU). Emissive-only selective bloom keeps the background clean.
4. **Tone mapping** last: ACES Filmic or AgX (r183 has AgX) — desaturates the brightest cores toward white ("overwhelmingly bright").
5. **Depth cues**: exponential dark blue-black fog + **depth-of-field/bokeh** (WebGPU `dof` node) → the "digitized cubic millimeter" volumetric depth.

**Synapse edges**: fat-lines (`Line2`/`LineSegmentsGeometry` + resolution-aware `LineMaterial`) or instanced tube meshes (generalized cylinders around the SWC/curve skeleton; 3 verts/ring suffices per large-line-set research). For traveling signal pulses: animate a moving `smoothstep` window of emissive intensity along the tube (UV offset with `time`). Reference: three.js forum **"SYNAPSES: Fat lines + Selective Bloom"** (CatmullRomCurve3 + selective bloom) — visually exactly the target. A cheap volumetric alt: additive radial-gradient sprites at node positions + DOF.

### 3. Procedural neuron morphology in 3D

**Algorithms**:
- **Space colonization (Runions, Lane & Prusinkiewicz 2007)** — best for dendritic/axonal trees. Seed attractor points in a volume; grow nodes toward nearby attractors within a radius of influence; kill attractors when reached. Prevents branch intersection, gives biologically-plausible density. GPU implementation exists (**mattatz/Dendrite**, Unity compute — port kernel to WGSL). Add generalized cylinders + parallel-transport frames for tubes.
- **L-systems** — rule-based, stylized/repetitive branching.
- **Diffusion-limited aggregation (DLA)** — chaotic/fractal fields.

**Real data** (higher fidelity for hero cells):
- **SWC format**: tree of `(id, type, x, y, z, radius, parent)`. Type 1=soma, 2=axon, 3=basal dendrite, 4=apical. Trivial to parse; build tubes directly from parent links.
- **NeuroMorpho.org**: 270,000+ reconstructions in SWC (largest free archive); `xyz2swc` converts 26 other formats.
- **H01** (Harvard Lichtman + Google Connectomics): ~1 mm³ human temporal cortex, 1.4 PB. Per Google Research's H01 blog: *"roughly one cubic millimeter of brain tissue, and includes tens of thousands of reconstructed neurons, millions of neuron fragments, 130 million annotated synapses, 104 proofread cells"* [Google Research](https://research.google/blog/a-browsable-petascale-reconstruction-of-the-human-cortex/) (the h01-release landing page cites 183 million synapses, 100 proofread cells). [Google APIs](https://h01-release.storage.googleapis.com/landing.html) Science paper: Shapson-Coe et al., *"A petavoxel fragment of human cerebral cortex reconstructed at nanoscale resolution."* Meshes via Neuroglancer's precomputed sharded format.
- **MICrONS "cubic millimeter"** (P87 mouse visual cortex, 1.4 × 0.87 × 0.84 mm): per MICrONS Explorer, *"more than an estimated 200,000 cells, and 120,000 neurons. Automated synapse detection measured more than 523 million synapses"* [MICrONS Explorer](https://www.microns-explorer.org/cortical-mm3) (Nature 2025, *"Functional connectomics spanning multiple areas of mouse visual cortex"*). Meshes on AWS/GCS public buckets via `cloud-volume`.
- **Loading strategy**: decimate meshes offline → instanced tubes + point somata; stream LOD. Pull proofread cell meshes only — never raw volumes.

### 4. Signal dynamics on the GPU (the math)

**Leaky Integrate-and-Fire (LIF)** — cheapest:
`τ_m dV/dt = −(V − V_rest) + R·I`; if `V ≥ V_th` → spike, `V ← V_reset`, refractory. ~1 mul-add/step; millions feasible.

**Izhikevich (2003)** — recommended (rich dynamics, cheap):
- `v' = 0.04v² + 5v + 140 − u + I`
- `u' = a(bv − u)`
- if `v ≥ 30 mV`: `v ← c`, `u ← u + d` [arxiv](https://arxiv.org/pdf/2509.23516)
- **Canonical parameters** (Izhikevich 2003, cross-confirmed by BioModels curated encoding + multiple peer-reviewed tables): **Regular Spiking (excitatory): a=0.02, b=0.2, c=−65, d=8**; **Fast Spiking (inhibitory): a=0.1, b=0.2, c=−65, d=2**; also Chattering (c=−50, d=2), Intrinsically Bursting (c=−55, d=4), Low-Threshold Spiking (b=0.25, d=2). [arxiv](https://arxiv.org/pdf/1905.05678)
- Reproduces ~20 cortical firing patterns. [arxiv](https://arxiv.org/pdf/1704.03150) Verbatim from E.M. Izhikevich, *"Simple Model of Spiking Neurons,"* IEEE Trans. Neural Networks 14(6):1569–1572, 2003: *"Using this model, one can simulate tens of thousands of spiking cortical neurons in real time (1 ms resolution) using a desktop PC."* [PubMed](https://pubmed.ncbi.nlm.nih.gov/18244602/) Integrate with forward Euler; split the v-update into two 0.5 ms half-steps for numerical stability (Izhikevich's recommendation).
- **Synaptic input**: `I_i = I_ext + Σ_j w_ij · s_j` where `s_j` is a spike flag or decaying conductance. Store weights in a storage buffer / connectivity texture; on spike, inject into post-synaptic accumulator (atomics for scatter, or iterate pre-synaptic).

**Hodgkin-Huxley + cable equation** — for true action-potential *propagation*:
- Membrane: `C_m dV/dt = −(g_Na m³h (V−E_Na) + g_K n⁴ (V−E_K) + g_L(V−E_L)) + I`, plus gating ODEs for m, h, n.
- **Cable equation** (spatial): `(a/2R_i) ∂²V/∂x² = C_m ∂V/∂t + I_ion` — a reaction-diffusion PDE. Discretize the axon into compartments (1D grid), explicit Euler with Neumann (zero-flux) boundaries → travelling spike solitons; this is what visually makes a pulse race down an axon. HH is ~10× LIF cost — reserve for a handful of hero axons; use Izhikevich point-neurons for the bulk field.

**Graph layout**: for abstract connectome graphs use GPU **force-directed layout** (Fruchterman-Reingold / Barnes-Hut). **GraphWaGu** (WebGPU) is the reference: *"capable of rendering up to 100,000 nodes and 2,000,000 edges with interactive frame rates (≥ 10 FPS)"* [stevepetruzza](https://stevepetruzza.io/pubs/graphwagu-2022.pdf) and *"we maintain rendering frame rates 4× higher than the next best library (NetV.js) and layout creation times up to half that of D3.js"* [stevepetruzza](https://stevepetruzza.io/pubs/graphwagu-2022.pdf) (Dyken et al., EGPGV 2022); rendering continues to 200,000 nodes / 4,000,000 edges. [stevepetruzza](https://stevepetruzza.io/pubs/graphwagu-2022.pdf) For anatomically-placed neurons, use real 3D coordinates instead.

**Reference frameworks for the math** (rendering stays web-native): **Brian2** (Python, readable models — prototype here, port the update to WGSL), **NEST** (large networks), **GeNN** (GPU code-gen, CUDA), **NEURON** (multicompartment + SWC import).

### 5. Sonification of neural signals (the distinctive artifact)

**Architecture**: compute shader simulates spikes → per-frame GPU reduction to a small activity buffer (spike count per region, mean rate, active indices) → `mapAsync` readback (or `copyBufferToBuffer` → staging) → `port.postMessage` or `SharedArrayBuffer`+`Atomics` → WASM DSP synthesizes in the AudioWorklet.

**Web Audio path**:
- **`AudioWorklet`** runs DSP on a dedicated high-priority thread — mandatory for glitch-free audio. The deprecated `ScriptProcessorNode` runs on the main thread; use only as old-Safari fallback.
- **WASM DSP**: **Faust** (functional DSP → WASM via `faustwasm`; ships an FFT AudioWorklet processor, polyphony, MIDI) — fastest to iterate; or **Rust/C++ → Emscripten** with WASM SIMD.

**Mapping (neural → sound)**:
- **Spike → event trigger**: each spike fires a grain/note (granular/additive), pitch by neuron id or region mapped to a scale (pentatonic/diatonic avoids dissonance). The **"Spikiss" project** maps inhibitory neurons → bass/rhythm, excitatory → melody.
- **Firing rate → density/amplitude/pitch**: population rate drives drone density or filter sweep.
- **Membrane potential → continuous timbre**: mean V → filter cutoff or FM index.
- **Synthesis choices**: granular (dense spike fields → texture), FM (bell/metallic spikes), additive (smooth spectra from rate), physical modeling (resonators per region).
- **Spatialization**: `PannerNode` per region → 3D audio matched to neuron position.

**Reverse (audio-reactive, Blender-style, secondary mode)**:
- Blender's **"Sample Sound Frequencies" node** (PR #156247, targeting Blender 5.2 per Jacques Lucke) does subframe-friendly FFT sampling of a frequency range without baking f-curves; the classic workflow is FFT size 4096 + Hann window driving geometry from amplitude.
- Browser equivalent: `AnalyserNode` with `fftSize = 4096` (→ 2048 bins), `smoothingTimeConstant ≈ 0.8`, `getByteFrequencyData()` per frame. For explicit Hann windowing beyond what `AnalyserNode` exposes, run a custom FFT in WASM/Faust.
- **Bridge FFT → GPU**: write the bin array into a 1D `DataTexture` / storage buffer each frame (`device.queue.writeBuffer`), sample it in the compute/vertex shader to modulate particle size, emissive intensity, or force fields. Three.js `AudioAnalyser` wraps `getFrequencyData()`.

### 6. The WASM + kernel + GPU stack

**Orchestration (per frame)**:
1. Compute kernel → simulation step (Izhikevich in WGSL; heavy CPU math in WASM SIMD if needed).
2. GPU storage buffers hold state (never leave GPU for rendering).
3. Render pipeline reads buffers → instanced sprites/tubes + bloom/DOF.
4. Small activity reduction → readback → Web Audio (`AudioWorklet` + WASM DSP).

**Toolchains**:
- **Rust + wgpu → wasm32-unknown-unknown** (`wasm-bindgen`): wgpu runs natively on WebGPU and on WebGL2 (feature `webgl`); same Rust native + web. WGSL passes through on WebGPU; Naga translates WGSL→GLSL on WebGL2. Max control, steeper curve.
- **WGSL compute directly**: `@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id)`; bind storage buffers via bind groups; `dispatchWorkgroups(ceil(N/64))`.
- **Emscripten C++ → WASM**: port existing C/CUDA neuron math (adapt GeNN-style kernels); WASM SIMD for CPU DSP.
- **Memory sharing**: WASM linear memory ↔ JS `ArrayBuffer` (zero-copy views); JS→GPU via `writeBuffer`/mapped buffers; `SharedArrayBuffer`+`Atomics` for main↔audio-worklet (requires COOP/COEP cross-origin isolation).
- **GPGPU convenience libs**: **taichi.js** (JS lambdas → WebGPU compute, top-level loops auto-parallelized), **gpu.io** (WebGL2/WebGPU ping-pong GPGPU), **gpu.js** (older, WebGL).

### 7. Frameworks, libraries, reference projects

- **Three.js r171+ `WebGPURenderer` + TSL** (recommended default). `import * as THREE from 'three/webgpu'`; `await renderer.init()`. TSL (`three/tsl`) compiles once → WGSL **and** GLSL → automatic WebGL2 fallback. `instancedArray`/`storage` for compute buffers, `compute()` for kernels, `RenderPipeline` for post-fx. Examples: `webgpu_tsl_compute_attractors_particles`, `webgpu_postprocessing_bloom_emissive`, `webgpu_compute_particles`.
- **Babylon.js** — WebGPU-first since v5 (2022); core shaders rewritten in WGSL (2024); Node Material Editor (visual shader graph), strong GPU particle system, render bundles (~10× faster scenes). Best if you want editor-driven workflow.
- **regl** — minimal functional WebGL (fallback-era GPGPU).
- **Reference projects**: three.js "SYNAPSES" fat-lines+bloom (brain-like); **mattatz/Dendrite** (GPU space-colonization); **GraphWaGu** (WebGPU graph layout); **Neuroglancer** (open-source H01/connectome viewer, WebGL); **piellardj/particles-webgpu**; **NewKrok/three-particles** (WebGPU compute, 50k–350k+ particles); **rreusser/webgpu-instanced-lines** (GPU line joins/caps); Faust "powered-by" demos incl. **Reflex-in** (brain-wave streams → Web Audio + WebGL).
- **Starters**: dgreenheck/webgpu-claude-skill; wawasensei TSL GPGPU course; Maxime Heckel "Field Guide to TSL and WebGPU."

### 8. Where AI is an optional enhancement (not the core)
- **Learned dynamics/style**: a small NN can modulate parameters or add stylistic motion — keep the physical Izhikevich/HH core authoritative.
- **Denoising/upscaling**: render lower-res + neural upscaler only if GPU-bound; watch temporal artifacts on fine filaments.
- **Procedural assistance**: generate attractor distributions / connectivity priors offline.
- **In-browser ML shares the same WebGPU** (TensorFlow.js, ONNX Runtime Web, transformers.js) — an optional AI layer needs no separate stack.
- **Explicitly avoid**: generative-image models as the renderer (violates native-real-time + can't do per-frame signal-accurate coupling).

---

## Recommended Reference Architecture

**Stack**: Three.js `WebGPURenderer` + TSL (WebGPU primary, WebGL2 auto-fallback) → compute kernels for simulation → `RenderPipeline` bloom/DOF → `AudioWorklet` + Faust-WASM for sonification.

**Data flow per frame**:
```
[Compute pass 1] Izhikevich update (storage buffers: v,u,I,spikeFlags)  — WGSL
        ↓ (spike flags feed synaptic accumulation, ping-pong)
[Compute pass 2] synaptic gather (weights buffer) + edge-pulse position
        ↓
[Render] instanced somata sprites (emissive HDR) + instanced synapse tubes (additive)
        ↓
[Post] selective bloom → DOF → AgX tone map → fog
        ↓ (parallel)
[Reduce] population rate + spike list → small buffer → mapAsync readback
        ↓ postMessage / SharedArrayBuffer
[AudioWorklet] Faust-WASM synth: spikes→grains/notes, rate→density, V→filter
```

**Scaling trade-offs (target 60fps ≈ 16.6 ms budget)**:
- **Rendered points/sprites**: 100k–1M comfortable on WebGPU desktop; ~1M is the practical hero-scene ceiling with bloom.
- **Dynamically simulated Izhikevich neurons**: realistically **10⁴–10⁵** in-browser with full synaptic gather. Native CUDA upper bounds for the math: NeMo (Fidjeland & Shanahan, IEEE ASAP 2009) — *"up to 400 million spikes per second… around 40 000 neurons under biologically plausible conditions with 1000 synapses per neuron and a mean firing rate of 10 Hz"* [IEEE Xplore](https://ieeexplore.ieee.org/document/5200021/) (its docs note high-end cards reaching "500,000s" of connected neurons); [GitHub](https://github.com/brainstudio-team/NeMo) GeNN reaches 100,000 neurons in real time (up to 3.5M capacity on a high-end GPU). [nih](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9950635/) **Decouple**: simulate 10⁴–10⁵ *neurons*, render 10⁵–10⁶ *particles/filaments* driven by them (one neuron → many trail particles).
- **HH/cable axons**: only tens of hero axons (~100 compartments each); rest are Izhikevich point neurons.
- **Synapses**: 130M+ (H01-scale) cannot be dynamically simulated — render a decimated subset as static/animated tubes; simulate connectivity only among the active neuron subset.

**Implementation path (staged)**:
1. **Spike + sprite MVP**: Three.js WebGPU; 10k Izhikevich neurons in a compute shader; emissive point sprites; verify spikes visually; benchmark FPS.
2. **Add glow**: emissive HDR + selective bloom + AgX + fog. Tune the cinematic look.
3. **Add morphology**: load a few SWC neurons (NeuroMorpho) as instanced tubes; add space-colonization procedural field.
4. **Add signal propagation**: animate emissive pulses along tubes on spike events; add 1–2 HH/cable hero axons.
5. **Add sonification**: `AudioWorklet` + Faust-WASM; spikes→grains, rate→density; spatialize with `PannerNode`.
6. **Add audio-reactive mode**: `AnalyserNode` fftSize 4096 (Hann) → DataTexture → modulate particle params (Blender-node parity).
7. **Scale + LOD**: push to 10⁵ neurons / 10⁶ particles; add spatial hashing; profile with stats-gl / GPU timestamp queries; QA the WebGL2 fallback.

**Benchmarks that change the plan**:
- Frame time >16 ms at 10k neurons → reduce synaptic density or drop to LIF; profile compute-vs-render split.
- Audio glitches → move all DSP into the WASM AudioWorklet, use `SharedArrayBuffer` (COOP/COEP), sonify every N frames instead of every frame.
- Safari/iOS target → verify Safari 26; keep WebGL2 fallback for pre-A12 iOS.

---

## Caveats
- **Performance numbers are hardware/driver dependent.** The 1M–10M particle figures come from specific GPUs (GTX 1060, RTX-class) and vendor demos; integrated/mobile GPUs are far lower. Benchmark on target devices.
- **The 400M spikes/sec ≈ 40,000 neurons figure is native CUDA (NeMo, Fidjeland & Shanahan 2009), not a browser.** In-browser WebGPU will be lower — an upper bound for the *math*, not a web guarantee. GeNN's 100,000-real-time / 3.5M-capacity figures are likewise native CUDA.
- **WebGPU compute readback is async (`mapAsync`) and adds latency.** Don't read back every neuron every frame — reduce on GPU first. This is the main risk to tight audio-visual sync.
- **WebGL2 fallback loses compute**: the simulation must degrade to float-texture ping-pong with a lower neuron count; budget separate QA.
- **`SharedArrayBuffer` requires cross-origin isolation** (COOP/COEP headers) — plan hosting accordingly.
- **Blender's "Sample Sound Frequencies" node is still in development** (PR #156247, targeting Blender 5.2) — a reference concept, not yet a shipped stable feature; the browser `AnalyserNode` equivalent is fully available today.
- **Real connectome datasets are huge** (H01 = 1.4 PB). Use only decimated proofread meshes in a browser, never raw volumes.
- **Firefox on Linux/Android WebGPU** is still rolling out through 2026 — verify per target and keep the WebGL2 fallback live.
