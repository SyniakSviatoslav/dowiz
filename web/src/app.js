// dowiz — Interface: Intent Engine composed render + Neural Field background

import { composeMenuScene, renderFrame, paintField, cartTotal } from './lib/compose/compose.mjs';
import { sceneForRole } from './lib/compose/fragments.mjs';
import { createJourney, Step } from './lib/compose/journey.mjs';

const CV_ID = 'sdf-canvas';
const API_BASE = '/api';

const App = {
  state: {
    theme: localStorage.getItem('dowiz-theme') || 'crimson',
    cart: [],
    page: 'menu',
    filter: 'all',
    role: 'customer',
    _stats: { orders: 1247, nodes: 342, uptime: '99.97%', tests: 1949, revenue: 0, active: 0 },
    _orders: [
      { id: 1001, status: 'pending', items: 3, total: 2680, time: '2 хв' },
      { id: 1002, status: 'confirmed', items: 1, total: 950, time: '8 хв' },
      { id: 1003, status: 'preparing', items: 5, total: 4130, time: '15 хв' },
      { id: 1004, status: 'ready', items: 2, total: 1640, time: '22 хв' },
      { id: 1005, status: 'in-delivery', items: 4, total: 3400, time: '35 хв' },
      { id: 1006, status: 'delivered', items: 2, total: 1230, time: '1 год' },
    ],
    _menu: [],
    _journey: createJourney(),
    _shiftActive: false,
    _shiftStart: null,
    _earningsToday: 0,
    _deliveriesToday: 0,
    _courierTab: 'tasks',
    _courierTasks: [
      { id: 2001, orderId: 1003, pickup: 'Rruga e Dibrës 45', dropoff: 'Bulevardi Zhan D\'Ark 12', status: 'assigned', items: 5, payout: 320 },
      { id: 2002, orderId: 1004, pickup: 'Rruga e Dibrës 45', dropoff: 'Rruga Myslym Shyri 78', status: 'picked-up', items: 2, payout: 180 },
    ],
    _earningsHistory: [
      { date: '2026-07-20', amount: 2450, trips: 8 },
      { date: '2026-07-19', amount: 3120, trips: 11 },
      { date: '2026-07-18', amount: 1890, trips: 6 },
    ],
    _audioCtx: null,
    _sdfCanvas: null,
    _sdfCtx: null,
    _neurons: null,
    _scene: null,
    _spikeCount: 0,
    _frameCount: 0,
    _spikeRate: 0,
  },

  async init() {
    this.restore();
    document.documentElement.dataset.theme = this.state.theme;
    await this.loadMenu();
    this.createSdfCanvas();
    await this.initNeuralField();
    this.initAudio();
    this.render();
    this.bindEvents();
    this.renderSdfLoop();
  },

  persist() {
    try {
      localStorage.setItem('dowiz-session', JSON.stringify({
        role: this.state.role,
        page: this.state.page,
        filter: this.state.filter,
        cart: this.state.cart,
        journeyStep: this.state._journey.current,
      }));
    } catch {}
  },

  restore() {
    try {
      const raw = localStorage.getItem('dowiz-session');
      if (!raw) return;
      const saved = JSON.parse(raw);
      if (saved.role) this.state.role = saved.role;
      if (saved.page) this.state.page = saved.page;
      if (saved.filter) this.state.filter = saved.filter;
      if (saved.cart) this.state.cart = saved.cart;
      if (saved.journeyStep) {
        const j = createJourney(saved.journeyStep);
        this.state._journey = j;
      }
    } catch {}
  },

  async loadMenu() {
    try {
      const mod = await import('./lib/vendor/dubin_sushi_menu.mjs');
      this.state._menu = mod.VENDOR.items;
    } catch {
      this.state._menu = [];
    }
  },

  createSdfCanvas() {
    const existing = document.getElementById(CV_ID);
    if (existing) return;
    const canvas = document.createElement('canvas');
    canvas.id = CV_ID;
    canvas.style.cssText = 'position:fixed;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:1;opacity:0.6;mix-blend-mode:screen';
    document.body.appendChild(canvas);
    this.state._sdfCanvas = canvas;
    this.state._sdfCtx = canvas.getContext('2d');
    const resize = () => {
      const w = window.innerWidth, h = window.innerHeight;
      canvas.width = Math.floor(w / 8); canvas.height = Math.floor(h / 8);
    };
    window.addEventListener('resize', resize);
    resize();
  },

  renderSdfLoop() {
    const cvs = this.state._sdfCanvas;
    const ctx = this.state._sdfCtx;
    if (!cvs || !ctx) return;
    const shapes = sceneForRole(this.state);
    const { width, height, data } = renderFrame(shapes, cvs.width, cvs.height, 0.5);
    if (data.length === 0) { requestAnimationFrame(() => this.renderSdfLoop()); return; }
    const roleVal = this.state.role === 'courier' ? 2 : this.state.role === 'owner' ? 1 : 0;
    paintField(ctx, { width, height, data }, roleVal);
    requestAnimationFrame(() => this.renderSdfLoop());
  },

  async initNeuralField() {
    const canvas = document.getElementById('bg-canvas');
    if (!canvas) return;
    try {
      const THREE = await import('three');
      const renderer = new THREE.WebGLRenderer({ canvas, alpha: false, antialias: true });
      renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
      renderer.toneMapping = THREE.ACESFilmicToneMapping;
      renderer.toneMappingExposure = 1.0;
      const scene = new THREE.Scene();
      scene.background = new THREE.Color(0x0a0a0f);
      const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 100);
      camera.position.set(0, 0, 4);
      const N = 2000;
      const pos = new Float32Array(N * 3);
      const colors = new Float32Array(N * 3);
      const sizes = new Float32Array(N);
      const v = new Float32Array(N);
      const pTypes = new Uint8Array(N);
      const c1 = new THREE.Color('#C1121F'), c2 = new THREE.Color('#F97316');
      for (let i = 0; i < N; i++) {
        pos[i*3] = (Math.random()-0.5)*8; pos[i*3+1] = (Math.random()-0.5)*5; pos[i*3+2] = (Math.random()-0.5)*2;
        v[i] = -65 + Math.random()*10; sizes[i] = 2 + Math.random()*3; pTypes[i] = Math.floor(Math.random()*4);
      }
      const geo = new THREE.BufferGeometry();
      geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
      geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
      geo.setAttribute('size', new THREE.BufferAttribute(sizes, 1));
      const mat = new THREE.PointsMaterial({ size: 0.04, vertexColors: true, transparent: true, opacity: 0.9, blending: THREE.AdditiveBlending, depthWrite: false, sizeAttenuation: true });
      scene.add(new THREE.Points(geo, mat));
      const rsz = () => { const w = window.innerWidth, h = window.innerHeight; camera.aspect = w/h; camera.updateProjectionMatrix(); renderer.setSize(w, h); };
      window.addEventListener('resize', rsz); rsz();
      canvas.addEventListener('click', () => this.sonifySpike(0.5));
      const start = performance.now();
      const frame = () => {
        const t = (performance.now()-start)/1000;
        const params = [{a:0.02,b:0.2,c:-65,d:8},{a:0.1,b:0.2,c:-65,d:2},{a:0.02,b:0.2,c:-55,d:4},{a:0.025,b:0.2,c:-65,d:2}];
        let spikes = 0;
        for (let i = 0; i < N; i++) {
          const p = params[pTypes[i]];
          const noise = Math.sin(i*12.9898+t*1.3)*0.5+1.5;
          const I = 5+noise;
          for (let s = 0; s < 2; s++) { v[i] += 0.5*(p.a*v[i]*v[i]+p.b*v[i]+140-p.u[i]+I); p.u[i] += 0.5*p.a*(p.b*v[i]-p.u[i]); }
          if (v[i] >= 30) { v[i] = p.c; p.u[i] += p.d; spikes++; colors[i*3]=1; colors[i*3+1]=0.45; colors[i*3+2]=0; }
          else { const blend = pTypes[i]/3; colors[i*3]=c1.r+(c2.r-c1.r)*blend; colors[i*3+1]=c1.g+(c2.g-c1.g)*blend; colors[i*3+2]=c1.b+(c2.b-c1.b)*blend; }
        }
        geo.attributes.color.needsUpdate = true;
        geo.attributes.position.needsUpdate = true;
        renderer.render(scene, camera);
        this.state._frameCount++;
        requestAnimationFrame(frame);
      };
      frame();
    } catch { /* graceful fallback to no background */ }
  },

  initAudio() {
    try { const C = window.AudioContext || window.webkitAudioContext; if (C) this.state._audioCtx = new C(); } catch {}
  },

  playTone(freq, dur, type='sine', vol=0.15) {
    if (!this.state._audioCtx) return;
    const ctx = this.state._audioCtx, o = ctx.createOscillator(), g = ctx.createGain();
    o.type = type; o.frequency.value = freq;
    g.gain.setValueAtTime(vol, ctx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime+dur);
    o.connect(g); g.connect(ctx.destination); o.start(); o.stop(ctx.currentTime+dur);
  },

  sonifySpike(r) { const pent = [262,294,330,392,440,524,588,660,784,880]; const f = pent[Math.floor(r*pent.length)%pent.length]; this.playTone(f, 0.08, 'sine', Math.min(0.2, r*0.5)); },
  playConfirm() { this.playTone(523,0.1); setTimeout(()=>this.playTone(659,0.1),80); setTimeout(()=>this.playTone(784,0.15),160); },
  playCancel() { this.playTone(330,0.15,'sawtooth'); },
  playOrder() { this.playTone(600,0.08); setTimeout(()=>this.playTone(800,0.08),60); setTimeout(()=>this.playTone(1000,0.12),120); },

  render() {
    document.getElementById('app').innerHTML = this.renderLayout();
    this.renderContent();
  },

  renderLayout() {
    const count = this.state.cart.reduce((s,i) => s+i.qty, 0);
    const navLinks = [
      { page: 'menu', label: 'Меню' },
      { page: 'orders', label: 'Замовлення' },
      { page: 'analytics', label: 'Аналітика' },
    ];
    return `
    <nav class="navbar">
      <div class="navbar-logo gradient-text spring">dowiz</div>
      <div class="navbar-links">
        ${navLinks.map(l => `<a href="#" class="${this.state.page===l.page?'active':''}" data-page="${l.page}">${l.label}</a>`).join('')}
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        <button class="btn btn-ghost btn-sm" onclick="App.setRole('customer')">👤</button>
        <button class="btn btn-ghost btn-sm" onclick="App.setRole('owner')">🏪</button>
        <button class="btn btn-ghost btn-sm" onclick="App.setRole('courier')">🛵</button>
        <button class="btn btn-ghost btn-sm desktop-only" onclick="App.toggleCart()">🛒 (${count})</button>
      </div>
    </nav>
    <main id="main-content">${this.state.role === 'owner' ? this.pageOwner() : this.pageForRole()}</main>
    <div class="cart-panel" id="cart-panel">
      <div class="cart-header"><h3>Кошик</h3><button class="btn btn-ghost btn-sm" onclick="App.toggleCart()">✕</button></div>
      <div class="cart-items" id="cart-items"></div>
      <div class="cart-total"><span>Разом</span><span id="cart-total">0 ALL</span></div>
      <div class="cart-actions"><button class="btn btn-primary w-full" onclick="App.checkout()">Замовити</button></div>
    </div>
    <footer><p>dowiz — децентралізований протокол доставки. ${this.state._stats.tests} тестів.</p></footer>`;
  },

  pageForRole() {
    if (this.state.role === 'courier') return this.pageCourier();
    return this.pageMenu();
  },

  pageMenu() {
    const cats = [...new Set(this.state._menu.map(i=>i.cat||i.catName))];
    return `
    <section class="menu-section">
      <h2 class="section-title">Меню</h2>
      <div class="cat-filters" id="cat-filters">
        <button class="btn btn-sm btn-primary" data-cat="all" onclick="App.filterMenu('all')">Усі</button>
        ${cats.map(c => `<button class="btn btn-sm btn-ghost" data-cat="${c}" onclick="App.filterMenu('${c}')">${c}</button>`).join('')}
      </div>
      <div class="menu-grid" id="menu-grid"></div>
    </section>`;
  },

  pageOwner() {
    return `
    <section class="owner-section">
      <h2 class="section-title">Панель керування</h2>
      <div class="stats-grid" id="owner-stats"></div>
      <div style="margin-top:24px">
        <h3>Замовлення</h3>
        <div id="owner-orders"></div>
      </div>
      <div style="margin-top:24px">
        <h3>Меню закладу</h3>
        <div id="owner-menu-grid"></div>
      </div>
    </section>`;
  },

  pageCourier() {
    const active = this.state._shiftActive;
    const shiftLabel = active ? 'Завершити зміну' : 'Почати зміну';
    const shiftClass = active ? 'btn-danger' : 'btn-primary';
    const earnings = this.state._earningsToday;
    const deliveries = this.state._deliveriesToday;
    const tasks = this.state._courierTasks || [];
    const pending = tasks.filter(t => t.status !== 'delivered').length;
    return `
    <section class="courier-section">
      <div class="courier-header">
        <div>
          <h2 class="section-title">Доставка</h2>
          <p class="section-subtitle">${active ? 'Зміна активна' : 'Зміна не активна'} · ${deliveries} доставок сьогодні</p>
        </div>
        <div style="text-align:right">
          <div class="courier-earnings">${earnings.toLocaleString()} ALL</div>
          <div style="font-size:0.8em;color:var(--brand-text-muted)">сьогодні</div>
        </div>
      </div>
      <div class="courier-shift-bar">
        <button class="btn btn-sm ${shiftClass}" onclick="App.toggleShift()">${shiftLabel}</button>
        <span style="margin-left:12px;font-size:0.85em;color:var(--brand-text-muted)">
          ${active ? `Активних завдань: ${pending}` : 'Увімкніть зміну для отримання завдань'}
        </span>
      </div>
      <div class="courier-tabs">
        <button class="btn btn-sm ${this.state._courierTab==='tasks'?'btn-primary':'btn-ghost'}" onclick="App.setCourierTab('tasks')">Завдання</button>
        <button class="btn btn-sm ${this.state._courierTab==='history'?'btn-primary':'btn-ghost'}" onclick="App.setCourierTab('history')">Історія</button>
      </div>
      <div id="courier-content"></div>
    </section>`;
  },

  renderContent() {
    if (this.state.role === 'owner') this.renderOwner();
    else if (this.state.role === 'courier') this.renderCourier();
    else this.renderMenuContent();
  },

  renderMenuContent() {
    const items = this.state.filter === 'all' ? this.state._menu : this.state._menu.filter(i => (i.cat||i.catName)===this.state.filter);
    const grid = document.getElementById('menu-grid');
    if (!grid) return;
    grid.innerHTML = items.map(i => `
      <div class="menu-item spring-fast">
        ${i.photo
          ? `<img class="menu-img" src="${i.photo}" alt="${i.name}" loading="lazy" onerror="this.style.display='none';this.parentNode.querySelector('.emoji-fallback').style.display='flex';"/><div class="menu-img emoji-fallback" style="display:none">🍣</div>`
          : `<div class="menu-img">🍣</div>`}
        <div class="menu-cat">${i.catName || i.cat}</div>
        <h4>${i.name}</h4>
        <p class="menu-desc">${i.ingredients || i.desc || ''}</p>
        <div class="menu-footer">
          <span class="menu-price">${i.drink ? 'Ask waiter' : `${i.price.toLocaleString()} ALL`}</span>
          ${i.drink ? '' : `<button class="btn btn-primary btn-sm" onclick="App.addToCart(${i.id})">Додати</button>`}
        </div>
      </div>
    `).join('');
  },

  renderOwner() {
    this.renderOwnerStats();
    this.renderOwnerOrders();
    this.renderOwnerMenu();
  },

  renderOwnerStats() {
    const el = document.getElementById('owner-stats');
    if (!el) return;
    const s = this.state._stats;
    el.innerHTML = [
      { label: 'Всього замовлень', value: s.orders },
      { label: 'Активних', value: s.active || this.state._orders.filter(o => o.status!=='delivered').length },
      { label: 'Тестів', value: s.tests },
    ].map(t => `<div class="stat-card spring"><div class="stat-number">${t.value}</div><div class="stat-label">${t.label}</div></div>`).join('');
  },

  renderOwnerOrders() {
    const el = document.getElementById('owner-orders');
    if (!el) return;
    const labels = { pending:'Очікує', confirmed:'Підтверджено', preparing:'Готується', ready:'Готово', 'in-delivery':'В дорозі', delivered:'Доставлено' };
    const colors = { pending:'#D97706', confirmed:'#2563EB', preparing:'#F59E0B', ready:'#0D9488', 'in-delivery':'#3B82F6', delivered:'#059669' };
    el.innerHTML = this.state._orders.map(o => `
      <div class="order-row spring-fast">
        <div><div class="order-id">#${o.id}</div><div class="order-meta">${o.items} позицій · ${o.total.toLocaleString()} ALL · ${o.time}</div></div>
        <div class="order-status">
          <span class="status-dot" style="background:${colors[o.status]||'#888'}"></span>
          <span>${labels[o.status]||o.status}</span>
        </div>
      </div>
    `).join('');
  },

  renderOwnerMenu() {
    const el = document.getElementById('owner-menu-grid');
    if (!el) return;
    const items = this.state._menu.slice(0, 12);
    el.innerHTML = items.map(i => `
      <div class="menu-item spring-fast" style="padding:8px;font-size:0.85em">
        <div style="display:flex;justify-content:space-between;align-items:center">
          <strong>${i.name}</strong>
          <span>${i.price.toLocaleString()} ALL</span>
        </div>
        <div style="font-size:0.8em;color:var(--brand-text-muted)">${i.catName||i.cat} · ${i.drink ? 'Напій' : 'Страва'}</div>
      </div>
    `).join('');
  },

  renderCourier() {
    const el = document.getElementById('courier-content');
    if (!el) return;
    if (this.state._courierTab === 'history') {
      const history = this.state._earningsHistory || [];
      el.innerHTML = `
        <div class="courier-history">
          <h3>Історія змін</h3>
          ${history.map(h => `
            <div class="order-row spring-fast">
              <div><div class="order-id">${h.date}</div><div class="order-meta">${h.trips} доставок</div></div>
              <div class="order-status"><strong>${h.amount.toLocaleString()} ALL</strong></div>
            </div>
          `).join('')}
          ${history.length === 0 ? '<p style="text-align:center;padding:24px;color:var(--brand-text-muted)">Історія порожня</p>' : ''}
        </div>`;
      return;
    }
    const tasks = this.state._courierTasks || [];
    const labels = { assigned:'Призначено', 'picked-up':'Забрано', delivered:'Доставлено' };
    if (tasks.length === 0) {
      el.innerHTML = '<p style="text-align:center;padding:48px;color:var(--brand-text-muted)">Немає активних завдань</p>';
      return;
    }
    el.innerHTML = tasks.map(t => `
      <div class="courier-task spring-fast">
        <div class="courier-task-header">
          <span class="order-badge ${t.status}">${labels[t.status] || t.status}</span>
          <span style="font-weight:700">${t.payout.toLocaleString()} ALL</span>
        </div>
        <div class="courier-task-addr"><strong>Забрати:</strong> ${t.pickup}</div>
        <div class="courier-task-addr"><strong>Доставити:</strong> ${t.dropoff}</div>
        <div class="courier-task-meta">${t.items} позицій · Замовлення #${t.orderId}</div>
        ${t.status === 'assigned' ? '<button class="btn btn-sm btn-primary" style="margin-top:8px" onclick="App.pickupTask('+t.id+')">Забрати замовлення</button>' : ''}
        ${t.status === 'picked-up' ? '<button class="btn btn-sm btn-primary" style="margin-top:8px" onclick="App.deliverTask('+t.id+')">Підтвердити доставку</button>' : ''}
      </div>
    `).join('');
  },

  setCourierTab(tab) {
    this.state._courierTab = tab;
    this.renderCourier();
  },

  toggleShift() {
    this.state._shiftActive = !this.state._shiftActive;
    if (this.state._shiftActive) {
      this.state._shiftStart = Date.now();
      this.playConfirm();
    } else {
      this.playCancel();
    }
    this.render();
    this.persist();
  },

  pickupTask(id) {
    const task = this.state._courierTasks.find(t => t.id === id);
    if (!task || task.status !== 'assigned') return;
    task.status = 'picked-up';
    this.playConfirm();
    this.renderCourier();
    this.persist();
  },

  deliverTask(id) {
    const task = this.state._courierTasks.find(t => t.id === id);
    if (!task || task.status !== 'picked-up') return;
    task.status = 'delivered';
    this.state._earningsToday += task.payout;
    this.state._deliveriesToday++;
    this.playOrder();
    this.render();
    this.persist();
  },

  filterMenu(cat) {
    this.state.filter = cat;
    this.renderMenuContent();
    document.querySelectorAll('#cat-filters .btn').forEach(b =>
      b.className = `btn btn-sm ${b.dataset.cat===cat ? 'btn-primary' : 'btn-ghost'}`
    );
    this.persist();
  },

  setRole(role) {
    const prev = this.state.role;
    this.state.role = role;
    if (role === 'customer') this.state.page = 'menu';
    else if (role === 'owner') this.state.page = 'owner';
    else if (role === 'courier') this.state.page = 'courier';
    this.render();
    this.state._journey.reset();
    this.persist();
  },

  addToCart(id) {
    const item = this.state._menu.find(i => i.id === id);
    if (!item || item.drink) return;
    const existing = this.state.cart.find(i => i.id === id);
    if (existing) existing.qty++;
    else this.state.cart.push({ ...item, qty: 1 });
    this.playConfirm();
    this.renderCart();
    this.toggleCart(true);
    this.persist();
  },

  removeFromCart(id) {
    this.state.cart = this.state.cart.filter(i => i.id !== id);
    this.playCancel();
    this.renderCart();
    this.persist();
  },

  renderCart() {
    const count = this.state.cart.reduce((s,i) => s+i.qty, 0);
    const total = this.state.cart.reduce((s,i) => s+i.price*i.qty, 0);
    const el = document.getElementById('cart-items');
    const tEl = document.getElementById('cart-total');
    if (!el) return;
    el.innerHTML = count === 0
      ? '<p style="text-align:center;color:var(--brand-text-muted);padding:48px 0">Кошик порожній</p>'
      : this.state.cart.map(i => `
        <div class="cart-item spring-fast">
          <div><strong>${i.name}</strong><span class="text-muted"> ×${i.qty}</span></div>
          <div style="display:flex;align-items:center;gap:8px">
            <span>${(i.price*i.qty).toLocaleString()} ALL</span>
            <button class="btn-remove" onclick="App.removeFromCart('${i.id}')">✕</button>
          </div>
        </div>
      `).join('');
    if (tEl) tEl.textContent = `${total.toLocaleString()} ALL`;
  },

  toggleCart(open) {
    const panel = document.getElementById('cart-panel');
    if (open !== undefined) this.state._cartOpen = open;
    else this.state._cartOpen = !this.state._cartOpen;
    panel?.classList.toggle('open', this.state._cartOpen);
  },

  async checkout() {
    if (this.state.cart.length === 0) return;
    const total = this.state.cart.reduce((s,i) => s+i.price*i.qty, 0);
    this.playOrder();
    const order = {
      items: this.state.cart.map(i => ({ itemId: i.id, name: i.name, price: i.price, qty: i.qty })),
      total,
    };
    try {
      const resp = await fetch(`${API_BASE}/order`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(order),
      });
      if (resp.ok) {
        const created = await resp.json();
        this.state._orders.unshift({ id: created.id || (1000+Date.now()%10000), status: 'pending', items: this.state.cart.length, total, time: 'щойно' });
      }
    } catch {
      this.state._orders.unshift({ id: 1000+Date.now()%10000, status: 'pending', items: this.state.cart.length, total, time: 'щойно' });
    }
    this.state.cart = [];
    this.renderCart();
    this.renderOwnerOrders?.();
    this.toggleCart(false);
    this.state._journey.advance();
    this.persist();
  },

  navigate(page) {
    this.state.page = page;
    this.state._cartOpen = false;
    document.getElementById('cart-panel')?.classList.remove('open');
    this.render();
    document.querySelectorAll('.navbar-links a').forEach(a =>
      a.classList.toggle('active', a.dataset.page === page)
    );
    this.persist();
  },

  bindEvents() {
    document.addEventListener('click', (e) => {
      const link = e.target.closest('[data-page]');
      if (link) { e.preventDefault(); this.navigate(link.dataset.page); }
    });
    document.addEventListener('keydown', (e) => {
      if (e.key === '1') this.navigate('menu');
      else if (e.key === '2') this.navigate('orders');
      else if (e.key === '3') this.navigate('analytics');
      else if (e.key === 'c') this.toggleCart();
      else if (e.key === 'Escape') this.toggleCart(false);
      else if (e.key === 'o') this.setRole('owner');
      else if (e.key === 'p') this.setRole('customer');
    });
  },
};

window.App = App;
document.addEventListener('DOMContentLoaded', () => App.init());
