// ─── dowiz: Single-file PWA (1289 lines) ─────────────────────────────
// Sections: ① Imports ② Constants ③ State ④ Init ⑤ Router ⑥ Pages
//           ⑦ Renderers ⑧ Cart ⑨ Owner ⑩ Courier ⑪ Telemetry ⑫ Events

import { composeMenuScene, renderFrame, paintField, cartTotal } from './lib/compose/compose.mjs';
import { sceneForRole } from './lib/compose/fragments.mjs';
import { createJourney } from './lib/compose/journey.mjs';
import { createSonifier } from './lib/audio/sonify.mjs';
import { createOracle } from './lib/telemetry/oracle.mjs';
import { createMarkov } from './lib/telemetry/markov.mjs';
import { observeVitals } from './lib/telemetry/vitals.mjs';
import { createHealthMonitor } from './lib/telemetry/health.mjs';
import { createTelegramBridge } from './lib/telemetry/telegram.mjs';
import { sanitize, rateLimit, formatETA, estimateETA, validatePhone, validateAddress, generateId } from './lib/utils.mjs';

const STATUS_CHAIN = ['pending', 'confirmed', 'preparing', 'ready', 'in-delivery', 'delivered'];
const STATUS_LABELS = { pending:'Очікує', confirmed:'Підтверджено', preparing:'Готується', ready:'Готово', 'in-delivery':'В дорозі', delivered:'Доставлено', cancelled:'Скасовано' };
const STATUS_COLORS = { pending:'#D97706', confirmed:'#2563EB', preparing:'#F59E0B', ready:'#0D9488', 'in-delivery':'#3B82F6', delivered:'#059669', cancelled:'#DC2626' };

function generateSeedOrders() {
  const names = ['Анна К.', 'Марко І.', 'Еріс Г.', 'Дардан Ш.', 'Леон Б.', 'Міра В.', 'Бесіана Н.', 'Ардіт М.'];
  const addresses = ['Rruga e Dibrës 12', 'Bulevardi Zhan D\'Ark 34', 'Rruga Myslym Shyri 56', 'Rruga Ibrahim Rugova 78', 'Rruga Ali Pashe Tepelena 90'];
  const phones = ['+355 69 111 1111', '+355 69 222 2222', '+355 69 333 3333', '+355 69 444 4444', '+355 69 555 5555'];
  const times = ['2 хв', '8 хв', '15 хв', '22 хв', '35 хв', '1 год'];
  return STATUS_CHAIN.map((status, i) => ({
    id: 1001 + i,
    status,
    items: 1 + Math.floor(Math.random() * 5),
    total: (5 + Math.floor(Math.random() * 30)) * 100,
    time: times[i % times.length],
    address: addresses[i % addresses.length],
    phone: phones[i % phones.length],
    note: i % 2 === 0 ? '' : 'Без цибулі, будь ласка',
  }));
}

function generateSeedTasks(orders) {
  const pickups = ['Rruga e Dibrës 45', 'Bulevardi Zhan D\'Ark 10'];
  const dropoffs = ['Rruga Myslym Shyri 12', 'Bulevardi Zhan D\'Ark 34', 'Rruga Ibrahim Rugova 56', 'Rruga Ali Pashe Tepelena 78'];
  const statuses = ['assigned', 'picked-up'];
  return orders.filter(o => o.status === 'preparing' || o.status === 'ready' || o.status === 'in-delivery')
    .map((o, i) => ({
      id: 2001 + i,
      orderId: o.id,
      pickup: pickups[i % pickups.length],
      dropoff: dropoffs[i % dropoffs.length],
      status: statuses[i % statuses.length],
      items: o.items,
      payout: Math.round(o.total * 0.12),
    }));
}

function generateSeedHistory() {
  const dates = ['2026-07-20', '2026-07-19', '2026-07-18'];
  return dates.map((d, i) => ({ date: d, amount: (8 + i * 4) * 100, trips: 6 + i * 2 }));
}

const CAT_UA = {
  "Chef's Picks": 'Рекомендуємо', Futomaki: 'Футомакі', Philadelphia: 'Філадельфія',
  California: 'Каліфорнія', 'Hot roll': 'Гарячі роли', Signature: 'Фірмові',
  Volcano: 'Вулкан', Maki: 'Макі', Sets: 'Набори', Snacks: 'Закуски',
  Nigiri: 'Нігірі', Bowls: 'Боули', 'Vegetarian Roll': 'Вегетаріанські',
  Cocktails: 'Коктейлі', Premium: 'Преміум',
};

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
    _orders: [],
    _menu: [],
    _deliveryAddress: '',
    _deliveryPhone: '',
    _deliveryNote: '',
    _lastOrderId: null,
    _installPrompt: null,
    _journey: createJourney(),
    _shiftActive: false,
    _shiftStart: null,
    _earningsToday: 0,
    _deliveriesToday: 0,
    _courierTab: 'tasks',
    _courierTasks: [],
    _earningsHistory: [],
    _orderDetail: null,
    _searchQuery: '',
    _ownerMenuEdit: false,
    _sonifier: null,
    _oracle: null,
    _pollTimer: null,
    _notifGranted: false,
    _pageHidden: false,
    _markov: null,
    _health: null,
    _telegram: null,
    _vitalsObserver: null,
    _stateHistory: [],
    _sdfCanvas: null,
    _sdfCtx: null,
    _neurons: null,
    _scene: null,
    _spikeCount: 0,
    _frameCount: 0,
    _spikeRate: 0,
  },

  // ── §3a Seed ─────────────────────────────────────────────────────
  seedData() {
    if (this.state._orders.length === 0) {
      const orders = generateSeedOrders();
      this.state._orders = orders;
      this.state._courierTasks = generateSeedTasks(orders);
      this.state._earningsHistory = generateSeedHistory();
    }
  },

  // ── §4 Init ──────────────────────────────────────────────────────
  async init() {
    this.seedData();
    this.restore();
    document.documentElement.dataset.theme = this.state.theme;
    await this.loadMenu();
    this.createSdfCanvas();
    await this.initNeuralField();
    this.state._sonifier = createSonifier();
    this.state._oracle = createOracle();
    this.state._oracle.mark('init');
    this.state._oracle.observeVitals();
    this.state._markov = createMarkov();
    this.state._health = createHealthMonitor();
    this.state._telegram = createTelegramBridge();
    this.state._vitalsObserver = observeVitals();
    this.requestNotifPermission();
    this.render();
    this.bindEvents();
    this.registerSw();
    this.setupInstallPrompt();
    this.renderSdfLoop();
    this.startPolling();
    document.addEventListener('visibilitychange', () => {
      this.state._pageHidden = document.hidden;
      this.state._animPaused = document.hidden;
    });

    setInterval(() => {
      const currentView = this.state.role === 'customer' ? this.state.page : this.state.role;
      const freeze = this.state._markov?.detectFreeze(currentView, 4);
      if (freeze) {
        this.state._health?.signal('freeze', freeze);
        const hintEl = document.querySelector('[data-hint]');
        if (hintEl) hintEl.style.display = 'block';
      }
    }, 10000);

    if (this.state._health) {
      setInterval(() => {
        const r = this.state._vitalsObserver?.report();
        if (r) {
          this.state._health?.signal('vitals', r);
          const poor = Object.entries(r).filter(([, v]) => v.rating === 'poor');
          if (poor.length > 0) {
            this.state._telegram?.alert('warning', `Web vital regression: ${poor.map(([k]) => k).join(', ')}`);
          }
        }
      }, 60000);
    }
  },

  // ── §5 Router ───────────────────────────────────────────────────
  navigate(page) {
    this.state._oracle?.mark('navigate-start');
    this.state._markov?.observe(`page:${page}`);
    this.state.page = page;
    this.state._cartOpen = false;
    document.getElementById('cart-panel')?.classList.remove('open');
    this.render();
    document.querySelectorAll('.navbar-links a').forEach(a =>
      a.classList.toggle('active', a.dataset.page === page)
    );
    this.sonify('navigate');
    this.state._oracle?.mark('navigate-end');
    this.persist();
  },

  // ── §4a Persist ──────────────────────────────────────────────────
  persist() {
    try {
      localStorage.setItem('dowiz-session', JSON.stringify({
        role: this.state.role,
        page: this.state.page,
        filter: this.state.filter,
        cart: this.state.cart,
        journeyStep: this.state._journey.current,
        _orders: this.state._orders,
        _courierTasks: this.state._courierTasks,
        _earningsToday: this.state._earningsToday,
        _deliveriesToday: this.state._deliveriesToday,
        _earningsHistory: this.state._earningsHistory,
        _shiftActive: this.state._shiftActive,
        _shiftStart: this.state._shiftStart,
        _deliveryAddress: this.state._deliveryAddress,
        _deliveryPhone: this.state._deliveryPhone,
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
      if (saved._orders) this.state._orders = saved._orders;
      if (saved._courierTasks) this.state._courierTasks = saved._courierTasks;
      if (saved._earningsToday !== undefined) this.state._earningsToday = saved._earningsToday;
      if (saved._deliveriesToday !== undefined) this.state._deliveriesToday = saved._deliveriesToday;
      if (saved._earningsHistory) this.state._earningsHistory = saved._earningsHistory;
      if (saved._shiftActive !== undefined) this.state._shiftActive = saved._shiftActive;
      if (saved._shiftStart !== undefined) this.state._shiftStart = saved._shiftStart;
      if (saved._deliveryAddress) this.state._deliveryAddress = saved._deliveryAddress;
      if (saved._deliveryPhone) this.state._deliveryPhone = saved._deliveryPhone;
    } catch {}
  },

  registerSw() {
    if ('serviceWorker' in navigator) {
      navigator.serviceWorker.register('/sw.js').catch(() => {});
    }
  },

  setupInstallPrompt() {
    window.addEventListener('beforeinstallprompt', (e) => {
      e.preventDefault();
      this.state._installPrompt = e;
      this.state._canInstall = true;
    });
    window.addEventListener('appinstalled', () => {
      this.state._installPrompt = null;
      this.state._canInstall = false;
    });
  },

  installApp() {
    const prompt = this.state._installPrompt;
    if (!prompt) return;
    prompt.prompt();
    prompt.userChoice.then(() => { this.state._installPrompt = null; this.state._canInstall = false; });
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
    if (this.state._pageHidden) { requestAnimationFrame(() => this.renderSdfLoop()); return; }
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
      canvas.addEventListener('click', () => {
        if (this.state._sonifier && this.state._sonifier.ctx) {
          const ctx = this.state._sonifier.ctx;
          const f = [262, 294, 330, 392, 440][Math.floor(Math.random() * 5)];
          const o = ctx.createOscillator(), g = ctx.createGain();
          o.type = 'sine'; o.frequency.value = f;
          g.gain.setValueAtTime(0.08, ctx.currentTime);
          g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.06);
          o.connect(g); g.connect(ctx.destination);
          o.start(); o.stop(ctx.currentTime + 0.06);
        }
      });
      const start = performance.now();
      const frame = () => {
        if (this.state._pageHidden) { requestAnimationFrame(frame); return; }
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

  sonify(event, money = false) {
    if (this.state._sonifier) this.state._sonifier.sonify(event, money);
  },

  toast(msg, type = 'info', dur = 4000) {
    const c = document.getElementById('toast-container') || (() => {
      const el = document.createElement('div');
      el.id = 'toast-container';
      el.setAttribute('aria-live', 'polite');
      el.setAttribute('aria-atomic', 'true');
      document.body.appendChild(el);
      return el;
    })();
    const t = document.createElement('div');
    t.className = `toast toast-${type}`;
    t.textContent = msg;
    c.appendChild(t);
    setTimeout(() => { t.classList.add('toast-out'); setTimeout(() => t.remove(), 300); }, dur);
  },

  requestNotifPermission() {
    if ('Notification' in window && Notification.permission === 'default') {
      Notification.requestPermission().then(p => { this.state._notifGranted = p === 'granted'; });
    } else if ('Notification' in window) {
      this.state._notifGranted = Notification.permission === 'granted';
    }
  },

  notify(title, body, tag) {
    this.toast(`${title}: ${body}`, 'info', 5000);
    if (this.state._notifGranted) {
      try { new Notification(title, { body, tag: tag || 'dowiz', icon: '/icon-192.png' }); } catch {}
    }
  },

  startPolling() {
    this.stopPolling();
    this.state._pollTimer = setInterval(() => {
      const prevPending = this.state._prevPendingCount || 0;
      const prevTasks = this.state._prevTaskCount || 0;
      const currPending = this.state._orders.filter(o => o.status === 'pending').length;
      const currTasks = this.state._courierTasks.filter(t => t.status === 'assigned').length;
      if (currPending > prevPending && prevPending > 0) {
        this.notify('Нове замовлення', `${currPending - prevPending} нове замовлення очікує`, 'new-order');
      }
      if (currTasks > prevTasks && prevTasks > 0) {
        this.notify('Нове завдання', `${currTasks - prevTasks} нове завдання курʼєру`, 'new-task');
      }
      this.state._prevPendingCount = currPending;
      this.state._prevTaskCount = currTasks;
    }, 15000);
  },

  stopPolling() {
    if (this.state._pollTimer) { clearInterval(this.state._pollTimer); this.state._pollTimer = null; }
  },

  sonifySpike(r) {
    const pent = [262, 294, 330, 392, 440, 524, 588, 660, 784, 880];
    const f = pent[Math.floor(r * pent.length) % pent.length];
    if (this.state._sonifier && this.state._sonifier.ctx) {
      const ctx = this.state._sonifier.ctx;
      const o = ctx.createOscillator(), g = ctx.createGain();
      o.type = 'sine'; o.frequency.value = f;
      g.gain.setValueAtTime(Math.min(0.2, r * 0.5), ctx.currentTime);
      g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.08);
      o.connect(g); g.connect(ctx.destination); o.start(); o.stop(ctx.currentTime + 0.08);
    }
  },

  render() {
    const app = document.getElementById('app');
    app.innerHTML = this.renderLayout();
    const main = document.getElementById('main-content');
    if (main) main.classList.add('page-enter');
    this.renderContent();
  },

  renderLayout() {
    const count = this.state.cart.reduce((s,i) => s+i.qty, 0);
    const activeOrders = this.state._orders.filter(o => o.status !== 'delivered').length;
    const pendingOrders = this.state._orders.filter(o => o.status === 'pending').length;
    const assignedTasks = this.state._courierTasks.filter(t => t.status === 'assigned').length;
    const pendingBadge = pendingOrders > 0 ? `<span class="notif-badge notif-badge-pending">${pendingOrders}</span>` : '';
    const taskBadge = assignedTasks > 0 ? `<span class="notif-badge notif-badge-task">${assignedTasks}</span>` : '';
    const navLinks = [
      { page: 'menu', label: 'Меню' },
      { page: 'orders', label: `Замовлення${activeOrders ? ' (' + activeOrders + ')' : ''}` },
      { page: 'analytics', label: 'Аналітика' },
      { page: '', label: '' },
    ];
    const ownerBadge = this.state.role === 'owner' && pendingOrders > 0 ? `<span class="notif-badge notif-badge-pending">${pendingOrders}</span>` : '';
    const courierBadge = this.state.role === 'courier' && assignedTasks > 0 ? `<span class="notif-badge notif-badge-task">${assignedTasks}</span>` : '';
    const roleNames = { customer: 'Клієнт', owner: 'Заклад', courier: 'Курʼєр' };
    const roleEmoji = { customer: '👤', owner: '🏪', courier: '🛵' };
    return `
    <nav class="navbar" role="navigation" aria-label="Головна навігація">
      <div class="navbar-logo gradient-text spring" aria-label="dowiz">dowiz</div>
      <div class="navbar-links" role="tablist">
        ${navLinks.map(l => `<a href="#" role="tab" aria-selected="${this.state.page===l.page}" class="${this.state.page===l.page?'active':''}" data-page="${l.page}">${sanitize(l.label)}</a>`).join('')}
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        <span class="role-badge" aria-live="polite">${roleEmoji[this.state.role]} ${roleNames[this.state.role]}${ownerBadge}${courierBadge}</span>
        <button class="btn btn-ghost btn-sm" onclick="App.setRole('customer')" aria-label="Переключити на клієнта" title="Клієнт">👤</button>
        <button class="btn btn-ghost btn-sm" onclick="App.setRole('owner')" aria-label="Переключити на заклад" title="Заклад">🏪</button>
        <button class="btn btn-ghost btn-sm" onclick="App.setRole('courier')" aria-label="Переключити на курʼєра" title="Курʼєр">🛵</button>
        <button class="btn btn-ghost btn-sm theme-btn" onclick="App.cycleTheme()" aria-label="Змінити тему" title="Змінити тему">🎨</button>
        <button class="btn btn-ghost btn-sm" onclick="App.toggleCart()" aria-label="Кошик" title="Кошик">🛒 ${count > 0 ? count : ''}</button>
        ${this.state._canInstall ? '<button class="btn btn-sm btn-primary" onclick="App.installApp()" aria-label="Встановити додаток">⬇ Встановити</button>' : ''}
      </div>
    </nav>
    <main id="main-content">${this.pageForCurrent()}</main>
    <div class="cart-panel" id="cart-panel">
      <div class="cart-header"><h3>Кошик</h3><button class="btn btn-ghost btn-sm" onclick="App.toggleCart()">✕</button></div>
      <div class="cart-items" id="cart-items"></div>
      <div class="cart-delivery" id="cart-delivery">
        <input class="cart-input" id="delivery-addr" placeholder="Адреса доставки" value="${this.state._deliveryAddress}" oninput="App.setDelivery('address', this.value)"/>
        <input class="cart-input" id="delivery-phone" placeholder="Телефон" value="${this.state._deliveryPhone}" oninput="App.setDelivery('phone', this.value)"/>
        <input class="cart-input" id="delivery-note" placeholder="Примітка (необов'язково)" value="${this.state._deliveryNote}" oninput="App.setDelivery('note', this.value)"/>
      </div>
      <div class="cart-total"><span>Разом</span><span id="cart-total">0 ALL</span></div>
      <div class="cart-actions"><button class="btn btn-primary w-full checkout-btn" onclick="App.checkout()">Замовити</button></div>
      <div data-hint style="display:none;padding:8px 20px;font-size:0.8em;color:var(--brand-primary);text-align:center">💡 Перевірте адресу та телефон перед замовленням</div>
    </div>
    <footer><p>dowiz — децентралізований протокол доставки. ${this.state._stats.tests} тестів.</p></footer>`;
  },

  // ── §6 Pages ────────────────────────────────────────────────────
  pageForCurrent() {
    if (this.state.page === 'order-detail') return this.pageOrderDetail();
    if (this.state.page === 'orders') return this.pageOrders();
    if (this.state.page === 'analytics') return this.pageAnalytics();
    if (this.state.role === 'courier') return this.pageCourier();
    if (this.state.role === 'owner') return this.pageOwner();
    return this.pageMenu();
  },

  pageOrders() {
    const count = this.state.cart.reduce((s,i) => s+i.qty, 0);
    return `
    <section>
      <h2 class="section-title">Мої замовлення</h2>
      <p class="section-subtitle">${count > 0 ? `У кошику ${count} позицій` : 'Кошик порожній'}</p>
      <div class="orders-card" id="orders-list"></div>
    </section>`;
  },

  pageAnalytics() {
    const s = this.state._stats;
    return `
    <section class="owner-section">
      <h2 class="section-title">Аналітика</h2>
      <div class="stats-grid" id="analytics-stats"></div>
      <div style="margin-top:24px">
        <h3>Метрики системи</h3>
        <div id="analytics-metrics"></div>
      </div>
      <div style="margin-top:24px">
        <h3>Активність замовлень</h3>
        <div id="analytics-timeline"></div>
      </div>
    </section>`;
  },

  pageOrderDetail() {
    const o = this.state._orderDetail;
    if (!o) return '<section><p class="section-subtitle">Замовлення не знайдено</p><button class="btn btn-ghost" onclick="App.navigate(\'orders\')">Назад</button></section>';
    if (o.status === 'cancelled') {
      return `
      <section class="order-detail">
        <div style="display:flex;align-items:center;gap:12px;margin-bottom:16px">
          <button class="btn btn-ghost btn-sm" onclick="App.navigate('orders')">← Назад</button>
          <h2 class="section-title" style="margin:0">Замовлення #${o.id}</h2>
        </div>
        <div class="order-detail-card" style="text-align:center;padding:48px">
          <div style="font-size:3em;margin-bottom:16px">❌</div>
          <h3>Замовлення скасовано</h3>
        </div>
      </section>`;
    }
    const labels = { pending:'Очікує підтвердження', confirmed:'Підтверджено', preparing:'Готується', ready:'Готово', 'in-delivery':'В дорозі', delivered:'Доставлено', cancelled:'Скасовано' };
    const colors = { pending:'#D97706', confirmed:'#2563EB', preparing:'#F59E0B', ready:'#0D9488', 'in-delivery':'#3B82F6', delivered:'#059669', cancelled:'#DC2626' };
    const chain = ['pending','confirmed','preparing','ready','in-delivery','delivered'];
    const idx = chain.indexOf(o.status);
    return `
    <section class="order-detail">
      <div style="display:flex;align-items:center;gap:12px;margin-bottom:16px">
        <button class="btn btn-ghost btn-sm" onclick="App.navigate('orders')">← Назад</button>
        <h2 class="section-title" style="margin:0">Замовлення #${o.id}</h2>
      </div>
      <div class="order-detail-card">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <span class="order-badge ${o.status}">${labels[o.status] || o.status}</span>
          <span style="font-weight:700;font-size:1.3em">${o.total.toLocaleString()} ALL</span>
        </div>
        <div style="margin:16px 0;color:var(--brand-text-muted);font-size:0.9em">
          <div>${o.items} позицій · ${o.time}</div>
          <div>${o.address || ''}${o.phone ? ' · ' + o.phone : ''}</div>
          ${o.note ? '<div>Примітка: ' + o.note + '</div>' : ''}
        </div>
        <div class="order-timeline">
          ${chain.map((s, i) => `
            <div class="timeline-step ${i <= idx ? 'active' : ''} ${i === idx ? 'current' : ''}">
              <div class="timeline-dot" style="${i <= idx ? 'background:' + colors[s] : ''}"></div>
              <div class="timeline-content">
                <div class="timeline-label">${labels[s]}</div>
                <div class="timeline-time">${i < idx ? '✓' : i === idx ? 'поточний' : ''}</div>
              </div>
            </div>
          `).join('')}
        </div>
        ${estimateETA(o) ? `<div style="margin-top:16px;padding:12px;background:var(--brand-surface);border-radius:8px;text-align:center"><span style="font-weight:600">Орієнтовний час: ${estimateETA(o)}</span></div>` : ''}
      </div>
      <div style="margin-top:16px;text-align:center">
        <button class="btn btn-ghost" onclick="App.navigate('orders')">До списку замовлень</button>
      </div>
    </section>`;
  },

  pageMenu() {
    const cats = [...new Set(this.state._menu.map(i=>i.cat||i.catName))];
    return `
    <section class="menu-section">
      <h2 class="section-title">Меню</h2>
      <div class="menu-search">
        <input class="menu-search-input" id="menu-search" type="text" placeholder="Пошук страв..." value="${this.state._searchQuery || ''}" oninput="App.searchMenu(this.value)" />
      </div>
      <div class="cat-filters" id="cat-filters" style="overflow-x:auto;white-space:nowrap;display:flex;gap:6px;padding:4px 0;-webkit-overflow-scrolling:touch;scrollbar-width:none">
        <button class="btn btn-sm btn-primary" data-cat="all" onclick="App.filterMenu('all')">Усі</button>
        ${cats.map(c => `<button class="btn btn-sm btn-ghost" style="flex-shrink:0" data-cat="${c}" onclick="App.filterMenu('${c}')">${CAT_UA[c] || c}</button>`).join('')}
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
    let shiftDuration = '';
    let earningsRate = '';
    if (active && this.state._shiftStart) {
      const elapsed = Math.floor((Date.now() - this.state._shiftStart) / 60000);
      const h = Math.floor(elapsed / 60);
      const m = elapsed % 60;
      shiftDuration = `${h} год ${m} хв`;
      earningsRate = h > 0 ? `${Math.round(earnings / h)} ALL/год` : '—';
    }
    return `
    <section class="courier-section">
      <div class="courier-header">
        <div>
          <h2 class="section-title">Доставка</h2>
          <p class="section-subtitle">${active ? 'Зміна активна' : 'Зміна не активна'} · ${deliveries} доставок сьогодні${shiftDuration ? ` · ${shiftDuration}` : ''}</p>
        </div>
        <div style="text-align:right">
          <div class="courier-earnings">${earnings.toLocaleString()} ALL</div>
          <div style="font-size:0.8em;color:var(--brand-text-muted)">${earningsRate || 'сьогодні'}</div>
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

  // ── §7 Renderers ────────────────────────────────────────────────
  renderContent() {
    try {
      if (this.state.page === 'order-detail') return;
      if (this.state.page === 'orders') this.renderOrders();
      else if (this.state.page === 'analytics') this.renderAnalytics();
      else if (this.state.role === 'owner') this.renderOwner();
      else if (this.state.role === 'courier') this.renderCourier();
      else this.renderMenuContent();
    } catch (e) {
      console.error('renderContent error:', e);
      this.state._health?.signalError('renderContent', e);
      const main = document.getElementById('main-content');
      if (main) main.innerHTML = '<section style="text-align:center;padding:48px"><p style="color:var(--brand-danger)">Помилка відображення. Спробуйте перезавантажити.</p><button class="btn btn-primary" onclick="location.reload()">Перезавантажити</button></section>';
    }
  },

  renderMenuContent() {
    let items = this.state.filter === 'all' ? this.state._menu : this.state._menu.filter(i => (i.cat||i.catName)===this.state.filter);
    if (this.state._searchQuery) {
      const q = this.state._searchQuery.toLowerCase();
      items = items.filter(i => i.name.toLowerCase().includes(q) || (i.ingredients||i.desc||'').toLowerCase().includes(q));
    }
    const grid = document.getElementById('menu-grid');
    if (!grid) return;
    grid.innerHTML = items.map(i => `
      <div class="menu-item spring-fast">
        ${i.photo
          ? `<img class="menu-img" src="${i.photo}" alt="${i.name}" loading="lazy" onerror="this.style.display='none';this.parentNode.querySelector('.emoji-fallback').style.display='flex';"/><div class="menu-img emoji-fallback" style="display:none">🍣</div>`
          : `<div class="menu-img">🍣</div>`}
        <div class="menu-cat">${CAT_UA[i.catName] || i.catName || i.cat}</div>
        <h4>${i.name}</h4>
        <p class="menu-desc">${i.ingredients || i.desc || ''}</p>
        <div class="menu-footer">
          <span class="menu-price">${i.drink ? 'Запитайте офіціанта' : `${i.price.toLocaleString()} ALL`}</span>
          ${i.drink ? '' : `<button class="btn btn-primary btn-sm" onclick="App.addToCart(${i.id})">Додати</button>`}
        </div>
      </div>
    `).join('');
  },

  renderOrders() {
    const el = document.getElementById('orders-list');
    if (!el) return;
    const activeOrders = this.state._orders.filter(o => o.status !== 'delivered');
    const pastOrders = this.state._orders.filter(o => o.status === 'delivered');
    const labels = { pending:'Очікує', confirmed:'Підтверджено', preparing:'Готується', ready:'Готово', 'in-delivery':'В дорозі', delivered:'Доставлено' };
    if (this.state._orders.length === 0 && this.state.cart.length === 0) {
      el.innerHTML = '<p style="text-align:center;padding:48px;color:var(--brand-text-muted)">Немає замовлень</p>';
      return;
    }
    const renderOrder = o => `
      <div class="order-row spring-fast" onclick="App.viewOrder(${o.id})" style="cursor:pointer">
        <div><div class="order-id">#${o.id}</div><div class="order-meta">${o.items} позицій · ${o.total.toLocaleString()} ALL</div></div>
        <div class="order-status" style="display:flex;align-items:center;gap:8px">
          <span class="order-badge ${o.status}">${labels[o.status] || o.status}</span>
          <span style="color:var(--brand-text-muted);font-size:0.8em">›</span>
        </div>
      </div>`;
    el.innerHTML = (activeOrders.length > 0
      ? `<div style="font-size:0.8em;color:var(--brand-text-muted);margin-bottom:8px;padding:0 4px">Активні (${activeOrders.length})</div>
         ${activeOrders.map(renderOrder).join('')}
         ${pastOrders.length > 0 ? `<div style="font-size:0.8em;color:var(--brand-text-muted);margin:16px 0 8px;padding:0 4px">Історія (${pastOrders.length})</div>
           ${pastOrders.slice(0, 5).map(renderOrder).join('')}` : ''}`
      : this.state._orders.slice(0, 5).map(renderOrder).join(''));
  },

  viewOrder(id) {
    const o = this.state._orders.find(x => x.id === id);
    if (!o) { this.toast('Замовлення не знайдено', 'error', 2000); return; }
    this.state._orderDetail = o;
    this.state.page = 'order-detail';
    this.state._cartOpen = false;
    this.render();
    this.persist();
  },

  renderAnalytics() {
    const s = this.state._stats;
    const statsEl = document.getElementById('analytics-stats');
    if (statsEl) {
      const activeOrders = this.state._orders.filter(o => o.status !== 'delivered').length;
      const totalRevenue = this.state._orders.reduce((sum, o) => sum + (o.total || 0), 0);
      statsEl.innerHTML = [
        { label: 'Всього замовлень', value: s.orders },
        { label: 'Активних', value: activeOrders },
        { label: 'Виручка', value: `${totalRevenue.toLocaleString()} ALL` },
        { label: 'Тестів', value: s.tests },
      ].map(t => `<div class="stat-card spring"><div class="stat-number">${t.value}</div><div class="stat-label">${t.label}</div></div>`).join('');
    }
    const metricsEl = document.getElementById('analytics-metrics');
    if (metricsEl) {
      const statusCounts = { pending:0, confirmed:0, preparing:0, ready:0, 'in-delivery':0, delivered:0 };
      this.state._orders.forEach(o => { if (statusCounts[o.status] !== undefined) statusCounts[o.status]++; });
      metricsEl.innerHTML = `
        <div class="analytics-metrics-grid">
          <div class="analytics-metric">
            <div class="metric-label">Очікує</div>
            <div class="metric-bar"><div class="metric-fill" style="width:${Math.round(statusCounts.pending / Math.max(1, this.state._orders.length) * 100)}%"></div></div>
            <div class="metric-value">${statusCounts.pending}</div>
          </div>
          <div class="analytics-metric">
            <div class="metric-label">Готується</div>
            <div class="metric-bar"><div class="metric-fill preparing" style="width:${Math.round((statusCounts.confirmed + statusCounts.preparing) / Math.max(1, this.state._orders.length) * 100)}%"></div></div>
            <div class="metric-value">${statusCounts.confirmed + statusCounts.preparing}</div>
          </div>
          <div class="analytics-metric">
            <div class="metric-label">В дорозі</div>
            <div class="metric-bar"><div class="metric-fill delivery" style="width:${Math.round((statusCounts.ready + statusCounts['in-delivery']) / Math.max(1, this.state._orders.length) * 100)}%"></div></div>
            <div class="metric-value">${statusCounts.ready + statusCounts['in-delivery']}</div>
          </div>
          <div class="analytics-metric">
            <div class="metric-label">Доставлено</div>
            <div class="metric-bar"><div class="metric-fill delivered" style="width:${Math.round(statusCounts.delivered / Math.max(1, this.state._orders.length) * 100)}%"></div></div>
            <div class="metric-value">${statusCounts.delivered}</div>
          </div>
        </div>`;
    }
    const timelineEl = document.getElementById('analytics-timeline');
    if (timelineEl) {
      const hours = ['10:00', '11:00', '12:00', '13:00', '14:00', '15:00', '16:00', '17:00', '18:00', '19:00', '20:00', '21:00'];
      const orderHourCounts = {};
      this.state._orders.forEach((o, idx) => {
        const h = 10 + (idx * 7) % 12;
        orderHourCounts[h] = (orderHourCounts[h] || 0) + 1;
      });
      const maxCount = Math.max(1, ...Object.values(orderHourCounts));
      timelineEl.innerHTML = `
        <div class="timeline-chart">
          ${hours.map((h, i) => {
            const hh = i + 10;
            const count = orderHourCounts[hh] || 0;
            const barH = Math.round((count / maxCount) * 80) || 4;
            return `<div class="timeline-bar"><div class="bar-fill" style="height:${barH}px"></div><div class="bar-value" style="font-size:0.65em;color:var(--brand-text-muted)">${count}</div><div class="bar-label">${h}</div></div>`;
          }).join('')}
        </div>`;
    }
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
    const labels = { pending:'Очікує', confirmed:'Підтверджено', preparing:'Готується', ready:'Готово', 'in-delivery':'В дорозі', delivered:'Доставлено', cancelled:'Скасовано' };
    const colors = { pending:'#D97706', confirmed:'#2563EB', preparing:'#F59E0B', ready:'#0D9488', 'in-delivery':'#3B82F6', delivered:'#059669', cancelled:'#DC2626' };
    const cancelLabel = { pending: 'Скасувати', confirmed: 'Скасувати' };
    const nextAction = {
      pending: { label: 'Підтвердити', cls: 'btn-primary' },
      confirmed: { label: 'Готувати', cls: 'btn-warning' },
      preparing: { label: 'Готово', cls: 'btn-success' },
      'in-delivery': { label: 'Доставлено', cls: 'btn-primary' },
    };
    el.innerHTML = this.state._orders.map(o => {
      const action = nextAction[o.status];
      return `
      <div class="order-row spring-fast">
        <div style="flex:1"><div class="order-id">#${o.id}</div><div class="order-meta">${o.items} позицій · ${o.total.toLocaleString()} ALL · ${o.time}</div></div>
        <div class="order-status" style="display:flex;align-items:center;gap:8px">
          <span class="status-dot" style="background:${colors[o.status]||'#888'}"></span>
          <span>${labels[o.status]||o.status}</span>
          ${cancelLabel[o.status] ? `<button class="btn btn-sm btn-ghost" onclick="App.cancelOrder(${o.id})" style="color:var(--brand-danger)">${cancelLabel[o.status]}</button>` : ''}
          ${action ? `<button class="btn btn-sm ${action.cls}" onclick="App.advanceOrder(${o.id})">${action.label}</button>` : ''}
        </div>
      </div>`;
    }).join('');
  },

  advanceOrder(id) {
    const order = this.state._orders.find(o => o.id === id);
    if (!order) return;
    const chain = ['pending', 'confirmed', 'preparing', 'ready', 'in-delivery', 'delivered'];
    const idx = chain.indexOf(order.status);
    if (idx < 0 || idx >= chain.length - 1) return;
    order.status = chain[idx + 1];
    this.sonify('advanceOrder');
    const labels = { pending:'Очікує', confirmed:'Підтверджено', preparing:'Готується', ready:'Готово', 'in-delivery':'В дорозі', delivered:'Доставлено' };
    this.toast(`Замовлення #${order.id}: ${labels[order.status]}`, 'success');
    if (order.status === 'ready') {
      this.state._courierTasks.push({
        id: 3000 + order.id,
        orderId: order.id,
        pickup: 'Rruga e Dibrës 45',
        dropoff: 'Bulevardi Zhan D\'Ark ' + (12 + this.state._courierTasks.length * 10),
        status: 'assigned',
        items: order.items,
        payout: Math.round(order.total * 0.12),
      });
    }
    if (order.status === 'in-delivery') {
      const task = this.state._courierTasks.find(t => t.orderId === order.id);
      if (task && task.status === 'assigned') task.status = 'picked-up';
    }
    if (order.status === 'delivered') {
      const task = this.state._courierTasks.find(t => t.orderId === order.id);
      if (task && task.status !== 'delivered') {
        task.status = 'delivered';
        this.state._earningsToday += task.payout;
        this.state._deliveriesToday++;
        this.state._earningsHistory.unshift({ date: new Date().toISOString().slice(0,10), amount: task.payout, trips: 1 });
      }
    }
    this.renderOwnerOrders();
    this.persist();
  },

  cancelOrder(id) {
    const order = this.state._orders.find(o => o.id === id);
    if (!order) return;
    if (order.status !== 'pending' && order.status !== 'confirmed') return;
    order.status = 'cancelled';
    this.sonify('advanceOrder');
    this.toast(`Замовлення #${order.id} скасовано`, 'warning');
    this.renderOwnerOrders();
    this.persist();
  },

  renderOwnerMenu() {
    const el = document.getElementById('owner-menu-grid');
    if (!el) return;
    el.innerHTML = `
      <div style="display:flex;gap:8px;margin-bottom:12px;flex-wrap:wrap">
        <button class="btn btn-sm btn-primary" onclick="App.addMenuItem()">+ Додати страву</button>
        <button class="btn btn-sm btn-ghost" onclick="App.toggleOwnerMenuMode()">${this.state._ownerMenuEdit ? '✅ Готово' : '✏ Редагувати'}</button>
      </div>
      <div style="display:grid;gap:8px;grid-template-columns:repeat(auto-fill,minmax(220px,1fr))">
      ${this.state._menu.map(i => `
        <div class="menu-item spring-fast" style="padding:8px;font-size:0.85em;position:relative">
          ${this.state._ownerMenuEdit ? `
            <div style="position:absolute;top:4px;right:4px;display:flex;gap:4px">
              <button class="btn btn-sm btn-ghost" style="padding:2px 6px;font-size:0.8em;color:var(--brand-text-muted)" onclick="App.editMenuItem(${i.id})" title="Редагувати">✏</button>
              <button class="btn btn-sm btn-ghost" style="padding:2px 6px;font-size:0.8em;color:var(--brand-danger)" onclick="App.toggleMenuItem(${i.id})" title="${i._hidden ? 'Показати' : 'Приховати'}">${i._hidden ? '👁' : '🙈'}</button>
            </div>
          ` : ''}
          <div style="display:flex;justify-content:space-between;align-items:center">
            <strong${i._hidden ? ' style="opacity:0.4;text-decoration:line-through"' : ''}>${i.name}</strong>
            <span>${i.price.toLocaleString()} ALL</span>
          </div>
          <div style="font-size:0.8em;color:var(--brand-text-muted)">${CAT_UA[i.catName]||i.catName||i.cat} · ${i.drink ? 'Напій' : 'Страва'}${i._hidden ? ' · <span style="color:var(--brand-danger)">приховано</span>' : ''}</div>
        </div>
      `).join('')}
      </div>`;
  },

  toggleOwnerMenuMode() {
    this.state._ownerMenuEdit = !this.state._ownerMenuEdit;
    this.renderOwnerMenu();
  },

  toggleMenuItem(id) {
    const item = this.state._menu.find(i => i.id === id);
    if (!item) return;
    item._hidden = !item._hidden;
    this.toast(`${item.name}: ${item._hidden ? 'приховано' : 'показано'}`, 'info');
    this.renderOwnerMenu();
    this.persist();
  },

  editMenuItem(id) {
    const item = this.state._menu.find(i => i.id === id);
    if (!item) return;
    const newPrice = prompt(`Нова ціна для "${item.name}" (поточна: ${item.price} ALL):`, item.price);
    if (newPrice !== null && !isNaN(newPrice) && Number(newPrice) > 0) {
      item.price = Number(newPrice);
      this.toast(`Ціну "${item.name}" змінено на ${item.price} ALL`, 'success');
      this.renderOwnerMenu();
      this.persist();
    }
  },

  addMenuItem() {
    const name = prompt('Назва страви:');
    if (!name) return;
    const price = prompt('Ціна (ALL):');
    if (!price || isNaN(price) || Number(price) <= 0) { this.toast('Некоректна ціна', 'error'); return; }
    const cat = prompt('Категорія (залиште порожньою для "Sets"):') || 'Sets';
    const id = Date.now();
    this.state._menu.push({ id, name, price: Number(price), catName: cat, cat, _hidden: false });
    this.toast(`"${name}" додано до меню`, 'success');
    this.renderOwnerMenu();
    this.persist();
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
        ${t.status === 'assigned' ? `<div style="font-size:0.85em;color:var(--brand-text-muted);margin-top:4px">🕐 ~10 хв до закладу</div>` : ''}
        ${t.status === 'picked-up' ? `<div style="font-size:0.85em;color:var(--brand-text-muted);margin-top:4px">🕐 Доставка ~15-20 хв</div>` : ''}
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
      this.sonify('shiftStart');
    this.toast('Зміна розпочата', 'info');
    } else {
      this.sonify('shiftEnd');
      this.toast('Зміна завершена', 'info');
    }
    this.render();
    this.persist();
  },

  pickupTask(id) {
    const task = this.state._courierTasks.find(t => t.id === id);
    if (!task || task.status !== 'assigned') return;
    task.status = 'picked-up';
    const order = this.state._orders.find(o => o.id === task.orderId);
    if (order && order.status === 'ready') order.status = 'in-delivery';
    this.sonify('advanceOrder');
    this.toast(`Замовлення #${task.orderId} забрано`, 'info');
    this.renderCourier();
    this.renderOwnerOrders?.();
    this.persist();
  },

  deliverTask(id) {
    const task = this.state._courierTasks.find(t => t.id === id);
    if (!task || task.status !== 'picked-up') return;
    task.status = 'delivered';
    const order = this.state._orders.find(o => o.id === task.orderId);
    if (order && order.status !== 'delivered') order.status = 'delivered';
    this.state._earningsToday += task.payout;
    this.state._deliveriesToday++;
    this.state._earningsHistory.unshift({ date: new Date().toISOString().slice(0,10), amount: task.payout, trips: 1 });
    this.sonify('deliver');
    this.toast(`Замовлення #${task.orderId} доставлено · ${task.payout.toLocaleString()} ALL`, 'success', 5000);
    this.render();
    this.persist();
  },

  searchMenu(q) {
    this.state._searchQuery = q;
    this.state.filter = 'all';
    this.renderMenuContent();
    document.querySelectorAll('#cat-filters .btn').forEach(b =>
      b.className = `btn btn-sm ${b.dataset.cat === 'all' ? 'btn-primary' : 'btn-ghost'}`
    );
  },

  cycleTheme() {
    const themes = ['crimson', 'ocean', 'midnight', 'sage', 'gold', 'coral'];
    const idx = themes.indexOf(this.state.theme);
    this.state.theme = themes[(idx + 1) % themes.length];
    document.documentElement.dataset.theme = this.state.theme;
    localStorage.setItem('dowiz-theme', this.state.theme);
    this.toast(`Тема: ${this.state.theme}`, 'info', 1500);
  },

  previewTheme(name) {
    document.documentElement.dataset.theme = name;
  },

  applyTheme(name) {
    this.state.theme = name;
    document.documentElement.dataset.theme = name;
    localStorage.setItem('dowiz-theme', name);
    this.toast(`Тема: ${name}`, 'info', 1500);
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
    this.state._oracle?.mark('setrole');
    this.state._markov?.observe(`role:${role}`);
    const prev = this.state.role;
    if (prev !== role && role !== 'customer') {
      const activeOrders = this.state._orders.filter(o => o.status !== 'delivered' && o.status !== 'cancelled').length;
      if (activeOrders > 0 && !confirm(`У вас ${activeOrders} активних замовлень. Змінити роль?`)) return;
    }
    this.state.role = role;
    if (role === 'customer') this.state.page = 'menu';
    else if (role === 'owner') this.state.page = 'owner';
    else if (role === 'courier') this.state.page = 'courier';
    this.render();
    this.state._journey.reset();
    this.persist();
  },

  // ── §8 Cart ─────────────────────────────────────────────────────
  addToCart(id) {
    const item = this.state._menu.find(i => i.id === id);
    if (!item || item.drink) return;
    const existing = this.state.cart.find(i => i.id === id);
    if (existing) existing.qty++;
    else this.state.cart.push({ ...item, qty: 1 });
    this.sonify('addToCart');
    this.toast(`${item.name} додано до кошика`, 'info', 2000);
    const btn = document.querySelector(`.menu-item .btn[onclick*="addToCart(${id})"]`);
    if (btn) { const orig = btn.textContent; btn.textContent = '✓'; btn.classList.add('btn-success'); setTimeout(() => { btn.textContent = orig; btn.classList.remove('btn-success'); }, 1200); }
    this.renderCart();
    this.persist();
  },

  removeFromCart(id) {
    this.state.cart = this.state.cart.filter(i => i.id !== id);
    this.sonify('removeFromCart');
    this.toast('Видалено з кошика', 'info', 2000);
    this.renderCart();
    this.persist();
  },

  setDelivery(field, value) {
    if (field === 'address') {
      this.state._deliveryAddress = value;
      const el = document.getElementById('delivery-addr');
      if (el) { el.classList.toggle('input-error', value.trim().length > 0 && !this.validateAddress(value)); el.classList.toggle('input-valid', value.trim().length > 0 && this.validateAddress(value)); }
    } else if (field === 'phone') {
      this.state._deliveryPhone = value;
      const el = document.getElementById('delivery-phone');
      if (el) { el.classList.toggle('input-error', value.trim().length > 0 && !this.validatePhone(value)); el.classList.toggle('input-valid', value.trim().length > 0 && this.validatePhone(value)); }
    } else if (field === 'note') this.state._deliveryNote = value;
  },

  changeQty(id, delta) {
    const item = this.state.cart.find(i => i.id === id);
    if (!item) return;
    item.qty += delta;
    if (item.qty <= 0) { this.removeFromCart(id); return; }
    this.sonify('addToCart');
    this.renderCart();
    this.persist();
  },

  renderCart() {
    const count = this.state.cart.reduce((s,i) => s+i.qty, 0);
    const total = this.state.cart.reduce((s,i) => s+i.price*i.qty, 0);
    const el = document.getElementById('cart-items');
    const tEl = document.getElementById('cart-total');
    if (!el) return;
    const deliveryEl = document.getElementById('cart-delivery');
    if (deliveryEl) deliveryEl.style.display = count === 0 ? 'none' : 'block';
    el.innerHTML = count === 0
      ? '<p style="text-align:center;color:var(--brand-text-muted);padding:48px 0">Кошик порожній</p>'
      : this.state.cart.map(i => `
        <div class="cart-item spring-fast">
          <div style="display:flex;align-items:center;gap:8px">
            <button class="btn btn-sm btn-ghost" onclick="App.changeQty('${i.id}', -1)" style="min-width:28px;padding:2px 6px">−</button>
            <span style="min-width:20px;text-align:center">${i.qty}</span>
            <button class="btn btn-sm btn-ghost" onclick="App.changeQty('${i.id}', 1)" style="min-width:28px;padding:2px 6px">+</button>
          </div>
          <div><strong>${i.name}</strong></div>
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
    this.state._oracle?.mark('checkout-start');
    this.state._markov?.observe('checkout');
    if (this.state.cart.length === 0) return;
    if (!this.validateAddress(this.state._deliveryAddress)) {
      document.getElementById('delivery-addr')?.focus();
      document.getElementById('delivery-addr')?.classList.add('input-error');
      this.toast('Вкажіть адресу доставки (не менше 5 символів)', 'error');
      return;
    }
    if (!this.validatePhone(this.state._deliveryPhone)) {
      document.getElementById('delivery-phone')?.focus();
      document.getElementById('delivery-phone')?.classList.add('input-error');
      this.toast('Вкажіть коректний номер телефону', 'error');
      return;
    }
    const btn = document.querySelector('.checkout-btn');
    if (btn) { btn.disabled = true; btn.textContent = 'Відправлення...'; }
    const total = this.state.cart.reduce((s,i) => s+i.price*i.qty, 0);
    const address = this.state._deliveryAddress.trim();
    const phone = this.state._deliveryPhone.trim();
    const note = this.state._deliveryNote.trim();
    const order = {
      items: this.state.cart.map(i => ({ itemId: i.id, name: i.name, price: i.price, qty: i.qty })),
      total, address, phone, note,
    };
    let orderId;
    let apiFailed = false;
    try {
      const resp = await fetch(`${API_BASE}/order`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(order),
      });
      if (resp.ok) {
        const created = await resp.json();
        orderId = created.id;
      } else {
        apiFailed = true;
      }
    } catch {
      apiFailed = true;
    }
    if (apiFailed) {
      if (btn) { btn.disabled = false; btn.textContent = 'Замовити'; }
      this.state._health?.signalCheckout(false, 0, null);
      this.state._telegram?.checkoutEvent(false, 0, null);
      this.toast('Помилка при створенні замовлення. Спробуйте ще раз.', 'error', 5000);
      return;
    }
    this.sonify('checkout');
    this.toast('Замовлення прийнято!', 'success');
    this.state._orders.unshift({ id: orderId, status: 'pending', items: this.state.cart.length, total, time: 'щойно', address, phone, note });
    this.state._lastOrderId = orderId;
    this.state.cart = [];
    this.state._deliveryAddress = '';
    this.state._deliveryPhone = '';
    this.state._deliveryNote = '';
    this.renderCart();
    this.renderOwnerOrders?.();
    this.toggleCart(false);
    this.showOrderConfirm(orderId);
    this.state._journey.advance();
    this.persist();
  },

  showOrderConfirm(orderId) {
    const main = document.getElementById('main-content');
    if (!main) return;
    const order = this.state._orders.find(o => o.id === orderId);
    if (!order) return;
    const labels = { pending:'Очікує підтвердження', confirmed:'Підтверджено', preparing:'Готується', ready:'Готово', 'in-delivery':'В дорозі', delivered:'Доставлено' };
    main.innerHTML = `
      <section style="text-align:center;padding:48px 24px">
        <div style="font-size:3em;margin-bottom:16px">✅</div>
        <h2 class="section-title">Замовлення прийнято</h2>
        <p class="section-subtitle">Номер замовлення: <strong>#${orderId}</strong></p>
        <div class="order-confirm-card" style="max-width:400px;margin:24px auto;background:var(--brand-surface);border-radius:16px;padding:24px">
          <div style="display:flex;justify-content:space-between;margin-bottom:12px">
            <span class="text-muted">Сума:</span>
            <span style="font-weight:700">${order.total.toLocaleString()} ALL</span>
          </div>
          <div style="display:flex;justify-content:space-between;margin-bottom:12px">
            <span class="text-muted">Статус:</span>
            <span class="order-badge pending">${labels[order.status]}</span>
          </div>
          <div style="display:flex;justify-content:space-between;margin-bottom:12px">
            <span class="text-muted">Доставка:</span>
            <span>${order.address}</span>
          </div>
        </div>
        <div style="display:flex;gap:12px;justify-content:center">
          <button class="btn btn-primary" onclick="App.navigate('orders')">Мої замовлення</button>
          <button class="btn btn-ghost" onclick="App.navigate('menu')">Повернутись до меню</button>
        </div>
      </section>`;
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
      else if (e.key === 'k') this.setRole('courier');
      else if (e.key === 't') this.cycleTheme();
      else if (e.key === 'r') location.reload();
    });
  },
};

window.App = App;
document.addEventListener('DOMContentLoaded', () => App.init());
