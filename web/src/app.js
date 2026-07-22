// ─── dowiz — Complete Interface Application ───
// All-in-one single-page app with shader background, storefront, analytics, and design system

const App = {
  state: {
    theme: 'crimson',
    cart: [],
    page: 'home',
    orders: [
      { id: 1001, status: 'pending', items: 3, total: 42.50, time: '2 min ago' },
      { id: 1002, status: 'confirmed', items: 1, total: 12.99, time: '8 min ago' },
      { id: 1003, status: 'preparing', items: 5, total: 67.30, time: '15 min ago' },
      { id: 1004, status: 'ready', items: 2, total: 24.00, time: '22 min ago' },
      { id: 1005, status: 'in-delivery', items: 4, total: 55.00, time: '35 min ago' },
      { id: 1006, status: 'delivered', items: 2, total: 18.50, time: '1 hour ago' },
    ],
    menu: [
      { id: 1, name: 'Margherita Pizza', price: 12.99, cat: 'Pizza', desc: 'San Marzano tomatoes, mozzarella, basil', emoji: '🍕', prep: '15 min' },
      { id: 2, name: 'Truffle Burger', price: 18.50, cat: 'Burgers', desc: 'Wagyu patty, truffle aioli, aged cheddar', emoji: '🍔', prep: '20 min' },
      { id: 3, name: 'Caesar Salad', price: 9.99, cat: 'Salads', desc: 'Romaine, parmesan, croutons', emoji: '🥗', prep: '8 min' },
      { id: 4, name: 'Sushi Platter', price: 24.00, cat: 'Japanese', desc: '12-piece assorted nigiri and maki', emoji: '🍣', prep: '25 min' },
      { id: 5, name: 'Pad Thai', price: 14.50, cat: 'Thai', desc: 'Rice noodles, shrimp, tamarind, peanuts', emoji: '🍜', prep: '18 min' },
      { id: 6, name: 'Tiramisu', price: 7.50, cat: 'Desserts', desc: 'Espresso-soaked ladyfingers, mascarpone', emoji: '🍰', prep: '5 min' },
      { id: 7, name: 'Berry Smoothie', price: 6.50, cat: 'Drinks', desc: 'Mixed berries, yogurt, honey', emoji: '🥤', prep: '3 min' },
      { id: 8, name: 'Bruschetta', price: 8.50, cat: 'Starters', desc: 'Tomato, basil, garlic on sourdough', emoji: '🥖', prep: '10 min' },
      { id: 9, name: 'Gnocchi', price: 15.00, cat: 'Pasta', desc: 'Potato dumplings, sage butter, parmesan', emoji: '🥟', prep: '22 min' },
      { id: 10, name: 'Matcha Latte', price: 5.50, cat: 'Drinks', desc: 'Ceremonial matcha, oat milk', emoji: '🍵', prep: '3 min' },
      { id: 11, name: 'Lamb Chops', price: 28.00, cat: 'Mains', desc: 'Herb-crusted, roasted vegetables', emoji: '🥩', prep: '30 min' },
      { id: 12, name: 'Chocolate Mousse', price: 8.00, cat: 'Desserts', desc: 'Dark chocolate, whipped cream, berries', emoji: '🍫', prep: '5 min' },
    ],
    filter: 'all',
    stats: { orders: 1247, nodes: 342, uptime: '99.97%', tests: 1949 }
  },

  init() {
    this.initTheme();
    this.initShader();
    this.initAudio();
    this.render();
    this.bindEvents();
    this.animate();
  },

  initTheme() {
    const saved = localStorage.getItem('dowiz-theme');
    if (saved) { document.documentElement.dataset.theme = saved; this.state.theme = saved; }
  },

  setTheme(t) {
    document.documentElement.dataset.theme = t;
    this.state.theme = t;
    localStorage.setItem('dowiz-theme', t);
    document.querySelectorAll('.theme-dot').forEach(d => d.classList.toggle('active', d.dataset.theme === t));
  },

  async initShader() {
    const canvas = document.getElementById('bg-canvas');
    if (!canvas) return;
    const adapter = await navigator.gpu?.requestAdapter();
    if (!adapter) return;
    const device = await adapter.requestDevice();
    const ctx = canvas.getContext('webgpu');
    const fmt = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format: fmt, alphaMode: 'premultiplied' });

    const mod = device.createShaderModule({ code: this.SHADER_SOURCE });
    const pipe = device.createRenderPipeline({
      layout: 'auto',
      vertex: { module: mod, entryPoint: 'vs' },
      fragment: { module: mod, entryPoint: 'fs', targets: [{ format: fmt }] }
    });

    const resB = device.createBuffer({ size: 8, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    const timeB = device.createBuffer({ size: 4, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    const mouseB = device.createBuffer({ size: 8, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    const bg = device.createBindGroup({
      layout: pipe.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: resB } },
        { binding: 1, resource: { buffer: timeB } },
        { binding: 2, resource: { buffer: mouseB } }
      ]
    });

    const resize = () => {
      canvas.width = innerWidth; canvas.height = innerHeight;
      ctx.configure({ device, format: fmt, alphaMode: 'premultiplied' });
      device.queue.writeBuffer(resB, 0, new Float32Array([canvas.width, canvas.height]));
    };
    addEventListener('resize', resize);
    resize();

    const mouse = { x: .5, y: .5 };
    canvas.addEventListener('mousemove', e => { mouse.x = e.clientX / innerWidth; mouse.y = e.clientY / innerHeight; });
    canvas.addEventListener('touchmove', e => { const t = e.touches[0]; mouse.x = t.clientX / innerWidth; mouse.y = t.clientY / innerHeight; }, { passive: true });

    const start = performance.now();
    const frame = () => {
      const t = (performance.now() - start) / 1000;
      device.queue.writeBuffer(timeB, 0, new Float32Array([t]));
      device.queue.writeBuffer(mouseB, 0, new Float32Array([mouse.x, mouse.y]));
      const enc = device.createCommandEncoder();
      const pass = enc.beginRenderPass({ colorAttachments: [{ view: ctx.getCurrentTexture().createView(), loadOp: 'clear', storeOp: 'store' }] });
      pass.setPipeline(pipe); pass.setBindGroup(0, bg); pass.draw(3); pass.end();
      device.queue.submit([enc.finish()]);
      requestAnimationFrame(frame);
    };
    frame();
  },

  initAudio() {
    this.audio = {
      ctx: null,
      play(freq, dur, type = 'sine', vol = 0.15) {
        if (!this.ctx) {
          const C = window.AudioContext || window.webkitAudioContext;
          if (!C) return;
          this.ctx = new C();
        }
        const o = this.ctx.createOscillator();
        const g = this.ctx.createGain();
        o.type = type; o.frequency.value = freq;
        g.gain.setValueAtTime(vol, this.ctx.currentTime);
        g.gain.exponentialRampToValueAtTime(0.001, this.ctx.currentTime + dur);
        o.connect(g); g.connect(this.ctx.destination);
        o.start(); o.stop(this.ctx.currentTime + dur);
      },
      confirm() { this.play(523, .1); setTimeout(() => this.play(659, .1), 80); setTimeout(() => this.play(784, .15), 160); },
      cancel() { this.play(330, .15, 'sawtooth'); },
      success() { this.play(784, .08); setTimeout(() => this.play(1047, .15), 80); },
      error() { this.play(200, .2, 'sawtooth'); },
      nav() { this.play(660, .05); },
      order() { this.play(600, .08); setTimeout(() => this.play(800, .08), 60); setTimeout(() => this.play(1000, .12), 120); }
    };
  },

  render() {
    document.getElementById('app').innerHTML = this.renderLayout();
    this.renderMenu();
    this.renderOrders();
    this.renderCart();
  },

  renderLayout() {
    return `
      <nav class="navbar">
        <div class="navbar-logo gradient-text spring">dowiz</div>
        <div class="navbar-links">
          ${['home', 'menu', 'orders', 'analytics'].map(p => `<a href="#" class="${this.state.page === p ? 'active' : ''}" data-page="${p}">${p.charAt(0).toUpperCase() + p.slice(1)}</a>`).join('')}
        </div>
        <button class="btn btn-ghost btn-sm" onclick="App.toggleCart()">Cart (${this.state.cart.reduce((s,i) => s + i.qty, 0)})</button>
      </nav>

      <main id="main-content">
        ${this.state.page === 'home' ? this.renderHome() : ''}
        ${this.state.page === 'menu' ? this.renderMenuPage() : ''}
        ${this.state.page === 'orders' ? this.renderOrdersPage() : ''}
        ${this.state.page === 'analytics' ? this.renderAnalytics() : ''}
      </main>

      <div class="cart-panel ${this.state._cartOpen ? 'open' : ''}" id="cart-panel">
        <div class="cart-header"><h3>Cart</h3><button class="btn btn-ghost btn-sm" onclick="App.toggleCart()">✕</button></div>
        <div class="cart-items" id="cart-items"></div>
        <div class="cart-total"><span>Total</span><span id="cart-total">$0.00</span></div>
        <div class="cart-actions"><button class="btn btn-primary w-full" onclick="App.checkout()">Checkout</button></div>
      </div>

      <footer>
        <p>dowiz — open source decentralized delivery protocol. ${this.state.stats.tests} tests passing.</p>
      </footer>`;
  },

  renderHome() {
    return `
      <section class="hero">
        <div class="hero-badge spring">✦ Post-Quantum Delivery Protocol</div>
        <h1>Decentralized <span>food delivery</span><br />for the next generation</h1>
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
            {icon:'🔒',title:'Post-Quantum Crypto',desc:'ML-DSA-65, ML-KEM-768, X25519 — NIST KAT-verified against quantum adversaries.'},
            {icon:'🌐',title:'Mesh Network',desc:'Decentralized P2P delivery with signed append-only logs and Merkle replication.'},
            {icon:'🧠',title:'Local AI Inference',desc:'Bebop 7B model runs entirely on-device. Voice, gesture, and friction-based interaction.'},
            {icon:'⚡',title:'Field Physics UI',desc:'Wave-equation SDF field rendering with WebGPU. Ripple animations, spring physics.'},
            {icon:'🎯',title:'Intent Engine',desc:'Deterministic intent classifier + friction FSM. All input through one InputSource.'},
            {icon:'📦',title:'Zero External Deps',desc:'1949 kernel tests, 0 failures. Pure Rust, no unsafe, auditable by anyone.'}
          ].map(f => `<div class="feature-card spring"><div class="feature-icon">${f.icon}</div><h3>${f.title}</h3><p>${f.desc}</p></div>`).join('')}
        </div>
      </section>`;
  },

  renderMenuPage() {
    return `
      <section class="menu-section">
        <h2 class="section-title">Today's Menu</h2>
        <p class="section-subtitle">Fresh, locally-sourced, prepared with care.</p>
        <div class="cat-filters" id="cat-filters">
          <button class="btn btn-sm btn-primary" data-cat="all" onclick="App.filterMenu('all')">All</button>
          ${[...new Set(this.state.menu.map(i => i.cat))].map(c => `<button class="btn btn-sm btn-ghost" data-cat="${c}" onclick="App.filterMenu('${c}')">${c}</button>`).join('')}
        </div>
        <div class="menu-grid" id="menu-grid"></div>
      </section>`;
  },

  renderMenu() {
    const filtered = this.state.filter === 'all' ? this.state.menu : this.state.menu.filter(i => i.cat === this.state.filter);
    const el = document.getElementById('menu-grid');
    if (!el) return;
    el.innerHTML = filtered.map(i => `
      <div class="menu-item spring-fast">
        <div class="menu-img">${i.emoji}</div>
        <div class="menu-cat">${i.cat} · ${i.prep}</div>
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
    document.querySelectorAll('#cat-filters .btn').forEach(b => b.className = `btn btn-sm ${b.dataset.cat === cat ? 'btn-primary' : 'btn-ghost'}`);
  },

  renderOrdersPage() {
    return `<section><h2 class="section-title">Active Orders</h2><p class="section-subtitle">Track your deliveries in real-time.</p><div class="orders-card" id="orders-card"></div></section>`;
  },

  renderOrders() {
    const el = document.getElementById('orders-card');
    if (!el) return;
    const colors = { pending:'#D97706', confirmed:'#2563EB', preparing:'#F59E0B', ready:'#0D9488', 'in-delivery':'#3B82F6', delivered:'#059669' };
    const labels = { pending:'Pending', confirmed:'Confirmed', preparing:'Preparing', ready:'Ready', 'in-delivery':'In Delivery', delivered:'Delivered' };
    el.innerHTML = this.state.orders.map(o => `
      <div class="order-row spring-fast">
        <div>
          <div class="order-id">#${o.id}</div>
          <div class="order-meta">${o.items} items · $${o.total.toFixed(2)} · ${o.time}</div>
        </div>
        <div class="order-status">
          <span class="status-dot" style="background:${colors[o.status]};animation:${o.status !== 'delivered' ? 'pulse 2s infinite' : 'none'}"></span>
          <span class="order-badge ${o.status}">${labels[o.status]}</span>
        </div>
      </div>`).join('');
  },

  renderAnalytics() {
    const s = this.state.stats;
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
        <div style="margin-top:48px;background:var(--brand-surface);border:1px solid var(--brand-border);border-radius:var(--brand-radius);padding:32px;text-align:center">
          <p style="color:var(--brand-text-muted);margin-bottom:24px">Live mesh visualization requires WebGPU compute shaders — coming in P38.</p>
          <div style="display:flex;gap:24px;justify-content:center;flex-wrap:wrap">
            ${[
              {label:'Kernel',value:'1,749 tests',color:'var(--brand-primary)'},
              {label:'Engine',value:'130 tests',color:'var(--brand-primary-hover)'},
              {label:'Node',value:'3 tests',color:'var(--color-success)'},
              {label:'Bebop',value:'68 tests',color:'var(--color-info)'}
            ].map(b => `<div style="padding:16px 24px;border-radius:var(--brand-radius-sm);background:var(--brand-surface-raised)"><div style="font-weight:700;font-size:18px;color:${b.color}">${b.value}</div><div style="font-size:12px;color:var(--brand-text-muted)">${b.label}</div></div>`).join('')}
          </div>
        </div>
      </section>`;
  },

  addToCart(id) {
    const item = this.state.menu.find(i => i.id === id);
    if (!item) return;
    const existing = this.state.cart.find(i => i.id === id);
    if (existing) existing.qty++;
    else this.state.cart.push({ ...item, qty: 1 });
    this.audio.confirm();
    this.renderCart();
    this.toggleCart(true);
  },

  removeFromCart(id) {
    this.state.cart = this.state.cart.filter(i => i.id !== id);
    this.audio.cancel();
    this.renderCart();
  },

  renderCart() {
    const count = this.state.cart.reduce((s, i) => s + i.qty, 0);
    const total = this.state.cart.reduce((s, i) => s + i.price * i.qty, 0);
    const el = document.getElementById('cart-items');
    const totalEl = document.getElementById('cart-total');
    if (!el) return;
    if (count === 0) {
      el.innerHTML = '<p style="text-align:center;color:var(--brand-text-muted);padding:48px 0">Your cart is empty</p>';
    } else {
      el.innerHTML = this.state.cart.map(i => `
        <div class="cart-item spring-fast">
          <div><strong>${i.name}</strong><span class="text-muted"> ×${i.qty}</span></div>
          <div style="display:flex;align-items:center;gap:8px">
            <span>$${(i.price * i.qty).toFixed(2)}</span>
            <button class="btn-remove" onclick="App.removeFromCart(${i.id})">✕</button>
          </div>
        </div>`).join('');
    }
    if (totalEl) totalEl.textContent = `$${total.toFixed(2)}`;
  },

  toggleCart(open) {
    const panel = document.getElementById('cart-panel');
    if (open !== undefined) { this.state._cartOpen = open; }
    else { this.state._cartOpen = !this.state._cartOpen; }
    panel?.classList.toggle('open', this.state._cartOpen);
  },

  checkout() {
    if (this.state.cart.length === 0) { this.audio.error(); return; }
    const total = this.state.cart.reduce((s, i) => s + i.price * i.qty, 0);
    this.audio.order();
    this.state.orders.unshift({ id: 1000 + Date.now() % 10000, status: 'pending', items: this.state.cart.length, total, time: 'just now' });
    this.state.cart = [];
    this.renderCart();
    this.renderOrders();
    this.toggleCart(false);
    this.navigate('orders');
  },

  navigate(page) {
    this.state.page = page;
    this.render();
    document.querySelectorAll('.navbar-links a').forEach(a => a.classList.toggle('active', a.dataset.page === page));
  },

  animate() {
    // Stagger animation
    const observer = new IntersectionObserver((entries) => {
      entries.forEach(e => { if (e.isIntersecting) e.target.classList.add('visible'); });
    }, { threshold: 0.1 });
    document.querySelectorAll('.stagger, [data-stagger]').forEach(el => observer.observe(el));
  },

  bindEvents() {
    document.addEventListener('click', (e) => {
      const pageLink = e.target.closest('[data-page]');
      if (pageLink) { e.preventDefault(); this.navigate(pageLink.dataset.page); }
    });
  },

  SHADER_SOURCE: `@group(0) @binding(0) var<uniform> res: vec2<f32>;
@group(0) @binding(1) var<uniform> t: f32;
@group(0) @binding(2) var<uniform> mo: vec2<f32>;
fn h(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2(127.1,311.7)))*43758.5453); }
fn n(p: vec2<f32>) -> f32 { let i=floor(p);let f=fract(p);let u=f*f*(3-2*f);return mix(mix(h(i+vec2(0,0)),h(i+vec2(1,0)),u.x),mix(h(i+vec2(0,1)),h(i+vec2(1,1)),u.x),u.y); }
fn fbm(p: vec2<f32>) -> f32 { var v=0.;var a=0.5;var o=p;for(var i=0u;i<5u;i++){v+=a*n(o);o*=2.;a*=0.5;}return v; }
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> { return vec4(f32(i>>1)*4-1,f32(i&1)*4-1,0,1); }
@fragment fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
  let uv=p.xy/res;let a=res.x/res.y;var c=uv*2.-1.;c.x*=a;
  let w=sin(c.x*3.+t)*cos(c.y*3.+t*.3)*.3+sin((c.x*2.+c.y*2.)*2.+t*.7)*.2+fbm(c*2.+t*.1)*.3;
  let m=mo*2.-1.;m.x*=a;let d=length(c-m);let r=sin(d*20.-t*4.)*.1/(d*2.+.5);
  let v=w+r;let c1=vec3(.91,.31,.09);let c2=vec3(1.,.63,.18);let c3=vec3(.05,.05,.1);
  let tv=v*.5+.5;let col=mix(c3,mix(c1,c2,tv),smoothstep(0.,1.,tv));
  return vec4(col*(.8+.4*(1.-length(c)*.4)),1.);
}`
};

document.addEventListener('DOMContentLoaded', () => App.init());
window.App = App;
