// ─── dowiz — Complete Interface with Neural Field + Sonification ───

const App = {
  state: {
    theme: localStorage.getItem('dowiz-theme') || 'crimson',
    cart: [],
    page: 'home',
    filter: 'all',
    _stats: { orders: 1247, nodes: 342, uptime: '99.97%', tests: 1949 },
    _orders: [
      { id: 1001, status: 'pending', items: 3, total: 42.50, time: '2 min' },
      { id: 1002, status: 'confirmed', items: 1, total: 12.99, time: '8 min' },
      { id: 1003, status: 'preparing', items: 5, total: 67.30, time: '15 min' },
      { id: 1004, status: 'ready', items: 2, total: 24.00, time: '22 min' },
      { id: 1005, status: 'in-delivery', items: 4, total: 55.00, time: '35 min' },
      { id: 1006, status: 'delivered', items: 2, total: 18.50, time: '1 hour' },
    ],
    _menu: [
      { id: 1, name: 'Margherita Pizza', price: 12.99, cat: 'Pizza', desc: 'San Marzano tomatoes, mozzarella, basil', emoji: '\u{1F355}', prep: '15 min' },
      { id: 2, name: 'Truffle Burger', price: 18.50, cat: 'Burgers', desc: 'Wagyu patty, truffle aioli, aged cheddar', emoji: '\u{1F354}', prep: '20 min' },
      { id: 3, name: 'Caesar Salad', price: 9.99, cat: 'Salads', desc: 'Romaine, parmesan, croutons', emoji: '\u{1F957}', prep: '8 min' },
      { id: 4, name: 'Sushi Platter', price: 24.00, cat: 'Japanese', desc: '12-piece assorted nigiri and maki', emoji: '\u{1F363}', prep: '25 min' },
      { id: 5, name: 'Pad Thai', price: 14.50, cat: 'Thai', desc: 'Rice noodles, shrimp, tamarind, peanuts', emoji: '\u{1F35C}', prep: '18 min' },
      { id: 6, name: 'Tiramisu', price: 7.50, cat: 'Desserts', desc: 'Espresso-soaked ladyfingers, mascarpone', emoji: '\u{1F370}', prep: '5 min' },
      { id: 7, name: 'Berry Smoothie', price: 6.50, cat: 'Drinks', desc: 'Mixed berries, yogurt, honey', emoji: '\u{1F964}', prep: '3 min' },
      { id: 8, name: 'Bruschetta', price: 8.50, cat: 'Starters', desc: 'Tomato, basil, garlic on sourdough', emoji: '\u{1F956}', prep: '10 min' },
      { id: 9, name: 'Gnocchi', price: 15.00, cat: 'Pasta', desc: 'Potato dumplings, sage butter, parmesan', emoji: '\u{1F95F}', prep: '22 min' },
      { id: 10, name: 'Matcha Latte', price: 5.50, cat: 'Drinks', desc: 'Ceremonial matcha, oat milk', emoji: '\u{1F375}', prep: '3 min' },
      { id: 11, name: 'Lamb Chops', price: 28.00, cat: 'Mains', desc: 'Herb-crusted, roasted vegetables', emoji: '\u{1F969}', prep: '30 min' },
      { id: 12, name: 'Chocolate Mousse', price: 8.00, cat: 'Desserts', desc: 'Dark chocolate, whipped cream, berries', emoji: '\u{1F36B}', prep: '5 min' },
    ],
    _neurons: null,
    _audioCtx: null,
    _gpu: null
  },

  async init() {
    document.documentElement.dataset.theme = this.state.theme;
    await this.initWebGPU();
    this.initAudio();
    this.render();
    this.bindEvents();
  },

  // ═══ WebGPU Neural Field ═══

  async initWebGPU() {
    const canvas = document.getElementById('bg-canvas');
    if (!canvas) return;
    const adapter = await navigator.gpu?.requestAdapter();
    if (!adapter) { this.renderFallback(canvas); return; }
    const device = await adapter.requestDevice();
    const ctx = canvas.getContext('webgpu');
    const fmt = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: fmt, alphaMode: 'premultiplied' });

    const dims = { x: 128, y: 80 };
    const total = dims.x * dims.y;

    // ── Izhikevich neurons (storage buffer) ──
    const initNeurons = new Float32Array(total * 3);
    for (let i = 0; i < total; i++) {
      initNeurons[i * 3] = -65 + Math.random() * 10;     // v
      initNeurons[i * 3 + 1] = Math.random() * 10;        // u
      initNeurons[i * 3 + 2] = 0;                           // spike
    }
    const neuronBuf = device.createBuffer({
      size: initNeurons.byteLength,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC,
      mappedAtCreation: true
    });
    new Float32Array(neuronBuf.getMappedRange()).set(initNeurons);
    neuronBuf.unmap();

    // ── Field buffer (wave equation) ──
    const fieldBuf = device.createBuffer({
      size: total * 4,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST
    });

    // ── Uniforms ──
    const timeBuf = device.createBuffer({ size: 4, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    const mouseBuf = device.createBuffer({ size: 8, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    const dimsBuf = device.createBuffer({ size: 8, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(dimsBuf, 0, new Uint32Array([dims.x, dims.y]));

    // ── Compute shader ──
    const computeModule = device.createShaderModule({ code: `
      struct Neuron { v: f32, u: f32, spike: u32, };
      @group(0) @binding(0) var<storage, read_write> neurons: array<Neuron>;
      @group(0) @binding(1) var<storage, read_write> field: array<f32>;
      @group(0) @binding(2) var<uniform> time: f32;
      @group(0) @binding(3) var<uniform> mouse: vec2<f32>;
      @group(0) @binding(4) var<uniform> dims: vec2<u32>;

      fn param(id: u32) -> vec4<f32> {
        let h = fract(f32(id) * 0.6180339);
        if (h < 0.5) { return vec4(0.02, 0.2, -65.0, 8.0); }
        else if (h < 0.75) { return vec4(0.1, 0.2, -65.0, 2.0); }
        else { return vec4(0.02, 0.2, -55.0, 4.0); }
      }

      @compute @workgroup_size(64)
      fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let idx = id.x;
        let total = dims.x * dims.y;
        if (idx >= total) { return; }
        var n = neurons[idx];
        let p = param(idx);
        let x = f32(idx % dims.x) / f32(dims.x);
        let y = f32(idx / dims.x) / f32(dims.y);
        let dist = distance(vec2(x, y), mouse);
        let mouseI = exp(-dist * dist * 20.0) * 15.0;
        let noise = fract(sin(f32(idx) * 12.9898 + time * 1.3) * 43758.5453) * 3.0;
        let fieldF = field[idx] * 0.3;
        let I = 5.0 + noise + mouseI + fieldF;
        for (var s = 0u; s < 2u; s++) {
          n.v = n.v + 0.5 * (0.04 * n.v * n.v + 5.0 * n.v + 140.0 - n.u + I);
          n.u = n.u + 0.5 * p.x * (p.y * n.v - n.u);
        }
        n.spike = 0u;
        if (n.v >= 30.0) { n.v = p.z; n.u = n.u + p.w; n.spike = 1u; }
        neurons[idx] = n;
        if (n.spike == 1u) {
          for (var dy = 0u; dy <= 2u; dy++) {
            for (var dx = 0u; dx <= 2u; dx++) {
              let nx = (idx % dims.x + dx) % dims.x;
              let ny = (idx / dims.x + dy) % dims.y;
              let ni = ny * dims.x + nx;
              if (ni < total) { field[ni] = field[ni] + exp(-f32(dx*dx + dy*dy) * 0.5) * 0.5; }
            }
          }
        }
      }
    `});

    const computePipeline = device.createComputePipeline({ layout: 'auto', compute: { module: computeModule, entryPoint: 'main' } });

    const computeBG = device.createBindGroup({
      layout: computePipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: neuronBuf } },
        { binding: 1, resource: { buffer: fieldBuf } },
        { binding: 2, resource: { buffer: timeBuf } },
        { binding: 3, resource: { buffer: mouseBuf } },
        { binding: 4, resource: { buffer: dimsBuf } },
      ]
    });

    // ── Render shader ──
    const renderModule = device.createShaderModule({ code: `
      @group(0) @binding(0) var<storage, read> neurons: array<Neuron>;
      @group(0) @binding(1) var<storage, read> field: array<f32>;
      @group(0) @binding(2) var<uniform> dims: vec2<u32>;
      @group(0) @binding(3) var<uniform> t: f32;
      struct Neuron { v: f32, u: f32, spike: u32, };

      fn h(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2(127.1,311.7)))*43758.5453); }
      fn n(p: vec2<f32>) -> f32 { let i=floor(p);let f=fract(p);let u=f*f*(3-2*f);return mix(mix(h(i+vec2(0,0)),h(i+vec2(1,0)),u.x),mix(h(i+vec2(0,1)),h(i+vec2(1,1)),u.x),u.y); }
      fn fbm(p: vec2<f32>) -> f32 { var v=0.;var a=0.5;var o=p;for(var i=0u;i<4u;i++){v+=a*n(o);o*=2.;a*=0.5;}return v; }

      @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> { return vec4(f32(i>>1)*4-1,f32(i&1)*4-1,0,1); }
      @fragment fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
        let uv = p.xy / vec2(f32(dims.x)*4, f32(dims.y)*4);
        let a = f32(dims.x)/f32(dims.y);
        var c = uv*2.-1.; c.x *= a;

        // Neural field visualization
        let fi = u32((uv.x * f32(dims.x))) % dims.x;
        let fj = u32((uv.y * f32(dims.y))) % dims.y;
        let nid = fj * dims.x + fi;
        var glow = 0.0;
        if (nid < 128u * 80u) {
          let spike = neurons[nid].spike;
          let fv = field[nid];
          glow = f32(spike) * 0.8 + fv * 0.5;
        }

        // Background wave field
        let w = sin(c.x*3.+t)*cos(c.y*3.+t*.3)*.15 + sin((c.x*2.+c.y*2.)*2.+t*.7)*.1 + fbm(c*2.+t*.1)*.15;
        let v = w + glow;

        // Color
        let c1 = vec3(.91,.31,.09);
        let c2 = vec3(1.,.63,.18);
        let c3 = vec3(.05,.05,.1);
        let tv = v*.5+.5;
        let col = mix(c3, mix(c1, c2, tv), smoothstep(0.,1.,tv));
        let vig = 1. - length(c)*.4;
        return vec4(col*(.8+.4*vig) + vec3(glow*.3, glow*.15, 0), 1.);
      }
    `});

    const renderPipeline = device.createRenderPipeline({
      layout: 'auto',
      vertex: { module: renderModule, entryPoint: 'vs' },
      fragment: { module: renderModule, entryPoint: 'fs', targets: [{ format: fmt }] }
    });

    const renderBG = device.createBindGroup({
      layout: renderPipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: neuronBuf } },
        { binding: 1, resource: { buffer: fieldBuf } },
        { binding: 2, resource: { buffer: dimsBuf } },
        { binding: 3, resource: { buffer: timeBuf } },
      ]
    });

    // ── Staging buffer for spike readback ──
    const stagingBuf = device.createBuffer({
      size: 4 * 1024,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST
    });

    // ── Mouse tracking ──
    const mouse = { x: 0.5, y: 0.5 };
    const mouseHandler = (e, t) => {
      const rect = canvas.getBoundingClientRect();
      const ex = t ? e.touches[0].clientX : e.clientX;
      const ey = t ? e.touches[0].clientY : e.clientY;
      mouse.x = (ex - rect.left) / rect.width;
      mouse.y = (ey - rect.top) / rect.height;
    };
    canvas.addEventListener('mousemove', e => mouseHandler(e));
    canvas.addEventListener('touchmove', e => mouseHandler(e, true), { passive: true });

    // ── Resize ──
    const resize = () => {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
      ctx.configure({ device, format: fmt, alphaMode: 'premultiplied' });
    };
    window.addEventListener('resize', resize);
    resize();

    // ── Spike counter for audio sonification ──
    let spikeCount = 0;
    let spikeRate = 0;
    let frameCount = 0;

    // ── Frame loop ──
    const start = performance.now();
    const frame = () => {
      const elapsed = (performance.now() - start) / 1000;
      device.queue.writeBuffer(timeBuf, 0, new Float32Array([elapsed]));
      device.queue.writeBuffer(mouseBuf, 0, new Float32Array([mouse.x, mouse.y]));

      // Compute
      const enc = device.createCommandEncoder();
      const computePass = enc.beginComputePass();
      computePass.setPipeline(computePipeline);
      computePass.setBindGroup(0, computeBG);
      computePass.dispatchWorkgroups(Math.ceil(total / 64));
      computePass.end();

      // Copy spike data for audio
      enc.copyBufferToBuffer(neuronBuf, 0, stagingBuf, 0, Math.min(4096, total * 12));

      // Render
      const pass = enc.beginRenderPass({
        colorAttachments: [{ view: ctx.getCurrentTexture().createView(), loadOp: 'clear', storeOp: 'store' }]
      });
      pass.setPipeline(renderPipeline);
      pass.setBindGroup(0, renderBG);
      pass.draw(3);
      pass.end();

      device.queue.submit([enc.finish()]);

      // Read spikes for audio (every 6th frame)
      frameCount++;
      if (frameCount % 6 === 0) {
        stagingBuf.mapAsync(GPUMapMode.READ).then(() => {
          const data = new Uint32Array(stagingBuf.getMappedRange());
          spikeCount = 0;
          for (let i = 0; i < 256; i++) {
            if (data[i * 3 + 2] > 0) spikeCount++;
          }
          stagingBuf.unmap();
          spikeRate = spikeCount / 256;
          if (spikeRate > 0.05) this.sonifySpike(spikeRate);
        }).catch(() => {});
      }

      requestAnimationFrame(frame);
    };
    frame();

    this.state._gpu = { device, computePipeline, renderPipeline, computeBG, renderBG, neuronBuf, fieldBuf, timeBuf, mouseBuf, dimsBuf, stagingBuf, dims, total };
  },

  renderFallback(canvas) {
    // Simple canvas background when WebGPU unavailable
    const ctx = canvas.getContext('2d');
    const resize = () => { canvas.width = window.innerWidth; canvas.height = window.innerHeight; };
    window.addEventListener('resize', resize);
    resize();

    const particles = Array.from({ length: 100 }, () => ({
      x: Math.random() * canvas.width, y: Math.random() * canvas.height,
      vx: (Math.random() - 0.5) * 2, vy: (Math.random() - 0.5) * 2,
      r: Math.random() * 2 + 1
    }));

    const frame = () => {
      ctx.fillStyle = 'rgba(18,18,18,0.05)';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      for (const p of particles) {
        p.x += p.vx; p.y += p.vy;
        if (p.x < 0 || p.x > canvas.width) p.vx *= -1;
        if (p.y < 0 || p.y > canvas.height) p.vy *= -1;
        ctx.fillStyle = 'rgba(234,79,22,' + (0.2 + p.r * 0.1) + ')';
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx.fill();
      }
      requestAnimationFrame(frame);
    };
    frame();
  },

  // ═══ Audio Sonification ═══

  initAudio() {
    try {
      const C = window.AudioContext || window.webkitAudioContext;
      if (C) this.state._audioCtx = new C();
    } catch (e) {}
  },

  playTone(freq, dur, type = 'sine', vol = 0.15) {
    if (!this.state._audioCtx) return;
    const ctx = this.state._audioCtx;
    const o = ctx.createOscillator();
    const g = ctx.createGain();
    o.type = type;
    o.frequency.value = freq;
    g.gain.setValueAtTime(vol, ctx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + dur);
    o.connect(g);
    g.connect(ctx.destination);
    o.start();
    o.stop(ctx.currentTime + dur);
  },

  sonifySpike(rate) {
    // Neural spike → frequency mapping (pentatonic scale)
    const pentatonic = [262, 294, 330, 392, 440, 524, 588, 660, 784, 880];
    const freq = pentatonic[Math.floor(rate * pentatonic.length) % pentatonic.length];
    const vol = Math.min(0.2, rate * 0.5);
    this.playTone(freq, 0.08, 'sine', vol);
  },

  playConfirm() { this.playTone(523, 0.1); setTimeout(() => this.playTone(659, 0.1), 80); setTimeout(() => this.playTone(784, 0.15), 160); },
  playCancel() { this.playTone(330, 0.15, 'sawtooth'); },
  playSuccess() { this.playTone(784, 0.08); setTimeout(() => this.playTone(1047, 0.15), 80); },
  playError() { this.playTone(200, 0.2, 'sawtooth'); },
  playOrder() { this.playTone(600, 0.08); setTimeout(() => this.playTone(800, 0.08), 60); setTimeout(() => this.playTone(1000, 0.12), 120); },

  // ═══ Rendering ═══

  render() {
    document.getElementById('app').innerHTML = this.renderApp();
    this.renderMenu();
    this.renderOrders();
    this.renderCart();
  },

  renderApp() {
    const [home, menu, orders, analytics] = ['home', 'menu', 'orders', 'analytics'];

    return `
    <nav class="navbar">
      <div class="navbar-logo gradient-text spring">dowiz</div>
      <div class="navbar-links">
        ${[home, menu, orders, analytics].map(p =>
          `<a href="#" class="${this.state.page === p ? 'active' : ''}" data-page="${p}">${p.charAt(0).toUpperCase() + p.slice(1)}</a>`
        ).join('')}
      </div>
      <button class="btn btn-ghost btn-sm desktop-only" onclick="App.toggleCart()">
        Cart (${this.state.cart.reduce((s, i) => s + i.qty, 0)})
      </button>
    </nav>

    <main id="main-content">
      ${this.state.page === 'home' ? this.pageHome() : ''}
      ${this.state.page === 'menu' ? this.pageMenu() : ''}
      ${this.state.page === 'orders' ? this.pageOrders() : ''}
      ${this.state.page === 'analytics' ? this.pageAnalytics() : ''}
    </main>

    <div class="cart-panel" id="cart-panel">
      <div class="cart-header"><h3>Cart</h3><button class="btn btn-ghost btn-sm" onclick="App.toggleCart()">✕</button></div>
      <div class="cart-items" id="cart-items"></div>
      <div class="cart-total"><span>Total</span><span id="cart-total">$0.00</span></div>
      <div class="cart-actions"><button class="btn btn-primary w-full" onclick="App.checkout()">Checkout</button></div>
    </div>

    <footer><p>dowiz — open source decentralized delivery protocol. ${this.state._stats.tests} tests passing.</p></footer>`;
  },

  pageHome() {
    return `
    <section class="hero">
      <div class="hero-badge spring">✦ Post-Quantum Delivery Protocol</div>
      <h1>Decentralized <span>food delivery</span><br/>for the next generation</h1>
      <p>Zero-trust mesh network with post-quantum cryptography, deterministic state machines, and AI-powered local inference.</p>
      <div class="hero-actions">
        <button class="btn btn-primary btn-lg spring" onclick="App.navigate('menu')">Browse Menu</button>
        <button class="btn btn-secondary btn-lg spring" onclick="App.navigate('orders')">Track Orders</button>
        <button class="btn btn-ghost btn-lg" onclick="App.navigate('analytics')">Network Stats</button>
      </div>
    </section>

    <section class="features-section" id="features">
      <h2 class="section-title">Built for the future</h2>
      <p class="section-subtitle">Post-quantum security, mesh networking, local-first AI — built from the ground up in Rust.</p>
      <div class="features-grid stagger">
        ${[
          {icon:'\u{1F512}',title:'Post-Quantum Crypto',desc:'ML-DSA-65, ML-KEM-768, X25519 — NIST KAT-verified against quantum adversaries.'},
          {icon:'\u{1F310}',title:'Mesh Network',desc:'Decentralized P2P delivery with signed append-only logs and Merkle replication.'},
          {icon:'\u{1F9E0}',title:'Local AI Inference',desc:'Bebop 7B model runs entirely on-device. Voice, gesture, friction-based interaction.'},
          {icon:'\u{26A1}',title:'Field Physics UI',desc:'Wave-equation SDF field rendering with WebGPU. Ripple animations, spring physics.'},
          {icon:'\u{1F3AF}',title:'Intent Engine',desc:'Deterministic intent classifier + friction FSM. All input through one InputSource.'},
          {icon:'\u{1F4E6}',title:'Zero External Deps',desc:'1949 kernel tests, 0 failures. Pure Rust, no unsafe, auditable by anyone.'}
        ].map(f =>
          `<div class="feature-card spring"><div class="feature-icon">${f.icon}</div><h3>${f.title}</h3><p>${f.desc}</p></div>`
        ).join('')}
      </div>
    </section>`;
  },

  pageMenu() {
    const cats = [...new Set(this.state._menu.map(i => i.cat))];
    return `
    <section class="menu-section">
      <h2 class="section-title">Today's Menu</h2>
      <p class="section-subtitle">Fresh, locally-sourced, prepared with care.</p>
      <div class="cat-filters" id="cat-filters">
        <button class="btn btn-sm btn-primary" data-cat="all" onclick="App.filterMenu('all')">All</button>
        ${cats.map(c => `<button class="btn btn-sm btn-ghost" data-cat="${c}" onclick="App.filterMenu('${c}')">${c}</button>`).join('')}
      </div>
      <div class="menu-grid" id="menu-grid"></div>
    </section>`;
  },

  renderMenu() {
    const items = this.state.filter === 'all' ? this.state._menu : this.state._menu.filter(i => i.cat === this.state.filter);
    const grid = document.getElementById('menu-grid');
    if (!grid) return;
    grid.innerHTML = items.map(i => `
      <div class="menu-item spring-fast">
        <div class="menu-img">${i.emoji}</div>
        <div class="menu-cat">${i.cat} \u00B7 ${i.prep}</div>
        <h4>${i.name}</h4>
        <p class="menu-desc">${i.desc}</p>
        <div class="menu-footer">
          <span class="menu-price">$${i.price.toFixed(2)}</span>
          <button class="btn btn-primary btn-sm" onclick="App.addToCart(${i.id})">Add</button>
        </div>
      </div>`).join('');
  },

  filterMenu(cat) {
    this.state.filter = cat;
    this.renderMenu();
    document.querySelectorAll('#cat-filters .btn').forEach(b =>
      b.className = `btn btn-sm ${b.dataset.cat === cat ? 'btn-primary' : 'btn-ghost'}`
    );
  },

  pageOrders() {
    return `<section><h2 class="section-title">Active Orders</h2><p class="section-subtitle">Track your deliveries in real-time.</p><div class="orders-card" id="orders-card"></div></section>`;
  },

  renderOrders() {
    const el = document.getElementById('orders-card');
    if (!el) return;
    const colors = { pending:'#D97706', confirmed:'#2563EB', preparing:'#F59E0B', ready:'#0D9488', 'in-delivery':'#3B82F6', delivered:'#059669' };
    const labels = { pending:'Pending', confirmed:'Confirmed', preparing:'Preparing', ready:'Ready', 'in-delivery':'In Delivery', delivered:'Delivered' };
    el.innerHTML = this.state._orders.map(o => `
      <div class="order-row spring-fast">
        <div><div class="order-id">#${o.id}</div><div class="order-meta">${o.items} items \u00B7 $${o.total.toFixed(2)} \u00B7 ${o.time}</div></div>
        <div class="order-status">
          <span class="status-dot" style="background:${colors[o.status]};animation:${o.status !== 'delivered' ? 'pulse 2s infinite' : 'none'}"></span>
          <span class="order-badge ${o.status}">${labels[o.status]}</span>
        </div>
      </div>`).join('');
  },

  pageAnalytics() {
    const s = this.state._stats;
    return `
    <section>
      <h2 class="section-title">Network Analytics</h2>
      <p class="section-subtitle">Real-time metrics from the dowiz mesh.</p>
      <div class="stats-grid">
        <div class="stat-card spring"><div class="stat-number">${s.orders}</div><div class="stat-label">Orders Delivered</div></div>
        <div class="stat-card spring"><div class="stat-number">${s.nodes}</div><div class="stat-label">Mesh Nodes</div></div>
        <div class="stat-card spring"><div class="stat-number">${s.uptime}</div><div class="stat-label">Network Uptime</div></div>
        <div class="stat-card spring"><div class="stat-number">${s.tests}</div><div class="stat-label">Tests Passing</div></div>
      </div>
      <div class="test-grid">
        ${[
          {label:'Kernel', value:'1,749', color:'var(--brand-primary)'},
          {label:'Engine', value:'130', color:'var(--brand-primary-hover)'},
          {label:'Node', value:'3', color:'var(--color-success)'},
          {label:'Bebop', value:'68', color:'var(--color-info)'}
        ].map(b => `
          <div class="test-card"><div class="test-value" style="color:${b.color}">${b.value}</div><div class="test-label">${b.label}</div></div>
        `).join('')}
      </div>
    </section>`;
  },

  // ═══ Cart ═══

  addToCart(id) {
    const item = this.state._menu.find(i => i.id === id);
    if (!item) return;
    const existing = this.state.cart.find(i => i.id === id);
    if (existing) existing.qty++;
    else this.state.cart.push({ ...item, qty: 1 });
    this.playConfirm();
    this.renderCart();
    this.toggleCart(true);
  },

  removeFromCart(id) {
    this.state.cart = this.state.cart.filter(i => i.id !== id);
    this.playCancel();
    this.renderCart();
  },

  renderCart() {
    const count = this.state.cart.reduce((s, i) => s + i.qty, 0);
    const total = this.state.cart.reduce((s, i) => s + i.price * i.qty, 0);
    const el = document.getElementById('cart-items');
    const tEl = document.getElementById('cart-total');
    if (!el) return;
    el.innerHTML = count === 0
      ? '<p style="text-align:center;color:var(--brand-text-muted);padding:48px 0">Your cart is empty</p>'
      : this.state.cart.map(i =>
        `<div class="cart-item spring-fast">
          <div><strong>${i.name}</strong><span class="text-muted"> \u00D7${i.qty}</span></div>
          <div style="display:flex;align-items:center;gap:8px">
            <span>$${(i.price * i.qty).toFixed(2)}</span>
            <button class="btn-remove" onclick="App.removeFromCart(${i.id})">\u2715</button>
          </div>
        </div>`).join('');
    if (tEl) tEl.textContent = `$${total.toFixed(2)}`;
  },

  toggleCart(open) {
    const panel = document.getElementById('cart-panel');
    if (open !== undefined) this.state._cartOpen = open;
    else this.state._cartOpen = !this.state._cartOpen;
    panel?.classList.toggle('open', this.state._cartOpen);
  },

  checkout() {
    if (this.state.cart.length === 0) { this.playError(); return; }
    const total = this.state.cart.reduce((s, i) => s + i.price * i.qty, 0);
    this.playOrder();
    this.state._orders.unshift({
      id: 1000 + Date.now() % 10000, status: 'pending',
      items: this.state.cart.length, total, time: 'just now'
    });
    this.state.cart = [];
    this.renderCart();
    this.renderOrders();
    this.toggleCart(false);
    this.navigate('orders');
  },

  // ═══ Navigation & Theme ═══

  navigate(page) {
    this.state.page = page;
    this.state._cartOpen = false;
    document.getElementById('cart-panel')?.classList.remove('open');
    this.render();
    document.querySelectorAll('.navbar-links a').forEach(a =>
      a.classList.toggle('active', a.dataset.page === page)
    );
    window.scrollTo({ top: 0, behavior: 'smooth' });
  },

  setTheme(t) {
    this.state.theme = t;
    document.documentElement.dataset.theme = t;
    localStorage.setItem('dowiz-theme', t);
    document.querySelectorAll('.theme-dot').forEach(d =>
      d.classList.toggle('active', d.dataset.theme === t)
    );
  },

  bindEvents() {
    document.addEventListener('click', (e) => {
      const link = e.target.closest('[data-page]');
      if (link) { e.preventDefault(); this.navigate(link.dataset.page); }
    });

    // Intersection observer for stagger animations
    const observer = new IntersectionObserver((entries) => {
      entries.forEach(e => { if (e.isIntersecting) e.target.classList.add('visible'); });
    }, { threshold: 0.1 });
    document.querySelectorAll('.stagger').forEach(el => observer.observe(el));

    // Keyboard shortcuts
    document.addEventListener('keydown', (e) => {
      if (e.key === '1') this.navigate('home');
      else if (e.key === '2') this.navigate('menu');
      else if (e.key === '3') this.navigate('orders');
      else if (e.key === '4') this.navigate('analytics');
      else if (e.key === 'c') this.toggleCart();
      else if (e.key === 'Escape') this.toggleCart(false);
    });
  }
};

document.addEventListener('DOMContentLoaded', () => App.init());
window.App = App;
